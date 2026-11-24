import type { Cover } from '../catalog';
import {
  colorsFromLook,
  packagingLook,
  type CoverLookId,
  type CoverTemplate,
} from '../coverPalette';
import { EFFECT_ID_MAX, EFFECT_NAME_MAX } from '../generated/site';

/** The input encodings of `grkit effects import --input`, in the order the page offers them. */
export const INPUTS = ['standard', 'srgb', 'linear', 'vlog', 'flog', 'flog2', 'flog2c'] as const;
export type Input = (typeof INPUTS)[number];

/** Names of the log encodings. `standard` and `srgb` have translated descriptions. */
export const INPUT_NAMES: Readonly<Record<Input, string>> = {
  standard: 'Standard',
  srgb: 'sRGB',
  linear: 'Linear',
  vlog: 'V-Log',
  flog: 'F-Log',
  flog2: 'F-Log2',
  flog2c: 'F-Log2 C',
};

/** The file types that the import dialog accepts: LUTs and Lightroom `.xmp` presets. */
const LUT_ACCEPT = '.cube,.png,.tif,.tiff,.xmp';

/**
 * The `accept` of the LUT file input. A touch device gets none: iOS and Android filter the
 * picker by system file types, and `.cube` has none there, so its files cannot be chosen.
 * The import tells the formats apart by content, so any file is safe to pass.
 */
export const lutAccept = (touch: boolean): string | undefined => (touch ? undefined : LUT_ACCEPT);

/** True when the file name is a Lightroom `.xmp` preset (any letter case). */
export function isPreset(fileName: string): boolean {
  return fileName.toLowerCase().endsWith('.xmp');
}

const MAX_NAME = EFFECT_NAME_MAX;
const MAX_ID = EFFECT_ID_MAX;

/**
 * The name of the `.GRF` file of an effect in a package: the first 8 hex digits of
 * the SHA-256 of its ID (`grkit_core::effects::filename`).
 */
export async function effectFilename(id: string): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(id));
  const hex = Array.from(new Uint8Array(digest).slice(0, 4), (b) => b.toString(16).padStart(2, '0')).join('');
  return `${hex.toUpperCase()}.GRF`;
}

/** An effect name has 1 to 16 printable ASCII characters and no edge spaces (`effects.rs`). */
export function validName(name: string): boolean {
  return (
    name.length >= 1 && name.length <= MAX_NAME && name === name.trim() && /^[\x20-\x7e]+$/.test(name)
  );
}

/**
 * A name from a file name: the stem without words such as F-Log or BT.709, with other
 * characters as spaces, cut to the whole words that fit 16 characters. A first word that is
 * longer keeps the hyphen-separated parts that fit, as "colorreps-kodak" of
 * "colorreps-kodak-gold".
 */
export function nameFromFile(file: string): string {
  const words = file
    .replace(/\.[^.]*$/, '')
    .replace(/[^\x21-\x7e]+|[_]+/g, ' ')
    .trim()
    .split(/ +/);
  const kept = words.filter((word) => !TECHNICAL.test(word));
  return fitWords(kept.length > 0 ? kept : words, MAX_NAME, '-') || 'Imported LUT';
}

/**
 * The leading whole words that fit `limit` characters with spaces between them. A first word
 * that is too long keeps its leading parts split at `separator` (joined by it), or else its
 * first `limit` characters.
 */
function fitWords(words: readonly string[], limit: number, separator: string): string {
  const [first = '', ...others] = words.filter(Boolean);
  let text = first;
  if (text.length > limit) {
    const parts = first.split('-').filter(Boolean);
    text = parts[0] ?? '';
    for (const part of parts.slice(1)) {
      if (`${text}${separator}${part}`.length > limit) break;
      text = `${text}${separator}${part}`;
    }
    return text.slice(0, limit).trim();
  }
  for (const word of others) {
    if (`${text} ${word}`.length > limit) break;
    text = `${text} ${word}`;
  }
  return text.trim();
}

/** The start of the ID of each LUT import (`effectId`). No catalog ID starts with it. */
const LUT_PREFIX = 'lut-';

/** True when `id` is the ID of a LUT import, so a package effect with it came from a browser. */
export const isLutImportId = (id: string): boolean => id.startsWith(LUT_PREFIX);

/** The IDs of the native looks (`NATIVE_LOOKS` of `native.ts`, which a worker cannot load). */
export const NATIVE_LOOK_IDS: readonly string[] = [
  'gr4-cinema-yellow',
  'gr4-cinema-green',
  'gr4-mono-standard',
  'gr4-mono-solid',
  'gr4-mono-soft',
  'gr4-mono-high-contrast',
  'gr4-mono-grainy',
  'gr4-mono-solarized-1',
  'gr4-mono-solarized-2',
  'gr4-mono-solarized-3',
  'gr2-positive-film',
  'gr2-positive-film-gr4',
];

/** True for the effect ID of a native look. */
export const isNativeLookId = (id: string): boolean => NATIVE_LOOK_IDS.includes(id);

/** The number of values of tone curves (grkit-core `site::ToneCurves::VALUES`). */
const TONE_VALUES = 512;

/** True for the tone curves of a native look: none (an empty array) or both curves. */
export const isTone = (value: unknown): value is Uint16Array =>
  value instanceof Uint16Array && (value.length === 0 || value.length === TONE_VALUES);

/**
 * A new effect ID for `name`: `lut-` and the lowercase letters and digits of the name, with a
 * number when the ID is taken. The prefix keeps imported IDs apart from the catalog IDs.
 */
export function effectId(name: string, taken: ReadonlySet<string>): string {
  const slug =
    name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '') || 'effect';
  // Room for "-" and two digits.
  const base = `${LUT_PREFIX}${slug}`.slice(0, MAX_ID - 3).replace(/-+$/, '');
  let id = base;
  for (let number = 2; taken.has(id); number += 1) {
    id = `${base}-${number}`;
  }
  return id;
}

/** The fields of an imported effect's icon: a film cover, as on the catalog covers. */
export interface CoverFields {
  template: CoverTemplate;
  /** Cover look from `COVER_LOOKS` — expands to RGB from `COVER_INKS`. */
  look: CoverLookId;
  brand: string;
  name: string;
  /** Empty for no number. The cover then shows the name large. */
  number: string;
  /** A light band at the bottom of the box (`box` only). */
  band: boolean;
}

const MAX_COVER_TEXT = 12;

/** A cover text has 1 to 12 printable ASCII characters (`artwork.rs`). */
export function validCoverText(text: string): boolean {
  return text.length >= 1 && text.length <= MAX_COVER_TEXT && /^[\x20-\x7e]+$/.test(text);
}

/** True when the fields make a valid cover. The number is optional. */
export function validCover({ brand, name, number }: CoverFields): boolean {
  return validCoverText(brand) && validCoverText(name) && (number === '' || validCoverText(number));
}

/** The effect.json `icon` of the cover fields: look RGB from `COVER_INKS`. */
export function coverIcon({ template, look, brand, name, number, band }: CoverFields): Cover {
  const base = {
    template,
    brand,
    name,
    colors: colorsFromLook(look),
  } as const;
  if (template === 'box') {
    return number ? { ...base, number, band } : { ...base, band };
  }
  return number ? { ...base, number } : { ...base };
}

const cut = (text: string) => text.slice(0, MAX_COVER_TEXT).trim();

/** Words of LUT file names that name an encoding, a gamut, or a format, not the look. */
const TECHNICAL =
  /^(f-?log2?c?|v-?log|s-?log\d?|[lndc]-?log\d?|logc\d?|log|f-?gamut|v-?gamut|to|bt\.?709|rec\.?709|709|srgb|displayp3|p3|d(50|55|60|65)|\d+grid|v\.?\d+(\.\d+)*|lut|luts|cube)$/i;

/** Film brands as the catalog covers write them. */
const BRANDS: Readonly<Record<string, string>> = {
  kodak: 'Kodak',
  fuji: 'FUJIFILM',
  fujifilm: 'FUJIFILM',
  ilford: 'ILFORD',
  agfa: 'AGFA',
  polaroid: 'Polaroid',
  rollei: 'Rollei',
  cinestill: 'CineStill',
  lomography: 'Lomography',
};

/**
 * Cover fields from a LUT file name, in the pattern of the catalog covers: "Kodak Portra 400 2"
 * gives Kodak, PORTRA, and 400. The number is the first film speed, such as 400 or 200T, or
 * empty when the name has none. The brand is a known film brand, also as a hyphen-separated
 * part of a word ("colorreps-kodak-gold" gives Kodak and GOLD). Else it is the first other
 * word in capitals, up to a hyphen only when the part before the hyphen is in lowercase, as a
 * publisher prefix such as "wunjo-"; the name is the rest. Words such as F-Log or BT.709 are
 * left out. The user can change each field.
 */
export function coverFromFile(file: string): CoverFields {
  const words = file
    .replace(/\.[^.]*$/, '')
    .split(/[^\x21-\x7e]+|[_\s]+/)
    .filter((word) => word && !TECHNICAL.test(word))
    .flatMap((word) => {
      // A known brand inside a hyphenated word starts its own word, and the parts before it
      // (a publisher) are left out.
      const parts = word.split('-');
      const at = parts.findIndex((part) => BRANDS[part.toLowerCase()]);
      return at < 0 ? [word] : [parts[at]!, ...(parts.length > at + 1 ? [parts.slice(at + 1).join('-')] : [])];
    });
  const number =
    words.find((word) => /^\d{2,}[A-Za-z]?$/.test(word)) ??
    words.find((word) => /^\d+[A-Za-z]?$/.test(word)) ??
    words.find((word) => /\d/.test(word));
  const known = words.find((word) => BRANDS[word.toLowerCase()]);
  const rest = words.filter(
    (word) => word !== number && !/^\d+$/.test(word) && !BRANDS[word.toLowerCase()],
  );
  // An unknown brand is the first other word in capitals, up to a hyphen: the cover draws a
  // brand that is too wide for its outline font in pixel capitals. The part after the hyphen
  // starts the name, so "wunjo-TEAL_ORANGE" gives WUNJO and TEAL ORANGE.
  const [first = '', ...others] = rest;
  const [head = '', ...tail] = first.split('-').filter(Boolean);
  const prefix = tail.length > 0 && head === head.toLowerCase();
  // No known film brand: use CUSTOM (short enough for the 40px icon) and the file words as
  // the film name. A single leftover word is the name, not a second "LUT".
  const [brand, ...name] = known
    ? [BRANDS[known.toLowerCase()]!, ...rest]
    : prefix
      ? [head.toUpperCase(), tail.join('-'), ...others]
      : rest.length >= 2 && tail.length === 0
        ? [head.toUpperCase(), ...others]
        : ['CUSTOM', ...rest];
  // The leading whole words that fit, so a cut never splits a word; a long hyphenated word
  // keeps its parts that fit, with spaces.
  const nameText = cut(fitWords(name, MAX_COVER_TEXT, ' ').toUpperCase()) || 'LOOK';
  const brandText = cut(brand!) || 'CUSTOM';
  const numberText = cut((number ?? '').toUpperCase());
  return {
    template: 'box',
    look: packagingLook(brandText, nameText, 'box', numberText),
    brand: brandText,
    name: nameText,
    // Without a film speed, the cover shows the name large.
    number: numberText,
    band: false,
  };
}

/** Standard base64 with padding, as the catalog gives icons. */
export function base64(bytes: Uint8Array): string {
  let text = '';
  for (let start = 0; start < bytes.length; start += 0x8000) {
    text += String.fromCharCode(...bytes.subarray(start, start + 0x8000));
  }
  return btoa(text);
}
