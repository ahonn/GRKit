export const LANGUAGES = [
  { code: 'en', label: 'English' },
  { code: 'zh-Hans', label: '简体中文' },
  { code: 'zh-Hant', label: '繁體中文' },
] as const;

export type Language = (typeof LANGUAGES)[number]['code'];

/** Text with inline code: a part that starts with a backtick is code. */
export type RichText = readonly string[];

const en = {
  language: 'Language',
  tagline:
    'Choose your camera and the effects you like. Then download one package and copy it to your SD card.',
  camera: 'Camera',
  firmware: 'Firmware',
  supported: 'Supported',
  planned: 'Planned',
  effects: 'Effects',
  modules: 'Modules',
  package: 'Package',
  install: 'Install',
  previewing: 'Previewing',
  samples: 'Sample photos',
  chart: 'Color chart',
  upload: 'Open your photo',
  yourPhoto: 'Your photo',
  photoError: 'This photo cannot be opened. Use a JPEG or PNG file.',
  previewError: 'The preview cannot be shown.',
  uploadNote: 'Your photo stays in this browser. It is not sent to a server.',
  previewNote:
    'The preview is an approximation. Photos from the camera can look slightly different.',
  compare: 'Compare',
  original: 'Original',
  preview: 'Preview',
  previewOf: (name: string) => `Preview ${name}`,
  /** The text before and after the selected count. */
  count: (max: number): readonly [string, string] => ['', ` of ${max} selected`],
  selectAll: 'Select all',
  clear: 'Clear',
  include: 'Include',
  icon: 'Icon',
  effect: 'Effect',
  look: 'Target look',
  type: 'Type',
  color: 'Color',
  mono: 'Mono',
  effectCount: 'Effects',
  loading: 'Loading effects…',
  loadError: 'The effects cannot be loaded. Reload the page to try again.',
  firmwareNote: 'Before you download, check the firmware version in the camera menu.',
  download: 'Download GRKit.zip',
  building: 'Preparing GRKit.zip…',
  ready: 'GRKit.zip is ready. Copy the contents of SD_ROOT to the card root.',
  buildError: 'GRKit.zip cannot be prepared. Reload the page to try again.',
  footer: 'Supports RICOH GR III firmware 2.10. GR IIIx and GR IV are planned.',
  licenses: 'Effect licenses',
  recovery: 'Recovery guide',
  source: 'Source',
  modulesText: {
    imageControl: 'The effects above, with live-view preview.',
    framing:
      'XPan 65:24, 16:9, or 4:3 photos. The viewfinder stays 3:2 and dims the area outside the photo.',
    guides: 'Thirds, center, golden ratio, or diagonal lines inside the Framing area.',
    dateStamp: 'The date on photos in orange seven-segment digits.',
  },
  steps: [
    ['Turn the camera off. Back up the SD card, including any existing startup scripts.'],
    ['Unzip GRKit.zip. Copy the contents of ', '`SD_ROOT', ' to the card root.'],
    [
      'Insert the card. If SD scripts are not enabled, hold MENU while you turn the camera on. Wait until the camera turns itself off.',
    ],
    [
      'With the camera off, remove ',
      '`00078350.588',
      ' and ',
      '`DEVELOP.MOD',
      ' from the card root. Keep the ',
      '`SCRIPT',
      ' directory.',
    ],
    ['Turn the camera on normally. Open ADJ and select the sixth entry, GRK.'],
  ] as readonly RichText[],
  looks: {
    'portra-400-endura-v1': 'Kodak Portra 400 on Portra Endura paper',
    'gold-200-premier-v1': 'Kodak Gold 200 on Endura Premier paper',
    'ektar-100-edge-v1': 'Kodak Ektar 100 on Ektacolor Edge paper',
    'pro-400h-crystal-v1': 'Fujifilm Pro 400H on Crystal Archive paper',
    'xtra-400-crystal-v1': 'Fujifilm X-Tra 400 on Crystal Archive paper',
    'vision3-250d-2383-v1': 'Kodak Vision3 250D on Vision 2383 print film',
    'vision3-500t-2393-v1': 'Kodak Vision3 500T on Vision Premier 2393 print film',
    'portra-800-push2-v1': 'Kodak Portra 800 pushed two stops, on Portra Endura paper',
    'kodachrome-64-v1': 'Kodak Kodachrome 64 slide film, scanned',
    'velvia-100-v1': 'Fujifilm Velvia 100 slide film, scanned',
    'portra-160-ultra-v1': 'Kodak Portra 160 on Ultra Endura paper',
    'trix-400-mono-v1': 'Kodak Tri-X 400 on Polymax Fine-Art paper',
    'red-filter-mono-v1': 'Tri-X 400 behind a Wratten 25 red filter',
    'fp-100c-v1': 'Fujifilm FP-100C instant film',
    'aerocolor-iv-v1': 'Kodak Aerocolor IV aerial negative on Portra Endura paper',
    'exr-200t-2383-v1': 'Kodak EXR 200T on Vision 2383 print film',
  } as Readonly<Record<string, string>>,
};

export type Strings = typeof en;

const zhHans: Strings = {
  language: '语言',
  tagline: '选择你的相机和喜欢的效果，下载安装包，然后复制到 SD 卡。',
  camera: '相机',
  firmware: '固件',
  supported: '已支持',
  planned: '计划支持',
  effects: '效果',
  modules: '模块',
  package: '安装包',
  install: '安装',
  previewing: '正在预览',
  samples: '示例照片',
  chart: '色卡',
  upload: '打开你的照片',
  yourPhoto: '你的照片',
  photoError: '无法打开这张照片。请使用 JPEG 或 PNG 文件。',
  previewError: '无法显示预览。',
  uploadNote: '照片只在本浏览器中处理，不会发送到服务器。',
  previewNote: '预览只是近似效果，相机拍出的照片可能略有不同。',
  compare: '对比',
  original: '原图',
  preview: '预览',
  previewOf: (name) => `预览 ${name}`,
  count: (max) => ['已选 ', ` / ${max}`],
  selectAll: '全选',
  clear: '清空',
  include: '包含',
  icon: '图标',
  effect: '效果',
  look: '目标风格',
  type: '类型',
  color: '彩色',
  mono: '黑白',
  effectCount: '效果数量',
  loading: '正在加载效果…',
  loadError: '无法加载效果。请刷新页面后重试。',
  firmwareNote: '下载前，请在相机菜单中确认固件版本。',
  download: '下载 GRKit.zip',
  building: '正在准备 GRKit.zip…',
  ready: 'GRKit.zip 已就绪。请把 SD_ROOT 中的内容复制到存储卡根目录。',
  buildError: '无法准备 GRKit.zip。请刷新页面后重试。',
  footer: '目前支持 RICOH GR III 固件 2.10。GR IIIx 和 GR IV 计划支持。',
  licenses: '效果许可',
  recovery: '恢复指南',
  source: '源代码',
  modulesText: {
    imageControl: '上面选择的效果，支持实时取景预览。',
    framing: '拍摄 XPan 65:24、16:9 或 4:3 照片。取景器保持 3:2，并遮暗照片外的区域。',
    guides: '在 Framing 区域内显示三分线、中心十字、黄金分割线或对角线。',
    dateStamp: '用橙色七段数码字体在照片上打印日期。',
  },
  steps: [
    ['关闭相机。备份 SD 卡，包括已有的启动脚本。'],
    ['解压 GRKit.zip。把 ', '`SD_ROOT', ' 中的内容复制到存储卡根目录。'],
    ['插入存储卡。如果尚未启用 SD 脚本，请按住 MENU 键开机，然后等待相机自动关机。'],
    ['在相机关机状态下，删除存储卡根目录中的 ', '`00078350.588', ' 和 ', '`DEVELOP.MOD', '。保留 ', '`SCRIPT', ' 目录。'],
    ['正常开机。打开 ADJ，选择第六个项目 GRK。'],
  ],
  looks: {
    'portra-400-endura-v1': 'Kodak Portra 400，Portra Endura 相纸',
    'gold-200-premier-v1': 'Kodak Gold 200，Endura Premier 相纸',
    'ektar-100-edge-v1': 'Kodak Ektar 100，Ektacolor Edge 相纸',
    'pro-400h-crystal-v1': 'Fujifilm Pro 400H，Crystal Archive 相纸',
    'xtra-400-crystal-v1': 'Fujifilm X-Tra 400，Crystal Archive 相纸',
    'vision3-250d-2383-v1': 'Kodak Vision3 250D，Vision 2383 拷贝片',
    'vision3-500t-2393-v1': 'Kodak Vision3 500T，Vision Premier 2393 拷贝片',
    'portra-800-push2-v1': 'Kodak Portra 800 增感两档，Portra Endura 相纸',
    'kodachrome-64-v1': 'Kodak Kodachrome 64 反转片，直接扫描',
    'velvia-100-v1': 'Fujifilm Velvia 100 反转片，直接扫描',
    'portra-160-ultra-v1': 'Kodak Portra 160，Ultra Endura 相纸',
    'trix-400-mono-v1': 'Kodak Tri-X 400，Polymax Fine-Art 相纸',
    'red-filter-mono-v1': 'Tri-X 400 加 Wratten 25 红滤镜',
    'fp-100c-v1': 'Fujifilm FP-100C 即显胶片',
    'aerocolor-iv-v1': 'Kodak Aerocolor IV 航拍负片，Portra Endura 相纸',
    'exr-200t-2383-v1': 'Kodak EXR 200T，Vision 2383 拷贝片',
  },
};

const zhHant: Strings = {
  language: '語言',
  tagline: '選擇你的相機和喜歡的效果，下載安裝包，然後複製到 SD 卡。',
  camera: '相機',
  firmware: '韌體',
  supported: '已支援',
  planned: '規劃中',
  effects: '效果',
  modules: '模組',
  package: '安裝包',
  install: '安裝',
  previewing: '正在預覽',
  samples: '範例照片',
  chart: '色卡',
  upload: '開啟你的照片',
  yourPhoto: '你的照片',
  photoError: '無法開啟這張照片。請使用 JPEG 或 PNG 檔案。',
  previewError: '無法顯示預覽。',
  uploadNote: '照片只在此瀏覽器中處理，不會傳送到伺服器。',
  previewNote: '預覽只是近似效果，相機拍出的照片可能略有不同。',
  compare: '對比',
  original: '原圖',
  preview: '預覽',
  previewOf: (name) => `預覽 ${name}`,
  count: (max) => ['已選 ', ` / ${max}`],
  selectAll: '全選',
  clear: '清除',
  include: '包含',
  icon: '圖示',
  effect: '效果',
  look: '目標風格',
  type: '類型',
  color: '彩色',
  mono: '黑白',
  effectCount: '效果數量',
  loading: '正在載入效果…',
  loadError: '無法載入效果。請重新整理頁面後再試。',
  firmwareNote: '下載前，請在相機選單中確認韌體版本。',
  download: '下載 GRKit.zip',
  building: '正在準備 GRKit.zip…',
  ready: 'GRKit.zip 已就緒。請把 SD_ROOT 中的內容複製到記憶卡根目錄。',
  buildError: '無法準備 GRKit.zip。請重新整理頁面後再試。',
  footer: '目前支援 RICOH GR III 韌體 2.10。GR IIIx 與 GR IV 規劃中。',
  licenses: '效果授權',
  recovery: '復原指南',
  source: '原始碼',
  modulesText: {
    imageControl: '上方選擇的效果，支援即時取景預覽。',
    framing: '拍攝 XPan 65:24、16:9 或 4:3 照片。取景器維持 3:2，並遮暗照片外的區域。',
    guides: '在 Framing 區域內顯示三分線、中心十字、黃金分割線或對角線。',
    dateStamp: '用橙色七段數字字型在照片上印上日期。',
  },
  steps: [
    ['關閉相機。備份 SD 卡，包括現有的啟動腳本。'],
    ['解壓縮 GRKit.zip。把 ', '`SD_ROOT', ' 中的內容複製到記憶卡根目錄。'],
    ['插入記憶卡。如果尚未啟用 SD 腳本，請按住 MENU 鍵開機，然後等待相機自動關機。'],
    ['在相機關機狀態下，刪除記憶卡根目錄中的 ', '`00078350.588', ' 和 ', '`DEVELOP.MOD', '。保留 ', '`SCRIPT', ' 目錄。'],
    ['正常開機。開啟 ADJ，選擇第六個項目 GRK。'],
  ],
  looks: {
    'portra-400-endura-v1': 'Kodak Portra 400，Portra Endura 相紙',
    'gold-200-premier-v1': 'Kodak Gold 200，Endura Premier 相紙',
    'ektar-100-edge-v1': 'Kodak Ektar 100，Ektacolor Edge 相紙',
    'pro-400h-crystal-v1': 'Fujifilm Pro 400H，Crystal Archive 相紙',
    'xtra-400-crystal-v1': 'Fujifilm X-Tra 400，Crystal Archive 相紙',
    'vision3-250d-2383-v1': 'Kodak Vision3 250D，Vision 2383 拷貝片',
    'vision3-500t-2393-v1': 'Kodak Vision3 500T，Vision Premier 2393 拷貝片',
    'portra-800-push2-v1': 'Kodak Portra 800 增感兩檔，Portra Endura 相紙',
    'kodachrome-64-v1': 'Kodak Kodachrome 64 正片，直接掃描',
    'velvia-100-v1': 'Fujifilm Velvia 100 正片，直接掃描',
    'portra-160-ultra-v1': 'Kodak Portra 160，Ultra Endura 相紙',
    'trix-400-mono-v1': 'Kodak Tri-X 400，Polymax Fine-Art 相紙',
    'red-filter-mono-v1': 'Tri-X 400 加 Wratten 25 紅濾鏡',
    'fp-100c-v1': 'Fujifilm FP-100C 即顯底片',
    'aerocolor-iv-v1': 'Kodak Aerocolor IV 航拍負片，Portra Endura 相紙',
    'exr-200t-2383-v1': 'Kodak EXR 200T，Vision 2383 拷貝片',
  },
};

export const STRINGS: Readonly<Record<Language, Strings>> = {
  en,
  'zh-Hans': zhHans,
  'zh-Hant': zhHant,
};

const STORAGE_KEY = 'grkit.language';

function isLanguage(value: unknown): value is Language {
  return LANGUAGES.some((language) => language.code === value);
}

/** Maps browser language tags to a supported language. Traditional script wins for TW, HK, and MO. */
export function matchLanguage(tags: readonly string[]): Language {
  for (const tag of tags) {
    const lower = tag.toLowerCase();
    if (lower.startsWith('zh')) {
      return /hant|-(tw|hk|mo)\b/.test(lower) ? 'zh-Hant' : 'zh-Hans';
    }
    if (lower.startsWith('en')) {
      return 'en';
    }
  }
  return 'en';
}

export function initialLanguage(): Language {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (isLanguage(saved)) {
      return saved;
    }
  } catch {
    // Storage can be unavailable, for example in a private window.
  }
  return matchLanguage(navigator.languages ?? [navigator.language]);
}

/** Applies a language to the document and remembers it for the next visit. */
export function applyLanguage(language: Language): void {
  document.documentElement.lang = language;
  try {
    localStorage.setItem(STORAGE_KEY, language);
  } catch {
    // The choice then lasts only for this page view.
  }
}
