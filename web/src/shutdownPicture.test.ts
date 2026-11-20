import { describe, expect, it } from 'vitest';
import { coverCrop, pictureChange } from './shutdownPicture';

describe('the shutdown picture', () => {
  it('takes the center of a source at the ratio of the screen', () => {
    expect(coverCrop(720, 480)).toEqual({ x: 0, y: 0, width: 720, height: 480 });
    // A portrait photo keeps its width, and a wide one its height.
    expect(coverCrop(3000, 4500)).toEqual({ x: 0, y: 1250, width: 3000, height: 2000 });
    expect(coverCrop(4000, 1000)).toEqual({ x: 1250, y: 0, width: 1500, height: 1000 });
  });

  it('changes the card only for a new file or a file to remove', () => {
    const file = new Uint8Array([1, 2, 3]);
    expect(pictureChange(file, null)).toBe('same');
    expect(pictureChange(null, 'remove')).toBe('same');
    expect(pictureChange(file, 'remove')).toBe('remove');
    expect(pictureChange(file, new Uint8Array([1, 2, 3]))).toBe('same');
    expect(pictureChange(file, new Uint8Array([1, 2, 4]))).toBe('write');
    expect(pictureChange(null, file)).toBe('write');
  });
});
