import { describe, expect, it } from 'vitest';
import { firmwareName, LATEST, latestCamera, newerRelease, OLDER, RELEASES, releaseName, SUPPORTED_NAMES } from './fastboot';

describe('releaseName', () => {
  it('names the camera, its HDF model, and the version of a release', () => {
    expect(releaseName('gr3-2.10')).toBe('GR III / GR III HDF 2.10');
    expect(releaseName('gr3x-1.70')).toBe('GR IIIx / GR IIIx HDF 1.70');
    expect(releaseName('gr4-1.20')).toBe('GR IV / GR IV HDF 1.20');
  });
});

describe('newerRelease', () => {
  it('names the latest release of the same camera', () => {
    expect(newerRelease('gr3-2.10')).toBe('gr3-2.21');
    expect(newerRelease('gr3-2.20')).toBe('gr3-2.21');
    expect(newerRelease('gr3x-1.60')).toBe('gr3x-1.71');
    expect(newerRelease('gr4-1.11')).toBe('gr4-1.21');
    expect(newerRelease('gr4-1.20')).toBe('gr4-1.21');
  });

  it('has none for the latest release of each camera', () => {
    expect(newerRelease('gr3-2.21')).toBeNull();
    expect(newerRelease('gr3x-1.71')).toBeNull();
    expect(newerRelease('gr4-1.21')).toBeNull();
  });
});

describe('SUPPORTED_NAMES', () => {
  it('lists each camera once with every version that the page makes', () => {
    expect(SUPPORTED_NAMES).toEqual([
      'GR III / GR III HDF 2.10 / 2.20 / 2.21',
      'GR IIIx / GR IIIx HDF 1.60 / 1.70 / 1.71',
      'GR IV / GR IV HDF 1.11 / 1.20 / 1.21',
      'GR IV Monochrome 1.10 / 1.11',
    ]);
  });
});

describe('latestCamera', () => {
  it('names the camera of each entry without its version', () => {
    expect(LATEST.map(latestCamera)).toEqual([
      'GR III / GR III HDF',
      'GR IIIx / GR IIIx HDF',
      'GR IV / GR IV HDF',
      'GR IV Monochrome',
    ]);
  });
});

describe('firmwareName', () => {
  it('names each camera whose official file the release has', () => {
    expect(firmwareName('gr4-1.21')).toBe('GR IV / GR IV HDF 1.21 / GR IV Monochrome 1.11');
    expect(firmwareName('gr3-2.21')).toBe('GR III / GR III HDF 2.21');
    expect(firmwareName('gr4-1.20')).toBe('GR IV / GR IV HDF 1.20 / GR IV Monochrome 1.10');
    expect(firmwareName('gr4-1.11')).toBe('GR IV / GR IV HDF 1.11');
  });
});

describe('LATEST', () => {
  // The release tables of grkit-firmware: the page makes the newest release of each camera.
  const tables = import.meta.glob<{ camera: string; firmware: string; update: { name: string } }>('../../fastboot/releases/*.json', {
    eager: true,
    import: 'default',
  });

  it('holds the newest release table of each camera', () => {
    const newest = new Map<string, string>();
    for (const { camera, firmware } of Object.values(tables)) {
      const current = newest.get(camera);
      if (!current || Number(firmware) > Number(current)) newest.set(camera, firmware);
    }
    expect(newest.size).toBeGreaterThan(0);
    expect([...new Set(LATEST.map((item) => item.release))].sort()).toEqual(
      [...newest].map(([camera, firmware]) => `${camera}-${firmware}`).sort(),
    );
  });

  it('makes every release table, and the latest of each camera is one of them', () => {
    expect(RELEASES.map((item) => item.release).sort()).toEqual(
      Object.values(tables)
        .map(({ camera, firmware }) => `${camera}-${firmware}`)
        .sort(),
    );
    for (const item of RELEASES) expect(item.names[0]).toBe(releaseName(item.release));
    for (const item of LATEST) expect(RELEASES.some((release) => release.release === item.release)).toBe(true);
  });

  it('names the update file of each release table', () => {
    const files = new Map(Object.values(tables).map((table) => [`${table.camera}-${table.firmware}`, table.update.name]));
    for (const item of LATEST) expect(item.file).toBe(files.get(item.release));
  });
});

describe('OLDER', () => {
  it('links the official ZIP of each version that is not the latest of its camera', () => {
    const latest = new Set(LATEST.map((item) => item.name ?? releaseName(item.release)));
    const older = RELEASES.flatMap((item) => item.names.map((name) => ({ name, release: item.release }))).filter(
      (item) => !latest.has(item.name),
    );
    expect(OLDER.map(({ name, release }) => ({ name, release }))).toEqual(older);
    for (const item of OLDER) expect(item.zip).toMatch(/^https:\/\/www\.ricoh-imaging\.co\.jp\/.+\.zip$/);
  });

  // Ricoh can remove these files. Run with CHECK_RICOH_LINKS=1 before a release.
  it.runIf(process.env.CHECK_RICOH_LINKS)('finds each ZIP on Ricoh\'s server', async () => {
    for (const item of OLDER) {
      const response = await fetch(item.zip, { headers: { Range: 'bytes=0-3' } });
      const head = new Uint8Array(await response.arrayBuffer());
      expect([item.name, head[0], head[1]]).toEqual([item.name, 0x50, 0x4b]);
    }
  });
});
