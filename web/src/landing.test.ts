import { describe, expect, it } from 'vitest';
import { guideLines } from './landing';

describe('guideLines', () => {
  it('starts the golden spiral at the bottom-left corner and keeps it in the frame', () => {
    const lines = guideLines('spiral', 1.5);
    // The first division line, then the first quarter ellipse from the corner.
    expect(lines[0]).toEqual([
      [61.8, 0],
      [61.8, 100],
    ]);
    const [x, y] = lines[1]![0]!;
    expect(x).toBeCloseTo(0);
    expect(y).toBeCloseTo(100);
    for (const line of lines) {
      for (const [px, py] of line) {
        expect(px).toBeGreaterThanOrEqual(-1e-9);
        expect(px).toBeLessThanOrEqual(100 + 1e-9);
        expect(py).toBeGreaterThanOrEqual(-1e-9);
        expect(py).toBeLessThanOrEqual(100 + 1e-9);
      }
    }
  });

  it('draws the dynamic symmetry reciprocals for the frame shape', () => {
    // In 3:2 a reciprocal from the top-left corner meets the bottom at (2/3)^2 of the width.
    const [, , reciprocal] = guideLines('symmetry', 1.5);
    expect(reciprocal![1]![0]).toBeCloseTo((100 * 4) / 9);
    // In 1:1 the reciprocals are the diagonals.
    const square = guideLines('symmetry', 1);
    expect(square[2]).toEqual([
      [0, 0],
      [100, 100],
    ]);
  });
});
