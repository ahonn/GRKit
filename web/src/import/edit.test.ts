import { describe, expect, it } from 'vitest';
import type { ImportedEffect } from '../catalog';
import { coverFields, editable } from './edit';

const effect = (kind: 'lut' | 'native' | 'restored') => ({ imported: { kind } }) as unknown as ImportedEffect;

describe('import edits', () => {
  it('change LUT imports, also those read back from a package, and not the native looks', () => {
    expect(editable(effect('lut'))).toBe(true);
    expect(editable(effect('restored'))).toBe(true);
    expect(editable(effect('native'))).toBe(false);
  });

  it('edit the fields of a film box cover only', () => {
    expect(coverFields({ template: 'box', brand: 'Kodak', name: 'GOLD', number: '200', band: true })).toEqual({
      brand: 'Kodak',
      name: 'GOLD',
      number: '200',
      band: true,
    });
    expect(coverFields({ template: 'box', brand: 'LUT', name: 'WARM' })).toEqual({
      brand: 'LUT',
      name: 'WARM',
      number: '',
      band: false,
    });
    expect(coverFields({ template: 'cine', brand: 'RICOH', name: 'CINEMA', number: 'YELLOW' })).toBeNull();
    expect(coverFields(undefined)).toBeNull();
  });
});
