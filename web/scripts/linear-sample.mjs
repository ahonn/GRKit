// Estimates the scene-linear pixels of a camera JPEG. The preview applies tables to scene-linear
// pixels, so a JPEG sample must start from them: this inverts the Standard development of the
// preview (preview/standard.lut) for each pixel. The baseline of the result then shows the JPEG,
// and each effect starts from the same pixels as on a RAW photo.
import jpeg from 'jpeg-js';
import { resizeLinear } from '../src/preview/linear.ts';
import { srgbCoordinate } from '../src/preview/lut.ts';

// Fixed-point steps of the inversion. More steps change the result by less than one 8-bit level.
const STEPS = 8;
// The tone curve of each channel on the neutral axis, sampled in the table's sRGB coordinate.
const SAMPLES = 4096;
const linear = (s) => (s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4);

/**
 * The linear picture of the JPEG `source`, resized as a RAW upload, and the mean difference
 * (of 255) between its Standard development and the JPEG. `table` is preview/standard.lut.
 */
export function linearSample(source, table) {
  const size = Math.round(Math.cbrt(table.length / 3));
  if (size ** 3 * 3 !== table.length) throw new Error('Expected an RGB table');

  /** The Standard development of linear RGB in 0..1, as 0..255 floats (the table, trilinear). */
  function develop(r, g, b, out) {
    const at = [r, g, b].map((v) => srgbCoordinate(Math.min(1, Math.max(0, v))) * (size - 1));
    const low = at.map((x) => Math.min(Math.floor(x), size - 2));
    const [fr, fg, fb] = at.map((x, c) => x - low[c]);
    const stride = [3, size * 3, size * size * 3];
    const base = low[0] * stride[0] + low[1] * stride[1] + low[2] * stride[2];
    for (let c = 0; c < 3; c++) {
      const t = (dr, dg, db) => table[base + c + dr * stride[0] + dg * stride[1] + db * stride[2]];
      const c00 = t(0, 0, 0) + (t(1, 0, 0) - t(0, 0, 0)) * fr;
      const c10 = t(0, 1, 0) + (t(1, 1, 0) - t(0, 1, 0)) * fr;
      const c01 = t(0, 0, 1) + (t(1, 0, 1) - t(0, 0, 1)) * fr;
      const c11 = t(0, 1, 1) + (t(1, 1, 1) - t(0, 1, 1)) * fr;
      const c0 = c00 + (c10 - c00) * fg;
      const c1 = c01 + (c11 - c01) * fg;
      out[c] = c0 + (c1 - c0) * fb;
    }
    return out;
  }

  const curves = [[], [], []];
  const out = [0, 0, 0];
  for (let i = 0; i <= SAMPLES; i++) {
    const v = linear(i / SAMPLES);
    develop(v, v, v, out);
    for (let c = 0; c < 3; c++) curves[c].push(out[c]);
  }
  /** The linear value whose neutral development in channel c is y (0..255). */
  function invert(c, y) {
    const curve = curves[c];
    let lo = 0;
    let hi = SAMPLES;
    while (hi - lo > 1) {
      const mid = (lo + hi) >> 1;
      if (curve[mid] < y) lo = mid;
      else hi = mid;
    }
    const span = curve[hi] - curve[lo];
    const s = (lo + (span > 0 ? Math.min(1, Math.max(0, (y - curve[lo]) / span)) : 0)) / SAMPLES;
    return linear(s);
  }

  // 1. Each channel through the inverse neutral curve, at full size, then the same area
  //    resampling in linear light as a RAW upload.
  const decoded = jpeg.decode(source, { useTArray: true, formatAsRGBA: false, maxMemoryUsageInMB: 1024 });
  const inverse = [0, 1, 2].map((c) => Uint16Array.from({ length: 256 }, (_, y) => Math.round(invert(c, y) * 65535)));
  const full = new Uint16Array(decoded.width * decoded.height * 3);
  for (let i = 0; i < full.length; i++) full[i] = inverse[i % 3][decoded.data[i]];
  const image = resizeLinear({ width: decoded.width, height: decoded.height, data: full });

  // 2. The full table mixes the channels and adds saturation. For each pixel, find the linear
  //    value x whose development is the neutral development of the resampled pixel x0:
  //    x += x0 - g(develop(x)), where g is the inverse neutral curve.
  let error = 0;
  const x = [0, 0, 0];
  const x0 = [0, 0, 0];
  for (let p = 0; p < image.data.length; p += 3) {
    for (let c = 0; c < 3; c++) x0[c] = x[c] = image.data[p + c] / 65535;
    const neutral = [0, 1, 2].map((c) => develop(x0[c], x0[c], x0[c], [0, 0, 0])[c]);
    for (let step = 0; step < STEPS; step++) {
      develop(x[0], x[1], x[2], out);
      for (let c = 0; c < 3; c++) x[c] = Math.min(1, Math.max(0, x[c] + x0[c] - invert(c, out[c])));
    }
    develop(x[0], x[1], x[2], out);
    for (let c = 0; c < 3; c++) {
      error += Math.abs(out[c] - neutral[c]);
      image.data[p + c] = Math.round(x[c] * 65535);
    }
  }
  return { image, meanError: error / image.data.length };
}
