import init, { effectLabel, effectPreview, guessInput, importLut } from '../wasm/grkit_wasm.js';
import wasmUrl from '../wasm/grkit_wasm_bg.wasm?url';
import { GRADES, type Grade } from '../catalog';
import { INPUTS, isLutImportId, isNativeLookId, type Input } from './names';

export interface ImportRequest {
  kind: 'import';
  bytes: ArrayBuffer;
  source: string;
  input: Input;
  baseGamma: Uint16Array;
  id: string;
  name: string;
  /** The JSON text of the effect.json `icon`. */
  icon: string;
}

export interface ImportResult {
  blob: Uint8Array;
  table: Uint8Array;
  icon: Uint8Array;
  monochrome: boolean;
  grade: Grade;
  warnings: string[];
}

/** Guesses the input that a LUT expects from its data (`fit::guess_input`). */
export interface GuessRequest {
  kind: 'guess';
  bytes: ArrayBuffer;
  baseGamma: Uint16Array;
}

/**
 * Reads the imports back from effect files of a package or a card. `known` gives the IDs that
 * the page has, which the response leaves out.
 */
export interface RestoreRequest {
  kind: 'restore';
  files: RestoreFile[];
  baseGamma: Uint16Array;
  known: readonly string[];
}

/**
 * An effect file, with the tone curves of a native look when an import library keeps them. A
 * native look comes back only with them, so that its preview is the preview of its unlock.
 */
export interface RestoreFile {
  bytes: ArrayBuffer;
  tone?: Uint16Array;
}

/** An import of an effect file. */
export interface RestoredFile {
  id: string;
  name: string;
  blob: Uint8Array;
  table: Uint8Array;
  icon: Uint8Array;
  monochrome: boolean;
}

export type ImportResponse =
  | { kind: 'import'; result: ImportResult }
  /** The new imports, and the number of imports that the page already has. */
  | { kind: 'restore'; effects: RestoredFile[]; kept: number }
  /** The progress of an import, a fraction from 0 to 1, before its result. */
  | { kind: 'progress'; fraction: number }
  | { kind: 'guess'; input: Input }
  | { kind: 'error'; message: string };

/** `value` as one of `values`, which the TypeScript types of grkit-wasm do not give. */
function oneOf<T extends string>(values: readonly T[], value: string): T {
  if (!(values as readonly string[]).includes(value)) {
    throw new Error(`Unexpected value from grkit-wasm: ${value}`);
  }
  return value as T;
}

/** The imports of effect files that `known` does not have. */
function restore({ files, baseGamma, known }: RestoreRequest): Extract<ImportResponse, { kind: 'restore' }> {
  const ids = new Set(known);
  const effects: RestoredFile[] = [];
  let kept = 0;
  for (const { bytes, tone } of files) {
    const blob = new Uint8Array(bytes);
    let label;
    try {
      label = effectLabel(blob);
    } catch {
      // A damaged effect file stays out, and the others still come back.
      continue;
    }
    try {
      // Catalog effects come back from the catalog, and a native look without its tone curves
      // from the firmware file.
      const native = isNativeLookId(label.id) && tone !== undefined;
      if (!isLutImportId(label.id) && !native) continue;
      if (ids.has(label.id)) {
        kept += 1;
        continue;
      }
      ids.add(label.id);
      effects.push({
        id: label.id,
        name: label.name,
        blob,
        table: effectPreview(blob, baseGamma, native ? tone : new Uint16Array()),
        icon: label.icon,
        monochrome: label.monochrome,
      });
    } finally {
      label.free();
    }
  }
  return { kind: 'restore', effects, kept };
}

/**
 * Fits one LUT, guesses its input, or restores the LUT imports of effect files. The page starts a
 * new worker for each request and terminates it after.
 */
self.onmessage = async ({ data }: MessageEvent<ImportRequest | GuessRequest | RestoreRequest>) => {
  try {
    await init({ module_or_path: wasmUrl });
    if (data.kind === 'restore') {
      const response = restore(data);
      self.postMessage(response satisfies ImportResponse, {
        transfer: response.effects.flatMap((effect) => [effect.blob.buffer, effect.table.buffer, effect.icon.buffer]),
      });
      return;
    }
    if (data.kind === 'guess') {
      const input = oneOf(INPUTS, guessInput(new Uint8Array(data.bytes), data.baseGamma));
      self.postMessage({ kind: 'guess', input } satisfies ImportResponse);
      return;
    }
    // One message for each percent: the fit reports about 180 steps.
    let percent = 0;
    const effect = importLut(
      new Uint8Array(data.bytes),
      data.source,
      data.input,
      data.baseGamma,
      data.id,
      data.name,
      data.icon,
      (fraction) => {
        const next = Math.floor(fraction * 100);
        if (next <= percent) return;
        percent = next;
        self.postMessage({ kind: 'progress', fraction } satisfies ImportResponse);
      },
    );
    let result: ImportResult;
    try {
      result = {
        blob: effect.blob,
        table: effect.preview,
        icon: effect.icon,
        monochrome: effect.monochrome,
        grade: oneOf(GRADES, effect.grade),
        warnings: effect.warnings,
      };
    } finally {
      effect.free();
    }
    self.postMessage({ kind: 'import', result } satisfies ImportResponse, {
      transfer: [result.blob.buffer, result.table.buffer, result.icon.buffer],
    });
  } catch (error) {
    self.postMessage({
      kind: 'error',
      message: error instanceof Error ? error.message : 'The LUT cannot be imported',
    } satisfies ImportResponse);
  }
};
