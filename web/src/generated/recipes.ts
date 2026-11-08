// Generated from crates/grkit-core/src/recipes/. Do not edit: change the
// presets there, then run
// GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated.

import type { Preset } from '../recipes';

/** The recipes that the site offers. */
export const PRESETS: readonly Preset[] = [
  {
    "id": "bennet-800",
    "effect": null,
    "author": "Bennet",
    "source": "https://www.reddit.com/r/ricohGR/comments/1k27hmk/",
    "recipes": [
      {
        "name": "Bennet 800",
        "family": "gr3",
        "look": {
          "kind": "native",
          "imageType": 6,
          "levels": [
            7,
            3,
            4,
            6,
            2,
            0,
            4,
            4,
            3
          ],
          "tone": 0,
          "intensity": 0,
          "filter": 0,
          "grainStrength": 0,
          "grainSize": 0,
          "solarization": 0
        },
        "grain": {
          "level": 0,
          "size": 0
        },
        "glow": {
          "bloom": 0,
          "halation": 0,
          "halationColor": 0
        },
        "whiteBalance": {
          "mode": 0,
          "shiftBa": 28,
          "shiftGm": 12,
          "kelvin": 5500
        },
        "corrections": [
          1,
          4,
          0,
          0
        ],
        "exposure": {
          "compensation": null,
          "isoLimit": null,
          "metering": null,
          "nd": null
        },
        "enabled": true,
        "hidden": false,
        "number": 0
      }
    ]
  },
  {
    "id": "reggie-negative",
    "effect": null,
    "author": "Reggie Ballesteros",
    "source": "https://www.reddit.com/r/ricohGR/comments/180a8nt/",
    "recipes": [
      {
        "name": "Reggie Negative",
        "family": "gr3",
        "look": {
          "kind": "native",
          "imageType": 6,
          "levels": [
            6,
            4,
            4,
            7,
            0,
            3,
            5,
            4,
            4
          ],
          "tone": 0,
          "intensity": 0,
          "filter": 0,
          "grainStrength": 0,
          "grainSize": 0,
          "solarization": 0
        },
        "grain": {
          "level": 0,
          "size": 0
        },
        "glow": {
          "bloom": 0,
          "halation": 0,
          "halationColor": 0
        },
        "whiteBalance": {
          "mode": 0,
          "shiftBa": 20,
          "shiftGm": 14,
          "kelvin": 5500
        },
        "corrections": [
          0,
          2,
          0,
          0
        ],
        "exposure": {
          "compensation": null,
          "isoLimit": null,
          "metering": null,
          "nd": null
        },
        "enabled": true,
        "hidden": false,
        "number": 0
      }
    ]
  },
  {
    "id": "cinestill-800t",
    "effect": "vision3-500t-supra-v1",
    "author": null,
    "source": null,
    "recipes": [
      {
        "name": "CineStill 800T",
        "family": "gr3",
        "look": {
          "kind": "effect",
          "code": 18750,
          "adjustments": [
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0
          ]
        },
        "grain": {
          "level": 2,
          "size": 0
        },
        "glow": {
          "bloom": 0,
          "halation": 2,
          "halationColor": 0
        },
        "whiteBalance": {
          "mode": 0,
          "shiftBa": 4,
          "shiftGm": 14,
          "kelvin": 5500
        },
        "corrections": [
          0,
          4,
          0,
          1
        ],
        "exposure": {
          "compensation": null,
          "isoLimit": null,
          "metering": null,
          "nd": null
        },
        "enabled": true,
        "hidden": false,
        "number": 0
      },
      {
        "name": "CineStill 800T",
        "family": "gr4",
        "look": {
          "kind": "effect",
          "code": 18750,
          "adjustments": [
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0
          ]
        },
        "grain": {
          "level": 2,
          "size": 0
        },
        "glow": {
          "bloom": 0,
          "halation": 2,
          "halationColor": 0
        },
        "whiteBalance": {
          "mode": 0,
          "shiftBa": 4,
          "shiftGm": 14,
          "kelvin": 5500
        },
        "corrections": [
          0,
          4,
          0,
          1
        ],
        "exposure": {
          "compensation": null,
          "isoLimit": null,
          "metering": null,
          "nd": null
        },
        "enabled": true,
        "hidden": false,
        "number": 0
      }
    ]
  }
];

/** The levels of a native look in portable order (`native::TYPES`). */
export const LEVEL_KEYS: readonly string[] = ["Saturation","ColorHue","ImageKey","Contrast","ContrastHighLight","ContrastShadow","Sharpness","Shading","ClarityControl"];

/** The labels of each white balance mode in portable order: GR III, then GR IV, or null. */
export const WHITE_BALANCE_MODES: readonly (readonly (string | null)[])[] = [["Auto","AutoStandard"],["Multi",null],[null,"AutoWarm"],[null,"AutoWhite"],["Daylight","Daylight"],["Shade","Shade"],["Cloudiness","Cloudiness"],["Flu_D","Flu_D"],["Flu_N","Flu_N"],["Flu_W","Flu_W"],["Flu_L","Flu_L"],["Tungsten","Tungsten"],["Cte","Cte"],["Manual1","Manual1"],["ColorTemp1","ColorTemp1"],["Custom1","Custom1"],["Custom2","Custom2"],["Custom3","Custom3"]];

/** The labels of the stored values of each correction setting, in portable order. */
export const CORRECTION_VALUES: readonly (readonly string[])[] = [["Auto","On","Off"],["Auto","Low","Mid","High","Off"],["Off","On"],["Auto","Low","Mid","High","Custom","Off"]];

/** The camera names of the types in each site language, by portable index. */
export const TYPE_NAMES: Readonly<Record<string, readonly string[]>> = {
  'en': ["Standard","Vivid","Monotone","Soft Monotone","Hard Monotone","Hi-Contrast B&W","Negative Film","Positive Film","Bleach Bypass","Retro","HDR Tone","Cross Processing","Cinema (Yellow)","Cinema (Green)","Standard","Soft","Hi-Contrast","Grainy","Solid","HDR Tone","Solarized Effect"],
  'zh-Hans': ["标准","鲜艳","单色","柔和单色","硬调单色","高对比度黑白","负片","正片","漂白效果","复古","HDR 色调","交叉冲印","电影（黄）","电影（绿）","标准","柔和","高对比度","颗粒感","无颗粒感","HDR色调","中途曝光效果"],
  'zh-Hant': ["標準","鮮豔","單色","柔和單色","硬調單色","高對比度黑白","負片","正片","漂白效果","復古","HDR 色調","交叉沖印","電影（黃）","電影（綠）","標準","柔和","高對比度","顆粒感","無顆粒感","HDR色調","中途曝光效果"],
  'ja': ["スタンダード","ビビッド","モノトーン","ソフトモノトーン","ハードモノトーン","ハイコントラスト白黒","ネガフィルム調","ポジフィルム調","ブリーチバイパス","レトロ","HDR調","クロスプロセス","シネマ調(イエロー)","シネマ調(グリーン)","スタンダード","ソフト","ハイコントラスト","グレイニー","ソリッド","HDR調","ソラリゼーション調"],
  'ko': ["스탠다드","비비드","모노톤","소프트 모노톤","하드 모노톤","하이 콘트라스트 흑백","네거티브 필름","포지티브 필름","블리치 바이패스","레트로","HDR 톤","크로스 프로세스","시네마(옐로)","시네마(그린)","표준","소프트","하이 콘트라스트","그레이니","솔리드","HDR톤","솔라리제이션톤"],
};
