/**
 * Applies an 8-bit RGB table with `size` points on each axis, red fastest, to RGBA pixels.
 * Trilinear interpolation. Alpha stays unchanged.
 */
export function applyLut(pixels: Uint8ClampedArray, table: Uint8Array, size: number): void {
  if (table.length !== size * size * size * 3) {
    throw new Error('Preview table size mismatch');
  }
  const scale = (size - 1) / 255;
  const stride = [3, size * 3, size * size * 3] as const;
  // Per input level: the lower grid index and the weight of the upper index.
  const low = new Uint16Array(256);
  const weight = new Float32Array(256);
  for (let v = 0; v < 256; v++) {
    const x = v * scale;
    const i = Math.min(Math.floor(x), size - 2);
    low[v] = i;
    weight[v] = x - i;
  }
  for (let p = 0; p < pixels.length; p += 4) {
    const r = pixels[p]!;
    const g = pixels[p + 1]!;
    const b = pixels[p + 2]!;
    const fr = weight[r]!;
    const fg = weight[g]!;
    const fb = weight[b]!;
    const base = low[r]! * stride[0] + low[g]! * stride[1] + low[b]! * stride[2];
    for (let c = 0; c < 3; c++) {
      const o = base + c;
      const c00 = table[o]! + (table[o + stride[0]]! - table[o]!) * fr;
      const c10 =
        table[o + stride[1]]! + (table[o + stride[1] + stride[0]]! - table[o + stride[1]]!) * fr;
      const c01 =
        table[o + stride[2]]! + (table[o + stride[2] + stride[0]]! - table[o + stride[2]]!) * fr;
      const c11 =
        table[o + stride[2] + stride[1]]! +
        (table[o + stride[2] + stride[1] + stride[0]]! - table[o + stride[2] + stride[1]]!) * fr;
      const c0 = c00 + (c10 - c00) * fg;
      const c1 = c01 + (c11 - c01) * fg;
      pixels[p + c] = c0 + (c1 - c0) * fb;
    }
  }
}
