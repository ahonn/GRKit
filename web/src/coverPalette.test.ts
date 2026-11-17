import { describe, expect, it } from 'vitest';
import {
  NEUTRAL_COLORS,
  colorsFromLook,
  displayColors,
  resolveColors,
} from './coverPalette';

/** Must stay equal to `artwork.rs` `NEUTRAL_PALETTE`. */
const RUST_NEUTRAL = {
  band: [214, 214, 214],
  brand: [40, 40, 40],
  body: [51, 51, 51],
  text: [255, 255, 255],
  accent: [214, 214, 214],
} as const;

describe('cover palette', () => {
  it('keeps the gray look identical to the Rust classic cover palette', () => {
    expect(colorsFromLook('gray')).toEqual(RUST_NEUTRAL);
    expect(NEUTRAL_COLORS).toEqual(RUST_NEUTRAL);
  });

  it('draws stored colors even when the shelf color-covers switch is off', () => {
    const colors = colorsFromLook('portra');
    const cover = { template: 'box' as const, brand: 'Kodak', name: 'PORTRA', number: '400', colors };
    expect(displayColors(cover, false)).toEqual(colors);
    expect(displayColors(cover, true)).toEqual(colors);
  });

  it('infers packaging colors only when colorful is on and the cover has no colors', () => {
    const cover = { template: 'box' as const, brand: 'Kodak', name: 'GOLD', number: '200' };
    expect(displayColors(cover, false)).toEqual(NEUTRAL_COLORS);
    expect(displayColors(cover, true)).toEqual(resolveColors(cover));
    expect(displayColors(cover, true)).toEqual(colorsFromLook('gold'));
  });
});
