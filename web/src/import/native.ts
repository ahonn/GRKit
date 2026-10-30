import type { Cover, ImportedEffect, NativeImport } from '../catalog';
import { provideTable } from '../preview/render';
import type { NativeRequest, NativeResponse } from './native.worker';
import { base64 } from './names';

/** grkit-core `gr4_looks::MAX_FILE_SIZE`. An official ZIP has about 35 MB. */
export const MAX_FIRMWARE_BYTES = 96 * 1024 * 1024;
/** The read of an official ZIP takes a few seconds. */
const READ_TIMEOUT = 60_000;
/** Makes each preview table key new, so a later unlock with the same ID gets no cached preview. */
let tableCount = 0;

/** Ricoh's GR IV support page, which shows the firmware with its license. */
export const FIRMWARE_PAGE = 'https://www.ricoh-imaging.co.jp/english/support/digital/gr4_s.html';

/** A cover of a native look. The icon that grkit-wasm draws has the same fields. */
const cinema = (number: string): Cover => ({ template: 'cine', brand: 'GR IV', name: 'CINEMA', number });

/**
 * The native looks that `nativeLooks` reads (the offered looks of grkit-core
 * `gr4_looks::LOOKS`), in its order.
 */
export const NATIVE_LOOKS: readonly { id: string; name: string; cover: Cover }[] = [
  { id: 'gr4-cinema-yellow', name: 'Cinema Yellow', cover: cinema('YELLOW') },
  { id: 'gr4-cinema-green', name: 'Cinema Green', cover: cinema('GREEN') },
];

/**
 * The cameras whose Standard the native looks follow. GR IV has the looks built in, and on it
 * these converted copies would show wrong colors.
 */
const NATIVE_LOOK_CAMERAS: ReadonlySet<string> = new Set(['gr3', 'gr3x']);

/** True when the camera entry `camera` (`Camera.key`) can use native looks. */
export const takesNativeLooks = (camera: string): boolean => NATIVE_LOOK_CAMERAS.has(camera);

/**
 * The imports that the camera entry `camera` can use: every import on GR III and GR IIIx, and
 * no native look on the others. The page keeps the others for a later change of camera.
 */
export function importsForCamera<T extends ImportedEffect>(
  imported: readonly T[],
  camera: string,
): readonly T[] {
  return takesNativeLooks(camera) ? imported : imported.filter((effect) => effect.imported.kind !== 'native');
}

/** The native looks that `imported` lacks, which the list offers to unlock. */
export const missingNativeLooks = (imported: readonly ImportedEffect[]) =>
  NATIVE_LOOKS.filter((look) => !imported.some((effect) => effect.id === look.id));

/** True when `imported` has each native look. */
export const hasNativeLooks = (imported: readonly ImportedEffect[]): boolean =>
  missingNativeLooks(imported).length === 0;

/** Why a firmware file gave no looks, as the dialog explains it. */
export type NativeProblem = 'unsupported' | 'notFirmware' | 'tooLarge' | 'failed';

/** The problem of an error of `unlockNativeLooks`, from the English messages of grkit-core. */
export function nativeProblem(error: unknown): NativeProblem {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes('not a supported GR IV firmware file')) return 'unsupported';
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
 * Reads Cinema Yellow and Cinema Green from an official GR IV ZIP or its firmware file, in a
 * worker, and returns them as effects of the page. The file stays in the browser. `baseGamma` is
 * the catalog's, for the previews.
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
      },
    };
  });
}
