/**
 * Reads and writes GRKit on an SD card through the File System Access API.
 *
 * The card layout is the one of `docs/architecture.md` (Card layout): the package in
 * `SCRIPT/GRKIT`, the data folder `GRKIT` at the card root. The page writes a package as a merge
 * copy of its `SD_ROOT` does, so the card keeps `GRKIT` and the `SCRIPT/PLUGINS` folder of
 * GRKit 1.2, which the first power-on of 1.3 migrates.
 *
 * Chrome on macOS writes a `._` file next to each file and folder that it writes on a FAT or
 * exFAT card, for the extended attributes. GRKit ignores them, but the page deletes the ones of
 * its own files. macOS deletes the `._` file of a file that the page deletes.
 */

/** The parts of `FileSystemDirectoryHandle` that the page uses. */
export interface CardDirectory {
  readonly kind: 'directory';
  readonly name: string;
  getDirectoryHandle(name: string, options?: { create?: boolean }): Promise<CardDirectory>;
  getFileHandle(name: string, options?: { create?: boolean }): Promise<CardFileHandle>;
  removeEntry(name: string): Promise<void>;
  entries(): AsyncIterable<[string, CardDirectory | CardFileHandle]>;
}

/** The parts of `FileSystemFileHandle` that the page uses. */
export interface CardFileHandle {
  readonly kind: 'file';
  readonly name: string;
  getFile(): Promise<Blob>;
  createWritable(): Promise<{ write(data: Uint8Array): Promise<void>; close(): Promise<void> }>;
}

/** One file of a package, at its path from the card root. */
export interface CardFile {
  path: string;
  bytes: Uint8Array;
}

export const PACKAGE_LIST = 'SCRIPT/GRKIT/PACKAGE.TXT';
const LOADER = 'SCRIPT/GRKIT/LOADER.TTL';
const STARTUP = 'SCRIPT/STARTUP.TTL';
const LEGACY = 'SCRIPT/PLUGINS';
export const LEGACY_EFFECTS = 'SCRIPT/PLUGINS/EFFECTS';
const LEGACY_PLUGINS = 'SCRIPT/PLUGINS/PLUGINS.TXT';
/** The plugins of GRKit 1.2 and the modules of GRKit 1.3 that replace them. */
const LEGACY_MODULES: Readonly<Record<string, string>> = {
  'image-control': 'FILMLOOK',
  framing: 'FRAMING',
  guides: 'GUIDES',
  'date-stamp': 'DATE',
  grain: 'GRAIN',
};
export const INBOX = 'SCRIPT/GRKIT/EFFECTS';
export const PACKAGE_MODULES = 'SCRIPT/GRKIT/MODULES';
export const LIBRARY = 'GRKIT/EFFECTS';
const USER_MODULES = 'GRKIT/MODULES';
const REPORT = 'GRKIT/REPORT.TXT';
/** The recipe library of the site (`grkit_core::recipes`), which a package can also hold. */
export const RECIPES = 'GRKIT/RECIPES.BIN';

/**
 * The files that a write puts on the card last, in this order. Until `STARTUP.TTL` changes, an
 * interrupted update from GRKit 1.2 leaves the old version to run. The package list names the
 * modules and effects of the new package only after their files are on the card.
 */
const LAST = [PACKAGE_LIST, STARTUP];
/** The number of files that a write sends at the same time. Chrome checks each file at its close. */
const PARALLEL = 6;

/** The first line of `GRKIT/REPORT.TXT`: `GRKit <version> (<target>)`. */
export interface CardReport {
  version: string;
  target: string;
}

export function parseReport(text: string): CardReport | null {
  const match = /^GRKit (\S+) \(([^)\s]+)\)/.exec(text);
  return match ? { version: match[1]!, target: match[2]! } : null;
}

/** `none`: no GRKit. `legacy`: GRKit 1.2 or earlier in `SCRIPT/PLUGINS`. `current`: `SCRIPT/GRKIT`. */
export type CardInstall = 'none' | 'legacy' | 'current';

export interface CardState {
  name: string;
  /**
   * The root has `DCIM`, `SCRIPT`, or `GRKIT`, or nothing but system files, so it is the root of a
   * camera card. A card that a computer formatted has no `DCIM` yet.
   */
  isCardRoot: boolean;
  install: CardInstall;
  /** The report of the last power-on with GRKit 1.3, or null. GRKit 1.2 writes no report. */
  report: CardReport | null;
  /**
   * The runtime builds that the installers on the card are for, such as `gr3-2.10`. Each installer
   * of GRKit 1.2 and 1.3 holds the build in a marker, also before the first power-on.
   */
  builds: readonly string[];
  /**
   * The GRKit version of the installer files on the card, also before the first power-on, or null.
   * GRKit 1.2 does not hold its version.
   */
  version: string | null;
  /** The card has the folder of GRKit 1.2, `SCRIPT/PLUGINS`. */
  hasLegacy: boolean;
  /** The effect and module files that the package list names. */
  listed: readonly string[];
  /** The effect files of the effects library, or null when the library does not exist yet. */
  library: readonly string[] | null;
  /** The effect files of GRKit 1.2 in `SCRIPT/PLUGINS/EFFECTS`, which the first power-on migrates. */
  legacyEffects: readonly string[];
  /**
   * The plugins that GRKit 1.2 installs, from `SCRIPT/PLUGINS/PLUGINS.TXT`, or null without the
   * file, when GRKit 1.2 installs each plugin.
   */
  legacyPlugins: readonly string[] | null;
  /** The module files that a person added in `GRKIT/MODULES`. */
  userModules: readonly string[];
  /** The names at the card root, to find a ModeSet file. */
  rootNames: readonly string[];
  /**
   * The ModeSet file at the card root and its command: `on` enables SD scripts, `off` turns them
   * off. Null without a ModeSet file.
   */
  modeset: { name: string; command: 'on' | 'off' | 'other' } | null;
  /** The bytes of the recipe library (`RECIPES`), or null without one. */
  recipes: Uint8Array | null;
}

/** A ModeSet file name: 8 digits, a dot, and 3 digits, such as `00078350.588`. */
const isModeset = (name: string) => /^\d{8}\.\d{3}$/.test(name);

/** A name that the camera reads: not a `._` file, a `.crswap` file, or a hidden file. */
const visible = (name: string) => !name.startsWith('.') && !name.endsWith('.crswap');
/** Folders that Windows makes at the root of a card. The macOS ones start with a dot. */
const SYSTEM_FOLDERS = new Set(['SYSTEM VOLUME INFORMATION', '$RECYCLE.BIN']);
export const isEffectFile = (name: string) => /^[0-9A-Z_]{1,8}\.GRF$/i.test(name);
export const isModuleFile = (name: string) => /^[0-9A-Z_]{1,8}\.GRM$/i.test(name);

function isNotFound(error: unknown): boolean {
  return (error as { name?: string } | null)?.name === 'NotFoundError';
}

const split = (path: string) => path.split('/').filter(Boolean);

/** The folder at `path`, or null if a part of it does not exist. */
async function folder(root: CardDirectory, path: string, create = false): Promise<CardDirectory | null> {
  let dir = root;
  for (const part of split(path)) {
    try {
      dir = await dir.getDirectoryHandle(part, { create });
    } catch (error) {
      // A missing part, or a file where a folder should be, means that the folder does not exist.
      const missing = isNotFound(error) || (error as { name?: string })?.name === 'TypeMismatchError';
      if (create || !missing) throw error;
      return null;
    }
  }
  return dir;
}

async function names(dir: CardDirectory | null): Promise<string[]> {
  if (!dir) return [];
  const found: string[] = [];
  for await (const [name] of dir.entries()) found.push(name);
  return found.sort();
}

export async function readFile(root: CardDirectory, path: string): Promise<Uint8Array | null> {
  const parts = split(path);
  const dir = await folder(root, parts.slice(0, -1).join('/'));
  if (!dir) return null;
  try {
    const file = await dir.getFileHandle(parts.at(-1)!);
    return new Uint8Array(await (await file.getFile()).arrayBuffer());
  } catch (error) {
    if (isNotFound(error) || (error as { name?: string })?.name === 'TypeMismatchError') return null;
    throw error;
  }
}

async function readText(root: CardDirectory, path: string): Promise<string | null> {
  const bytes = await readFile(root, path);
  return bytes && new TextDecoder().decode(bytes);
}

/** The names of the package list: ASCII whitespace separates them (`runtime/install_library.h`). */
export function parseList(text: string): string[] {
  return text.split(/[ \t\r\n]+/).filter(Boolean);
}

export async function readCard(root: CardDirectory): Promise<CardState> {
  const rootNames = await names(root);
  const upper = new Set(rootNames.map((name) => name.toUpperCase()));
  const hasLoader = (await readFile(root, LOADER)) !== null;
  const legacy = (await folder(root, LEGACY)) !== null;
  const library = await folder(root, LIBRARY);
  const report = await readText(root, REPORT);
  const list = await readText(root, PACKAGE_LIST);
  return {
    name: root.name,
    isCardRoot:
      ['DCIM', 'SCRIPT', 'GRKIT'].some((name) => upper.has(name)) ||
      rootNames.every((name) => !visible(name) || SYSTEM_FOLDERS.has(name.toUpperCase())),
    install: hasLoader ? 'current' : legacy ? 'legacy' : 'none',
    report: report === null ? null : parseReport(report),
    ...(await readInstallers(root)),
    hasLegacy: legacy,
    listed: list === null ? [] : parseList(list),
    library: library && (await names(library)).filter((name) => visible(name) && isEffectFile(name)),
    legacyEffects: (await names(await folder(root, LEGACY_EFFECTS))).filter(
      (name) => visible(name) && isEffectFile(name),
    ),
    legacyPlugins: await readText(root, LEGACY_PLUGINS).then((text) => (text === null ? null : parseList(text))),
    userModules: (await names(await folder(root, USER_MODULES))).filter(
      (name) => visible(name) && isModuleFile(name),
    ),
    rootNames: rootNames.filter(visible),
    modeset: await readModeset(root, rootNames.filter((name) => visible(name) && isModeset(name))),
    recipes: await readFile(root, RECIPES),
  };
}

/** The installers of GRKit 1.3 (`CORE.BIN`, `CORE1.BIN`, ...) and of GRKit 1.2 (`INSTALL.BIN`, ...). */
const INSTALLERS: readonly [string, RegExp][] = [
  ['SCRIPT/GRKIT', /^CORE\d?\.BIN$/i],
  ['SCRIPT/PLUGINS', /^INSTALL\d?\.BIN$/i],
];

/** The build markers of a binary: `GRKIT_TARGET=` and the build (`grkit_core::packaging`). */
export function buildMarkers(bytes: Uint8Array): string[] {
  const text = new TextDecoder('latin1').decode(bytes);
  return [...text.matchAll(/GRKIT_TARGET=([a-z0-9][a-z0-9.-]*)/g)].map((match) => match[1]!);
}

/** The GRKit version in an installer of GRKit 1.3, which its report and About show. */
export function versionMarker(bytes: Uint8Array): string | null {
  const text = new TextDecoder('latin1').decode(bytes);
  return /GRKit (\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?)/.exec(text)?.[1] ?? null;
}

async function readInstallers(root: CardDirectory): Promise<{ builds: string[]; version: string | null }> {
  const builds = new Set<string>();
  let version: string | null = null;
  for (const [path, pattern] of INSTALLERS) {
    for (const name of (await names(await folder(root, path))).filter((item) => pattern.test(item))) {
      const bytes = await readFile(root, `${path}/${name}`);
      if (!bytes) continue;
      for (const build of buildMarkers(bytes)) builds.add(build);
      version ??= versionMarker(bytes);
    }
  }
  return { builds: [...builds].sort(), version };
}

/** The builds that tell the camera: the build that ran last, else the builds of the installers. */
export function cardBuilds(state: CardState): readonly string[] {
  return state.report ? [state.report.target] : state.builds;
}

async function readModeset(
  root: CardDirectory,
  names: readonly string[],
): Promise<CardState['modeset']> {
  const name = names[0];
  if (!name) return null;
  const text = (await readText(root, name)) ?? '';
  const command = /SCRIPT_EN_MODE\s+DIS/.test(text) ? 'off' : /SCRIPT_EN_MODE\s+EN/.test(text) ? 'on' : 'other';
  return { name, command };
}

async function writeOne(root: CardDirectory, file: CardFile): Promise<void> {
  const parts = split(file.path);
  const dir = await folder(root, parts.slice(0, -1).join('/'), true);
  const handle = await dir!.getFileHandle(parts.at(-1)!, { create: true });
  const writable = await handle.createWritable();
  await writable.write(file.bytes);
  await writable.close();
}

/** Calls `task` for each item, at most `limit` at the same time. */
async function pool<T>(items: readonly T[], limit: number, task: (item: T) => Promise<void>): Promise<void> {
  let next = 0;
  const worker = async () => {
    while (next < items.length) await task(items[next++]!);
  };
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
}

function equal(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i += 1) if (a[i] !== b[i]) return false;
  return true;
}

/** A file on the card that differs from the file that the page wrote. */
export class CardCheckError extends Error {
  constructor(readonly path: string) {
    super(`The card has another ${path} than the page wrote`);
    this.name = 'CardCheckError';
  }
}

/**
 * Deletes the `._` files of `paths` and of their folders, and the `.crswap` files that Chrome
 * leaves in those folders after an interrupted write. A file that is not there is skipped.
 */
export async function removeCompanions(root: CardDirectory, paths: readonly string[]): Promise<void> {
  const targets = new Map<string, Set<string>>();
  const add = (dir: string, name: string) => {
    const set = targets.get(dir) ?? new Set<string>();
    set.add(name);
    targets.set(dir, set);
  };
  for (const path of paths) {
    const parts = split(path);
    for (let depth = 1; depth <= parts.length; depth += 1) {
      add(parts.slice(0, depth - 1).join('/'), parts[depth - 1]!);
    }
  }
  for (const [path, owned] of targets) {
    const dir = await folder(root, path);
    if (!dir) continue;
    for (const name of await names(dir)) {
      const companion = name.startsWith('._') && owned.has(name.slice(2));
      const swap = name.endsWith('.crswap') && owned.has(name.slice(0, -'.crswap'.length));
      if (companion || swap) {
        await dir.removeEntry(name).catch((error: unknown) => {
          if (!isNotFound(error)) throw error;
        });
      }
    }
  }
}

/**
 * Writes `files` to the card, then reads each one back. `onProgress` receives the number of
 * written files. The files of `LAST` go last, one at a time. Then the page deletes the `._`
 * files of the written files.
 */
export async function writeFiles(
  root: CardDirectory,
  files: readonly CardFile[],
  onProgress: (written: number) => void = () => {},
): Promise<void> {
  if (files.length === 0) return;
  const last = LAST.flatMap((path) => files.filter((file) => file.path === path));
  const first = files.filter((file) => !last.includes(file));
  // The folders first, so the parallel writes do not create one folder twice.
  const folders = [...new Set(files.map((file) => split(file.path).slice(0, -1).join('/')))];
  for (const path of folders.sort()) await folder(root, path, true);
  let written = 0;
  const write = async (file: CardFile) => {
    await writeOne(root, file);
    onProgress((written += 1));
  };
  await pool(first, PARALLEL, write);
  for (const file of last) await write(file);
  for (const file of files) {
    const bytes = await readFile(root, file.path);
    if (!bytes || !equal(bytes, file.bytes)) throw new CardCheckError(file.path);
  }
  await removeCompanions(root, files.map((file) => file.path));
}

/** Deletes one file. A file that is not there is skipped. */
export async function removeFile(root: CardDirectory, path: string): Promise<void> {
  const parts = split(path);
  const dir = await folder(root, parts.slice(0, -1).join('/'));
  if (!dir) return;
  await dir.removeEntry(parts.at(-1)!).catch((error: unknown) => {
    if (!isNotFound(error)) throw error;
  });
  await removeCompanions(root, [path]);
}

const upper = (names: readonly string[]) => new Set(names.map((name) => name.toUpperCase()));
const minus = (a: readonly string[], b: readonly string[]) => {
  const other = upper(b);
  return a.filter((name) => !other.has(name.toUpperCase()));
};

/**
 * The effect files that the card has now, as paths from the card root: the library, else the
 * effects of a package that the camera has not started and the effects of GRKit 1.2, which the
 * first power-on moves into the library.
 */
export function cardEffects(state: CardState): string[] {
  if (state.library) return state.library.map((name) => `${LIBRARY}/${name}`);
  const listed = state.install === 'current' ? state.listed.filter(isEffectFile) : [];
  return [
    ...listed.map((name) => `${INBOX}/${name}`),
    ...state.legacyEffects.map((name) => `${LEGACY_EFFECTS}/${name}`),
  ];
}

/**
 * The module IDs on the card: the modules of the package, or the modules that replace the plugins
 * of GRKit 1.2. GRKit 1.2 shows Image Control only with effects.
 */
export function cardModules(state: CardState): string[] {
  if (state.install === 'current') {
    return state.listed.filter(isModuleFile).map((name) => name.slice(0, -4).toUpperCase());
  }
  if (state.install !== 'legacy') return [];
  const plugins = state.legacyPlugins ?? Object.keys(LEGACY_MODULES);
  return plugins.flatMap((plugin) => {
    const id = LEGACY_MODULES[plugin];
    return id && (id !== 'FILMLOOK' || state.legacyEffects.length > 0) ? [id] : [];
  });
}

/** What a write changes on the card. */
export interface CardChanges {
  /** `install` on a card without GRKit, `update` from GRKit 1.2 or another version, else `same`. */
  grkit: 'install' | 'update' | 'same';
  /** The card has GRKit and the files that turned SD scripts off, so the write turns them on. */
  scriptsOn: boolean;
  /** GRKit 1.3 has started and moved the 1.2 effects, so the write deletes `SCRIPT/PLUGINS`. */
  cleanLegacy: boolean;
  addModules: string[];
  removeModules: string[];
  addEffects: string[];
  removeEffects: string[];
  /** The recipe library: the write puts the library of the site on the card, or deletes it. */
  recipes: 'same' | 'write' | 'remove';
}

/**
 * The changes from the card to the selection. `modules` are the module IDs of the package.
 * `effects` gives the effects on the card and the checked effects, each by the same kind of key,
 * or it is null to leave the effects as they are.
 */
export function planChanges(
  state: CardState,
  version: string,
  modules: readonly string[],
  effects: { before: readonly string[]; after: readonly string[] } | null,
  recipes: 'same' | 'write' | 'remove' = 'same',
): CardChanges {
  const onCard = cardModules(state);
  return {
    grkit:
      state.install === 'none'
        ? 'install'
        : state.install === 'current' && cardVersion(state) === version
          ? 'same'
          : 'update',
    scriptsOn: turnsScriptsOn(state),
    cleanLegacy: cleansLegacy(state),
    addModules: minus(modules, onCard),
    removeModules: minus(onCard, modules),
    addEffects: effects ? minus(effects.after, effects.before) : [],
    removeEffects: effects ? minus(effects.before, effects.after) : [],
    recipes,
  };
}

/** The GRKit version on the card: of its files, else of its last report. */
export function cardVersion(state: CardState): string | null {
  return state.version ?? state.report?.version ?? null;
}

/** The camera has started the GRKit files on the card. */
export function cardStarted(state: CardState): boolean {
  return !!state.report && (state.version === null || state.report.version === state.version);
}

const turnsScriptsOn = (state: CardState) => state.install !== 'none' && state.modeset?.command === 'off';
const cleansLegacy = (state: CardState) =>
  state.install === 'current' && state.hasLegacy && state.library !== null && state.report !== null;

export function hasChanges(changes: CardChanges): boolean {
  return (
    changes.grkit !== 'same' ||
    changes.scriptsOn ||
    changes.cleanLegacy ||
    changes.recipes !== 'same' ||
    changes.addModules.length + changes.removeModules.length + changes.addEffects.length + changes.removeEffects.length > 0
  );
}

/** The progress of `applyChanges`: the files written of the files that differ. */
export interface ApplyProgress {
  written: number;
  total: number;
}

/**
 * Makes the card hold `files`, the card files of the selected package, and changes the effects by
 * `library`: `add` gives effect files to put in the library, and `remove` gives the paths of the
 * effect files to delete (`cardEffects`). It writes only the files that differ. The ModeSet files
 * at the card root go only on a card without GRKit, because a card with GRKit has SD scripts on
 * already, or over the files that turned SD scripts off. Then it deletes the package files that
 * the new package list does not name, the removed effects, and the GRKit 1.2 folder once GRKit 1.3
 * has moved its effects.
 */
export async function applyChanges(
  root: CardDirectory,
  state: CardState,
  files: readonly CardFile[],
  library: { add: readonly CardFile[]; remove: readonly string[] },
  onProgress: (progress: ApplyProgress) => void = () => {},
): Promise<void> {
  // The ModeSet files go on a card without GRKit, and over the files that turned SD scripts off.
  const wanted = files.filter(
    (file) => file.path.includes('/') || state.install === 'none' || turnsScriptsOn(state),
  );
  if (state.library) {
    wanted.push(...library.add.map((file) => ({ path: `${LIBRARY}/${file.path}`, bytes: file.bytes })));
  }
  const differ: CardFile[] = [];
  for (const file of wanted) {
    const bytes = await readFile(root, file.path);
    if (!bytes || !equal(bytes, file.bytes)) differ.push(file);
  }
  onProgress({ written: 0, total: differ.length });
  await writeFiles(root, differ, (written) => onProgress({ written, total: differ.length }));

  const remove: string[] = [];
  if (state.install === 'current') {
    const kept = upper(files.map((file) => file.path));
    for (const [path, isOwn] of [
      [INBOX, isEffectFile],
      [PACKAGE_MODULES, isModuleFile],
    ] as const) {
      for (const name of await names(await folder(root, path))) {
        if (isOwn(name) && !kept.has(`${path}/${name}`.toUpperCase())) remove.push(`${path}/${name}`);
      }
    }
  }
  remove.push(...library.remove);
  for (const path of remove) await removeFile(root, path);
  if (cleansLegacy(state)) {
    const scripts = await folder(root, 'SCRIPT');
    if (scripts) await removeTree(scripts, 'PLUGINS');
    await removeCompanions(root, [LEGACY]);
  }
}

/**
 * Deletes a folder and everything in it, one entry at a time. macOS deletes the `._` file of a
 * file with the file, so an entry that is gone is skipped.
 */
async function removeTree(parent: CardDirectory, name: string): Promise<void> {
  let dir: CardDirectory;
  try {
    dir = await parent.getDirectoryHandle(name);
  } catch (error) {
    if (isNotFound(error)) return;
    throw error;
  }
  const entries: [string, 'file' | 'directory'][] = [];
  for await (const [child, handle] of dir.entries()) entries.push([child, handle.kind]);
  // Files before their `._` files.
  entries.sort(([a], [b]) => Number(a.startsWith('._')) - Number(b.startsWith('._')));
  for (const [child, kind] of entries) {
    if (kind === 'directory') await removeTree(dir, child);
    else await dir.removeEntry(child).catch(skipMissing);
  }
  await parent.removeEntry(name).catch(skipMissing);
}

function skipMissing(error: unknown) {
  if (!isNotFound(error)) throw error;
}

/** GRKit's own `STARTUP.TTL` runs the loader of GRKit 1.3 or the one of GRKit 1.2. */
const isOwnStartup = (text: string) => /SCRIPT\\(GRKIT|PLUGINS)\\/i.test(text);

/**
 * Removes GRKit from the card: the package of GRKit 1.3, the folder of GRKit 1.2, and GRKit's own
 * `STARTUP.TTL`. Other files in `SCRIPT` stay, and the folder goes only when it is empty. With
 * `data`, it also deletes the data folder `GRKIT`, which holds the settings and the effects.
 */
export async function removeGrkit(root: CardDirectory, data: boolean): Promise<void> {
  const scripts = await folder(root, 'SCRIPT');
  if (scripts) {
    await removeTree(scripts, 'GRKIT');
    await removeTree(scripts, 'PLUGINS');
    const startup = await readText(root, STARTUP);
    if (startup !== null && isOwnStartup(startup)) await removeFile(root, STARTUP);
    await removeCompanions(root, ['SCRIPT/GRKIT', 'SCRIPT/PLUGINS']);
    if ((await names(scripts)).every((name) => name.startsWith('._'))) await removeTree(root, 'SCRIPT');
  }
  if (data) await removeTree(root, 'GRKIT');
  await removeCompanions(root, ['SCRIPT', ...(data ? ['GRKIT'] : [])]);
}

/** Deletes the ModeSet file and `DEVELOP.MOD` at the card root. */
export async function removeModesetFiles(root: CardDirectory, state: CardState): Promise<void> {
  if (state.modeset) await removeFile(root, state.modeset.name);
  await removeFile(root, 'DEVELOP.MOD');
}

/** Opens the card picker. It throws an `AbortError` when the user closes it. */
export async function pickCard(): Promise<CardDirectory> {
  const host = window as unknown as {
    showDirectoryPicker(options: { id: string; mode: 'readwrite' }): Promise<CardDirectory>;
  };
  return host.showDirectoryPicker({ id: 'grkit-card', mode: 'readwrite' });
}

export const canWriteCards = () => typeof window !== 'undefined' && 'showDirectoryPicker' in window;

/** The parts of `Navigator` that tell a phone or a tablet. */
export interface DeviceNavigator {
  userAgent: string;
  platform: string;
  maxTouchPoints: number;
  userAgentData?: { mobile?: boolean };
}

/** Whether the page runs on a phone or a tablet, where no browser can write to a card. */
export function isMobileDevice(device: DeviceNavigator): boolean {
  if (device.userAgentData?.mobile) return true;
  if (/Android|iPhone|iPad|iPod|Mobile/i.test(device.userAgent)) return true;
  // Safari on an iPad gives the user agent of a Mac, but a Mac has no touch screen.
  return device.platform === 'MacIntel' && device.maxTouchPoints > 1;
}

/**
 * Whether the page names the browsers that can write to a card: on a computer whose browser
 * cannot, such as Safari or Firefox.
 */
export const suggestCardBrowser = () =>
  typeof navigator !== 'undefined' && !canWriteCards() && !isMobileDevice(navigator as DeviceNavigator);
