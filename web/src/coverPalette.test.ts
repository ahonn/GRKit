import { describe, expect, it } from 'vitest';
import { NEUTRAL_COLORS, colorsFromLook } from './coverPalette';

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
});
