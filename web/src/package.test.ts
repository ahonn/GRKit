import { describe, expect, it } from 'vitest';
import { packageFilename } from './package';

describe('packageFilename', () => {
  it('names a download by the version and the local time, with leading zeros', () => {
    expect(packageFilename('1.4.2', new Date(2026, 10, 5, 18, 30))).toBe('GRKit-1.4.2-20261105-1830.zip');
    expect(packageFilename('1.5.0', new Date(2027, 0, 2, 3, 4))).toBe('GRKit-1.5.0-20270102-0304.zip');
  });
});
