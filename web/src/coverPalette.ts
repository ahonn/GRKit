/** Cover layout templates. Keep in sync with `artwork.rs`. */
export const COVER_TEMPLATES = ['box', 'cine', 'stack'] as const;
export type CoverTemplate = (typeof COVER_TEMPLATES)[number];

/**
 * Layout choices in the import / edit UI. `box2` is `box` with the bottom accent band; the wire
 * format still uses `template: "box"` and `band: true`.
 */
export const COVER_LAYOUTS = ['box', 'box2', 'cine', 'stack'] as const;
export type CoverLayout = (typeof COVER_LAYOUTS)[number];

/** UI layout from wire template + band. */
export function coverLayout(template: CoverTemplate, band: boolean): CoverLayout {
  return template === 'box' && band ? 'box2' : template === 'box' ? 'box' : template;
}

/** Wire template + band from a UI layout. */
export function layoutFields(layout: CoverLayout): { template: CoverTemplate; band: boolean } {
  if (layout === 'box2') return { template: 'box', band: true };
  if (layout === 'box') return { template: 'box', band: false };
  return { template: layout, band: false };
}

/** One RGB channel triple for the cover JSON / Rust `colors` object. */
export type CoverRgb = readonly [number, number, number];

/** Flat fills: brand band, brand text, body, body text, bottom accent. */
export interface CoverColors {
  band: CoverRgb;
  brand: CoverRgb;
  body: CoverRgb;
  text: CoverRgb;
  accent: CoverRgb;
}

/**
 * Curated inks for film covers. Stock packaging and the import editor only pick from this list,
 * so the shelf stays coherent and LUT imports can reuse the same colors.
 */
export const COVER_INKS = {
  white: [255, 255, 255],
  paper: [248, 244, 240],
  cream: [255, 240, 200],
  warmWhite: [255, 236, 220],
  portraCream: [232, 212, 176],
  portraText: [255, 242, 228],
  goldText: [255, 220, 120],
  silver: [236, 236, 236],
  gray: [214, 214, 214],
  steel: [168, 168, 176],
  mist: [200, 200, 200],
  ash: [64, 64, 64],
  charcoal: [48, 48, 48],
  /** Classic cover brand text (`artwork.rs` `NEUTRAL_PALETTE.brand`). */
  classicBrand: [40, 40, 40],
  /** Classic cover body (`artwork.rs` `NEUTRAL_PALETTE.body`). */
  classicBody: [51, 51, 51],
  graphite: [36, 36, 36],
  black: [20, 20, 20],
  ink: [16, 16, 16],
  void: [12, 12, 12],

  kodakYellow: [242, 196, 24],
  kodakRed: [196, 24, 24],
  ektarRed: [220, 48, 32],
  ricohRed: [220, 32, 40],
  orange: [240, 112, 48],
  agfaOrange: [224, 96, 32],
  vistaOrange: [232, 120, 40],
  amber: [242, 168, 64],

  portraBrown: [96, 56, 40],
  portraBody: [44, 36, 32],
  portraAccent: [212, 168, 120],
  brown: [40, 24, 16],
  warmDark: [40, 32, 28],
  navy: [24, 40, 72],
  purple: [96, 40, 128],
  purpleDeep: [40, 24, 56],
  purpleSoft: [255, 236, 255],
  lilac: [236, 228, 255],
  violet: [120, 72, 168],
  violetBody: [24, 16, 36],

  chromeBlue: [32, 140, 196],
  sky: [200, 232, 255],
  ocean: [20, 36, 56],
  teal: [24, 96, 120],
  blueAccent: [64, 144, 200],

  velviaGreen: [8, 88, 64],
  fujiGreen: [28, 120, 64],
  superiaGreen: [36, 160, 64],
  chartreuse: [180, 200, 48],
  forest: [20, 96, 56],
  pine: [16, 88, 72],
  eternaGreen: [16, 56, 40],
  sage: [120, 168, 136],
  sageDark: [40, 48, 40],
  mint: [160, 200, 184],
  mintText: [220, 255, 240],
  leaf: [220, 255, 236],
  moss: [200, 240, 208],
  cineGreen: [56, 160, 88],
  cineGreenBody: [14, 40, 24],
  cineGreenText: [168, 232, 184],
  cineYellowBody: [40, 28, 8],
  greenDark: [28, 64, 28],
  greenBody: [32, 40, 28],
  eternaBody: [12, 16, 14],
  eternaAccent: [120, 140, 120],

  pink: [255, 120, 160],
  rose: [216, 152, 160],
  blush: [255, 236, 228],
} as const satisfies Record<string, CoverRgb>;

export type CoverInkId = keyof typeof COVER_INKS;

export const COVER_INK_IDS = Object.keys(COVER_INKS) as CoverInkId[];

/** One cover look: each slot is an ink id from `COVER_INKS`. */
export type CoverLookSlots = { readonly [K in keyof CoverColors]: CoverInkId };

/**
 * Named looks built only from `COVER_INKS`. The import editor offers these; stock packaging
 * maps film names onto them.
 */
export const COVER_LOOKS = {
  // Must match `artwork.rs` `NEUTRAL_PALETTE` so omitting `colors` and writing `gray` bake the same pixels.
  gray: { band: 'gray', brand: 'classicBrand', body: 'classicBody', text: 'white', accent: 'gray' },
  kodak: { band: 'kodakYellow', brand: 'kodakRed', body: 'graphite', text: 'white', accent: 'kodakYellow' },
  gold: { band: 'kodakYellow', brand: 'kodakRed', body: 'navy', text: 'goldText', accent: 'kodakYellow' },
  ultramax: { band: 'purple', brand: 'purpleSoft', body: 'purpleDeep', text: 'purpleSoft', accent: 'kodakYellow' },
  ektar: { band: 'ektarRed', brand: 'warmWhite', body: 'black', text: 'warmWhite', accent: 'orange' },
  ektachrome: { band: 'chromeBlue', brand: 'white', text: 'sky', body: 'ocean', accent: 'kodakYellow' },
  kodachrome: { band: 'kodakRed', brand: 'warmWhite', body: 'black', text: 'warmWhite', accent: 'kodakYellow' },
  aerocolor: { band: 'charcoal', brand: 'kodakYellow', body: 'black', text: 'white', accent: 'kodakRed' },
  portra: {
    band: 'portraCream',
    brand: 'portraBrown',
    body: 'portraBody',
    text: 'portraText',
    accent: 'portraAccent',
  },
  trix: { band: 'black', brand: 'kodakYellow', body: 'ink', text: 'white', accent: 'kodakRed' },
  tmax: { band: 'steel', brand: 'ink', body: 'charcoal', text: 'silver', accent: 'kodakYellow' },
  doublex: { band: 'black', brand: 'kodakYellow', body: 'void', text: 'white', accent: 'mist' },
  vision: { band: 'charcoal', brand: 'kodakYellow', body: 'ink', text: 'white', accent: 'kodakYellow' },
  velvia: { band: 'velviaGreen', brand: 'cream', body: 'ocean', text: 'leaf', accent: 'ektarRed' },
  provia: { band: 'teal', brand: 'white', body: 'ocean', text: 'sky', accent: 'blueAccent' },
  astia: { band: 'sage', brand: 'sageDark', body: 'portraBody', text: 'blush', accent: 'rose' },
  superia: { band: 'superiaGreen', brand: 'white', body: 'greenBody', text: 'cream', accent: 'kodakYellow' },
  c200: { band: 'chartreuse', brand: 'greenDark', body: 'greenBody', text: 'cream', accent: 'fujiGreen' },
  pro: { band: 'forest', brand: 'white', body: 'black', text: 'mintText', accent: 'mint' },
  nps: { band: 'pine', brand: 'mintText', body: 'graphite', text: 'mintText', accent: 'mint' },
  acros: { band: 'ink', brand: 'white', body: 'charcoal', text: 'silver', accent: 'fujiGreen' },
  instax: { band: 'fujiGreen', brand: 'white', body: 'white', text: 'graphite', accent: 'pink' },
  eterna: { band: 'eternaGreen', brand: 'moss', body: 'eternaBody', text: 'moss', accent: 'eternaAccent' },
  hp5: { band: 'ink', brand: 'white', body: 'black', text: 'white', accent: 'kodakYellow' },
  delta: { band: 'silver', brand: 'ink', body: 'black', text: 'white', accent: 'kodakYellow' },
  xp2: { band: 'violet', brand: 'white', body: 'violetBody', text: 'lilac', accent: 'kodakYellow' },
  precisa: { band: 'agfaOrange', brand: 'white', body: 'warmDark', text: 'warmWhite', accent: 'ektarRed' },
  vista: { band: 'vistaOrange', brand: 'brown', body: 'warmDark', text: 'portraText', accent: 'amber' },
  cinema: { band: 'ash', brand: 'mist', body: 'ink', text: 'white', accent: 'steel' },
  cinemaYellow: {
    band: 'ricohRed',
    brand: 'white',
    body: 'cineYellowBody',
    text: 'goldText',
    accent: 'kodakYellow',
  },
  cinemaGreen: {
    band: 'ricohRed',
    brand: 'white',
    body: 'cineGreenBody',
    text: 'cineGreenText',
    accent: 'cineGreen',
  },
  ricoh: { band: 'ricohRed', brand: 'white', body: 'ink', text: 'white', accent: 'mist' },
  fuji: { band: 'fujiGreen', brand: 'white', body: 'graphite', text: 'white', accent: 'fujiGreen' },
  ilford: { band: 'ink', brand: 'white', body: 'black', text: 'white', accent: 'kodakYellow' },
  agfa: { band: 'agfaOrange', brand: 'white', body: 'warmDark', text: 'warmWhite', accent: 'ektarRed' },
} as const satisfies Record<string, CoverLookSlots>;

export type CoverLookId = keyof typeof COVER_LOOKS;

/**
 * Looks offered in the import / edit cover UI. A short brand-level set; stock packaging may
 * still use the fuller `COVER_LOOKS` table.
 */
export const COVER_LOOK_IDS = [
  'gray',
  'kodak',
  'gold',
  'portra',
  'ektar',
  'ektachrome',
  'trix',
  'velvia',
  'fuji',
  'instax',
  'ilford',
  'xp2',
  'precisa',
  'cinemaYellow',
  'cinemaGreen',
  'cinema',
] as const satisfies readonly CoverLookId[];

export type CoverEditorLookId = (typeof COVER_LOOK_IDS)[number];

export function isCoverLookId(value: unknown): value is CoverLookId {
  return typeof value === 'string' && value in COVER_LOOKS;
}

export function isCoverEditorLookId(value: unknown): value is CoverEditorLookId {
  return typeof value === 'string' && (COVER_LOOK_IDS as readonly string[]).includes(value);
}

/** Expand a look into RGB for the cover JSON / SVG. */
export function colorsFromLook(look: CoverLookId): CoverColors {
  const slots = COVER_LOOKS[look];
  return {
    band: COVER_INKS[slots.band],
    brand: COVER_INKS[slots.brand],
    body: COVER_INKS[slots.body],
    text: COVER_INKS[slots.text],
    accent: COVER_INKS[slots.accent],
  };
}

/** Classic gray box when the stock is unknown. */
export const NEUTRAL_COLORS: CoverColors = colorsFromLook('gray');

const STOCK_LOOKS: ReadonlyArray<{ readonly match: RegExp; readonly look: CoverLookId }> = [
  { match: /PORTRA/, look: 'portra' },
  { match: /GOLD/, look: 'gold' },
  { match: /ULTRAMAX/, look: 'ultramax' },
  { match: /EKTAR/, look: 'ektar' },
  { match: /EKTACHROME/, look: 'ektachrome' },
  { match: /KODACHROME/, look: 'kodachrome' },
  { match: /AEROCOLOR/, look: 'aerocolor' },
  { match: /TRI-?X/, look: 'trix' },
  { match: /T-?MAX/, look: 'tmax' },
  { match: /DOUBLE-?X|5222/, look: 'doublex' },
  { match: /VISION|5207|5219/, look: 'vision' },
  { match: /VELVIA/, look: 'velvia' },
  { match: /PROVIA/, look: 'provia' },
  { match: /ASTIA/, look: 'astia' },
  { match: /SUPERIA/, look: 'superia' },
  { match: /FUJICOLOR|C200/, look: 'c200' },
  { match: /160C/, look: 'nps' },
  { match: /\bPRO\b|400H/, look: 'pro' },
  { match: /ACROS|NEOPAN/, look: 'acros' },
  { match: /INSTAX/, look: 'instax' },
  { match: /ETERNA|8573/, look: 'eterna' },
  { match: /HP5/, look: 'hp5' },
  { match: /DELTA/, look: 'delta' },
  { match: /XP2/, look: 'xp2' },
  { match: /PRECISA/, look: 'precisa' },
  { match: /VISTA/, look: 'vista' },
  { match: /CINEMA YELLOW|YELLOW RICOH/, look: 'cinemaYellow' },
  { match: /CINEMA GREEN|GREEN RICOH/, look: 'cinemaGreen' },
  { match: /CINEMA|RICOH/, look: 'ricoh' },
];

const BRAND_LOOKS: ReadonlyArray<{ readonly match: RegExp; readonly look: CoverLookId }> = [
  { match: /kodak|eastman/i, look: 'kodak' },
  { match: /fuji/i, look: 'fuji' },
  { match: /ilford|harman/i, look: 'ilford' },
  { match: /agfa/i, look: 'agfa' },
];

export function isCoverTemplate(value: unknown): value is CoverTemplate {
  return typeof value === 'string' && (COVER_TEMPLATES as readonly string[]).includes(value);
}

export function isCoverRgb(value: unknown): value is CoverRgb {
  return (
    Array.isArray(value) &&
    value.length === 3 &&
    value.every((n) => typeof n === 'number' && Number.isInteger(n) && n >= 0 && n <= 255)
  );
}

export function isCoverColors(value: unknown): value is CoverColors {
  if (typeof value !== 'object' || value === null) return false;
  const colors = value as Record<string, unknown>;
  return (['band', 'brand', 'body', 'text', 'accent'] as const).every((key) => isCoverRgb(colors[key]));
}

/** CSS `rgb(r g b)` for SVG fills. */
export function rgbCss(rgb: CoverRgb): string {
  return `rgb(${rgb[0]} ${rgb[1]} ${rgb[2]})`;
}

const colorsKey = (colors: CoverColors) =>
  [colors.band, colors.brand, colors.body, colors.text, colors.accent].map((rgb) => rgb.join(',')).join('|');

/** Match wire colors back to a look id when editing, else null. */
export function lookFromColors(colors: CoverColors): CoverLookId | null {
  const want = colorsKey(colors);
  for (const id of Object.keys(COVER_LOOKS) as CoverLookId[]) {
    if (colorsKey(colorsFromLook(id)) === want) return id;
  }
  return null;
}

/** Look for a cover from brand + film name / number, else brand, else gray / cinema. */
export function packagingLook(
  brand: string,
  name: string,
  template: CoverTemplate = 'box',
  number = '',
): CoverLookId {
  const label = `${name} ${number} ${brand}`.toUpperCase();
  for (const stock of STOCK_LOOKS) {
    if (stock.match.test(label)) return stock.look;
  }
  if (template === 'cine') return 'cinema';
  for (const entry of BRAND_LOOKS) {
    if (entry.match.test(brand)) return entry.look;
  }
  return 'gray';
}

/** Packaging colors for a cover — always a look expanded from `COVER_INKS`. */
export function packagingColors(
  brand: string,
  name: string,
  template: CoverTemplate = 'box',
  number = '',
): CoverColors {
  return colorsFromLook(packagingLook(brand, name, template, number));
}

/**
 * Resolve RGB for drawing: stored `colors`, else packaging hues from brand + film name.
 * When the shelf color-covers switch is off, pass `colorful={false}` so covers without stored
 * `colors` stay classic gray; covers that already store `colors` keep those hues either way.
 */
export function resolveColors(cover: {
  brand: string;
  name: string;
  number?: string;
  template: CoverTemplate;
  colors?: CoverColors;
}): CoverColors {
  return cover.colors ?? packagingColors(cover.brand, cover.name, cover.template, cover.number ?? '');
}

/** Shelf / preview fills: stored colors always win; otherwise packaging when colorful, else gray. */
export function displayColors(
  cover: {
    brand: string;
    name: string;
    number?: string;
    template: CoverTemplate;
    colors?: CoverColors;
  },
  colorful: boolean,
): CoverColors {
  if (cover.colors || colorful) return resolveColors(cover);
  return NEUTRAL_COLORS;
}

const COLORFUL_COVERS_KEY = 'grkit.colorfulCovers';

/**
 * Whether the film shelf draws packaging colors. Default off so older GRFs (no stored `colors`)
 * stay gray until the user turns the switch on; preference is persisted in localStorage.
 */
export function readColorfulCovers(): boolean {
  try {
    return localStorage.getItem(COLORFUL_COVERS_KEY) === '1';
  } catch {
    return false;
  }
}

export function writeColorfulCovers(on: boolean): void {
  try {
    localStorage.setItem(COLORFUL_COVERS_KEY, on ? '1' : '0');
  } catch {
    // Preference only; without storage the page asks again next visit.
  }
}

/**
 * Default look for a LUT import. Color covers on: a random editor look. Off: the first look
 * (`gray`), matching the neutral shelf tiles.
 */
export function importCoverLook(colorful = readColorfulCovers()): CoverEditorLookId {
  if (!colorful) return COVER_LOOK_IDS[0];
  return randomCoverLook();
}

function pickRandom<T>(items: readonly T[], except?: unknown): T {
  const others = except === undefined ? items : items.filter((item) => item !== except);
  const pool = others.length > 0 ? others : items;
  return pool[Math.floor(Math.random() * pool.length)]!;
}

/** A random editor look, preferring one other than `except` when there is a choice. */
export function randomCoverLook(except?: CoverLookId): CoverEditorLookId {
  return pickRandom(COVER_LOOK_IDS, except);
}

/** A random cover layout (including box with band), preferring one other than `except`. */
export function randomCoverLayout(except?: CoverLayout): CoverLayout {
  return pickRandom(COVER_LAYOUTS, except);
}
