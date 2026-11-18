import { describe, expect, it } from 'vitest';
import type { ImportedEffect } from '../catalog';
import { colorsFromLook } from '../coverPalette';
import {
  coverFields,
  editable,
  editableCoverFields,
  sameCoverFields,
} from './edit';

const effect = (kind: 'lut' | 'native' | 'restored', name = 'Warm') =>
  ({ name, imported: { kind } }) as unknown as ImportedEffect;

describe('import edits', () => {
  it('change LUT imports, also those read back from a package, and not the native looks', () => {
    expect(editable(effect('lut'))).toBe(true);
    expect(editable(effect('restored'))).toBe(true);
    expect(editable(effect('native'))).toBe(false);
  });

  it('edit the fields of film covers without inventing packaging looks for colorless covers', () => {
    expect(coverFields({ template: 'box', brand: 'Kodak', name: 'GOLD', number: '200', band: true })).toEqual({
      template: 'box',
      look: 'gray',
      brand: 'Kodak',
      name: 'GOLD',
      number: '200',
      band: true,
    });
    expect(
      coverFields({
        template: 'box',
        brand: 'Kodak',
        name: 'GOLD',
        number: '200',
        band: true,
        colors: colorsFromLook('gold'),
      }),
    ).toEqual({
      template: 'box',
      look: 'gold',
      brand: 'Kodak',
      name: 'GOLD',
      number: '200',
      band: true,
    });
    expect(coverFields({ template: 'box', brand: 'LUT', name: 'WARM' })).toEqual({
      template: 'box',
      look: 'gray',
      brand: 'LUT',
      name: 'WARM',
      number: '',
      band: false,
    });
    expect(
      coverFields({
        template: 'cine',
        brand: 'RICOH',
        name: 'CINEMA',
        number: 'YELLOW',
        colors: colorsFromLook('cinemaYellow'),
      }),
    ).toEqual({
      template: 'cine',
      look: 'cinemaYellow',
      brand: 'RICOH',
      name: 'CINEMA',
      number: 'YELLOW',
      band: false,
    });
    expect(coverFields(undefined)).toBeNull();
  });

  it('treats unchanged cover fields as a rename-only save', () => {
    const opened = {
      template: 'box' as const,
      look: 'gray' as const,
      brand: 'Kodak',
      name: 'GOLD',
      number: '200',
      band: false,
    };
    expect(sameCoverFields(opened, { ...opened })).toBe(true);
    expect(sameCoverFields(opened, { ...opened, look: 'gold' })).toBe(false);
  });

  it('invents cover fields for a restored import without cover metadata', () => {
    const restored = effect('restored', 'Kodak Portra 400');
    expect(editableCoverFields(restored)).toEqual({
      template: 'box',
      look: 'portra',
      brand: 'Kodak',
      name: 'PORTRA',
      number: '400',
      band: false,
    });
    expect(editableCoverFields(effect('native'))).toBeNull();
  });

});
