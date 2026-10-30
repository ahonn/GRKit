import type { ImportedEffect, LutImport } from '../catalog';
import { loadWasm, save } from '../package';
import { provideTable } from '../preview/render';
import type { GuessRequest, ImportRequest, ImportResponse } from './lut.worker';
import { base64, coverIcon, effectId, type CoverFields, type Input } from './names';

/** grkit-wasm `MAX_LUT_SIZE`. */
export const MAX_LUT_BYTES = 128 * 1024 * 1024;
/** A level 16 HALD image takes the longest. The fit itself takes a few seconds. */
const IMPORT_TIMEOUT = 180_000;
/** The HALD level that `grkit effects hald` writes by default: a 1728 x 1728 image. */
export const HALD_LEVEL = 12;
/** Makes each preview table key new, so a later import with the same ID gets no cached preview. */
let tableCount = 0;

type WorkerRequest = ImportRequest | GuessRequest;
/** The response to a request of kind `K`. */
type Reply<K extends WorkerRequest['kind']> = Extract<ImportResponse, { kind: K }>;

/** Runs one request in a new worker, which it terminates after. `onProgress` gets the progress
 * messages of an import. */
function run<K extends WorkerRequest['kind']>(
  request: WorkerRequest & { kind: K },
  signal: AbortSignal,
  onProgress?: (fraction: number) => void,
): Promise<Reply<K>> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL('./lut.worker.ts', import.meta.url), { type: 'module' });
    const finish = (response?: ImportResponse, error?: unknown) => {
      clearTimeout(timeout);
      signal.removeEventListener('abort', abort);
      worker.terminate();
      if (response?.kind === 'error') reject(new Error(response.message));
      else if (response?.kind === request.kind) resolve(response as Reply<K>);
      else reject(error ?? new Error('Invalid import response'));
    };
    const abort = () => finish(undefined, signal.reason);
    const timeout = setTimeout(
      () => finish(undefined, new Error('The import timed out')),
      IMPORT_TIMEOUT,
    );
    signal.addEventListener('abort', abort, { once: true });
    worker.onmessage = ({ data }: MessageEvent<ImportResponse>) => {
      if (data.kind === 'progress') onProgress?.(data.fraction);
      else finish(data);
    };
    worker.onerror = () => finish(undefined, new Error('The import failed'));
    worker.onmessageerror = () => finish(undefined, new Error('Invalid import response'));
    worker.postMessage(request, [request.bytes]);
  });
}

/** The bytes of a LUT file within the size limit of grkit-wasm. */
async function readLut(file: File, signal: AbortSignal): Promise<ArrayBuffer> {
  if (file.size > MAX_LUT_BYTES) {
    throw new Error(`The LUT file is larger than ${MAX_LUT_BYTES >> 20} MB`);
  }
  const bytes = await file.arrayBuffer();
  signal.throwIfAborted();
  return bytes;
}

/**
 * The input that the LUT most likely expects, from its data alone, not its file name: Standard
 * or a log encoding (`fit::guess_input`). It reads the whole file in a worker.
 */
export async function guessInput(
  file: File,
  baseGamma: Uint16Array,
  signal: AbortSignal,
): Promise<Input> {
  const bytes = await readLut(file, signal);
  const { input } = await run({ kind: 'guess', bytes, baseGamma }, signal);
  return input;
}

/**
 * Why the camera cannot draw the cover, such as a number that is too wide, or null when it
 * can. The import checks the same rule.
 */
export async function coverProblem(cover: CoverFields): Promise<string | null> {
  const { iconPixels } = await loadWasm();
  try {
    iconPixels(JSON.stringify(coverIcon(cover)));
    return null;
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
}

/**
 * Fits a `.cube` file or a LUT image over the catalog's base gamma, in a worker, and returns it
 * as an effect of the page with the film cover `cover` as its icon. `taken` holds the IDs of the
 * catalog and of earlier imports. `onProgress` gets the progress of the fit, from 0 to 1.
 */
export async function importLut(
  file: File,
  name: string,
  input: Input,
  cover: CoverFields,
  baseGamma: Uint16Array,
  taken: ReadonlySet<string>,
  signal: AbortSignal,
  onProgress?: (fraction: number) => void,
): Promise<ImportedEffect<LutImport>> {
  const bytes = await readLut(file, signal);
  const id = effectId(name, taken);
  const { result } = await run(
    {
      kind: 'import',
      bytes,
      source: file.name,
      input,
      baseGamma,
      id,
      name,
      icon: JSON.stringify(coverIcon(cover)),
    },
    signal,
    onProgress,
  );
  // The preview finds the table under this key instead of fetching a catalog path.
  const table = `imported/${id}/${++tableCount}`;
  provideTable(table, result.table);
  return {
    id,
    name,
    icon: base64(result.icon),
    // The page draws the cover with web fonts, as it draws the catalog covers.
    cover: coverIcon(cover),
    monochrome: result.monochrome,
    preview: table,
    imported: {
      kind: 'lut',
      blob: result.blob,
      table: result.table,
      grade: result.grade,
      input,
      warnings: result.warnings,
      source: file.name,
    },
  };
}

/** Saves an identity HALD image, to turn an editor preset into a LUT image. */
export async function saveHaldImage(): Promise<void> {
  const { haldImage } = await loadWasm();
  const png = haldImage(HALD_LEVEL);
  save(new Blob([png as Uint8Array<ArrayBuffer>], { type: 'image/png' }), `GRKit-HALD-${HALD_LEVEL}.png`);
}
