import { GRADES, type Cover, type Grade, type ImportedEffect, type RestoredImport } from '../catalog';
import { isCoverColors } from '../coverPalette';
import { loadWasm } from '../package';
import { INPUTS, isTone, validName, type Input } from './names';

// The import library keeps each import of a browser in its packages (grkit-core `imports`), also
// the imports that the package does not install. The camera does not read it. Another browser
// restores the imports from it, with the details that the page shows of each.

/** The import library in a package and on a card (grkit-core `card::IMPORTS_FILE`). */
export const IMPORTS_FILE = 'SCRIPT/GRKIT/IMPORTS.BIN';

/** What the library keeps of an import besides its effect file, which holds its name and icon. */
export interface LibraryDetails {
  /**
   * `native` for a native look, `cover` for the customization of a site film look, absent for a
   * LUT import. A `cover` entry holds the site effect with its custom name and cover, and only the
   * fields that the user changed; pages before custom covers skip it as an effect that the
   * catalog has.
   */
  kind?: 'native' | 'cover';
  cover?: Cover;
  /** The custom name of a site film look (`kind: 'cover'`). */
  name?: string;
  grade?: Grade;
  input?: Input;
  warnings?: readonly string[];
  source?: string;
  /** The GR IV version of the firmware file of a native look. */
  version?: string;
  /** The tone curves of a native look, which redraw its preview. */
  tone?: Uint16Array;
}

/** The customization of a site film look, and its effect file with it. */
export interface LibraryCover {
  cover?: Cover;
  name?: string;
  effect: Uint8Array;
}

/** An import of a library: its effect file and its details. */
export interface LibraryEntry {
  effect: Uint8Array;
  details: LibraryDetails;
}

/** Tone curves as base64 text of their little-endian bytes, as the JSON details keep them. */
function toneText(tone: Uint16Array): string {
  const bytes = new Uint8Array(tone.length * 2);
  const view = new DataView(bytes.buffer);
  tone.forEach((value, i) => view.setUint16(i * 2, value, true));
  let text = '';
  for (const byte of bytes) text += String.fromCharCode(byte);
  return btoa(text);
}

function readTone(value: unknown): Uint16Array | undefined {
  if (typeof value !== 'string') return undefined;
  let text;
  try {
    text = atob(value);
  } catch {
    return undefined;
  }
  if (text.length % 2 !== 0) return undefined;
  const tone = new Uint16Array(text.length / 2);
  for (let i = 0; i < tone.length; i++) tone[i] = text.charCodeAt(i * 2) | (text.charCodeAt(i * 2 + 1) << 8);
  return isTone(tone) ? tone : undefined;
}

/** The JSON details of `effect`, or null for an import that the library cannot keep. */
function detailsText(effect: ImportedEffect): string | null {
  const { imported } = effect;
  switch (imported.kind) {
    case 'lut':
      return JSON.stringify({
        cover: effect.cover,
        grade: imported.grade,
        input: imported.input,
        warnings: imported.warnings,
        source: imported.source,
      });
    case 'restored':
      return JSON.stringify({ cover: effect.cover, source: imported.source });
    case 'native':
      // A look unlocked before the page kept its tone curves cannot redraw its preview.
      return imported.tone
        ? JSON.stringify({
            kind: 'native',
            cover: effect.cover,
            version: imported.version,
            source: imported.source,
            tone: toneText(imported.tone),
          })
        : null;
  }
}

/**
 * The import library of `imports` and `covers`, or null without an entry that it can keep. A
 * native look goes in with the tone curves of its preview, so another browser shows it as after
 * its unlock. Each customization of a site film look goes in with the effect file that has it.
 */
export async function encodeLibrary(
  imports: readonly ImportedEffect[],
  covers: readonly LibraryCover[] = [],
): Promise<Uint8Array | null> {
  const kept = [
    ...imports.flatMap((effect) => {
      const details = detailsText(effect);
      return details ? [{ details, blob: effect.imported.blob }] : [];
    }),
    ...covers.map(({ cover, name, effect }) => ({
      details: JSON.stringify({ kind: 'cover', cover, name }),
      blob: effect,
    })),
  ];
  if (kept.length === 0) return null;
  const { ImportLibrary } = await loadWasm();
  const library = new ImportLibrary();
  try {
    for (const { details, blob } of kept) library.add(details, blob);
    return library.bytes();
  } finally {
    library.free();
  }
}

const isText = (value: unknown, max: number): value is string =>
  typeof value === 'string' && value.length >= 1 && value.length <= max;

/** `value` as a cover that the camera can draw, or undefined. */
export function readCover(value: unknown): Cover | undefined {
  const cover = value as Partial<Cover> | null;
  if (
    typeof cover !== 'object' ||
    cover === null ||
    (cover.template !== 'box' && cover.template !== 'cine' && cover.template !== 'stack') ||
    !isText(cover.brand, 12) ||
    !isText(cover.name, 12) ||
    (cover.number !== undefined && !isText(cover.number, 12)) ||
    (cover.band !== undefined && typeof cover.band !== 'boolean') ||
    (cover.colors !== undefined && !isCoverColors(cover.colors))
  ) {
    return undefined;
  }
  return {
    template: cover.template,
    brand: cover.brand,
    name: cover.name,
    ...(cover.number !== undefined && { number: cover.number }),
    ...(cover.band !== undefined && { band: cover.band }),
    ...(cover.colors !== undefined && { colors: cover.colors }),
  };
}

/** The details of the JSON text `text`, keeping only the values that this page can use. */
export function readDetails(text: string): LibraryDetails {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return {};
  }
  if (typeof value !== 'object' || value === null) return {};
  const item = value as Record<string, unknown>;
  const details: LibraryDetails = {};
  const cover = readCover(item.cover);
  if (cover) details.cover = cover;
  if (GRADES.includes(item.grade as Grade)) details.grade = item.grade as Grade;
  if (INPUTS.includes(item.input as Input)) details.input = item.input as Input;
  if (Array.isArray(item.warnings) && item.warnings.every((warning) => typeof warning === 'string')) {
    details.warnings = item.warnings as string[];
  }
  if (isText(item.source, 255)) details.source = item.source;
  if (item.kind === 'native' || item.kind === 'cover') details.kind = item.kind;
  if (isText(item.version, 16)) details.version = item.version;
  if (details.kind === 'cover' && typeof item.name === 'string' && validName(item.name)) details.name = item.name;
  const tone = readTone(item.tone);
  if (tone) details.tone = tone;
  return details;
}

/** The imports of an import library file. It fails for a file that is not a library. */
export async function readLibrary(bytes: Uint8Array): Promise<LibraryEntry[]> {
  const { readImportLibrary } = await loadWasm();
  return readImportLibrary(bytes).map((item) => {
    try {
      return { effect: item.effect, details: readDetails(item.details) };
    } finally {
      item.free();
    }
  });
}

/**
 * The tone curves of `details` when they hold all that a native look needs. Only then does the
 * page restore the look, as `withDetails` makes it.
 */
export function nativeTone(details: LibraryDetails | undefined): Uint16Array | undefined {
  return details?.kind === 'native' && details.cover && details.version && details.source
    ? details.tone
    : undefined;
}

/**
 * `restored` with the details of the library: a native look or a LUT import again when the
 * library has all of its details, or else a restored import with the cover that the library has.
 */
export function withDetails(
  restored: ImportedEffect<RestoredImport>,
  details: LibraryDetails | undefined,
): ImportedEffect {
  if (!details) return restored;
  const { cover, grade, input, warnings, source, version, tone } = details;
  if (details.kind === 'native') {
    if (!nativeTone(details) || !cover || !version || !source || !tone) return restored;
    return {
      ...restored,
      cover,
      imported: { kind: 'native', blob: restored.imported.blob, table: restored.imported.table, version, source, tone },
    };
  }
  if (cover && grade && input && warnings && source) {
    return {
      ...restored,
      cover,
      imported: {
        kind: 'lut',
        blob: restored.imported.blob,
        table: restored.imported.table,
        grade,
        input,
        warnings,
        source,
        hints: '',
      },
    };
  }
  return cover ? { ...restored, cover } : restored;
}
