import type { Language } from './i18n';
import { RECIPE_STRINGS } from './recipeStrings';
import {
  effectCode,
  nativeLook,
  NEUTRAL,
  newRecipe,
  NO_SHIFT,
  type Family,
  type LibraryRecipe,
  type Model,
  type ModelType,
  type NativeLook,
  type Recipe,
} from './recipes';

/**
 * The text form of a recipe: the import of a recipe that a person pastes, such as a community
 * recipe, and the export that the import reads back. The import reads the labels of the camera
 * menus and of common recipe posts in English, Chinese, Japanese, and Korean, and the shift
 * formats "A:7 G:3", "G3B3", and "+B5". It never guesses a value: each value that it cannot read,
 * or that the camera line does not have, becomes a flag, and the person corrects it in the
 * editor. The text stays in the browser.
 */

/** A value of the text that the recipe does not hold as written. */
export interface Flag {
  /** The field: a parameter key, or a field of the recipe. */
  field: string;
  /** The text of the value, or of the line. */
  text: string;
}

export interface Parsed {
  recipe: Recipe;
  author: string;
  source: string;
  flags: Flag[];
}

/** An effect that a recipe can use: its ID and name. */
export interface EffectName {
  id: string;
  name: string;
}

/** Folds case, width, and spaces, so that labels compare as written in any of the forms. */
export function fold(text: string): string {
  return text
    .normalize('NFKC')
    .toLowerCase()
    .replace(/[−–—]/g, '-')
    .replace(/[\s·・_]/g, '')
    .replace(/[()（）[\]【】「」]/g, '');
}

const LEVEL_KEYS = [
  'Saturation',
  'ColorHue',
  'ImageKey',
  'Contrast',
  'ContrastHighLight',
  'ContrastShadow',
  'Sharpness',
  'Shading',
  'ClarityControl',
] as const;

/** The labels of each field beyond the ones of the site strings, folded on use. */
const ALIASES: Readonly<Record<string, readonly string[]>> = {
  name: ['name', 'recipe', 'title', '名称', '名稱', '配方', '配方名', 'レシピ', 'レシピ名', '名前', '이름', '레시피'],
  author: ['author', 'by', 'creator', '作者', '作者名', '作成者', '작성자'],
  source: ['source', 'link', 'url', '来源', '來源', '链接', '連結', '出典', 'リンク', '출처'],
  camera: ['camera', 'model', 'body', '相机', '相機', '机型', '機種', 'カメラ', '카메라'],
  look: [
    'imagecontrol',
    'image',
    'base',
    'filmsimulation',
    'filmlook',
    'effect',
    'look',
    'style',
    '图像控制',
    '影像控制',
    '图像',
    '影像',
    '风格',
    '風格',
    '基础',
    '基調',
    'イメージコントロール',
    'ベース',
    '仕上がり',
    'イメコン',
    '이미지컨트롤',
    '룩',
  ],
  Saturation: ['saturation', 'sat', '饱和度', '飽和度', '彩度', '채도'],
  ColorHue: ['hue', 'colorhue', '色相', '색상'],
  ImageKey: ['key', 'highlowkey', 'high/lowkey', 'highkey/lowkey', 'imagekey', '高调低调', '高調低調', '高调/低调', '高調/低調', '明暗', 'キー', 'ハイ/ローキー', '하이/로우키'],
  Contrast: ['contrast', '对比度', '對比度', '反差', 'コントラスト', '콘트라스트'],
  ContrastHighLight: [
    'contrasthighlight',
    'highlightcontrast',
    'contrast-highlight',
    '高光对比度',
    '高光對比度',
    '对比度高光',
    '對比度高光',
    'コントラストハイライト',
    'ハイライトコントラスト',
    '콘트라스트하이라이트',
  ],
  ContrastShadow: [
    'contrastshadow',
    'shadowcontrast',
    'contrast-shadow',
    '阴影对比度',
    '陰影對比度',
    '对比度阴影',
    '對比度陰影',
    'コントラストシャドー',
    'シャドーコントラスト',
    '콘트라스트섀도',
  ],
  Sharpness: ['sharpness', 'sharp', '锐度', '銳度', '锐利度', '銳利度', 'シャープネス', '선명도'],
  Shading: ['shading', 'vignette', 'vignetting', '暗角', 'シェーディング', '셰이딩'],
  ClarityControl: ['clarity', '清晰度', '明瞭度', '명료도'],
  tone: ['toning', 'tone', 'tint', '色调', '色調', '调色', '調色', '조색'],
  filter: ['filter', 'filtereffect', '滤镜', '濾鏡', '滤镜效果', '濾鏡效果', 'フィルター', 'フィルター効果', '필터', '필터효과'],
  intensity: ['hdrlevel', 'hdrtonelevel', 'hdrintensity', 'hdr色调强度', 'hdr色調強度', 'hdr調の強さ', 'hdr톤강도'],
  solarization: ['solarizationtype', 'solarization', '中途曝光类型', '中途曝光類型', 'ソラリゼーションタイプ', '솔라리제이션종류'],
  nativeGrain: ['graineffect', 'grain', 'filmgrain', '颗粒', '顆粒', '颗粒效果', '顆粒效果', '颗粒感', '顆粒感', '粒状感', '粒子', '입자', '입자효과', '입자감'],
  nativeGrainSize: ['grainsize', 'size', '颗粒大小', '顆粒大小', '粒状感サイズ', '粒子サイズ', '입자크기'],
  grkitGrain: ['graingrkit', 'grkitgrain', '颗粒感grkit', '粒紋感grkit', '粒状感grkit', '입자감grkit'],
  whiteBalance: ['whitebalance', 'wb', '白平衡', 'ホワイトバランス', 'wb設定', '화이트밸런스'],
  shift: [
    'wbshift',
    'whitebalanceshift',
    'whitebalancecompensation',
    'whitebalancefinetune',
    'a-b',
    'g-m',
    'shift',
    'wbcompensation',
    'wbcorrection',
    'wbadjust',
    'wbadjustment',
    'finetune',
    'wbfinetune',
    '白平衡偏移',
    '白平衡微调',
    '白平衡微調',
    '白平衡补偿',
    '白平衡補償',
    '微调',
    '微調',
    '色彩偏移',
    'ホワイトバランス補正',
    'wb補正',
    '微調整',
    '미세조정',
  ],
  kelvin: ['colortemperature', 'colortemp', 'kelvin', 'temperature', '色温', '色溫', '色温度', '색온도'],
  correction0: ['highlightcorrection', 'highlightcorrect', 'highlight', 'hlcorrection', '高光补偿', '高光補償', '高光修正', '高光校正', 'ハイライト補正', '하이라이트보정'],
  correction1: ['shadowcorrection', 'shadowcorrect', 'shadow', '阴影补偿', '陰影補償', '阴影修正', '阴影校正', 'シャドー補正', 'シャドウ補正', '섀도보정'],
  correction2: [
    'peripheralilluminationcorrection',
    'peripheralillumination',
    'lenscorrection',
    'vignettingcorrection',
    '周边光量补偿',
    '周邊光量補償',
    '周边光量校正',
    '暗角校正',
    '周辺光量補正',
    '주변광량보정',
  ],
  correction3: [
    'highisonr',
    'highisonoisereduction',
    'noisereduction',
    'nr',
    'highisonoise',
    '高iso降噪',
    '高感降噪',
    '高感光度降噪',
    '降噪',
    '高感度nr',
    'ノイズリダクション',
    '고감도nr',
    '노이즈감소',
  ],
  exposure: ['exposurecompensation', 'exposure', 'ev', 'expcomp', '曝光补偿', '曝光補償', '曝光', '露出補正', '露出', '노출보정'],
  isoLimit: ['iso', 'isolimit', 'isomax', 'autoiso', 'isorange', 'iso上限', '感光度', 'iso感度', '감도'],
  metering: ['metering', 'meteringmode', '测光', '測光', '测光模式', '測光模式', '測光モード', '측광'],
  nd: ['nd', 'ndfilter', 'nd滤镜', 'nd濾鏡', 'ndフィルター', 'nd필터'],
};

/** The words of each value beyond the ones of the site strings. */
const WORDS: Readonly<Record<string, readonly string[]>> = {
  off: ['off', 'none', 'no', '0', '关', '關', '关闭', '關閉', '无', '無', 'オフ', 'なし', '꺼짐', '없음'],
  on: ['on', 'yes', '开', '開', '开启', '開啟', 'オン', 'あり', '켜짐'],
  auto: ['auto', 'automatic', '自动', '自動', 'オート', '자동'],
  low: ['low', 'weak', 'small', 'l', '低', '弱', '小', 'ロー', '약', '낮음'],
  mid: ['mid', 'medium', 'middle', 'm', 'standard', 'normal', '中', '中等', '标准', '標準', 'ミドル', '중', '중간'],
  high: ['high', 'strong', 'large', 'h', '高', '强', '強', '大', 'ハイ', '강', '높음'],
  custom: ['custom', '自定义', '自訂', 'カスタム', '사용자설정'],
};

/** The site labels of each language, which the export uses and the import reads. */
function siteLabels(field: string): string[] {
  const out: string[] = [];
  for (const r of Object.values(RECIPE_STRINGS)) {
    const parameter = r.parameters[field];
    if (parameter) out.push(parameter);
    if (field === 'name') out.push(r.name);
    if (field === 'author') out.push(r.author.replace(/[(（].*$/, ''));
    if (field === 'source') out.push(r.source.replace(/[(（].*$/, ''));
    if (field === 'look') out.push(r.look, r.lookNative, r.lookEffect);
    if (field === 'nativeGrain') out.push(r.nativeGrain);
    if (field === 'nativeGrainSize') out.push(`${r.nativeGrain}·${r.size}`);
    if (field === 'grkitGrain') out.push(r.grain);
    if (field === 'grkitGrainSize') out.push(`${r.grain}·${r.size}`);
    if (field === 'whiteBalance') out.push(r.whiteBalance);
    if (field === 'shift') out.push(r.shiftBa, r.shiftGm);
    if (field === 'kelvin') out.push(r.kelvin);
    if (field.startsWith('correction')) out.push(r.corrections[Number(field.slice(10))]!);
    if (field === 'exposure') out.push(r.exposure);
    if (field === 'isoLimit') out.push(r.isoLimit);
    if (field === 'metering') out.push(r.metering);
    if (field === 'nd') out.push(r.nd);
  }
  return out;
}

const FIELDS = [
  ...Object.keys(ALIASES),
  'grkitGrainSize',
  ...LEVEL_KEYS.filter((key) => !(key in ALIASES)),
];

/** The labels of each field, folded, the longest first, so a longer label wins over its start. */
const LABELS: readonly [string, string][] = FIELDS.flatMap((field) =>
  [...(ALIASES[field] ?? []), ...siteLabels(field)].map((label) => [fold(label), field] as [string, string]),
)
  .filter(([label]) => label.length > 0)
  .sort((a, b) => b[0].length - a[0].length);

/** The field of a label, or null. */
function fieldOf(label: string): string | null {
  const folded = fold(label);
  return LABELS.find(([known]) => known === folded)?.[1] ?? null;
}

/** A line as a label and a value: `label: value`, `label = value`, or the longest known label. */
function split(line: string): { label: string; value: string; field: string | null } | null {
  const separator = line.match(/^([^:：=]{1,40})[:：=]\s*(.*)$/);
  if (separator) {
    const label = separator[1]!.trim();
    return { label, value: separator[2]!.trim(), field: fieldOf(label) };
  }
  const folded = fold(line);
  for (const [known, field] of LABELS) {
    if (known.length >= 2 && folded.startsWith(known)) {
      // The value is what follows the label in the line.
      let consumed = 0;
      let index = 0;
      while (index < line.length && consumed < known.length) {
        consumed += fold(line[index]!).length;
        index += 1;
      }
      // A label of letters ends at a word boundary: "Grainy" is not "Grain".
      if (/[a-z]$/i.test(line.slice(0, index)) && /^[a-z]/i.test(line.slice(index))) continue;
      return { label: line.slice(0, index).trim(), value: line.slice(index).trim(), field };
    }
  }
  return null;
}

/** A signed number, such as +2, -1, ±0, +1.0, or 2. */
function number(text: string): number | null {
  const match = fold(text).match(/^(?:±)?([+-]?\d+(?:\.\d+)?)/);
  return match ? Number(match[1]) : null;
}

/** A level -4 to +4 as a stored value 0 to 8, or null. */
function levelOf(text: string): number | null {
  const value = number(text);
  if (value === null || !Number.isInteger(value) || Math.abs(value) > 4) return null;
  return value + NEUTRAL;
}

/** The value word of a text: off, on, auto, low, mid, high, custom, or null. */
function wordOf(text: string): string | null {
  const folded = fold(text);
  for (const [word, words] of Object.entries(WORDS)) {
    if (words.some((item) => fold(item) === folded)) return word;
  }
  return null;
}

/** The stored value of a choice of a camera label, by its label or a translation of it. */
function choiceOf(choices: readonly { value: number; label: string }[], text: string): number | null {
  const folded = fold(text);
  for (const choice of choices) {
    const names = [choice.label, ...Object.values(RECIPE_STRINGS).map((r) => r.values[choice.label] ?? '')];
    if (names.some((name) => name && fold(name) === folded)) return choice.value;
  }
  const word = wordOf(text);
  if (!word) return null;
  const labels: Record<string, string[]> = {
    off: ['OFF', 'Off'],
    on: ['On'],
    auto: ['Auto'],
    low: ['Low'],
    mid: ['Mid'],
    high: ['High'],
    custom: ['Custom'],
  };
  return choices.find((choice) => labels[word]?.includes(choice.label))?.value ?? null;
}

/** The type of a name of any language, by the camera name, the label, or a common name. */
function typeOf(model: Model, text: string, names: readonly (readonly string[])[]): ModelType | null {
  const folded = fold(text);
  const common: Record<string, string[]> = {
    Standard: ['standard', 'std', '标准', '標準', 'スタンダード', '표준'],
    Vivid: ['vivid', '鲜艳', '鮮豔', 'ビビッド', '비비드'],
    MonotoneGR: ['monotone', 'mono', 'monochrome', 'b&w', '单色', '單色', '黑白', 'モノトーン', '모노톤'],
    SoftMonotone: ['softmonotone', 'softmono', '柔和单色', '柔和單色', 'ソフトモノトーン'],
    HardMonotone: ['hardmonotone', 'hardmono', '硬调单色', '硬調單色', 'ハードモノトーン'],
    HighContrastBW: ['hi-contrastb&w', 'highcontrastb&w', 'hicontrastbw', 'highcontrastbw', '高对比度黑白', '高對比度黑白', '高对比黑白', 'ハイコントラスト白黒'],
    NegativeFilm: ['negativefilm', 'negative', 'nega', '负片', '負片', 'ネガフィルム調', 'ネガフィルム', 'ネガ', '네거티브필름'],
    PositiveFilm: ['positivefilm', 'positive', 'posi', '正片', 'ポジフィルム調', 'ポジフィルム', 'ポジ', '포지티브필름'],
    BleechBypass_GR: ['bleachbypass', 'bleach', '漂白效果', '漂白', '银残留', 'ブリーチバイパス', '블리치바이패스'],
    Retro: ['retro', '复古', '復古', 'レトロ', '레트로'],
    HdrCuisine: ['hdrtone', 'hdr', 'hdr色调', 'hdr色調', 'hdr調', 'hdr톤'],
    CrossProcess: ['crossprocessing', 'crossprocess', 'cross', '交叉冲印', '交叉沖印', 'クロスプロセス', '크로스프로세스'],
    CinemaYellow: ['cinemayellow', 'cinema yellow', '电影黄', '電影黃', 'シネマ調イエロー', '시네마옐로'],
    CinemaGreen: ['cinemagreen', '电影绿', '電影綠', 'シネマ調グリーン', '시네마그린'],
  };
  for (const type of model.types) {
    const known = [
      type.label,
      type.name,
      ...names.map((list) => list[type.index] ?? ''),
      ...(common[type.label] ?? []),
    ];
    if (known.some((name) => name && fold(name) === folded)) return type;
  }
  return null;
}

/** The white balance mode of a text, by its label, a site name, or a common name. */
function modeOf(model: Model, text: string): number | null {
  const folded = fold(text);
  const common: Record<string, string[]> = {
    Auto: ['auto', 'awb', 'autowb', 'autowhitebalance', '自动', '自動', '自动白平衡', '自動白平衡', 'オート', '자동'],
    Multi: ['multi', 'multi-patternauto', 'multipatternauto', 'multiauto', 'multiautowb', '多重自动白平衡', '复合自动白平衡', '多重自動白平衡', 'マルチパターンオート'],
    AutoStandard: ['auto', 'awb', 'autowb', 'awbstandard', 'autostandard', '自动', '自動', '自动白平衡', 'オート', '자동'],
    AutoWarm: ['awbwarm', 'autowarm', 'awbwarmpriority', '暖色', 'オート暖色', '따뜻하게'],
    AutoWhite: ['awbwhite', 'autowhite', 'awbwhitepriority', 'オートホワイト'],
    Daylight: ['daylight', 'sunny', 'sun', '日光', '晴天', '太陽光', '태양광'],
    Shade: ['shade', 'shadow', '阴影', '陰影', '日陰', '그늘'],
    Cloudiness: ['cloudy', 'cloud', 'overcast', '阴天', '陰天', '曇天', '흐림'],
    Flu_D: ['fluorescentd', 'fluorescentdaylight', '荧光灯d', '日光燈d', '蛍光灯d'],
    Flu_N: ['fluorescentn', '荧光灯n', '日光燈n', '蛍光灯n'],
    Flu_W: ['fluorescentw', '荧光灯w', '日光燈w', '蛍光灯w'],
    Flu_L: ['fluorescentl', '荧光灯l', '日光燈l', '蛍光灯l'],
    Tungsten: ['tungsten', 'incandescent', '白炽灯', '鎢絲燈', '白熱灯', '백열등'],
    Cte: ['cte'],
    Manual1: ['manual', '手动', '手動', 'マニュアル', '수동'],
    ColorTemp1: ['colortemperature', 'colortemp', 'kelvin', '色温', '色溫', '色温度', '색온도'],
  };
  for (const mode of model.modes) {
    const names = [
      mode.label,
      ...Object.values(RECIPE_STRINGS).map((r) => r.modes[mode.label] ?? ''),
      ...(common[mode.label] ?? []),
    ];
    if (names.some((name) => name && fold(name) === folded)) return mode.index;
  }
  return null;
}

/**
 * The A-B and G-M shifts of a text: "A:7 G:3", "A7, G3", "G3B3", "+B5", "M:6 B:11", "B2 M1". Each
 * axis that the text gives; a plain number such as "+3" or "0" is a shift of `axis`, A or M for
 * a positive one. Null when the text has none.
 */
export function shiftsOf(text: string, axis: 'ba' | 'gm' | null = null): { ba?: number; gm?: number } | null {
  const folded = fold(text).replace(/[:：,，、/]/g, '');
  const pairs = [...folded.matchAll(/([+-]?)([abgm])([+-]?\d+)|([+-]?\d+)([abgm])/g)];
  if (pairs.length === 0) {
    if (/^(normal|none|无|無|なし)$/.test(folded)) return axis ? { [axis]: NO_SHIFT } : { ba: NO_SHIFT, gm: NO_SHIFT };
    const plain = folded.match(/^±?([+-]?\d+)$/);
    if (!plain || !axis || Math.abs(Number(plain[1])) > NO_SHIFT) return null;
    return { [axis]: NO_SHIFT + Number(plain[1]) };
  }
  const shifts: { ba?: number; gm?: number } = {};
  for (const pair of pairs) {
    const letter = pair[2] ?? pair[5]!;
    const amount = Math.abs(Number(pair[3] ?? pair[4]));
    if (amount > NO_SHIFT) return null;
    if (letter === 'a') shifts.ba = NO_SHIFT + amount;
    if (letter === 'b') shifts.ba = NO_SHIFT - amount;
    if (letter === 'm') shifts.gm = NO_SHIFT + amount;
    if (letter === 'g') shifts.gm = NO_SHIFT - amount;
  }
  return shifts;
}

/**
 * Exposure compensation in thirds of a stop: "+2/3", "-1 1/3", "-0.7", "+1EV". A range such as
 * "+1/3 to +2/3" gives its first value and `range`.
 */
export function exposureOf(text: string): { thirds: number; range: boolean } | null {
  // Spaces stay: "1 1/3" is one and a third.
  const plain = text.normalize('NFKC').toLowerCase().replace(/[−–—]/g, '-').replace(/ev/g, '').trim();
  const match = plain.match(/^±?([+-]?)\s*(\d+)(?:\s+(\d)\/3|\/3|\.(\d+))?/);
  if (!match) return null;
  const sign = match[1] === '-' ? -1 : 1;
  let thirds: number;
  if (match[3]) thirds = Number(match[2]) * 3 + Number(match[3]);
  else if (match[0].endsWith('/3')) thirds = Number(match[2]);
  else thirds = Math.round(Number(`${match[2]}.${match[4] ?? '0'}`) * 3);
  if (thirds > 15) return null;
  const rest = plain.slice(match[0].length).trim();
  return { thirds: sign * thirds, range: /^(~|-|to|至|到|から|～)/.test(rest) && /\d/.test(rest) };
}

/** The highest ISO number of a text: "Auto, up to 3200", "200-6400", "ISO 1600". */
function isoOf(text: string): number | null {
  const values = [...fold(text).matchAll(/\d{3,6}/g)].map((match) => Number(match[0]));
  const best = Math.max(0, ...values);
  return best >= 100 && best <= 204800 ? best : null;
}

function meteringOf(text: string): Recipe['hints']['metering'] | null {
  const folded = fold(text);
  const names: Record<NonNullable<Recipe['hints']['metering']>, string[]> = {
    multi: ['multi', 'multi-segment', 'evaluative', 'matrix', 'pattern', '多区', '多區', '评价', '分割', '분할'],
    center: ['center', 'centre', 'center-weighted', 'centerweighted', '中央', '中央重点', '中央重點', '중앙'],
    spot: ['spot', '点', '點', 'スポット', '스폿'],
    highlight: ['highlight', 'highlight-weighted', 'highlightweighted', '高光', 'ハイライト', '하이라이트'],
  };
  for (const [key, words] of Object.entries(names) as [NonNullable<Recipe['hints']['metering']>, string[]][]) {
    if (words.some((word) => folded.startsWith(fold(word)))) return key;
  }
  for (const [key, index] of [
    ['multi', 0],
    ['center', 1],
    ['spot', 2],
    ['highlight', 3],
  ] as const) {
    if (Object.values(RECIPE_STRINGS).some((r) => fold(r.meterings[index]!) === folded)) return key;
  }
  return null;
}

function ndOf(text: string): Recipe['hints']['nd'] | null {
  const word = wordOf(text.replace(/^nd\s*/i, ''));
  return word === 'off' || word === 'on' || word === 'auto' ? word : null;
}

/** The native grain strength 0 Off to 3 High of a text. */
function strengthOf(text: string): number | null {
  const word = wordOf(text);
  if (word === 'off') return 0;
  if (word === 'low') return 1;
  if (word === 'mid') return 2;
  if (word === 'high') return 3;
  for (const r of Object.values(RECIPE_STRINGS)) {
    const index = r.nativeGrainStrengths.findIndex((item) => fold(item) === fold(text));
    if (index >= 0) return index;
  }
  return null;
}

/** The native grain size 0 Small to 2 Large of a text. */
function sizeOf(text: string): number | null {
  const word = wordOf(text);
  if (word === 'low') return 0;
  if (word === 'mid') return 1;
  if (word === 'high') return 2;
  for (const r of Object.values(RECIPE_STRINGS)) {
    const index = r.nativeGrainSizes.findIndex((item) => fold(item) === fold(text));
    if (index >= 0) return index;
  }
  return null;
}

/** The GRKit Grain level 0 to 4 and size 0 Auto to 3 of a text. */
function grkitGrainOf(text: string, sizes: boolean): number | null {
  for (const r of Object.values(RECIPE_STRINGS)) {
    const list = sizes ? r.grainSizes : r.grainLevels;
    const index = list.findIndex((item) => fold(item) === fold(text));
    if (index >= 0) return index;
  }
  return null;
}

/**
 * Reads the text of one recipe for a camera line. `effects` are the effects that an effect
 * recipe can name, and `names` the camera names of the types in each language
 * (`generated/recipes.ts`).
 */
export function parseRecipeText(
  text: string,
  family: Family,
  model: Model,
  effects: readonly EffectName[] = [],
  names: readonly (readonly string[])[] = [],
): Parsed {
  const recipe = newRecipe(family, model, '');
  const flags: Flag[] = [];
  let author = '';
  let source = '';
  let type: ModelType = model.types[0]!;
  // The values of the type, which apply once the type is known: a recipe can name the type after
  // its parameters.
  const values: [string, string, string][] = [];
  const flag = (field: string, value: string) => flags.push({ field, text: value });
  const lines = text
    .split(/\r?\n|[;；|]/)
    .map((line) => line.replace(/^[\s\-*•・·>]+/, '').trim())
    .filter((line) => line.length > 0);
  let named = false;

  for (const line of lines) {
    if (/^https?:\/\//i.test(line)) {
      source ||= line;
      continue;
    }
    const parts = split(line);
    if (!parts || !parts.field) {
      if (!named && !parts) {
        // A first line without a label is the name.
        recipe.name = line;
        named = true;
      } else {
        flag('line', line);
      }
      continue;
    }
    const { field, value } = parts;
    switch (field) {
      case 'name':
        recipe.name = value;
        named = true;
        break;
      case 'author':
        author = value;
        break;
      case 'source':
        source = value;
        break;
      case 'camera':
        break;
      case 'look': {
        const found = typeOf(model, value, names);
        const effect = effects.find((item) => fold(item.name) === fold(value) || fold(item.id) === fold(value));
        if (found) {
          type = found;
          recipe.look = nativeLook(found);
        } else if (effect) {
          recipe.look = { kind: 'effect', code: effectCode(effect.id), adjustments: Array(9).fill(NEUTRAL) };
        } else {
          flag('look', value);
        }
        break;
      }
      case 'whiteBalance': {
        // A white balance line can also hold the shifts and the temperature: "Daylight, A3 G1".
        const [mode, ...rest] = value.split(/[,，、]\s*|\s+(?=[+-]?[ABGMabgm]\s*[:：]?\s*\d)/);
        const index = modeOf(model, mode ?? '');
        if (model.modes.length === 0) {
          flag('whiteBalance', value);
          break;
        }
        if (index === null) {
          const kelvin = fold(value).match(/(\d{4,5})k/);
          if (kelvin) {
            recipe.whiteBalance = whiteBalance(recipe, 14, Number(kelvin[1]));
            break;
          }
          flag('whiteBalance', value);
          break;
        }
        recipe.whiteBalance = whiteBalance(recipe, index);
        const shifts = rest.length > 0 ? shiftsOf(rest.join(' ')) : null;
        if (shifts)
          recipe.whiteBalance = {
            ...recipe.whiteBalance,
            shiftBa: shifts.ba ?? recipe.whiteBalance.shiftBa,
            shiftGm: shifts.gm ?? recipe.whiteBalance.shiftGm,
          };
        const kelvin = fold(value).match(/(\d{4,5})k/);
        if (kelvin) recipe.whiteBalance = { ...recipe.whiteBalance, kelvin: Number(kelvin[1]) };
        break;
      }
      case 'shift': {
        // A label of one axis: "Fine-tune A-B", "G-M".
        const label = fold(parts.label);
        const axis = /g-?m/.test(label) ? 'gm' : /a-?b/.test(label) ? 'ba' : null;
        const shifts = shiftsOf(value, axis);
        if (!shifts || model.modes.length === 0) {
          flag('shift', value);
          break;
        }
        const balance = whiteBalance(recipe, recipe.whiteBalance?.mode ?? model.modes[0]!.index);
        recipe.whiteBalance = {
          ...balance,
          shiftBa: shifts.ba ?? balance.shiftBa,
          shiftGm: shifts.gm ?? balance.shiftGm,
        };
        break;
      }
      case 'kelvin': {
        const kelvin = number(value.replace(/k$/i, ''));
        if (kelvin === null || model.modes.length === 0) flag('kelvin', value);
        else recipe.whiteBalance = { ...whiteBalance(recipe, 14), kelvin };
        break;
      }
      case 'grkitGrain': {
        const level = grkitGrainOf(value, false);
        if (level === null) flag('grain', value);
        else recipe.grain = { level, size: recipe.grain?.size ?? 0 };
        break;
      }
      case 'grkitGrainSize': {
        const size = grkitGrainOf(value, true);
        if (size === null) flag('grain', value);
        else recipe.grain = { level: recipe.grain?.level ?? 0, size };
        break;
      }
      case 'exposure': {
        const exposure = exposureOf(value);
        if (!exposure) flag('exposure', value);
        else {
          recipe.hints = { ...recipe.hints, exposure: exposure.thirds };
          if (exposure.range) flag('exposure', value);
        }
        break;
      }
      case 'isoLimit': {
        const iso = isoOf(value);
        if (iso === null) flag('isoLimit', value);
        else recipe.hints = { ...recipe.hints, isoLimit: iso };
        break;
      }
      case 'metering': {
        const metering = meteringOf(value);
        if (!metering) flag('metering', value);
        else recipe.hints = { ...recipe.hints, metering };
        break;
      }
      case 'nd': {
        const nd = ndOf(value);
        if (!nd) flag('nd', value);
        else recipe.hints = { ...recipe.hints, nd };
        break;
      }
      default:
        if (field.startsWith('correction')) {
          const index = Number(field.slice(10));
          const choice = choiceOf(model.corrections[index]!.choices, value);
          if (choice === null) flag(field, value);
          else recipe.corrections[index] = choice;
        } else {
          values.push([field, value, line]);
        }
    }
  }

  // The values of the look: the levels of an effect or of the type, then the other parameters
  // of the type.
  const look = recipe.look;
  for (const [field, value] of values) {
    const index = (LEVEL_KEYS as readonly string[]).indexOf(field);
    if (look.kind === 'effect') {
      const stored = index >= 0 ? levelOf(value) : null;
      if (stored === null) flag(field, value);
      else look.adjustments[index] = stored;
      continue;
    }
    if (look.kind !== 'native') {
      flag(field, value);
      continue;
    }
    applyNative(look, type, field, value, flag);
  }
  if (!recipe.name) flag('name', '');
  return { recipe, author, source, flags };
}

/** The white balance of a recipe with mode `mode`, keeping its other values. */
function whiteBalance(recipe: Recipe, mode: number, kelvin?: number): NonNullable<Recipe['whiteBalance']> {
  return {
    mode,
    shiftBa: recipe.whiteBalance?.shiftBa ?? NO_SHIFT,
    shiftGm: recipe.whiteBalance?.shiftGm ?? NO_SHIFT,
    kelvin: kelvin ?? recipe.whiteBalance?.kelvin ?? 5500,
  };
}

/** Sets a value of the native look, or flags it. */
function applyNative(
  look: NativeLook & { kind: 'native' },
  type: ModelType,
  field: string,
  value: string,
  flag: (field: string, value: string) => void,
) {
  const index = (LEVEL_KEYS as readonly string[]).indexOf(field);
  const parameter = (key: string) => type.parameters.find((item) => item.key === key);
  if (index >= 0) {
    const stored = levelOf(value);
    if (stored === null || (!parameter(field) && stored !== NEUTRAL)) flag(field, value);
    else look.levels[index] = stored;
    return;
  }
  if (field === 'tone') {
    const tone = type.parameters.find((item) => item.key.startsWith('ColorTone'));
    const stored = tone
      ? tone.key === 'ColorToneRetro'
        ? levelOf(value)
        : choiceOf(tone.choices, value)
      : null;
    if (stored === null) flag('tone', value);
    else look.tone = stored;
    return;
  }
  const keys: Record<string, [string, 'intensity' | 'filter' | 'solarization']> = {
    intensity: ['HdrIntense', 'intensity'],
    filter: ['FilterEffectCustom', 'filter'],
    solarization: ['SolarizationType', 'solarization'],
  };
  if (keys[field]) {
    const [key, slot] = keys[field]!;
    const stored = parameter(key) ? choiceOf(parameter(key)!.choices, value) : null;
    if (stored === null) flag(field, value);
    else look[slot] = stored;
    return;
  }
  if (field === 'nativeGrain') {
    // "Mid, Size: Large" or "Mid / Large" on one line.
    const [strength, size] = value.split(/[,，、/]\s*(?:size|大小|サイズ|크기)?\s*[:：]?\s*/i);
    const stored = strengthOf(strength ?? '');
    if (stored === null || (stored > 0 && !type.grain)) {
      flag('nativeGrain', value);
      return;
    }
    look.grainStrength = stored;
    if (stored === 0) look.grainSize = 0;
    if (size) applyNative(look, type, 'nativeGrainSize', size, flag);
    return;
  }
  if (field === 'nativeGrainSize') {
    const stored = sizeOf(value);
    if (stored === null || (!type.grainSizes && stored !== 0)) flag('nativeGrainSize', value);
    else if (look.grainStrength > 0) look.grainSize = stored;
    return;
  }
  flag(field, value);
}

/** A level as text: -4 to +4. */
const levelText = (stored: number) =>
  stored === NEUTRAL ? '0' : stored > NEUTRAL ? `+${stored - NEUTRAL}` : `${stored - NEUTRAL}`;

/** Exposure compensation in thirds as text: -2/3, +1 1/3, +2, 0. */
export function thirdsText(thirds: number): string {
  if (thirds === 0) return '0';
  const sign = thirds > 0 ? '+' : '-';
  const whole = Math.floor(Math.abs(thirds) / 3);
  const rest = Math.abs(thirds) % 3;
  return `${sign}${whole ? whole : ''}${whole && rest ? ' ' : ''}${rest ? `${rest}/3` : ''}`;
}

/** The text of a recipe in a site language, which `parseRecipeText` reads back. */
export function recipeText(
  item: LibraryRecipe,
  model: Model,
  language: Language,
  effects: readonly EffectName[] = [],
): string {
  const r = RECIPE_STRINGS[language];
  const { recipe } = item;
  const lines: string[] = [`${r.name}: ${recipe.name}`];
  const look = recipe.look;
  const value = (label: string) => r.values[label] ?? label;
  if (look.kind === 'effect') {
    const effect = effects.find((candidate) => effectCode(candidate.id) === look.code);
    lines.push(`${r.look}: ${effect?.id ?? r.lookEffect}`);
    look.adjustments.forEach((stored, index) => {
      if (stored !== NEUTRAL) lines.push(`${r.parameters[LEVEL_KEYS[index]!]}: ${levelText(stored)}`);
    });
  } else if (look.kind === 'native') {
    const type = model.types.find((candidate) => candidate.index === look.imageType)!;
    lines.push(`${r.look}: ${type.name}`);
    for (const parameter of type.parameters) {
      const index = (LEVEL_KEYS as readonly string[]).indexOf(parameter.key);
      const label = r.parameters[parameter.key] ?? parameter.key;
      if (index >= 0) lines.push(`${label}: ${levelText(look.levels[index]!)}`);
      else if (parameter.key === 'ColorToneRetro') lines.push(`${label}: ${levelText(look.tone)}`);
      else {
        const stored = parameter.key.startsWith('ColorTone')
          ? look.tone
          : parameter.key === 'HdrIntense'
            ? look.intensity
            : parameter.key === 'FilterEffectCustom'
              ? look.filter
              : look.solarization;
        const choice = parameter.choices.find((candidate) => candidate.value === stored);
        lines.push(`${label}: ${choice ? value(choice.label) : stored}`);
      }
    }
    if (type.grain) {
      lines.push(`${r.nativeGrain}: ${r.nativeGrainStrengths[look.grainStrength]}`);
      if (type.grainSizes && look.grainStrength > 0)
        lines.push(`${r.nativeGrain} · ${r.size}: ${r.nativeGrainSizes[look.grainSize]}`);
    }
  }
  if (recipe.grain) {
    lines.push(`${r.grain}: ${r.grainLevels[recipe.grain.level]}`);
    lines.push(`${r.grain} · ${r.size}: ${r.grainSizes[recipe.grain.size]}`);
  }
  const balance = recipe.whiteBalance;
  if (balance) {
    const mode = model.modes.find((candidate) => candidate.index === balance.mode);
    lines.push(`${r.whiteBalance}: ${mode ? (r.modes[mode.label] ?? mode.label) : balance.mode}`);
    if (mode?.shifts) {
      const ba = balance.shiftBa - NO_SHIFT;
      const gm = balance.shiftGm - NO_SHIFT;
      lines.push(`${r.shiftBa}: ${ba === 0 ? '0' : `${ba > 0 ? 'A' : 'B'}${Math.abs(ba)}`}`);
      lines.push(`${r.shiftGm}: ${gm === 0 ? '0' : `${gm > 0 ? 'M' : 'G'}${Math.abs(gm)}`}`);
    }
    if (mode?.temperature) lines.push(`${r.kelvin}: ${balance.kelvin}`);
  }
  recipe.corrections.forEach((stored, index) => {
    if (stored === null) return;
    const choice = model.corrections[index]!.choices.find((candidate) => candidate.value === stored);
    lines.push(`${r.corrections[index]}: ${choice ? value(choice.label) : stored}`);
  });
  const hints = recipe.hints;
  if (hints.exposure !== null) lines.push(`${r.exposure}: ${thirdsText(hints.exposure)}`);
  if (hints.isoLimit !== null) lines.push(`${r.isoLimit}: ${hints.isoLimit}`);
  if (hints.metering)
    lines.push(`${r.metering}: ${r.meterings[['multi', 'center', 'spot', 'highlight'].indexOf(hints.metering)]}`);
  if (hints.nd) lines.push(`${r.nd}: ${r.nds[['off', 'on', 'auto'].indexOf(hints.nd)]}`);
  if (item.author) lines.push(`${r.author.replace(/\s*[(（].*$/, '')}: ${item.author}`);
  if (item.source) lines.push(`${r.source.replace(/\s*[(（].*$/, '')}: ${item.source}`);
  return lines.join('\n');
}
