import type { Cover, ImportedEffect } from '../catalog';
import { lookFromColors } from '../coverPalette';
import { loadWasm } from '../package';
import { base64, coverFromFile, coverIcon, type CoverFields } from './names';

// A LUT import keeps its ID when the user changes its name or its icon, so the camera keeps its Fn
// adjustments and the card keeps it under the same file name. Only the effect file changes.

/** Whether the page can change the name of `effect`: a LUT import, also one read back from a
 * package. A native look keeps the name of the camera. */
export const editable = (effect: ImportedEffect): boolean =>
  effect.imported.kind === 'lut' || effect.imported.kind === 'restored';

/** True when the edit dialog cover fields are unchanged (rename-only should keep the GRF icon). */
export function sameCoverFields(a: CoverFields, b: CoverFields): boolean {
  return (
    a.template === b.template &&
    a.look === b.look &&
    a.brand === b.brand &&
    a.name === b.name &&
    a.number === b.number &&
    a.band === b.band
  );
}

/**
 * The cover fields of an editable cover, or null when the page cannot edit it. Covers without
 * stored `colors` open as classic gray — not packaging hues — so saving a rename does not bake
 * stock colors into an old GRF.
 */
export function coverFields(cover: Cover | undefined): CoverFields | null {
  if (!cover || (cover.template !== 'box' && cover.template !== 'cine' && cover.template !== 'stack')) {
    return null;
  }
  return {
    template: cover.template,
    look: (cover.colors && lookFromColors(cover.colors)) || 'gray',
    brand: cover.brand,
    name: cover.name,
    number: cover.number ?? '',
    band: cover.band ?? false,
  };
}

/**
 * Cover fields for the edit dialog. Restored LUT imports without library cover metadata get a
 * cover deduced from the effect name, so the user can set brand, layout, and colors.
 */
export function editableCoverFields(effect: ImportedEffect): CoverFields | null {
  if (!editable(effect)) return null;
  return coverFields(effect.cover) ?? coverFromFile(effect.name);
}

/**
 * `effect` with the name `name`, and with the film box `cover` as its icon when it is given. It
 * fails for a name or a cover that the camera cannot show.
 */
export async function relabelImport(
  effect: ImportedEffect,
  name: string,
  cover: CoverFields | null,
): Promise<ImportedEffect> {
  const { iconPixels, relabelEffect } = await loadWasm();
  const icon = cover ? iconPixels(JSON.stringify(coverIcon(cover))) : undefined;
  const blob = relabelEffect(effect.imported.blob, name, icon);
  return {
    ...effect,
    name,
    ...(cover && icon && { cover: coverIcon(cover), icon: base64(icon) }),
    imported: { ...effect.imported, blob },
  } as ImportedEffect;
}
