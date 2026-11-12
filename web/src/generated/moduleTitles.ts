// Generated from the text of each official module (modules/<module>/text). Do not
// edit: change the title there, then run
// GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated.

import type { ModuleKey } from './modules';

/** The title of each official module in each language, as the camera menu shows it. */
export const MODULE_TITLES: {
  readonly [language in 'en' | 'zh-Hans' | 'zh-Hant' | 'ja' | 'ko']: Readonly<Record<ModuleKey, string>>;
} = {
  en: {
    recipes: 'Recipes',
    filmLook: 'Film Look',
    framing: 'Framing',
    crop: 'Crop',
    guides: 'Guides',
    dateStamp: 'Date Imprint',
    glow: 'Glow',
    border: 'Border',
    grain: 'Grain',
  },
  'zh-Hans': {
    recipes: '配方',
    filmLook: '胶片风格',
    framing: '画幅',
    crop: '裁切',
    guides: '辅助线',
    dateStamp: '加印日期',
    glow: '光晕',
    border: '边框',
    grain: '颗粒感',
  },
  'zh-Hant': {
    recipes: '配方',
    filmLook: '底片風格',
    framing: '構圖比例',
    crop: '裁切',
    guides: '輔助線',
    dateStamp: '加印日期',
    glow: '光暈',
    border: '邊框',
    grain: '粒紋感',
  },
  ja: {
    recipes: 'レシピ',
    filmLook: 'フィルムルック',
    framing: 'フレーミング',
    crop: 'クロップ',
    guides: '構図ガイド',
    dateStamp: '日付入れ撮影',
    glow: 'グロー',
    border: 'ボーダー',
    grain: '粒状感',
  },
  ko: {
    recipes: '레시피',
    filmLook: '필름 룩',
    framing: '프레이밍',
    crop: '크롭',
    guides: '구도 가이드',
    dateStamp: '날짜 각인',
    glow: '글로우',
    border: '테두리',
    grain: '입자감',
  },
};
