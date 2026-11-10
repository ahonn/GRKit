import { siteUrl } from './catalog';
import type { Language } from './i18n';
export { LANDING_LOOKS, LANDING_PHOTOS } from './landing-data';
import menuEn from '../../resources/images/grkit-menu.jpg';
import menuJa from '../../resources/images/grkit-menu.ja.jpg';
import menuKo from '../../resources/images/grkit-menu.ko.jpg';
import menuZhHans from '../../resources/images/grkit-menu.zh-Hans.jpg';
import menuZhHant from '../../resources/images/grkit-menu.zh-Hant.jpg';
import rootEn from '../../resources/images/grkit-root.jpg';
import rootJa from '../../resources/images/grkit-root.ja.jpg';
import rootKo from '../../resources/images/grkit-root.ko.jpg';
import rootZhHans from '../../resources/images/grkit-root.zh-Hans.jpg';
import rootZhHant from '../../resources/images/grkit-root.zh-Hant.jpg';

/** The Framing ratios in the order of the camera menu, with their width over height. */
export const LANDING_RATIOS = [
  { label: 'XPan 65:24', ratio: 65 / 24 },
  { label: '16:9', ratio: 16 / 9 },
  { label: '4:3', ratio: 4 / 3 },
  { label: '2.35:1', ratio: 2.35 },
  { label: '1:1', ratio: 1 },
  { label: '5:4', ratio: 5 / 4 },
  { label: '7:6', ratio: 7 / 6 },
] as const;

/** The id of the landing hero. The App Store bar waits until the hero scrolls away (AppPromo). */
export const LANDING_HERO = 'landing-hero';

/** The guide styles in the order of the camera menu (`landing.guideRows`). */
export const LANDING_GUIDES = ['off', 'thirds', 'center', 'golden', 'diagonals', 'spiral', 'symmetry'] as const;
export type LandingGuide = (typeof LANDING_GUIDES)[number];

/** The image of the landing page with the name `name` in public/landing/. */
export function landingImage(name: string): string {
  return siteUrl(`landing/${name}`);
}

/** The camera screens of the README in each language: the ADJ page and the MENU section. */
export const CAMERA_SCREENS: Record<Language, { adj: string; menu: string }> = {
  en: { adj: rootEn, menu: menuEn },
  'zh-Hans': { adj: rootZhHans, menu: menuZhHans },
  'zh-Hant': { adj: rootZhHant, menu: menuZhHant },
  ja: { adj: rootJa, menu: menuJa },
  ko: { adj: rootKo, menu: menuKo },
};

/** A guide line through points, in percent of the frame width and height. */
export type GuideLine = readonly (readonly [number, number])[];

/** The part of the rest of the box that each step of the golden spiral cuts. */
const SPIRAL_CUT = 0.618;
/** The golden spiral stops before a part narrower than this, in percent of the frame. */
const SPIRAL_END = 1.5;

/**
 * The guide lines inside a frame of `aspect` (width / height), in percent of its width and
 * height, as the Guides module draws them (modules/guides/guides.c).
 */
export function guideLines(guide: LandingGuide, aspect: number): readonly GuideLine[] {
  const at = (p: number): GuideLine[] => [
    [
      [p, 0],
      [p, 100],
    ],
    [
      [100 - p, 0],
      [100 - p, 100],
    ],
    [
      [0, p],
      [100, p],
    ],
    [
      [0, 100 - p],
      [100, 100 - p],
    ],
  ];
  const diagonals: GuideLine[] = [
    [
      [0, 0],
      [100, 100],
    ],
    [
      [100, 0],
      [0, 100],
    ],
  ];
  switch (guide) {
    case 'thirds':
      return at(100 / 3);
    case 'golden':
      return at(38.2);
    case 'center':
      return [
        [
          [46, 50],
          [54, 50],
        ],
        [
          [50, 44],
          [50, 56],
        ],
      ];
    case 'diagonals':
      return diagonals;
    case 'spiral':
      return goldenSpiral();
    case 'symmetry': {
      // Each reciprocal starts at a corner, is perpendicular to the other diagonal, and ends on
      // the opposite side. In a 1:1 frame it is the diagonal.
      const across = Math.min(100, 100 / (aspect * aspect));
      return [
        ...diagonals,
        [
          [0, 0],
          [across, 100],
        ],
        [
          [100, 0],
          [100 - across, 100],
        ],
        [
          [0, 100],
          [across, 0],
        ],
        [
          [100, 100],
          [100 - across, 0],
        ],
      ];
    }
    default:
      return [];
  }
}

/**
 * The golden spiral, stretched to the frame, with its division lines. Each step cuts 0.618 of the
 * rest of the frame, in turn on the left, the top, the right, and the bottom, and draws a quarter
 * ellipse in the part that it cuts. The spiral starts at the bottom-left corner.
 */
function goldenSpiral(): GuideLine[] {
  const lines: GuideLine[] = [];
  let [l, t, r, b] = [0, 0, 100, 100];
  for (let k = 0; r - l >= SPIRAL_END && b - t >= SPIRAL_END; ++k) {
    const quarter = k % 4;
    const sineX = quarter % 2 === 1;
    const signX = quarter === 0 || quarter === 3 ? -1 : 1;
    const signY = quarter < 2 ? -1 : 1;
    let cx: number, cy: number, rx: number, ry: number;
    switch (quarter) {
      case 0:
        cx = l + (r - l) * SPIRAL_CUT;
        cy = b;
        rx = cx - l;
        ry = b - t;
        lines.push([
          [cx, t],
          [cx, b],
        ]);
        l = cx;
        break;
      case 1:
        cx = l;
        cy = t + (b - t) * SPIRAL_CUT;
        rx = r - l;
        ry = cy - t;
        lines.push([
          [l, cy],
          [r, cy],
        ]);
        t = cy;
        break;
      case 2:
        cx = r - (r - l) * SPIRAL_CUT;
        cy = t;
        rx = r - cx;
        ry = b - t;
        lines.push([
          [cx, t],
          [cx, b],
        ]);
        r = cx;
        break;
      default:
        cx = r;
        cy = b - (b - t) * SPIRAL_CUT;
        rx = r - l;
        ry = b - cy;
        lines.push([
          [l, cy],
          [r, cy],
        ]);
        b = cy;
    }
    const arc: [number, number][] = [];
    for (let i = 0; i <= 16; ++i) {
      const angle = (i / 16) * (Math.PI / 2);
      const sine = Math.sin(angle);
      const cosine = Math.cos(angle);
      arc.push([cx + signX * rx * (sineX ? sine : cosine), cy + signY * ry * (sineX ? cosine : sine)]);
    }
    lines.push(arc);
  }
  return lines;
}

/**
 * The width of a crop of `ratio` inside the 3:2 display, in percent of the display width: the
 * full width for a wider ratio, the full height for a narrower one.
 */
export function cropWidth(ratio: number): number {
  return ratio >= 1.5 ? 100 : (ratio / 1.5) * 100;
}
