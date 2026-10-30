import { describe, expect, it } from 'vitest';
import {
  applyChanges,
  CardCheckError,
  cardEffects,
  cardModules,
  hasChanges,
  isMobileDevice,
  parseList,
  parseReport,
  planChanges,
  buildMarkers,
  cardStarted,
  readCard,
  versionMarker,
  removeGrkit,
  removeModesetFiles,
  writeFiles,
  type CardDirectory,
  type CardFileHandle,
} from './card';

const error = (name: string) => Object.assign(new Error(name), { name });

/**
 * A card as Chrome on macOS sees a FAT card: names ignore case, a write adds a `._` file next to
 * the file and next to each new folder, and a delete also deletes the `._` file.
 */
class Folder implements CardDirectory {
  readonly kind = 'directory';
  readonly entries_ = new Map<string, Folder | Uint8Array>();
  /** Paths written in order, for the order checks. */
  constructor(
    readonly name: string,
    readonly log: string[] = [],
    readonly path = '',
    /** Changes the bytes that a write stores, to fake a bad card. */
    readonly corrupt: (path: string) => boolean = () => false,
  ) {}

  private key(name: string) {
    return [...this.entries_.keys()].find((key) => key.toUpperCase() === name.toUpperCase());
  }

  async getDirectoryHandle(name: string, options?: { create?: boolean }): Promise<CardDirectory> {
    const key = this.key(name);
    const found = key === undefined ? undefined : this.entries_.get(key);
    if (found instanceof Folder) return found;
    if (found) throw error('TypeMismatchError');
    if (!options?.create) throw error('NotFoundError');
    const child = new Folder(name, this.log, `${this.path}${name}/`, this.corrupt);
    this.entries_.set(name, child);
    this.entries_.set(`._${name}`, new Uint8Array([0]));
    return child;
  }

  async getFileHandle(name: string, options?: { create?: boolean }): Promise<CardFileHandle> {
    const key = this.key(name);
    const found = key === undefined ? undefined : this.entries_.get(key);
    if (found instanceof Folder) throw error('TypeMismatchError');
    if (!found && !options?.create) throw error('NotFoundError');
    const stored = key ?? name;
    if (!found) this.entries_.set(stored, new Uint8Array());
    const path = `${this.path}${stored}`;
    return {
      kind: 'file',
      name: stored,
      getFile: async () => new Blob([this.entries_.get(stored) as Uint8Array<ArrayBuffer>]),
      createWritable: async () => {
        let data = new Uint8Array();
        return {
          write: async (bytes: Uint8Array) => {
            data = bytes.slice();
          },
          close: async () => {
            if (this.corrupt(path)) data = new Uint8Array([...data, 1]);
            this.entries_.set(stored, data);
            this.entries_.set(`._${stored}`, new Uint8Array([0]));
            this.log.push(path);
          },
        };
      },
    };
  }

  async removeEntry(name: string): Promise<void> {
    const key = this.key(name);
    if (key === undefined) throw error('NotFoundError');
    this.entries_.delete(key);
    this.entries_.delete(`._${key}`);
  }

  async *entries(): AsyncIterable<[string, Folder | CardFileHandle]> {
    for (const [name, value] of [...this.entries_]) {
      yield [name, value instanceof Folder ? value : await this.getFileHandle(name)];
    }
  }

  /** All paths under this folder, folders with a trailing slash. */
  tree(): string[] {
    return [...this.entries_].flatMap(([name, value]) =>
      value instanceof Folder ? [`${name}/`, ...value.tree().map((path) => `${name}/${path}`)] : [name],
    ).sort();
  }

  async put(path: string, text: string) {
    const parts = path.split('/');
    let dir: CardDirectory = this;
    for (const part of parts.slice(0, -1)) dir = await dir.getDirectoryHandle(part, { create: true });
    const file = await dir.getFileHandle(parts.at(-1)!, { create: true });
    const writable = await file.createWritable();
    await writable.write(new TextEncoder().encode(text));
    await writable.close();
  }

  text(path: string): string | undefined {
    const parts = path.split('/');
    let dir: Folder = this;
    for (const part of parts.slice(0, -1)) dir = dir.entries_.get(part) as Folder;
    const bytes = dir?.entries_.get(parts.at(-1)!);
    return bytes instanceof Uint8Array ? new TextDecoder().decode(bytes) : undefined;
  }
}

const bytes = (text: string) => new TextEncoder().encode(text);

/** A card after the first power-on with GRKit 1.3. */
async function current(): Promise<Folder> {
  const card = new Folder('NO NAME');
  await card.put('DCIM/100RICOH/R0000001.JPG', 'jpeg');
  await card.put('SCRIPT/STARTUP.TTL', 'startup');
  await card.put('SCRIPT/GRKIT/LOADER.TTL', 'loader');
  await card.put('SCRIPT/GRKIT/PACKAGE.TXT', 'AAAAAAAA.GRF\nGUIDES.GRM\n');
  await card.put('SCRIPT/GRKIT/MODULES/GUIDES.GRM', 'guides');
  await card.put('GRKIT/EFFECTS/AAAAAAAA.GRF', 'a');
  await card.put('GRKIT/REPORT.TXT', 'GRKit 1.3.0 (gr3-2.10)\r\n\r\nModules\r\n');
  card.log.length = 0;
  return card;
}

describe('the report and the package list', () => {
  it('reads the version and the target of the report', () => {
    expect(parseReport('GRKit 1.3.0 (gr3-2.10)\r\nmore')).toEqual({ version: '1.3.0', target: 'gr3-2.10' });
    expect(parseReport('GRKit 1.2.1 (gr4-1.20-0528)\r\n')).toEqual({ version: '1.2.1', target: 'gr4-1.20-0528' });
    expect(parseReport('something else')).toBeNull();
  });

  it('reads the names of the package list', () => {
    expect(parseList('B.GRM\r\nA.GRF  C.GRF\n')).toEqual(['B.GRM', 'A.GRF', 'C.GRF']);
  });
});

describe('readCard', () => {
  it('finds GRKit 1.3, its report, its package list, and its library', async () => {
    const state = await readCard(await current());
    expect(state).toMatchObject({
      name: 'NO NAME',
      isCardRoot: true,
      install: 'current',
      report: { version: '1.3.0', target: 'gr3-2.10' },
      listed: ['AAAAAAAA.GRF', 'GUIDES.GRM'],
      library: ['AAAAAAAA.GRF'],
      userModules: [],
    });
  });

  it('tells GRKit 1.2, no GRKit, and another folder apart', async () => {
    const legacy = new Folder('NO NAME');
    await legacy.put('SCRIPT/PLUGINS/PLUGINS.TTL', 'old');
    await legacy.put('SCRIPT/STARTUP.TTL', 'old');
    expect(await readCard(legacy)).toMatchObject({ install: 'legacy', library: null, report: null, isCardRoot: true });

    const empty = new Folder('CARD');
    await empty.put('DCIM/100RICOH/R0000001.JPG', 'jpeg');
    expect(await readCard(empty)).toMatchObject({ install: 'none', isCardRoot: true });

    const blank = new Folder('NO NAME');
    await blank.put('.Spotlight-V100/Store-V2/x', 'x');
    await blank.put('System Volume Information/IndexerVolumeGuid', 'x');
    expect((await readCard(blank)).isCardRoot).toBe(true);

    const other = new Folder('Downloads');
    await other.put('notes.txt', 'x');
    expect((await readCard(other)).isCardRoot).toBe(false);
  });
});

describe('writeFiles', () => {
  it('writes each file, the package list and STARTUP.TTL last, and deletes the ._ files', async () => {
    const card = new Folder('NO NAME');
    await card.put('SCRIPT/PLUGINS/PLUGINS.TTL', 'old');
    await card.put('SCRIPT/STARTUP.TTL', 'old');
    card.log.length = 0;
    const files = [
      { path: 'SCRIPT/STARTUP.TTL', bytes: bytes('new') },
      { path: 'SCRIPT/GRKIT/PACKAGE.TXT', bytes: bytes('GUIDES.GRM\n') },
      { path: 'SCRIPT/GRKIT/CORE.BIN', bytes: bytes('core') },
      { path: 'SCRIPT/GRKIT/MODULES/GUIDES.GRM', bytes: bytes('guides') },
      { path: '00078350.588', bytes: bytes('[SCRIPT_EN_MODE EN]\r\n') },
    ];
    const progress: number[] = [];
    await writeFiles(card, files, (written) => progress.push(written));

    expect(card.log.slice(-2)).toEqual(['SCRIPT/GRKIT/PACKAGE.TXT', 'SCRIPT/STARTUP.TTL']);
    expect(progress).toEqual([1, 2, 3, 4, 5]);
    expect(card.text('SCRIPT/STARTUP.TTL')).toBe('new');
    // The merge keeps GRKit 1.2, which the first power-on migrates.
    expect(card.text('SCRIPT/PLUGINS/PLUGINS.TTL')).toBe('old');
    const tree = card.tree();
    expect(tree.filter((path) => path.split('/').at(-1)!.startsWith('._'))).toEqual([
      // The page deletes only the companions of what it wrote.
      'SCRIPT/PLUGINS/._PLUGINS.TTL',
      'SCRIPT/._PLUGINS',
    ].sort());
  });

  it('fails when the card returns other bytes', async () => {
    const card = new Folder('NO NAME', [], '', (path) => path === 'SCRIPT/GRKIT/CORE.BIN');
    await expect(
      writeFiles(card, [{ path: 'SCRIPT/GRKIT/CORE.BIN', bytes: bytes('core') }]),
    ).rejects.toBeInstanceOf(CardCheckError);
  });
});

/** The effects on the card and the checked ones, by file name. */
const effects = (state: Awaited<ReturnType<typeof readCard>>, after: string[]) => ({
  before: cardEffects(state).map((path) => path.split('/').at(-1)!),
  after,
});

/** The card files of a package with the modules and the effects. */
function packageFiles(modules: string[], effects: string[], core = 'core'): { path: string; bytes: Uint8Array }[] {
  const list = [...effects, ...modules.map((id) => `${id}.GRM`)].sort();
  return [
    { path: '00078350.588', bytes: bytes('[SCRIPT_EN_MODE EN]\r\n') },
    { path: 'DEVELOP.MOD', bytes: bytes('develop') },
    { path: 'SCRIPT/STARTUP.TTL', bytes: bytes('startup') },
    { path: 'SCRIPT/GRKIT/LOADER.TTL', bytes: bytes('loader') },
    { path: 'SCRIPT/GRKIT/CORE.BIN', bytes: bytes(core) },
    { path: 'SCRIPT/GRKIT/PACKAGE.TXT', bytes: bytes(list.map((name) => `${name}\n`).join('')) },
    ...modules.map((id) => ({ path: `SCRIPT/GRKIT/MODULES/${id}.GRM`, bytes: bytes(id.toLowerCase()) })),
    ...effects.map((name) => ({ path: `SCRIPT/GRKIT/EFFECTS/${name}`, bytes: bytes(name[0]!.toLowerCase()) })),
  ];
}

describe('planChanges', () => {
  it('compares the card with the selection', async () => {
    const state = await readCard(await current());
    expect(planChanges(state, '1.3.0', ['GUIDES'], effects(state, ['AAAAAAAA.GRF']))).toEqual({
      grkit: 'same',
      scriptsOn: false,
      cleanLegacy: false,
      addModules: [],
      removeModules: [],
      addEffects: [],
      removeEffects: [],
      recipes: 'same',
    });
    const changes = planChanges(state, '1.3.1', ['DATE'], effects(state, ['CCCCCCCC.GRF']));
    expect(changes).toEqual({
      grkit: 'update',
      scriptsOn: false,
      cleanLegacy: false,
      addModules: ['DATE'],
      removeModules: ['GUIDES'],
      addEffects: ['CCCCCCCC.GRF'],
      recipes: 'same',
      removeEffects: ['AAAAAAAA.GRF'],
    });
    expect(hasChanges(changes)).toBe(true);
    // Without Film Look, the effects stay as they are.
    expect(planChanges(state, '1.3.0', ['GUIDES'], null).removeEffects).toEqual([]);
  });

  it('installs on a card without GRKit', async () => {
    const empty = new Folder('CARD');
    await empty.put('DCIM/100RICOH/R0000001.JPG', 'jpeg');
    const emptyState = await readCard(empty);
    expect(planChanges(emptyState, '1.3.0', ['GUIDES'], effects(emptyState, ['AAAAAAAA.GRF']))).toMatchObject({
      grkit: 'install',
      addModules: ['GUIDES'],
      addEffects: ['AAAAAAAA.GRF'],
    });
  });

  it('compares a card with GRKit 1.2 as a card with GRKit 1.3', async () => {
    const legacy = new Folder('NO NAME');
    await legacy.put('SCRIPT/PLUGINS/PLUGINS.TTL', 'old');
    await legacy.put('SCRIPT/PLUGINS/EFFECTS/AAAAAAAA.GRF', 'a');
    const state = await readCard(legacy);
    // Without PLUGINS.TXT, GRKit 1.2 installs each plugin.
    expect(cardModules(state)).toEqual(['FILMLOOK', 'FRAMING', 'GUIDES', 'DATE', 'GRAIN']);
    expect(planChanges(state, '1.3.0', ['FILMLOOK', 'GUIDES'], effects(state, ['CCCCCCCC.GRF']))).toEqual({
      grkit: 'update',
      scriptsOn: false,
      cleanLegacy: false,
      addModules: [],
      removeModules: ['FRAMING', 'DATE', 'GRAIN'],
      addEffects: ['CCCCCCCC.GRF'],
      recipes: 'same',
      removeEffects: ['AAAAAAAA.GRF'],
    });
  });

  it('reads the version of the installer files before the first power-on', async () => {
    expect(versionMarker(bytes('\0GRKit 1.3.0-test\0GRKit did not\0'))).toBe('1.3.0-test');
    const card = await current();
    // A newer package, copied after the last power-on.
    await card.put('SCRIPT/GRKIT/CORE.BIN', '\0GRKit 1.3.1\0');
    const state = await readCard(card);
    expect(state.version).toBe('1.3.1');
    expect(cardStarted(state)).toBe(false);
    expect(planChanges(state, '1.3.1', ['GUIDES'], null).grkit).toBe('same');
    expect(planChanges(state, '1.3.0', ['GUIDES'], null).grkit).toBe('update');
  });

  it('turns SD scripts back on, and deletes GRKit 1.2 after GRKit 1.3 moved its effects', async () => {
    const card = await current();
    await card.put('00078350.588', '[SCRIPT_EN_MODE DIS]\r\n');
    await card.put('SCRIPT/PLUGINS/EFFECTS/AAAAAAAA.GRF', 'a');
    const changes = planChanges(await readCard(card), '1.3.0', ['GUIDES'], null);
    expect(changes).toMatchObject({ grkit: 'same', scriptsOn: true, cleanLegacy: true });
    expect(hasChanges(changes)).toBe(true);

    // Before the first power-on of GRKit 1.3, the 1.2 folder stays for the migration.
    const legacy = new Folder('NO NAME');
    await legacy.put('SCRIPT/PLUGINS/PLUGINS.TTL', 'old');
    expect(planChanges(await readCard(legacy), '1.3.0', [], null).cleanLegacy).toBe(false);
  });

  it('reads the plugins of GRKit 1.2, and Image Control only with effects', async () => {
    const legacy = new Folder('NO NAME');
    await legacy.put('SCRIPT/PLUGINS/PLUGINS.TTL', 'old');
    await legacy.put('SCRIPT/PLUGINS/PLUGINS.TXT', 'image-control\nguides\ndate-stamp\n');
    expect(cardModules(await readCard(legacy))).toEqual(['GUIDES', 'DATE']);
    await legacy.put('SCRIPT/PLUGINS/EFFECTS/AAAAAAAA.GRF', 'a');
    expect(cardModules(await readCard(legacy))).toEqual(['FILMLOOK', 'GUIDES', 'DATE']);
  });
});

describe('applyChanges', () => {
  it('writes only what differs, and deletes what the selection removes', async () => {
    const card = await current();
    // The card already has the files of a package with Guides and effect A.
    for (const file of packageFiles(['GUIDES'], ['AAAAAAAA.GRF'])) {
      await card.put(file.path, new TextDecoder().decode(file.bytes));
    }
    card.log.length = 0;
    const state = await readCard(card);
    const files = packageFiles(['DATE'], ['CCCCCCCC.GRF']);
    const changes = planChanges(state, '1.3.0', ['DATE'], effects(state, ['CCCCCCCC.GRF']));
    const progress: number[] = [];
    const library = {
      add: changes.addEffects.map((path) => ({ path, bytes: bytes(path[0]!.toLowerCase()) })),
      remove: changes.removeEffects.map((name) => `GRKIT/EFFECTS/${name}`),
    };
    await applyChanges(card, state, files, library, ({ written, total }) => progress.push(written, total));

    // The runtime and the ModeSet files are the same, so the page does not write them again.
    expect(card.log.sort()).toEqual([
      'GRKIT/EFFECTS/CCCCCCCC.GRF',
      'SCRIPT/GRKIT/EFFECTS/CCCCCCCC.GRF',
      'SCRIPT/GRKIT/MODULES/DATE.GRM',
      'SCRIPT/GRKIT/PACKAGE.TXT',
    ]);
    expect(progress.slice(-2)).toEqual([4, 4]);
    const tree = card.tree();
    expect(tree).not.toContain('SCRIPT/GRKIT/MODULES/GUIDES.GRM');
    expect(tree).not.toContain('SCRIPT/GRKIT/EFFECTS/AAAAAAAA.GRF');
    expect(tree).not.toContain('GRKIT/EFFECTS/AAAAAAAA.GRF');
    expect(tree).toContain('DCIM/100RICOH/R0000001.JPG');
    // The ._ files of what the page wrote or deleted are gone.
    for (const path of [...card.log, 'SCRIPT/GRKIT/MODULES/GUIDES.GRM', 'GRKIT/EFFECTS/AAAAAAAA.GRF']) {
      const parts = path.split('/');
      expect(tree).not.toContain([...parts.slice(0, -1), `._${parts.at(-1)}`].join('/'));
    }
    expect(await readCard(card)).toMatchObject({ library: ['CCCCCCCC.GRF'], listed: ['CCCCCCCC.GRF', 'DATE.GRM'] });
  });

  it('updates GRKit 1.2 without the ModeSet files, and keeps SCRIPT/PLUGINS', async () => {
    const card = new Folder('NO NAME');
    await card.put('SCRIPT/STARTUP.TTL', 'old');
    await card.put('SCRIPT/PLUGINS/PLUGINS.TTL', 'old');
    await card.put('SCRIPT/PLUGINS/EFFECTS/AAAAAAAA.GRF', 'a');
    card.log.length = 0;
    const state = await readCard(card);
    const files = packageFiles(['GUIDES'], ['AAAAAAAA.GRF']);
    await applyChanges(card, state, files, { add: [], remove: [] });

    expect(card.log).not.toContain('00078350.588');
    expect(card.log.slice(-2)).toEqual(['SCRIPT/GRKIT/PACKAGE.TXT', 'SCRIPT/STARTUP.TTL']);
    expect(card.text('SCRIPT/STARTUP.TTL')).toBe('startup');
    expect(card.text('SCRIPT/PLUGINS/EFFECTS/AAAAAAAA.GRF')).toBe('a');
    expect((await readCard(card)).library).toBeNull();
  });

  it('deletes a removed effect of GRKit 1.2 before the first power-on migrates it', async () => {
    const card = new Folder('NO NAME');
    await card.put('SCRIPT/PLUGINS/PLUGINS.TTL', 'old');
    await card.put('SCRIPT/PLUGINS/EFFECTS/AAAAAAAA.GRF', 'a');
    await card.put('SCRIPT/PLUGINS/EFFECTS/BBBBBBBB.GRF', 'b');
    const state = await readCard(card);
    await applyChanges(card, state, packageFiles(['GUIDES'], ['BBBBBBBB.GRF']), { add: [], remove: ['SCRIPT/PLUGINS/EFFECTS/AAAAAAAA.GRF'] });
    expect((await readCard(card)).legacyEffects).toEqual(['BBBBBBBB.GRF']);
  });

  it('writes the ModeSet files over the files that turned SD scripts off', async () => {
    const card = await current();
    await card.put('00078350.588', '[SCRIPT_EN_MODE DIS]\r\n');
    const state = await readCard(card);
    await applyChanges(card, state, packageFiles(['GUIDES'], ['AAAAAAAA.GRF']), { add: [], remove: [] });
    expect((await readCard(card)).modeset).toEqual({ name: '00078350.588', command: 'on' });
  });

  it('deletes SCRIPT/PLUGINS once GRKit 1.3 has started and moved the 1.2 effects', async () => {
    const card = await current();
    await card.put('SCRIPT/PLUGINS/EFFECTS/AAAAAAAA.GRF', 'a');
    await card.put('SCRIPT/PLUGINS/INSTALL.BIN', 'old');
    const state = await readCard(card);
    await applyChanges(card, state, packageFiles(['GUIDES'], ['AAAAAAAA.GRF']), { add: [], remove: [] });
    expect(card.tree().filter((path) => path.startsWith('SCRIPT/PLUGINS'))).toEqual([]);
    expect(card.text('GRKIT/EFFECTS/AAAAAAAA.GRF')).toBe('a');
  });

  it('writes the ModeSet files on a card without GRKit', async () => {
    const card = new Folder('CARD');
    await card.put('DCIM/100RICOH/R0000001.JPG', 'jpeg');
    card.log.length = 0;
    const state = await readCard(card);
    const files = packageFiles(['GUIDES'], []);
    await applyChanges(card, state, files, { add: [], remove: [] });
    expect(card.log).toContain('00078350.588');
    expect(card.log).toContain('DEVELOP.MOD');
  });
});

describe('removeGrkit', () => {
  it('removes GRKit and keeps the photos, the other scripts, and the settings', async () => {
    const card = await current();
    await card.put('SCRIPT/PLUGINS/PLUGINS.TTL', 'old');
    await card.put('SCRIPT/STARTUP.TTL', "include 'C:\\SCRIPT\\GRKIT\\LOADER.TTL'\r\n");
    await card.put('SCRIPT/MINE.TTL', 'mine');
    await removeGrkit(card, false);
    const tree = card.tree();
    const scripts = tree.filter((path) => path.startsWith('SCRIPT/') && path !== 'SCRIPT/');
    expect(scripts.filter((path) => !path.split('/').at(-1)!.startsWith('._'))).toEqual([
      'SCRIPT/MINE.TTL',
    ]);
    expect(tree).toContain('DCIM/100RICOH/R0000001.JPG');
    expect(tree).toContain('GRKIT/EFFECTS/AAAAAAAA.GRF');
    expect(await readCard(card)).toMatchObject({ install: 'none' });
  });

  it('removes an empty SCRIPT folder and, with the settings, the GRKIT folder', async () => {
    const card = await current();
    await card.put('SCRIPT/STARTUP.TTL', "include 'C:\\SCRIPT\\PLUGINS\\PLUGINS.TTL'\r\n");
    await removeGrkit(card, true);
    expect(card.tree().filter((path) => !path.split('/').at(-1)!.startsWith('._'))).toEqual([
      'DCIM/',
      'DCIM/100RICOH/',
      'DCIM/100RICOH/R0000001.JPG',
    ]);
  });

  it('keeps a STARTUP.TTL that is not the one of GRKit', async () => {
    const card = await current();
    await card.put('SCRIPT/STARTUP.TTL', "include 'C:\\SCRIPT\\OTHER.TTL'\r\n");
    await removeGrkit(card, false);
    expect(card.text('SCRIPT/STARTUP.TTL')).toContain('OTHER');
  });
});

describe('the ModeSet files', () => {
  it('reads the command of the ModeSet file, and deletes the files', async () => {
    const card = await current();
    expect((await readCard(card)).modeset).toBeNull();
    await card.put('00078350.588', '[SCRIPT_EN_MODE EN]\r\n');
    expect((await readCard(card)).modeset).toEqual({ name: '00078350.588', command: 'on' });
    await card.put('00078350.588', '[SCRIPT_EN_MODE DIS]\r\n');
    await card.put('DEVELOP.MOD', 'x');
    const state = await readCard(card);
    expect(state.modeset).toEqual({ name: '00078350.588', command: 'off' });
    await removeModesetFiles(card, state);
    expect(card.tree().filter((path) => !path.includes('/'))).not.toContain('DEVELOP.MOD');
    expect((await readCard(card)).modeset).toBeNull();
  });
});

describe('the builds of the installers', () => {
  it('reads the build markers of GRKit 1.2 and 1.3 installers', async () => {
    expect(buildMarkers(bytes('\0\0GRKIT_TARGET=gr4-1.20-0528\0x'))).toEqual(['gr4-1.20-0528']);
    const legacy = new Folder('NO NAME');
    await legacy.put('SCRIPT/PLUGINS/PLUGINS.TTL', 'old');
    await legacy.put('SCRIPT/PLUGINS/INSTALL.BIN', '\0GRKIT_TARGET=gr4-1.11\0');
    await legacy.put('SCRIPT/PLUGINS/INSTALL2.BIN', '\0GRKIT_TARGET=gr4-1.20\0');
    await legacy.put('SCRIPT/PLUGINS/._INSTALL.BIN', '\0GRKIT_TARGET=gr9-1.00\0');
    expect((await readCard(legacy)).builds).toEqual(['gr4-1.11', 'gr4-1.20']);

    const card = await current();
    await card.put('SCRIPT/GRKIT/CORE.BIN', '\0GRKIT_TARGET=gr3-2.10\0');
    expect((await readCard(card)).builds).toEqual(['gr3-2.10']);
  });
});

describe('cardEffects', () => {
  it('counts the effects of GRKit 1.2 until the library exists', async () => {
    const card = new Folder('NO NAME');
    await card.put('SCRIPT/PLUGINS/EFFECTS/AAAAAAAA.GRF', 'a');
    await card.put('SCRIPT/GRKIT/LOADER.TTL', 'l');
    await card.put('SCRIPT/GRKIT/PACKAGE.TXT', 'BBBBBBBB.GRF\nGUIDES.GRM\n');
    expect(cardEffects(await readCard(card))).toEqual([
      'SCRIPT/GRKIT/EFFECTS/BBBBBBBB.GRF',
      'SCRIPT/PLUGINS/EFFECTS/AAAAAAAA.GRF',
    ]);
    await card.put('GRKIT/EFFECTS/AAAAAAAA.GRF', 'a');
    expect(cardEffects(await readCard(card))).toEqual(['GRKIT/EFFECTS/AAAAAAAA.GRF']);
  });
});

describe('isMobileDevice', () => {
  const ANDROID = 'Mozilla/5.0 (Linux; Android 14) Chrome/130.0 Mobile Safari/537.36';
  const IPHONE = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) Mobile/15E148';
  const SAFARI = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Version/18.0 Safari/605.1.15';
  const FIREFOX = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:132.0) Gecko/20100101 Firefox/132.0';
  const LINUX = 'Mozilla/5.0 (X11; Linux x86_64) Chrome/130.0 Safari/537.36';
  const device = (userAgent: string, platform: string, maxTouchPoints = 0, mobile?: boolean) => ({
    userAgent,
    platform,
    maxTouchPoints,
    ...(mobile === undefined ? {} : { userAgentData: { mobile } }),
  });

  it('names phones and tablets, where no browser writes to a card', () => {
    expect(isMobileDevice(device(ANDROID, 'Linux armv8l', 5, true))).toBe(true);
    expect(isMobileDevice(device(IPHONE, 'iPhone', 5))).toBe(true);
    // Safari on an iPad gives the user agent of a Mac.
    expect(isMobileDevice(device(SAFARI, 'MacIntel', 5))).toBe(true);
  });

  it('leaves computers, where Chrome or Edge can write to a card', () => {
    expect(isMobileDevice(device(SAFARI, 'MacIntel'))).toBe(false);
    expect(isMobileDevice(device(FIREFOX, 'Win32'))).toBe(false);
    expect(isMobileDevice(device(LINUX, 'Linux x86_64', 0, false))).toBe(false);
  });
});
