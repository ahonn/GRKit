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
 * The latest release of each camera and Ricoh's support page of the camera, which has it. HDF
 * models share the page. The page recommends these releases, and also makes the fast-boot firmware
 * of the older releases in `RELEASES`. GR IV Monochrome 1.11 has the official file of GR IV 1.21
 * with its own version, so it has an entry of that release with its own `name` and page. `file` is
 * the name of the update file, which the camera looks for in the card root.
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

/**
 * Each release that the page makes the fast-boot firmware of: the release tables of
 * `grkit_firmware`, oldest first for each camera. GR IV Monochrome runs the official files of
 * GR IV with its own versions: 1.10 has the file of GR IV 1.20, and 1.11 that of GR IV 1.21.
 */
export const RELEASES: readonly { release: string; names: readonly string[] }[] = [
  { release: 'gr3-2.10', names: ['GR III 2.10'] },
  { release: 'gr3-2.20', names: ['GR III 2.20'] },
  { release: 'gr3-2.21', names: ['GR III 2.21'] },
  { release: 'gr3x-1.60', names: ['GR IIIx 1.60'] },
  { release: 'gr3x-1.70', names: ['GR IIIx 1.70'] },
  { release: 'gr3x-1.71', names: ['GR IIIx 1.71'] },
  { release: 'gr4-1.11', names: ['GR IV 1.11'] },
  { release: 'gr4-1.20', names: ['GR IV 1.20', 'GR IV Monochrome 1.10'] },
  { release: 'gr4-1.21', names: ['GR IV 1.21', 'GR IV Monochrome 1.11'] },
];

/** Ricoh's folder of the official firmware ZIP files. */
const RICOH_FIRMWARE = 'https://www.ricoh-imaging.co.jp/resources/japan/support/download/digital/FIRMWARE';

/**
 * The official ZIP of each older version in `RELEASES`. Ricoh's pages link the latest version
 * only, but its server keeps the ZIP of these. Each holds the update file of its release table
 * (checked 2026-11-14). A camera cannot go back to an older version, so these are for a camera
 * that still has that version.
 */
export const OLDER: readonly { name: string; release: string; zip: string }[] = [
  { name: 'GR III 2.10', release: 'gr3-2.10', zip: `${RICOH_FIRMWARE}/gr3_v210.zip` },
  { name: 'GR III 2.20', release: 'gr3-2.20', zip: `${RICOH_FIRMWARE}/gr3_v220.zip` },
  { name: 'GR IIIx 1.60', release: 'gr3x-1.60', zip: `${RICOH_FIRMWARE}/gr3x_v160.zip` },
  { name: 'GR IIIx 1.70', release: 'gr3x-1.70', zip: `${RICOH_FIRMWARE}/gr3x_v170.zip` },
  { name: 'GR IV 1.11', release: 'gr4-1.11', zip: `${RICOH_FIRMWARE}/gr4_v111.zip` },
  { name: 'GR IV 1.20', release: 'gr4-1.20', zip: `${RICOH_FIRMWARE}/gr4_v120.zip` },
  { name: 'GR IV Monochrome 1.10', release: 'gr4-1.20', zip: `${RICOH_FIRMWARE}/gr4_mono_v110.zip` },
];

/** The camera and version of an entry of `LATEST`, such as `GR IV Monochrome 1.11`. */
export function latestName(item: (typeof LATEST)[number]): string {
  return item.name ?? releaseName(item.release);
}

/** The camera of an entry of `LATEST` without its version, such as `GR IV Monochrome`. */
export function latestCamera(item: (typeof LATEST)[number]): string {
  return latestName(item).replace(/ [\d.]+$/, '');
}

/**
 * The cameras and versions that the page makes, each camera once with its versions, such as
 * `GR III 2.10 / 2.20 / 2.21`.
 */
export const SUPPORTED_NAMES: readonly string[] = (() => {
  const versions = new Map<string, string[]>();
  for (const name of RELEASES.flatMap((item) => item.names)) {
    const camera = name.replace(/ [\d.]+$/, '');
    versions.set(camera, [...(versions.get(camera) ?? []), name.slice(camera.length + 1)]);
  }
  return [...versions].map(([camera, list]) => `${camera} ${list.join(' / ')}`);
})();

/**
 * The cameras and versions whose official file is the file of `release`, such as
 * `GR IV 1.21 / GR IV Monochrome 1.11`. A release that the page does not make has its `releaseName`.
 */
export function firmwareName(release: string): string {
  const names = RELEASES.find((item) => item.release === release)?.names;
  return names ? names.join(' / ') : releaseName(release);
}

/** The latest release of the camera of `release`, or null when `release` is the latest. */
export function newerRelease(release: string): string | null {
  const camera = release.split('-')[0];
  const latest = LATEST.find((item) => item.release.split('-')[0] === camera)?.release;
  return latest && latest !== release ? latest : null;
}

export type FastbootResult =
  /** `fileName` is the name of the official update file, which the camera looks for. `latest` is
   the latest release of the camera when `release` is an older one, else null. */
  | { state: 'ready'; release: string; fileName: string; latest: string | null }
  /** The file is not the official update file of a supported release. */
  | { state: 'unknown' };

/**
 * Makes the fast-boot firmware of the official ZIP or update file of a release in `RELEASES` in
 * the browser and downloads it under the name of the official update file. The file is not
 * uploaded.
 */
export async function makeFastbootFirmware(file: Blob): Promise<FastbootResult> {
  if (file.size > MAX_FIRMWARE_SIZE) return { state: 'unknown' };
  const [wasm, bytes] = await Promise.all([loadWasm(), file.arrayBuffer()]);
  // Undefined for a file that is not an official firmware file. A failure of the patch itself
  // throws, and the page shows it as an error.
  const firmware = wasm.patchFirmware(new Uint8Array(bytes));
  if (!firmware) return { state: 'unknown' };
  try {
    const fileName = firmware.fileName;
    save(new Blob([firmware.bytes as Uint8Array<ArrayBuffer>]), fileName);
    return { state: 'ready', release: firmware.release, fileName, latest: newerRelease(firmware.release) };
  } finally {
    firmware.free();
  }
}
