import { siteUrl } from '../catalog';
import type { Language } from '../i18n';
import modeset from '../../../resources/images/modeset.png';

/** The camera model that a screen shows, when the docs compare models. */
type ScreenModel = 'GR III' | 'GR IIIx' | 'GR IV';

interface ScreenText {
  model?: ScreenModel;
  alt: Readonly<Record<Language, string>>;
}

/** The camera screens of the docs: captures of each camera language in `public/docs-screens/`. */
export const DOC_SCREENS = {
  modeset: {
    alt: {
      en: 'The Mode Set Command screen shows Script - Enable and Complete.',
      'zh-Hans': 'Mode Set Command 画面显示 Script - Enable 和 Complete。',
      'zh-Hant': 'Mode Set Command 畫面顯示 Script - Enable 和 Complete。',
      ja: 'Mode Set Command の画面に Script - Enable と Complete が表示されています。',
      ko: 'Mode Set Command 화면에 Script - Enable과 Complete가 표시되어 있습니다.',
    },
  },
  'adj-grk': {
    model: 'GR III',
    alt: {
      en: 'The shooting screen shows GRKit at the top left, and ADJ. mode is open on GRK.',
      'zh-Hans': '拍摄画面左上方显示 GRKit，ADJ 模式停在 GRK。',
      'zh-Hant': '拍攝畫面左上方顯示 GRKit，ADJ 模式停在 GRK。',
      ja: '撮影画面の左上に GRKit が表示され、ADJ モードで GRK が選ばれています。',
      ko: '촬영 화면 왼쪽 위에 GRKit이 표시되고 ADJ 모드에서 GRK가 선택되어 있습니다.',
    },
  },
  'adj-grk-gr4': {
    model: 'GR IV',
    alt: {
      en: 'The GR IV shooting screen shows GRKit at the top left, and ADJ. mode is open on GRK.',
      'zh-Hans': 'GR IV 拍摄画面左上方显示 GRKit，ADJ 模式停在 GRK。',
      'zh-Hant': 'GR IV 拍攝畫面左上方顯示 GRKit，ADJ 模式停在 GRK。',
      ja: 'GR IV の撮影画面の左上に GRKit が表示され、ADJ モードで GRK が選ばれています。',
      ko: 'GR IV 촬영 화면 왼쪽 위에 GRKit이 표시되고 ADJ 모드에서 GRK가 선택되어 있습니다.',
    },
  },
  'film-look': {
    alt: {
      en: 'ADJ. mode lists the film looks, with one look highlighted.',
      'zh-Hans': 'ADJ 模式列出胶片风格，其中一项处于高亮状态。',
      'zh-Hant': 'ADJ 模式列出底片風格，其中一項處於反白狀態。',
      ja: 'ADJ モードにフィルムルックの一覧が表示され、1 つが選ばれています。',
      ko: 'ADJ 모드에 필름 룩 목록이 표시되고 하나가 선택되어 있습니다.',
    },
  },
  framing: {
    alt: {
      en: 'Framing in ADJ. mode with XPan highlighted. The live view shows the XPan frame.',
      'zh-Hans': 'ADJ 模式的画幅选中 XPan，实时显示中出现 XPan 取景框。',
      'zh-Hant': 'ADJ 模式的構圖比例選中 XPan，實時顯示中出現 XPan 取景框。',
      ja: 'ADJ モードのフレーミングで XPan が選ばれ、ライブビューに XPan の枠が表示されています。',
      ko: 'ADJ 모드의 프레이밍에서 XPan이 선택되고 라이브 뷰에 XPan 프레임이 표시되어 있습니다.',
    },
  },
  crop: {
    model: 'GR III',
    alt: {
      en: 'Crop in ADJ. mode with 50mm highlighted.',
      'zh-Hans': 'ADJ 模式的裁切选中 50mm。',
      'zh-Hant': 'ADJ 模式的裁切選中 50mm。',
      ja: 'ADJ モードのクロップで 50mm が選ばれています。',
      ko: 'ADJ 모드의 크롭에서 50mm가 선택되어 있습니다.',
    },
  },
  'crop-gr3x': {
    model: 'GR IIIx',
    alt: {
      en: 'Crop in ADJ. mode on GR IIIx, with a GR IIIx focal length highlighted.',
      'zh-Hans': 'GR IIIx 的 ADJ 模式裁切选中一个 GR IIIx 焦距。',
      'zh-Hant': 'GR IIIx 的 ADJ 模式裁切選中一個 GR IIIx 焦距。',
      ja: 'GR IIIx の ADJ モードのクロップで GR IIIx の焦点距離が選ばれています。',
      ko: 'GR IIIx의 ADJ 모드 크롭에서 GR IIIx 초점 거리가 선택되어 있습니다.',
    },
  },
  recipes: {
    alt: {
      en: 'The Recipes page in ADJ. mode, with Save Settings.',
      'zh-Hans': 'ADJ 模式的配方页面，其中有保存设置。',
      'zh-Hant': 'ADJ 模式的配方頁面，其中有儲存設定。',
      ja: 'ADJ モードのレシピのページに「設定を保存」があります。',
      ko: 'ADJ 모드의 레시피 페이지에 설정 저장이 있습니다.',
    },
  },
  'menu-grkit': {
    alt: {
      en: 'The GRKit section of the Customize Settings menu.',
      'zh-Hans': '自定义设定菜单中的 GRKit 部分。',
      'zh-Hant': '自定義設定功能表中的 GRKit 部分。',
      ja: 'カスタマイズ設定メニューの GRKit セクション。',
      ko: '사용자 설정 메뉴의 GRKit 섹션.',
    },
  },
  'grkit-settings': {
    alt: {
      en: 'The GRKit Settings page in the menu.',
      'zh-Hans': '菜单中的 GRKit 设置页面。',
      'zh-Hant': '功能表中的 GRKit 設定頁面。',
      ja: 'メニューの GRKit 設定のページ。',
      ko: '메뉴의 GRKit 설정 페이지.',
    },
  },
  'mode-dial': {
    model: 'GR III',
    alt: {
      en: 'The Mode Dial page lists P, Av, Tv, M, and U1 to U3.',
      'zh-Hans': '模式转盘页面列出 P、Av、Tv、M 和 U1 到 U3。',
      'zh-Hant': '模式轉盤頁面列出 P、Av、Tv、M 和 U1 到 U3。',
      ja: 'モードダイヤルのページに P、Av、Tv、M、U1〜U3 が表示されています。',
      ko: '모드 다이얼 페이지에 P, Av, Tv, M, U1~U3가 표시되어 있습니다.',
    },
  },
  'mode-dial-gr4': {
    model: 'GR IV',
    alt: {
      en: 'The GR IV Mode Dial page also lists Sn.',
      'zh-Hans': 'GR IV 的模式转盘页面还列出 Sn。',
      'zh-Hant': 'GR IV 的模式轉盤頁面還列出 Sn。',
      ja: 'GR IV のモードダイヤルのページには Sn も表示されています。',
      ko: 'GR IV의 모드 다이얼 페이지에는 Sn도 표시되어 있습니다.',
    },
  },
  'raw-dev': {
    alt: {
      en: 'RAW development shows the GRK row at the end of the list.',
      'zh-Hans': 'RAW 处理列表末尾显示 GRK 行。',
      'zh-Hant': 'RAW 處理清單最後顯示 GRK 列。',
      ja: 'RAW 現像の一覧の最後に GRK の行が表示されています。',
      ko: 'RAW 현상 목록 끝에 GRK 행이 표시되어 있습니다.',
    },
  },
} as const satisfies Readonly<Record<string, ScreenText>>;

export type DocScreen = keyof typeof DOC_SCREENS;

/** The address of a camera screen in the camera language of the page. */
export function screenUrl(screen: DocScreen, language: Language): string {
  return screen === 'modeset' ? modeset : siteUrl(`docs-screens/${screen}.${language}.jpg`);
}

/** The model of a screen, for its caption. */
export function screenModel(screen: DocScreen): ScreenModel | undefined {
  return (DOC_SCREENS[screen] as ScreenText).model;
}
