import { describe, expect, it } from 'vitest';
import { cameraForCard, CAMERAS, camerasForCard, firmwareForTarget, releases } from './cameras';
import { CAMERA_RELEASES } from './generated/cameras';

describe('cameras', () => {
  it('packages each runtime build of each camera release', () => {
    for (const [id, camera] of Object.entries(CAMERA_RELEASES)) {
      for (const { target } of camera.releases) {
        expect(
          CAMERAS.some((item) => item.id === id && item.support?.targets.includes(target)),
          `${id} ${target}`,
        ).toBe(true);
      }
    }
  });

  it('shows each camera once', () => {
    const names = CAMERAS.map((item) => item.name);
    expect(new Set(names).size).toBe(names.length);
  });

  it('lists the versions and builds of a camera', () => {
    expect(releases('gr3')).toEqual({
      firmware: '2.10 / 2.20',
      targets: ['gr3-2.10'],
      modeset: '00078350.588',
    });
    // Two builds of 1.20 show the version once.
    expect(releases('gr4')).toMatchObject({
      firmware: '1.11 / 1.20',
      targets: ['gr4-1.11', 'gr4-1.20', 'gr4-1.20-0528'],
    });
  });

  it('gives GR IV Monochrome the GR IV builds with its own versions', () => {
    const mono = CAMERAS.find((item) => item.key === 'gr4m')!;
    expect(mono.support).toMatchObject({
      firmware: '1.00 / 1.10',
      targets: ['gr4-1.11', 'gr4-1.20', 'gr4-1.20-0528'],
    });
  });
});

describe('cameraForCard', () => {
  const camera = (key: string) => CAMERAS.find((item) => item.key === key)!;

  it('keeps the selected camera when the card fits it', () => {
    expect(cameraForCard(camera('gr3x'), ['gr3-2.10'], [])).toBe(camera('gr3x'));
    expect(cameraForCard(camera('gr4m'), ['gr4-1.20'], [])).toBe(camera('gr4m'));
    expect(cameraForCard(camera('gr3'), [], ['DCIM'])).toBe(camera('gr3'));
  });

  it('selects the camera of the reported build and of the ModeSet file', () => {
    expect(cameraForCard(camera('gr3'), ['gr4-1.11'], [])).toBe(camera('gr4'));
    expect(cameraForCard(camera('gr3'), ['gr3-2.10'], ['00078490.609'])).toBe(camera('gr3x'));
    expect(cameraForCard(camera('gr4'), [], ['00078350.588'])).toBe(camera('gr3'));
  });

  it('finds no camera for an unknown build', () => {
    expect(cameraForCard(camera('gr3'), ['gr9-1.00'], [])).toBeNull();
  });
});

describe('camerasForCard', () => {
  const keys = (cameras: ReturnType<typeof camerasForCard>) => cameras?.map((camera) => camera.key) ?? null;

  it('gives each entry that runs the reported build', () => {
    expect(keys(camerasForCard(['gr3-2.10'], []))).toEqual(['gr3', 'gr3x']);
    expect(keys(camerasForCard(['gr4-1.20-0528'], []))).toEqual(['gr4', 'gr4m']);
  });

  it('narrows the entries by the ModeSet file on the card', () => {
    expect(keys(camerasForCard(['gr3-2.10'], ['00078490.609', 'DCIM']))).toEqual(['gr3x']);
    expect(keys(camerasForCard([], ['00078350.588']))).toEqual(['gr3']);
  });

  it('fits each build of a package with more than one', () => {
    expect(keys(camerasForCard(['gr4-1.11', 'gr4-1.20', 'gr4-1.20-0528'], []))).toEqual(['gr4', 'gr4m']);
  });

  it('tells nothing without a build and a ModeSet file', () => {
    expect(camerasForCard([], ['DCIM'])).toBeNull();
    expect(keys(camerasForCard(['gr9-1.00'], []))).toEqual([]);
  });
});

describe('firmwareForTarget', () => {
  const camera = (key: string) => CAMERAS.find((item) => item.key === key)!;

  it('gives the versions that run the build', () => {
    expect(firmwareForTarget(camera('gr3'), 'gr3-2.10')).toBe('2.10 / 2.20');
    expect(firmwareForTarget(camera('gr4'), 'gr4-1.11')).toBe('1.11');
    expect(firmwareForTarget(camera('gr4'), 'gr4-1.20-0528')).toBe('1.20');
  });

  it('keeps the versions of an entry that shows its own', () => {
    expect(firmwareForTarget(camera('gr4m'), 'gr4-1.11')).toBe('1.00 / 1.10');
    expect(firmwareForTarget(camera('gr3x'), null)).toBe('1.60 / 1.70');
  });
});
