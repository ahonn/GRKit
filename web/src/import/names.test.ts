import { describe, expect, it } from 'vitest';
import { readBaseGamma } from '../catalog';
import { EFFECT_FILENAME_SAMPLE } from '../generated/site';
import {
  base64,
  coverFromFile,
  coverIcon,
  effectFilename,
  effectId,
  isLutImportId,
  lutAccept,
  nameFromFile,
  validCover,
  validCoverText,
  validName,
} from './names';

describe('LUT file input', () => {
  it('filters by extension only on a device without touch', () => {
    expect(lutAccept(false)).toBe('.cube,.png,.tif,.tiff');
    expect(lutAccept(true)).toBeUndefined();
  });
});

describe('imported effect names', () => {
  it('takes a valid name from the file name', () => {
    expect(nameFromFile('Kodak_Portra-400.cube')).toBe('Kodak Portra-400');
    expect(nameFromFile('A very long LUT name for a film.cube')).toBe('A very long name');
    expect(nameFromFile('フィルム.png')).toBe('Imported LUT');
    expect(nameFromFile('XE5_FLog_FGamut_to_ETERNA_BT.709_33grid_V.1.00.cube')).toBe('XE5 ETERNA');
    // Whole words, or the hyphen-separated parts of a long first word, never a cut word.
    expect(nameFromFile('Cineon_to_Kodak_2383_D65_VLog.cube')).toBe('Cineon Kodak');
    expect(nameFromFile('colorreps-kodak-gold.cube')).toBe('colorreps-kodak');
    expect(nameFromFile('FLog2C_to_ETERNA-BB_VLog.cube')).toBe('ETERNA-BB');
    for (const file of ['Kodak_Portra-400.cube', 'A very long LUT name for a film.cube', 'x.tif']) {
      expect(validName(nameFromFile(file))).toBe(true);
    }
  });

  it('names the effect file as effects.rs does', async () => {
    expect(await effectFilename(EFFECT_FILENAME_SAMPLE.id)).toBe(EFFECT_FILENAME_SAMPLE.filename);
  });

  it('checks names as effects.rs does', () => {
    expect(validName('Warm')).toBe(true);
    expect(validName('')).toBe(false);
    expect(validName(' Warm')).toBe(false);
    expect(validName('Seventeen chars!!')).toBe(false);
    expect(validName('Café')).toBe(false);
  });

  it('makes new IDs within 31 lowercase characters', () => {
    const taken = new Set(['lut-warm']);
    expect(effectId('Warm', new Set())).toBe('lut-warm');
    expect(effectId('Warm', taken)).toBe('lut-warm-2');
    expect(effectId('Warm', new Set([...taken, 'lut-warm-2']))).toBe('lut-warm-3');
    expect(effectId('***', new Set())).toBe('lut-effect');
    // A package effect with such an ID came from a browser, so a restore reads it back.
    expect(isLutImportId(effectId('Warm', new Set()))).toBe(true);
    expect(isLutImportId('gr4-cinema-yellow')).toBe(false);
    expect(isLutImportId('portra-400-v1')).toBe(false);
    const long = effectId('An extremely long name here', new Set());
    expect(long.length).toBeLessThanOrEqual(28);
    expect(effectId('An extremely long name here', new Set([long])).length).toBeLessThanOrEqual(31);
    expect(long).toMatch(/^[a-z0-9][a-z0-9-]*[a-z0-9]$/);
  });

  it('makes cover fields in the pattern of the catalog covers', () => {
    const fields = (file: string) => {
      const { brand, name, number } = coverFromFile(file);
      return [brand, name, number];
    };
    expect(fields('Kodak Portra 400 2.png')).toEqual(['Kodak', 'PORTRA', '400']);
    expect(fields('Ilford HP5 Plus 400.png')).toEqual(['ILFORD', 'HP5 PLUS', '400']);
    expect(fields('Fuji_Velvia_50.cube')).toEqual(['FUJIFILM', 'VELVIA', '50']);
    expect(fields('kodak_5247__kodak_vision_2383.cube')).toEqual(['Kodak', 'VISION', '5247']);
    expect(fields('XE5_FLog_FGamut_to_ETERNA_BT.709_33grid_V.1.00.cube')).toEqual([
      'LUT',
      'ETERNA',
      'XE5',
    ]);
    expect(fields('Polaroid PX-100UV+ Cold 3.png')).toEqual(['Polaroid', 'PX-100UV+', '3']);
    expect(fields('warm.cube')).toEqual(['LUT', 'WARM', '']);
    expect(fields('wunjo-TEAL_ORANGE.cube')).toEqual(['WUNJO', 'TEAL ORANGE', '']);
    expect(fields('obs-grayscale.cube')).toEqual(['OBS', 'GRAYSCALE', '']);
    expect(fields('threejs-Chemical 168.CUBE')).toEqual(['THREEJS', 'CHEMICAL', '168']);
    // A brand inside a hyphenated word, and no brand from a word in capitals.
    expect(fields('colorreps-kodak-gold.cube')).toEqual(['Kodak', 'GOLD', '']);
    expect(fields('colorreps-portra.cube')).toEqual(['COLORREPS', 'PORTRA', '']);
    expect(fields('FLog2C_to_ETERNA-BB_VLog.cube')).toEqual(['LUT', 'ETERNA-BB', '']);
    expect(fields('L-Log_to_Classic_VLog.cube')).toEqual(['LUT', 'CLASSIC', '']);
    expect(fields('Cineon_to_Kodak_2383_D65_VLog.cube')).toEqual(['Kodak', 'CINEON', '2383']);
    for (const file of ['Kodak Portra 400 2.png', 'A very long brand name here 12345678901234.cube', '-.cube']) {
      expect(validCover(coverFromFile(file))).toBe(true);
    }
  });

  it('writes a film box icon with an optional number', () => {
    const cover = coverFromFile('Kodak Portra 400.png');
    expect(coverIcon(cover)).toEqual({ template: 'box', brand: 'Kodak', name: 'PORTRA', number: '400', band: false });
    expect(coverIcon({ ...cover, number: '' })).toEqual({
      template: 'box',
      brand: 'Kodak',
      name: 'PORTRA',
      band: false,
    });
    expect(validCover({ ...cover, number: '' })).toBe(true);
    expect(validCover({ ...cover, name: '' })).toBe(false);
    expect(validCoverText('')).toBe(false);
    expect(validCoverText('Thirteen char')).toBe(false);
  });

  it('encodes icons as the catalog does', () => {
    expect(base64(new Uint8Array([0, 1, 2, 255]))).toBe('AAEC/w==');
    expect(base64(new Uint8Array(40 * 40 * 4)).length).toBe(8536);
  });
});

describe('catalog base gamma', () => {
  it('accepts 256 values from 0 to 16384', () => {
    const values = Array.from({ length: 256 }, (_, i) => i * 64);
    expect(readBaseGamma(values)).toEqual(Uint16Array.from(values));
  });

  it('ignores a missing or malformed table', () => {
    expect(readBaseGamma(undefined)).toBeUndefined();
    expect(readBaseGamma([0, 1, 2])).toBeUndefined();
    expect(readBaseGamma(Array.from({ length: 256 }, () => 1.5))).toBeUndefined();
    expect(readBaseGamma(Array.from({ length: 256 }, () => 70000))).toBeUndefined();
  });
});
