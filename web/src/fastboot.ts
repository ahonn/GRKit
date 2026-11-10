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
 * GR IV Monochrome 1.11 has the official file of GR IV 1.21 with its own version, so it has an entry
 * of that release with its own `name` and page. `file` is the name of the update file, which the
 * camera looks for in the card root.
 */
export const LATEST: readonly { release: string; page: string; file: string; name?: string }[] = [
  { release: 'gr3-2.21', file: 'fwdc239b.bin', page: 'https://www.ricoh-imaging.co.jp/english/support/digital/gr3_s.html' },
  { release: 'gr3x-1.71', file: 'fwdc243b.bin', page: 'https://www.ricoh-imaging.co.jp/english/support/digital/gr3x_s.html' },
  { release: 'gr4-1.21', file: 'fwdc248b.bin', page: 'https://www.ricoh-imaging.co.jp/english/support/digital/gr4_s.html' },
  {
    release: 'gr4-1.21',
    file: 'fwdc248b.bin',
    name: 'GR IV Monochrome 1.11',
    page: 'https://www.ricoh-imaging.co.jp/english/support/digital/gr-4-mono_s.html',
  },
];

/** The camera and version of an entry of `LATEST`, such as `GR IV Monochrome 1.11`. */
export function latestName(item: (typeof LATEST)[number]): string {
  return item.name ?? releaseName(item.release);
}

/** The camera of an entry of `LATEST` without its version, such as `GR IV Monochrome`. */
export function latestCamera(item: (typeof LATEST)[number]): string {
  return latestName(item).replace(/ [\d.]+$/, '');
}

/** The cameras and versions that the page makes, as `latestName` names them. */
export const LATEST_NAMES: readonly string[] = LATEST.map(latestName);

/**
 * The cameras and versions whose official file is the file of `release`, such as
 * `GR IV 1.21 / GR IV Monochrome 1.11`. A release that the page does not make has its `releaseName`.
 */
export function firmwareName(release: string): string {
  const names = LATEST.filter((item) => item.release === release).map(latestName);
  return names.length ? names.join(' / ') : releaseName(release);
}

/** The latest release of the camera of `release`, or null when `release` is the latest. */
export function newerRelease(release: string): string | null {
  const camera = release.split('-')[0];
  const latest = LATEST.find((item) => item.release.split('-')[0] === camera)?.release;
  return latest && latest !== release ? latest : null;
}

export type FastbootResult =
  /** `fileName` is the name of the official update file, which the camera looks for. */
  | { state: 'ready'; release: string; fileName: string }
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
    const fileName = firmware.fileName;
    save(new Blob([firmware.bytes as Uint8Array<ArrayBuffer>]), fileName);
    return { state: 'ready', release: firmware.release, fileName };
  } finally {
    firmware.free();
  }
}
