import {
  cardEffects,
  INBOX,
  readCard,
  readFile,
  type CardDirectory,
  type CardFile,
  type CardState,
} from './card';
import type { CardEffect, CatalogEffect, ImportedEffect, PackageEffect } from './catalog';
import { effectFile, effectLabel } from './package';

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
 * stays out: the page neither shows nor changes it.
 */
export async function openSession(
  root: CardDirectory,
  known: readonly (CatalogEffect | ImportedEffect)[],
): Promise<CardSession> {
  const state = await readCard(root);
  const byFile = new Map<string, string>();
  for (const effect of known) byFile.set((await effectFile(effect)).toUpperCase(), effect.id);
  const ids = new Set(known.map((effect) => effect.id));
  const imports = new Set(known.flatMap((effect) => ('imported' in effect && effect.imported ? [effect.id] : [])));
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
    }
    if (imports.has(id) && !contents.has(id)) {
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
  };
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  return a.length === b.length && a.every((byte, i) => byte === b[i]);
}

/**
 * The IDs of the imports whose file on the card differs from the effect of the page, such as an
 * import with a new name. A write puts the new file on the card.
 */
export function changedImports(session: CardSession, imported: readonly ImportedEffect[]): Set<string> {
  const changed = new Set<string>();
  for (const effect of imported) {
    const bytes = session.contents?.get(effect.id);
    if (bytes && !sameBytes(bytes, effect.imported.blob)) changed.add(effect.id);
  }
  return changed;
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
