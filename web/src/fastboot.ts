import { loadWasm, save } from './package';

/** The largest file that the page reads: an official ZIP has about 35 MB, its update file about 40 MB. */
export const MAX_FIRMWARE_SIZE = 64 * 1024 * 1024;

const CAMERA_NAMES: Readonly<Record<string, string>> = { gr3: 'GR III', gr3x: 'GR IIIx', gr4: 'GR IV' };

/** The camera and version of a release of `grkit_firmware`, such as `gr3-2.10`: `GR III 2.10`. */
export function releaseName(release: string): string {
  const [camera = '', version = ''] = release.split('-');
  return `${CAMERA_NAMES[camera] ?? camera} ${version}`;
}

/**
 * The release of each camera that the page makes the fast-boot firmware of, the latest, and
 * Ricoh's support page of the camera, which has it. HDF models share the page. A camera on an older
 * version installs the fast-boot firmware as an update. `grkit firmware patch` makes all releases.
 */
export const LATEST: readonly { release: string; page: string }[] = [
  { release: 'gr3-2.20', page: 'https://www.ricoh-imaging.co.jp/english/support/digital/gr3_s.html' },
  { release: 'gr3x-1.70', page: 'https://www.ricoh-imaging.co.jp/english/support/digital/gr3x_s.html' },
  { release: 'gr4-1.20', page: 'https://www.ricoh-imaging.co.jp/english/support/digital/gr4_s.html' },
];

/** The releases that the page makes, as `releaseName` names them. */
export const LATEST_NAMES: readonly string[] = LATEST.map((item) => releaseName(item.release));

/** The latest release of the camera of `release`, or null when `release` is the latest. */
export function newerRelease(release: string): string | null {
  const camera = release.split('-')[0];
  const latest = LATEST.find((item) => item.release.split('-')[0] === camera)?.release;
  return latest && latest !== release ? latest : null;
}

export type FastbootResult =
  | { state: 'ready'; release: string }
  /** The official update file of an older release: the page asks for `latest`. */
  | { state: 'outdated'; release: string; latest: string }
  /** The file is not the official update file of a supported release. */
  | { state: 'unknown' };

/**
 * Makes the fast-boot firmware of the official ZIP or update file of a latest release in the
 * browser and downloads it under the name of the official update file. The file is not uploaded.
 */
export async function makeFastbootFirmware(file: Blob): Promise<FastbootResult> {
  if (file.size > MAX_FIRMWARE_SIZE) return { state: 'unknown' };
  const [wasm, bytes] = await Promise.all([loadWasm(), file.arrayBuffer()]);
  // Undefined for a file that is not an official firmware file. A failure of the patch itself
  // throws, and the page shows it as an error.
  const firmware = wasm.patchFirmware(new Uint8Array(bytes));
  if (!firmware) return { state: 'unknown' };
  try {
    const latest = newerRelease(firmware.release);
    if (latest) return { state: 'outdated', release: firmware.release, latest };
    save(new Blob([firmware.bytes as Uint8Array<ArrayBuffer>]), firmware.fileName);
    return { state: 'ready', release: firmware.release };
  } finally {
    firmware.free();
  }
}
