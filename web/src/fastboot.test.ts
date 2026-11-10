import { describe, expect, it } from 'vitest';
import { firmwareName, LATEST, latestCamera, LATEST_NAMES, newerRelease, releaseName } from './fastboot';

describe('releaseName', () => {
  it('names the camera and the version of a release', () => {
    expect(releaseName('gr3-2.10')).toBe('GR III 2.10');
    expect(releaseName('gr3x-1.70')).toBe('GR IIIx 1.70');
    expect(releaseName('gr4-1.20')).toBe('GR IV 1.20');
  });
});

describe('newerRelease', () => {
  it('asks for the latest release of the same camera', () => {
    expect(newerRelease('gr3-2.10')).toBe('gr3-2.21');
    expect(newerRelease('gr3-2.20')).toBe('gr3-2.21');
    expect(newerRelease('gr3x-1.60')).toBe('gr3x-1.71');
    expect(newerRelease('gr4-1.11')).toBe('gr4-1.21');
    expect(newerRelease('gr4-1.20')).toBe('gr4-1.21');
  });

  it('accepts the latest release of each camera', () => {
    expect(newerRelease('gr3-2.21')).toBeNull();
    expect(newerRelease('gr3x-1.71')).toBeNull();
    expect(newerRelease('gr4-1.21')).toBeNull();
    expect(LATEST_NAMES).toEqual(['GR III 2.21', 'GR IIIx 1.71', 'GR IV 1.21', 'GR IV Monochrome 1.11']);
  });
});

describe('latestCamera', () => {
  it('names the camera of each entry without its version', () => {
    expect(LATEST.map(latestCamera)).toEqual(['GR III', 'GR IIIx', 'GR IV', 'GR IV Monochrome']);
  });
});

describe('firmwareName', () => {
  it('names each camera whose official file the release has', () => {
    expect(firmwareName('gr4-1.21')).toBe('GR IV 1.21 / GR IV Monochrome 1.11');
    expect(firmwareName('gr3-2.21')).toBe('GR III 2.21');
    expect(firmwareName('gr4-1.20')).toBe('GR IV 1.20');
    expect(firmwareName('gr4-1.11')).toBe('GR IV 1.11');
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

  it('names the update file of each release table', () => {
    const files = new Map(Object.values(tables).map((table) => [`${table.camera}-${table.firmware}`, table.update.name]));
    for (const item of LATEST) expect(item.file).toBe(files.get(item.release));
  });
});
