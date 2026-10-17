import { applyLut } from './lut';

/** An image to preview. The key identifies it in the caches. */
export interface Source {
  key: string;
  load: () => Promise<Blob>;
}

export interface PreviewEffect {
  id: string;
  /** The URL of the effect's preview table. */
  table: string;
  size: number;
}

export interface Picture {
  /** The unprocessed image at full size. */
  url: string;
  pixels: ImageData;
}

export interface Preview {
  original: string;
  processed: string;
}

/** The long edge of the processed image, in pixels. */
const MAX_EDGE = 1600;
/** Processed images to keep. The current one is always the newest. */
const MAX_RESULTS = 12;

const pictures = new Map<string, Promise<Picture>>();
const tables = new Map<string, Promise<Uint8Array>>();
const results = new Map<string, Promise<Preview>>();

async function decode(source: Source): Promise<Picture> {
  const blob = await source.load();
  const bitmap = await createImageBitmap(blob, { imageOrientation: 'from-image' });
  try {
    const scale = Math.min(1, MAX_EDGE / Math.max(bitmap.width, bitmap.height));
    const width = Math.max(1, Math.round(bitmap.width * scale));
    const height = Math.max(1, Math.round(bitmap.height * scale));
    const canvas = new OffscreenCanvas(width, height);
    const context = canvas.getContext('2d', { willReadFrequently: true });
    if (!context) {
      throw new Error('2D canvas is not available');
    }
    context.drawImage(bitmap, 0, 0, width, height);
    return { url: URL.createObjectURL(blob), pixels: context.getImageData(0, 0, width, height) };
  } finally {
    bitmap.close();
  }
}

/** Decodes a source once. A failed decode is not cached, so the source can be tried again. */
export function picture(source: Source): Promise<Picture> {
  let result = pictures.get(source.key);
  if (!result) {
    result = decode(source);
    pictures.set(source.key, result);
    result.catch(() => pictures.delete(source.key));
  }
  return result;
}

async function fetchTable(url: string): Promise<Uint8Array> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Preview table request failed: ${response.status}`);
  }
  return new Uint8Array(await response.arrayBuffer());
}

function table(url: string): Promise<Uint8Array> {
  let result = tables.get(url);
  if (!result) {
    result = fetchTable(url);
    tables.set(url, result);
    result.catch(() => tables.delete(url));
  }
  return result;
}

async function process(source: Source, effect: PreviewEffect): Promise<Preview> {
  const [image, lut] = await Promise.all([picture(source), table(effect.table)]);
  const { width, height } = image.pixels;
  const pixels = new ImageData(new Uint8ClampedArray(image.pixels.data), width, height);
  applyLut(pixels.data, lut, effect.size);
  const canvas = new OffscreenCanvas(width, height);
  canvas.getContext('2d')?.putImageData(pixels, 0, 0);
  const blob = await canvas.convertToBlob({ type: 'image/jpeg', quality: 0.9 });
  return { original: image.url, processed: URL.createObjectURL(blob) };
}

function release(key: string, result: Promise<Preview>): void {
  results.delete(key);
  result.then((preview) => URL.revokeObjectURL(preview.processed)).catch(() => undefined);
}

/**
 * The preview of one source with one effect. The same arguments return the same promise,
 * so a component can pass the result to `use()`.
 */
export function preview(source: Source, effect: PreviewEffect): Promise<Preview> {
  const key = `${source.key}\n${effect.id}`;
  let result = results.get(key);
  if (result) {
    // Map order is the use order: move the entry to the end.
    results.delete(key);
  } else {
    result = process(source, effect);
    result.catch(() => results.delete(key));
  }
  results.set(key, result);
  for (const [oldKey, oldResult] of results) {
    if (results.size <= MAX_RESULTS) {
      break;
    }
    release(oldKey, oldResult);
  }
  return result;
}

/** Releases a source that the page no longer shows, such as a replaced photo. */
export function forget(source: Source): void {
  const image = pictures.get(source.key);
  pictures.delete(source.key);
  image?.then((value) => URL.revokeObjectURL(value.url)).catch(() => undefined);
  for (const [key, result] of results) {
    if (key.startsWith(`${source.key}\n`)) {
      release(key, result);
    }
  }
}
