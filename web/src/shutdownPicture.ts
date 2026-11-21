import type { CardChanges } from './card';
import { loadWasm } from './package';

/** The size of the picture of Shutdown Screen (`grkit_core::goodbye`). */
export const PICTURE_WIDTH = 720;
export const PICTURE_HEIGHT = 480;
/** The name of the file in the `GRKIT` folder of the card. */
export const PICTURE_NAME = 'GOODBYE.JPG';

/**
 * The shutdown picture of the page: the file that the user made from a picture, `remove` to
 * delete the file of the card, or null to leave the card as it is.
 */
export type PictureChoice = Uint8Array | 'remove' | null;

/** The part of a source of `width` by `height` that fills the picture: centered, at its ratio. */
export function coverCrop(width: number, height: number): { x: number; y: number; width: number; height: number } {
  const wide = width * PICTURE_HEIGHT > height * PICTURE_WIDTH;
  const crop = wide
    ? { width: (height * PICTURE_WIDTH) / PICTURE_HEIGHT, height }
    : { width, height: (width * PICTURE_HEIGHT) / PICTURE_WIDTH };
  return { x: (width - crop.width) / 2, y: (height - crop.height) / 2, ...crop };
}

/**
 * The file of the picture in `source`, an image file of any size: the center of the picture at
 * the ratio of the screen, as a JPEG that the camera shows. It fails for a file that the browser
 * cannot decode.
 */
export async function makePicture(source: Blob): Promise<Uint8Array> {
  const bitmap = await createImageBitmap(source, { imageOrientation: 'from-image' });
  try {
    const canvas = new OffscreenCanvas(PICTURE_WIDTH, PICTURE_HEIGHT);
    const context = canvas.getContext('2d')!;
    const crop = coverCrop(bitmap.width, bitmap.height);
    context.imageSmoothingQuality = 'high';
    context.drawImage(bitmap, crop.x, crop.y, crop.width, crop.height, 0, 0, PICTURE_WIDTH, PICTURE_HEIGHT);
    const pixels = context.getImageData(0, 0, PICTURE_WIDTH, PICTURE_HEIGHT).data;
    const wasm = await loadWasm();
    return wasm.shutdownPicture(new Uint8Array(pixels.buffer, pixels.byteOffset, pixels.byteLength));
  } finally {
    bitmap.close();
  }
}

/** Whether the camera shows `bytes`, the file of a card. */
export async function isPicture(bytes: Uint8Array): Promise<boolean> {
  return (await loadWasm()).shutdownPictureUsable(bytes);
}

/** The change of the file of the card, `onCard`, that a write makes for `choice`. */
export function pictureChange(onCard: Uint8Array | null, choice: PictureChoice): CardChanges['shutdown'] {
  if (choice === null) return 'same';
  if (choice === 'remove') return onCard ? 'remove' : 'same';
  const same = onCard !== null && onCard.length === choice.length && onCard.every((byte, i) => byte === choice[i]);
  return same ? 'same' : 'write';
}
