import { describe, expect, it } from 'vitest';
import { applyLut } from './lut';

function table(size: number, map: (rgb: [number, number, number]) => [number, number, number]) {
  const result = new Uint8Array(size ** 3 * 3);
  const level = (i: number) => Math.round((i * 255) / (size - 1));
  for (let b = 0; b < size; b++) {
    for (let g = 0; g < size; g++) {
      for (let r = 0; r < size; r++) {
        result.set(map([level(r), level(g), level(b)]), ((b * size + g) * size + r) * 3);
      }
    }
  }
  return result;
}

describe('applyLut', () => {
  it('keeps pixels with an identity table and keeps alpha', () => {
    const pixels = new Uint8ClampedArray([0, 0, 0, 255, 255, 255, 255, 10, 12, 130, 201, 77]);
    const before = pixels.slice();
    applyLut(pixels, table(33, (rgb) => rgb), 33);
    for (let i = 0; i < pixels.length; i++) {
      expect(Math.abs(pixels[i]! - before[i]!)).toBeLessThanOrEqual(1);
    }
    expect(pixels[7]).toBe(10);
  });

  it('reads the table with red fastest', () => {
    const pixels = new Uint8ClampedArray([255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255]);
    applyLut(pixels, table(2, ([r, g, b]) => [b, r, g]), 2);
    expect([...pixels]).toEqual([0, 255, 0, 255, 0, 0, 255, 255, 255, 0, 0, 255]);
  });

  it('interpolates between grid points', () => {
    const pixels = new Uint8ClampedArray([128, 128, 128, 255]);
    applyLut(pixels, table(2, ([r]) => [r, 0, 255 - r]), 2);
    expect([...pixels]).toEqual([128, 0, 127, 255]);
  });

  it('rejects a table of another size', () => {
    expect(() => applyLut(new Uint8ClampedArray(4), new Uint8Array(10), 33)).toThrow();
  });
});
