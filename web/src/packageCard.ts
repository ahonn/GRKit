import type { CardDirectory, CardFile, CardFileHandle } from './card';
import { loadWasm } from './package';

/** A GRKit.zip has a few MB. `packaging::MAX_TOTAL_SIZE` limits its contents to 32 MB. */
export const MAX_PACKAGE_BYTES = 64 * 1024 * 1024;

const error = (name: string) => Object.assign(new Error(name), { name });

/**
 * A read-only card that holds `files`, at their paths from the card root. Names ignore case, as
 * on a FAT card. A write or a delete fails, because the page only reads such a card.
 */
export function readOnlyCard(files: readonly CardFile[], name = ''): CardDirectory {
  type Node = Map<string, Node | Uint8Array>;
  const root: Node = new Map();
  for (const { path, bytes } of files) {
    const parts = path.split('/');
    let dir = root;
    for (const part of parts.slice(0, -1)) {
      let next = dir.get(part);
      if (!(next instanceof Map)) {
        next = new Map();
        dir.set(part, next);
      }
      dir = next;
    }
    dir.set(parts.at(-1)!, bytes);
  }

  const find = (dir: Node, wanted: string) =>
    [...dir.keys()].find((key) => key.toUpperCase() === wanted.toUpperCase());

  const file = (fileName: string, bytes: Uint8Array): CardFileHandle => ({
    kind: 'file',
    name: fileName,
    getFile: async () => new Blob([bytes as Uint8Array<ArrayBuffer>]),
    createWritable: async () => {
      throw error('NotAllowedError');
    },
  });

  const directory = (dirName: string, dir: Node): CardDirectory => ({
    kind: 'directory',
    name: dirName,
    async getDirectoryHandle(wanted, options) {
      if (options?.create) throw error('NotAllowedError');
      const key = find(dir, wanted);
      const found = key === undefined ? undefined : dir.get(key);
      if (found instanceof Map) return directory(key!, found);
      throw error(found ? 'TypeMismatchError' : 'NotFoundError');
    },
    async getFileHandle(wanted, options) {
      if (options?.create) throw error('NotAllowedError');
      const key = find(dir, wanted);
      const found = key === undefined ? undefined : dir.get(key);
      if (found instanceof Uint8Array) return file(key!, found);
      throw error(found ? 'TypeMismatchError' : 'NotFoundError');
    },
    async removeEntry() {
      throw error('NotAllowedError');
    },
    async *entries() {
      for (const [key, value] of dir) {
        yield [key, value instanceof Map ? directory(key, value) : file(key, value)];
      }
    },
  });

  return directory(name, root);
}

/**
 * The card that a GRKit.zip makes when the user copies it: the files of its `SD_ROOT`. A ZIP of
 * an older release works too. It fails for a file that is not a ZIP within the package limits.
 */
export async function packageCard(zip: File): Promise<CardDirectory> {
  if (zip.size > MAX_PACKAGE_BYTES) {
    throw new Error(`The ZIP file is larger than ${MAX_PACKAGE_BYTES >> 20} MB`);
  }
  const bytes = new Uint8Array(await zip.arrayBuffer());
  const { packageFiles } = await loadWasm();
  const files = packageFiles(bytes).map((item) => {
    try {
      return { path: item.path, bytes: item.bytes };
    } finally {
      item.free();
    }
  });
  return readOnlyCard(files, zip.name);
}
