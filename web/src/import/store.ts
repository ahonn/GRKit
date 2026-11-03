import type { ImportedEffect } from '../catalog';
import { IMPORTS, run } from '../database';
import { provideTable } from '../preview/render';

// Keeps the effects that this page imported from LUTs and the native looks that it read from a
// firmware file in the browser, so a later visit has them without a new import. Each call fails
// quietly, and the page works without it.

/** The start of each compiled effect (`grkit_core::effects::MAGIC`). */
const MAGIC = 'GRFX0001';

interface Stored extends ImportedEffect {
  /** The time of the import, so the page shows the imports in their order. */
  savedAt: number;
}

/**
 * A stored import that this version of the page can use, with the fields and a compiled effect,
 * or null. Pages before the native looks stored LUT imports without `kind`, so a record without
 * it is a LUT import.
 */
export function storedImport(value: unknown): Stored | null {
  const item = value as Partial<Stored> | null;
  const imported = item?.imported as Record<string, unknown> | undefined;
  if (
    typeof item?.id !== 'string' ||
    typeof item.name !== 'string' ||
    typeof item.icon !== 'string' ||
    typeof item.preview !== 'string' ||
    !item.cover ||
    !imported ||
    !(imported.blob instanceof Uint8Array) ||
    !(imported.table instanceof Uint8Array) ||
    new TextDecoder().decode(imported.blob.subarray(0, MAGIC.length)) !== MAGIC
  ) {
    return null;
  }
  const kind = imported.kind ?? 'lut';
  if (kind !== 'lut' && kind !== 'native') return null;
  if (kind === 'native' && typeof imported.version !== 'string') return null;
  return { ...item, imported: { ...imported, kind } } as Stored;
}

/**
 * The stored imports, in import order. Each one has its preview table again, so the preview finds
 * it as after the import.
 */
export async function loadImports(): Promise<ImportedEffect[]> {
  try {
    const items = (await run(IMPORTS, 'readonly', (store) => store.getAll())).flatMap(
      (value) => storedImport(value) ?? [],
    );
    items.sort((a, b) => a.savedAt - b.savedAt);
    return items.map(({ savedAt: _, ...effect }) => {
      provideTable(effect.preview, effect.imported.table);
      return effect;
    });
  } catch (error) {
    console.warn('The imported effects cannot be read from this browser.', error);
    return [];
  }
}

export async function saveImport(effect: ImportedEffect): Promise<void> {
  try {
    await run(IMPORTS, 'readwrite', (store) => store.put({ ...effect, savedAt: Date.now() } satisfies Stored));
  } catch (error) {
    console.warn('The imported effect cannot be kept in this browser.', error);
  }
}

export async function deleteImport(id: string): Promise<void> {
  try {
    await run(IMPORTS, 'readwrite', (store) => store.delete(id));
  } catch (error) {
    console.warn('The imported effect cannot be deleted from this browser.', error);
  }
}
