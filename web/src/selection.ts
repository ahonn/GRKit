import { MAX_EFFECTS, type Grade, type Imported } from './catalog';
import { EFFECT_ID_MAX } from './generated/site';

/**
 * The effects that the user checked, or `null` before the first change. The page then checks
 * every catalog effect, up to the package limit. The site serves the catalog of the latest
 * release, so the default comes from the loaded catalog, not from a list in the page.
 */
export type Selection = ReadonlySet<string> | null;

/**
 * The checked effect IDs. `catalog` gives the IDs of the catalog effects in catalog order, and
 * `others` the IDs of the other effects of the page: imports and effects only on the card. A
 * choice keeps only the IDs of these effects, so an effect that a later release or another tab
 * removed does not count.
 */
export function resolveSelection(
  selection: Selection,
  catalog: readonly string[],
  others: readonly string[] = [],
): ReadonlySet<string> {
  if (!selection) return new Set(catalog.slice(0, MAX_EFFECTS));
  const known = new Set([...catalog, ...others]);
  return new Set([...selection].filter((id) => known.has(id)));
}

const SELECTION_KEY = 'grkit.selection';
/** More IDs than a page can show: the catalog, the imports, and the effects of a card. */
const MAX_SAVED = 256;

/**
 * The choice of the last visit, or `null` when this browser has none. The ZIP download has no
 * card to read the effects from, so the page keeps the choice for the next visit.
 */
export function loadSelection(storage: Pick<Storage, 'getItem'> | null): Selection {
  try {
    const value: unknown = JSON.parse(storage?.getItem(SELECTION_KEY) ?? 'null');
    if (
      Array.isArray(value) &&
      value.length <= MAX_SAVED &&
      value.every((id) => typeof id === 'string' && id.length > 0 && id.length <= EFFECT_ID_MAX)
    ) {
      return new Set(value as string[]);
    }
  } catch {
    // An unreadable value is the same as none.
  }
  return null;
}

export function saveSelection(storage: Pick<Storage, 'setItem'> | null, selection: ReadonlySet<string>) {
  try {
    storage?.setItem(SELECTION_KEY, JSON.stringify([...selection]));
  } catch {
    // The choice is a convenience. Without storage, the next visit checks the default effects.
  }
}

/** How closely an import follows its source: the grade of a LUT fit, or `exact` for a native look. */
export type ImportFit = Grade | 'exact';

export const importFit = (imported: Imported): ImportFit =>
  imported.kind === 'native' ? 'exact' : imported.grade;

/**
 * The selection after an import of the effects `ids`. They are checked when they follow their
 * source closely (a native look, or a LUT fit of grade A or B) and the package has room. A looser
 * fit waits for the user to check it. `full` is true when the package had no room for one of them.
 */
export function selectImport(
  checked: ReadonlySet<string>,
  ids: readonly string[],
  fit: ImportFit,
): { selection: ReadonlySet<string>; full: boolean } {
  const close = fit === 'exact' || fit === 'A' || fit === 'B';
  if (!close) return { selection: checked, full: false };
  const next = new Set(checked);
  let full = false;
  for (const id of ids) {
    if (next.has(id)) continue;
    if (next.size >= MAX_EFFECTS) full = true;
    else next.add(id);
  }
  return { selection: next.size === checked.size ? checked : next, full };
}
