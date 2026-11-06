import { describe, expect, it } from 'vitest';
import { packageFilename } from './package';

describe('packageFilename', () => {
  it('names a download by the version, the camera, and the local time, with leading zeros', () => {
    expect(packageFilename('1.4.2', 'GR III / GR III HDF', new Date(2026, 10, 5, 18, 30))).toBe(
      'GRKit-1.4.2-GR-III-20261105-1830.zip',
    );
    expect(packageFilename('1.5.0', 'GR IV Monochrome', new Date(2027, 0, 2, 3, 4))).toBe(
      'GRKit-1.5.0-GR-IV-Monochrome-20270102-0304.zip',
    );
  });

  it('names each camera entry by its first model', () => {
    const time = new Date(2026, 10, 7, 9, 12);
    expect(packageFilename('1.4.4', 'GR IIIx / GR IIIx HDF', time)).toBe('GRKit-1.4.4-GR-IIIx-20261107-0912.zip');
    expect(packageFilename('1.4.4', 'GR IV / GR IV HDF', time)).toBe('GRKit-1.4.4-GR-IV-20261107-0912.zip');
  });
});
