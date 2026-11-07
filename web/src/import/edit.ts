import type { Cover, ImportedEffect } from '../catalog';
import { loadWasm } from '../package';
import { base64, coverIcon, type CoverFields } from './names';

// A LUT import keeps its ID when the user changes its name or its icon, so the camera keeps its Fn
// adjustments and the card keeps it under the same file name. Only the effect file changes.

/** Whether the page can change the name of `effect`: a LUT import, also one read back from a
 * package. A native look keeps the name of the camera. */
export const editable = (effect: ImportedEffect): boolean =>
  effect.imported.kind === 'lut' || effect.imported.kind === 'restored';

/** The cover fields of a film box cover, or null for a cover that the page cannot edit. */
export function coverFields(cover: Cover | undefined): CoverFields | null {
  if (cover?.template !== 'box') return null;
  return { brand: cover.brand, name: cover.name, number: cover.number ?? '', band: cover.band ?? false };
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
