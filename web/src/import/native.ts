import type { ImportedEffect, NativeImport } from '../catalog';
import { provideTable } from '../preview/render';
import type { NativeRequest, NativeResponse } from './native.worker';
import { base64 } from './names';

/** grkit-core `gr4_looks::MAX_FILE_SIZE`. An official GR IV ZIP has about 35 MB, a GR II ZIP less. */
export const MAX_FIRMWARE_BYTES = 96 * 1024 * 1024;
/** The read of an official ZIP takes a few seconds. */
const READ_TIMEOUT = 60_000;
/** Makes each preview table key new, so a later unlock with the same ID gets no cached preview. */
let tableCount = 0;

import { NATIVE_LOOKS, type NativeFirmware } from '../packageCovers';
import { isReleased } from '../release';

export { NATIVE_LOOKS, type NativeFirmware };

type Look = (typeof NATIVE_LOOKS)[number];

/** Ricoh's support page of each camera, which shows the firmware with its license. */
export const FIRMWARE_PAGES: Readonly<Record<NativeFirmware, string>> = {
  gr4: 'https://www.ricoh-imaging.co.jp/english/support/digital/gr4_s.html',
  gr2: 'https://www.ricoh-imaging.co.jp/english/support/digital/gr2_s.html',
};

/** The native looks that the camera entry `camera` (`Camera.key`) can use in release `version`. */
export const nativeLooksFor = (camera: string, version: string = __GRKIT_VERSION__) =>
  NATIVE_LOOKS.filter(
    (look) => look.cameras.includes(camera) && (!look.since || isReleased(look.since, version)),
  );

/**
 * The imports that the camera entry `camera` can use: each LUT import, and the native looks of
 * `nativeLooksFor`. The page keeps the others for a later change of camera.
 */
export function importsForCamera<T extends ImportedEffect>(
  imported: readonly T[],
  camera: string,
  version: string = __GRKIT_VERSION__,
): readonly T[] {
  const looks = nativeLooksFor(camera, version);
  return imported.filter(
    (effect) => effect.imported.kind !== 'native' || looks.some((look) => look.id === effect.id),
  );
}

/**
 * `selection` for the camera entry `camera`. A checked native look that the camera cannot use
 * gives way to its copy for this camera: GR II Positive has one effect for GR III and GR IIIx and
 * one for GR IV, and a change of camera keeps it checked. Returns `selection` when nothing changes.
 */
export function selectionForCamera(
  selection: ReadonlySet<string>,
  camera: string,
  version: string = __GRKIT_VERSION__,
): ReadonlySet<string> {
  const looks = nativeLooksFor(camera, version);
  const next = new Set<string>();
  let changed = false;
  for (const id of selection) {
    const look = NATIVE_LOOKS.find((item) => item.id === id);
    const copy =
      look && !looks.includes(look)
        ? looks.find((item) => item.firmware === look.firmware && item.name === look.name)
        : undefined;
    if (copy) changed = true;
    next.add(copy?.id ?? id);
  }
  return changed ? next : selection;
}

/**
 * The looks of `looks` that the page lacks, which the list offers to unlock. `owned` is imports
 * and card-only effects: a native look already on the SD card must not also show as an unlock tile.
 */
export const missingNativeLooks = (
  owned: readonly { readonly id: string }[],
  looks: readonly Look[] = NATIVE_LOOKS,
) => looks.filter((look) => !owned.some((effect) => effect.id === look.id));

/** True when `imported` has each native look. */
export const hasNativeLooks = (imported: readonly ImportedEffect[]): boolean =>
  missingNativeLooks(imported).length === 0;

/** Why a firmware file gave no looks, as the dialog explains it. */
export type NativeProblem = 'unsupported' | 'notFirmware' | 'tooLarge' | 'failed';

/** The problem of an error of `unlockNativeLooks`, from the English messages of grkit-core. */
export function nativeProblem(error: unknown): NativeProblem {
  const message = error instanceof Error ? error.message : String(error);
  if (/not a supported GR (II|IV) firmware file/.test(message)) return 'unsupported';
  if (message.includes('The ZIP file has no') || message.includes('Invalid ZIP file')) return 'notFirmware';
  if (message.includes('too large')) return 'tooLarge';
  return 'failed';
}

/** Reads one file in a new worker, which it terminates after. */
function run(request: NativeRequest, signal: AbortSignal): Promise<NativeResponse & { kind: 'looks' }> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL('./native.worker.ts', import.meta.url), { type: 'module' });
    const finish = (response?: NativeResponse, error?: unknown) => {
      clearTimeout(timeout);
      signal.removeEventListener('abort', abort);
      worker.terminate();
      if (response?.kind === 'error') reject(new Error(response.message));
      else if (response?.kind === 'looks') resolve(response);
      else reject(error ?? new Error('Invalid firmware response'));
    };
    const abort = () => finish(undefined, signal.reason);
    const timeout = setTimeout(
      () => finish(undefined, new Error('The firmware read timed out')),
      READ_TIMEOUT,
    );
    signal.addEventListener('abort', abort, { once: true });
    worker.onmessage = ({ data }: MessageEvent<NativeResponse>) => finish(data);
    worker.onerror = () => finish(undefined, new Error('The firmware read failed'));
    worker.onmessageerror = () => finish(undefined, new Error('Invalid firmware response'));
    worker.postMessage(request, [request.bytes]);
  });
}

/**
 * Reads the native looks of an official firmware ZIP or its firmware file (GR IV: the Cinema and
 * Monochrome looks; GR II: GR II Positive), in a worker, and returns them as effects of the page.
 * The file stays in the browser. `baseGamma` is the catalog's, for the previews.
 */
export async function unlockNativeLooks(
  file: File,
  baseGamma: Uint16Array,
  signal: AbortSignal,
): Promise<ImportedEffect<NativeImport>[]> {
  if (file.size > MAX_FIRMWARE_BYTES) throw new Error('The file is too large');
  const bytes = await file.arrayBuffer();
  signal.throwIfAborted();
  const { looks } = await run({ bytes, baseGamma }, signal);
  return looks.map((look) => {
    const known = NATIVE_LOOKS.find((item) => item.id === look.id);
    if (!known) throw new Error(`Unexpected native look: ${look.id}`);
    // The preview finds the table under this key instead of fetching a catalog path.
    const table = `native/${look.id}/${++tableCount}`;
    provideTable(table, look.table);
    return {
      id: look.id,
      name: look.name,
      icon: base64(look.icon),
      // The page draws the cover with web fonts, as it draws the catalog covers.
      cover: known.cover,
      monochrome: look.monochrome,
      preview: table,
      imported: {
        kind: 'native',
        blob: look.blob,
        table: look.table,
        version: look.version,
        source: file.name,
        tone: look.tone,
      },
    };
  });
}
