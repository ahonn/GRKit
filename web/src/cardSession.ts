import {
  cardEffects,
  INBOX,
  LIBRARY,
  readCard,
  readFile,
  type CardDirectory,
  type CardFile,
  type CardState,
} from './card';
import type { CardEffect, CatalogEffect, Cover, ImportedEffect, PackageEffect } from './catalog';
import { NATIVE_LOOKS } from './import/native';
import {
  effectiveCover,
  grayCover,
  isFilmCover,
  isMonoLook,
  schemeCover,
  type CoverScheme,
  type Customizations,
} from './packageCovers';
import { coverPixels, effectFile, effectLabel, expectedCoverIcon, importBytes, withCoverIcon } from './package';
import { readRecipeFile, sameRecipes, type Family, type Recipe } from './recipes';

/** The connected card, as the page compares it with the selection. */
export interface CardSession {
  root: CardDirectory;
  state: CardState;
  /** The effects on the card by ID, with the paths of their files on the card (`cardEffects`). */
  effects: ReadonlyMap<string, readonly string[]>;
  /** The effects on the card that the catalog and the imports do not have. */
  extra: readonly CardEffect[];
  /** The file on the card of each import that the card has, to find the changed ones. */
  contents?: ReadonlyMap<string, Uint8Array>;
  /** The camera line that the page reads the recipe file for. The file holds no camera line. */
  family: Family;
  /** The recipes of the camera on the card (`RECIPE_FILE`) for `family`, or null without the file. */
  recipes: readonly Recipe[] | null;
  /**
   * The page wrote the GRKit of this site to the card. The report shows the new version only
   * after the next power-on.
   */
  written?: boolean;
}

function base64(bytes: Uint8Array): string {
  let text = '';
  for (const byte of bytes) text += String.fromCharCode(byte);
  return btoa(text);
}

/**
 * Reads the card and finds its effects among `known`, the catalog and the imports, by file name.
 * An effect file that `known` does not have becomes a card effect. A file that is not an effect
 * stays out: the page neither shows nor changes it. The recipes are those of the camera line
 * `family`.
 */
export async function openSession(
  root: CardDirectory,
  known: readonly (CatalogEffect | ImportedEffect)[],
  family: Family,
): Promise<CardSession> {
  const state = await readCard(root);
  const byFile = new Map<string, string>();
  for (const effect of known) byFile.set((await effectFile(effect)).toUpperCase(), effect.id);
  const ids = new Set(known.map((effect) => effect.id));
  const effects = new Map<string, string[]>();
  const contents = new Map<string, Uint8Array>();
  const extra: CardEffect[] = [];
  for (const path of cardEffects(state)) {
    const file = path.split('/').at(-1)!;
    let id = byFile.get(file.toUpperCase());
    if (!id) {
      const bytes = await readFile(root, path);
      const label = bytes && (await effectLabel(bytes));
      if (!bytes || !label) continue;
      id = label.id;
      if (!ids.has(id)) {
        ids.add(id);
        extra.push({
          id,
          name: label.name,
          icon: base64(label.icon),
          monochrome: label.monochrome,
          card: { path, bytes },
        });
      }
      if (!contents.has(id)) contents.set(id, bytes);
    } else if (!contents.has(id)) {
      // Keep card bytes for imports and catalog covers so a later write can replace changed icons.
      const bytes = await readFile(root, path);
      if (bytes) contents.set(id, bytes);
    }
    effects.set(id, [...(effects.get(id) ?? []), path]);
  }
  return {
    root,
    state,
    effects,
    extra,
    contents,
    family,
    recipes: await readRecipes(state, family),
  };
}

async function readRecipes(state: CardState, family: Family): Promise<Recipe[] | null> {
  return state.recipeText === null ? null : readRecipeFile(state.recipeText, family);
}

/**
 * The session with the recipes of the camera line `family`. One card fits GR IV and GR IV
 * Monochrome, so a change of camera can change the line of the same card.
 */
export async function withFamily(session: CardSession, family: Family): Promise<CardSession> {
  if (session.family === family) return session;
  return { ...session, family, recipes: await readRecipes(session.state, family) };
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  return a.length === b.length && a.every((byte, i) => byte === b[i]);
}

/** Cover to bake for an effect id: catalog/import cover, else a known Cinema cover. */
function coverForPackaging(
  id: string,
  catalog: readonly CatalogEffect[],
  imported: readonly ImportedEffect[],
): Cover | undefined {
  const fromCatalog = catalog.find((effect) => effect.id === id)?.cover;
  if (isFilmCover(fromCatalog)) return fromCatalog;
  const fromImport = imported.find((effect) => effect.id === id)?.cover;
  if (isFilmCover(fromImport)) return fromImport;
  return NATIVE_LOOKS.find((look) => look.id === id)?.cover;
}

/**
 * The film looks on the card whose file differs from what a write puts there, so a write updates
 * them:
 * - an import, when its card file differs from what a write puts there (`importBytes`), such as
 *   after a new name, or for a native look after a change of `scheme`;
 * - a site film look, when its name differs from its custom name, or from the site name without
 *   one, or its icon from its cover (`effectiveCover`: individual, else of `scheme`);
 * - a native look that only the card has, when its icon differs from the cover of `scheme`.
 */
export async function changedOnCard(
  session: CardSession,
  catalog: readonly CatalogEffect[],
  imported: readonly ImportedEffect[],
  custom: Customizations,
  scheme: CoverScheme = 'gray',
): Promise<Set<string>> {
  const changed = new Set<string>();
  const contents = session.contents;
  if (!contents) return changed;
  for (const effect of imported) {
    const bytes = contents.get(effect.id);
    if (bytes && !sameBytes(bytes, await importBytes(effect, scheme))) changed.add(effect.id);
  }
  const importIds = new Set(imported.map((effect) => effect.id));
  const siteIds = new Set(catalog.map((effect) => effect.id));
  for (const id of new Set([...siteIds, ...NATIVE_LOOKS.map((look) => look.id)])) {
    if (importIds.has(id) || !session.effects.has(id)) continue;
    const cover = coverForPackaging(id, catalog, imported);
    const bytes = contents.get(id);
    if (!isFilmCover(cover) || !bytes) continue;
    const label = await effectLabel(bytes);
    if (!label) continue;
    const site = siteIds.has(id) ? catalog.find((effect) => effect.id === id) : undefined;
    const expected = await expectedCoverIcon(
      cover,
      site ? effectiveCover(site, custom, scheme) : schemeCover(cover, scheme),
    );
    const name = site && (custom.get(id)?.name ?? site.name);
    if (!sameBytes(label.icon, expected) || (name !== undefined && label.name !== name)) changed.add(id);
  }
  return changed;
}

/** How many bytes of `a` and `b` differ, with the length difference counted. */
function byteDistance(a: Uint8Array, b: Uint8Array): number {
  let count = Math.abs(a.length - b.length);
  for (let i = 0; i < Math.min(a.length, b.length); i++) if (a[i] !== b[i]) count++;
  return count;
}

/**
 * The cover scheme that the effect files of a card or a ZIP tell. Only the built-in looks of
 * `looks` (site film looks and native looks) with film box colors and without an individual cover
 * recorded on the card (`individual`) count, in two groups: color looks and B&W looks
 * (`isMonoLook`). A group has film box covers when each of its looks has the icon of its film box.
 * Both groups: `filmBox`; only the color group: `colorFilmBox`; else `gray`. A group without a look
 * on the card takes what `browser` gives it. A write records no scheme, so the files are the only
 * record. An icon counts as film box when it is nearer to the film box icon than to the gray one,
 * so that a small change of the icon drawing since the write does not change the scheme.
 */
export async function schemeOnCard(
  session: CardSession,
  looks: readonly { id: string; cover?: Cover; monochrome?: boolean }[],
  individual: ReadonlySet<string>,
  browser: CoverScheme,
): Promise<CoverScheme> {
  // Per group: undefined without a counted look, else whether each has the film box icon.
  const boxed: { color?: boolean; mono?: boolean } = {};
  for (const look of looks) {
    const box = schemeCover(look.cover, 'filmBox');
    const bytes = session.contents?.get(look.id);
    if (!box || !bytes || !session.effects.has(look.id) || individual.has(look.id)) continue;
    const label = await effectLabel(bytes);
    if (!label) continue;
    const toBox = byteDistance(label.icon, await coverPixels(box));
    const toGray = byteDistance(label.icon, await expectedCoverIcon(look.cover!));
    const group = isMonoLook(look) ? 'mono' : 'color';
    boxed[group] = (boxed[group] ?? true) && toBox < toGray;
  }
  const color = boxed.color ?? browser !== 'gray';
  const mono = boxed.mono ?? browser === 'filmBox';
  return color && mono ? 'filmBox' : color ? 'colorFilmBox' : 'gray';
}

/**
 * Library files of native looks that only the card has, with the icon that `changedOnCard` expects:
 * the cover of `scheme`.
 * The package has no file for them: the page cannot rebuild such a look without its firmware file,
 * so a write relabels the card's own file. Site film looks and imports come from the package.
 */
export async function coverLibraryAdds(
  session: CardSession,
  changed: ReadonlySet<string>,
  catalog: readonly CatalogEffect[],
  imported: readonly ImportedEffect[],
  already: readonly CardFile[],
  scheme: CoverScheme = 'gray',
): Promise<CardFile[]> {
  const added = new Set(already.map((file) => file.path.toUpperCase()));
  const own = new Set([...catalog, ...imported].map((effect) => effect.id));
  const out: CardFile[] = [];
  for (const id of changed) {
    if (own.has(id)) continue;
    const paths = session.effects.get(id) ?? [];
    const libraryPath = paths.find((path) => path.toUpperCase().startsWith(`${LIBRARY}/`));
    if (!libraryPath) continue;
    const fileName = libraryPath.split('/').at(-1)!;
    if (added.has(fileName.toUpperCase())) continue;
    const bytes = session.contents?.get(id);
    const cover = coverForPackaging(id, catalog, imported);
    if (!bytes || !isFilmCover(cover)) continue;
    const label = await effectLabel(bytes);
    if (!label) continue;
    out.push({ path: fileName, bytes: await withCoverIcon(bytes, label.name, schemeCover(cover, scheme) ?? grayCover(cover)) });
    added.add(fileName.toUpperCase());
  }
  return out;
}

/**
 * The card effects that current imports do not replace. A later import can have the same ID as
 * an effect that was unknown when the card opened. Keep the session's copies so removing that
 * import makes the card effect available again.
 */
export function extraCardEffects(
  effects: readonly CardEffect[],
  imported: readonly ImportedEffect[],
): readonly CardEffect[] {
  const ids = new Set(imported.map((effect) => effect.id));
  return effects.filter((effect) => !ids.has(effect.id));
}

/**
 * The change of the recipes of the camera: `site` is the library of the site for a package with the
 * Recipes module, or null to leave the card as it is. A write replaces the recipe file.
 */
export function recipeChange(session: CardSession, site: readonly Recipe[] | null): 'same' | 'write' {
  if (!site) return 'same';
  return sameRecipes(session.recipes ?? [], site) ? 'same' : 'write';
}

/**
 * The effects of the package for the card: the checked ones. A card effect goes into the package
 * only when it is a package effect that the camera has not started, because then the package list
 * keeps it on the card. An effect of the library or of GRKit 1.2 stays where it is.
 */
export function packageEffects(checked: readonly PackageEffect[]): PackageEffect[] {
  return checked.filter((effect) => !effect.card || effect.card.path.startsWith(`${INBOX}/`));
}

/**
 * The changes of the effects: an added effect goes into the library from the package files, and a
 * removed one is each file that the card has for it.
 */
export async function libraryChanges(
  session: CardSession,
  files: readonly CardFile[],
  checked: readonly PackageEffect[],
  add: readonly string[],
  remove: readonly string[],
): Promise<{ add: CardFile[]; remove: string[] }> {
  const added: CardFile[] = [];
  for (const id of add) {
    const effect = checked.find((item) => item.id === id);
    if (!effect || effect.card) continue;
    const name = await effectFile(effect);
    const source = files.find((file) => file.path.toUpperCase() === `${INBOX}/${name}`.toUpperCase());
    if (source) added.push({ path: name, bytes: source.bytes });
  }
  return {
    add: added,
    remove: remove.flatMap((id) => session.effects.get(id) ?? []),
  };
}
