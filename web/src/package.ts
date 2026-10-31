import type { CardFile } from './card';
import { effectUrl, siteUrl, type Effect, type PackageEffect } from './catalog';
import { effectFilename } from './import/names';
import { fetchFile, type Progress } from './transfer';
import wasmUrl from './wasm/grkit_wasm_bg.wasm?url';

/** One file that the package needs. A download keeps its bytes for later packages. */
interface Transfer extends Progress {
  bytes: Promise<Uint8Array>;
}

const transfers = new Map<string, Transfer>();
/** Called on each received chunk and each retry, so a running build can report its progress. */
const watchers = new Set<() => void>();

function notify() {
  for (const watcher of watchers) watcher();
}

/** The download of `url`: a new one, or the one in progress or done. A failed one starts again. */
function transfer(url: string): Transfer {
  let item = transfers.get(url);
  if (!item) {
    const created: Transfer = { bytes: Promise.resolve(new Uint8Array()), received: 0, retrying: false };
    created.bytes = fetchFile(url, created, notify).catch((error: unknown) => {
      if (transfers.get(url) === created) transfers.delete(url);
      throw error;
    });
    transfers.set(url, created);
    item = created;
  }
  return item;
}

type Wasm = typeof import('./wasm/grkit_wasm.js');
let wasm: Promise<Wasm> | undefined;

/**
 * The packager, loaded once. Its module downloads like the other files, with a stall limit.
 * The page also uses it for the HALD image of the LUT import.
 */
export function loadWasm(): Promise<Wasm> {
  wasm ??= (async () => {
    const [module, bytes] = await Promise.all([
      import('./wasm/grkit_wasm.js'),
      transfer(wasmUrl).bytes,
    ]);
    await module.default({ module_or_path: bytes });
    return module;
  })().catch((error: unknown) => {
    wasm = undefined;
    throw error;
  });
  return wasm;
}

const buildUrl = (target: string) => siteUrl(`runtime/${target}/build.json`);
const guideUrl = (camera: string, language: string) => siteUrl(`guides/${camera}/${language}.pdf`);
const runtimeUrl = (target: string, path: string) => siteUrl(`runtime/${target}/sd/${path}`);
/** The site file of an official module. The release publishes each one once, for every camera. */
const moduleUrl = (id: string) => siteUrl(`modules/${id}.GRM`);

/**
 * The runtime paths that `build.json` lists, read without the packager so that the files can
 * download while the packager loads. The packager checks each file against the manifest.
 */
function listedPaths(build: Uint8Array): string[] {
  try {
    const files: unknown = JSON.parse(new TextDecoder().decode(build)).files;
    return Array.isArray(files)
      ? files.flatMap((file) => (typeof file?.path === 'string' ? [file.path as string] : []))
      : [];
  } catch {
    return [];
  }
}

/**
 * Starts the downloads of a package for `camera` before the click on Download: the packager, the
 * runtime of each build in `targets`, and the guide. A later `buildPackage` uses them. Errors stay
 * silent here, and the build requests a failed file again.
 */
export function prefetchPackage(targets: readonly string[], camera: string, language: string): void {
  const ignore = () => {};
  loadWasm().catch(ignore);
  transfer(guideUrl(camera, language)).bytes.catch(ignore);
  for (const target of targets) {
    transfer(buildUrl(target))
      .bytes.then((build) => {
        for (const path of listedPaths(build)) {
          transfer(runtimeUrl(target, path)).bytes.catch(ignore);
        }
      })
      .catch(ignore);
  }
}

async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', bytes as Uint8Array<ArrayBuffer>);
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, '0')).join('');
}

type Packager = InstanceType<Wasm['Package']>;

/**
 * Assembles the package in the browser from the published runtime of each build in `targets`, in
 * release order, and the selected effects, then passes the packager to `finish`. The builds of a
 * camera have the same effects, so the effects come from the first build.
 * `camera` selects the ModeSet file. `modules` names the module files that the package holds.
 * `language` selects the installation guide, which the site publishes as `guides/<camera>/<language>.pdf`.
 * grkit-wasm checks each runtime file against `build.json` and each effect's format.
 * `onProgress` receives the number of bytes of the files of this package that have arrived, and
 * whether a file of it waits for a new attempt.
 */
async function assemble<T>(
  targets: readonly string[],
  camera: string,
  effects: readonly PackageEffect[],
  modules: readonly string[],
  language: string,
  onProgress: (received: number, retrying: boolean) => void,
  finish: (builder: Packager) => T,
): Promise<T> {
  const used = new Set<string>([wasmUrl]);
  const use = (url: string) => {
    used.add(url);
    return transfer(url).bytes;
  };
  const report = () => {
    let received = 0;
    let retrying = false;
    for (const url of used) {
      const item = transfers.get(url);
      received += item?.received ?? 0;
      retrying ||= item?.retrying ?? false;
    }
    onProgress(received, retrying);
  };
  watchers.add(report);
  try {
    // The packager, the runtime files, the guide, and the effects download at the same time.
    const ignore = () => {};
    for (const target of targets) {
      use(buildUrl(target))
        .then((build) => {
          for (const path of listedPaths(build)) use(runtimeUrl(target, path)).catch(ignore);
        })
        .catch(ignore);
    }
    const moduleBytes = Promise.all(modules.map((id) => use(moduleUrl(id))));
    moduleBytes.catch(ignore);
    const effectBytes = Promise.all(
      effects.map((effect) =>
        effect.card
          ? Promise.resolve(effect.card.bytes)
          : effect.imported
            ? Promise.resolve(effect.imported.blob)
            : use(effectUrl(targets[0]!, effect.file)),
      ),
    );
    // The build awaits the effects later. Until then, a failure is not an unhandled rejection.
    effectBytes.catch(ignore);
    const [{ Package }, builds, guide] = await Promise.all([
      loadWasm(),
      Promise.all(targets.map((target) => use(buildUrl(target)))),
      use(guideUrl(camera, language)),
    ]);
    report();
    const builder = new Package(builds[0]!);
    try {
      builder.setCamera(camera);
      builder.setLanguage(language);
      builder.setGuide(guide);
      for (const build of builds.slice(1)) builder.addBuild(build);
      for (const [number, target] of targets.entries()) {
        const paths = builder.runtimePaths(number);
        const files = await Promise.all(paths.map((path) => use(runtimeUrl(target, path))));
        for (const [index, bytes] of files.entries()) {
          builder.addRuntimeFile(number, paths[index]!, bytes);
        }
      }
      for (const [index, blob] of (await effectBytes).entries()) {
        const effect = effects[index]!;
        // An imported effect or an effect from the card has no site file to check.
        if (!effect.imported && !effect.card && (await sha256(blob)) !== effect.sha256) {
          throw new Error(`Effect checksum mismatch: ${effect.id}`);
        }
        builder.addEffect(blob);
      }
      for (const bytes of await moduleBytes) {
        builder.addModule(bytes);
      }
      return finish(builder);
    } finally {
      builder.free();
    }
  } catch (error) {
    // A file can be stale after a deployment, so a new attempt downloads them all again.
    for (const url of used) {
      if (url !== wasmUrl) transfers.delete(url);
    }
    throw error;
  } finally {
    watchers.delete(report);
  }
}

/** Builds GRKit.zip in the browser. See `assemble` for the arguments. */
export function buildPackage(
  targets: readonly string[],
  camera: string,
  effects: readonly PackageEffect[],
  modules: readonly string[],
  language: string,
  onProgress: (received: number, retrying: boolean) => void,
): Promise<Blob> {
  return assemble(targets, camera, effects, modules, language, onProgress, (builder) => {
    const zip = builder.build();
    return new Blob([zip as Uint8Array<ArrayBuffer>], { type: 'application/zip' });
  });
}

/**
 * The files that the same package puts on the card, at their paths from the card root: the
 * `SD_ROOT` of the verified GRKit.zip. See `assemble` for the arguments.
 */
export function buildCardFiles(
  targets: readonly string[],
  camera: string,
  effects: readonly PackageEffect[],
  modules: readonly string[],
  language: string,
  onProgress: (received: number) => void,
): Promise<CardFile[]> {
  return assemble(targets, camera, effects, modules, language, onProgress, (builder) =>
    builder.cardFiles().map((file) => {
      const item = { path: file.path, bytes: file.bytes };
      file.free();
      return item;
    }),
  );
}

/** The file name of an effect on the card, the same in a package and in the library. */
export async function effectFile(effect: Effect): Promise<string> {
  return effect.imported ? effectFilename(effect.id) : effect.file;
}

/** The label of an effect file on a card, or null for a file that is not an effect. */
export async function effectLabel(
  bytes: Uint8Array,
): Promise<{ id: string; name: string; icon: Uint8Array; monochrome: boolean } | null> {
  const wasm = await loadWasm();
  let label;
  try {
    label = wasm.effectLabel(bytes);
  } catch {
    return null;
  }
  try {
    return { id: label.id, name: label.name, icon: label.icon, monochrome: label.monochrome };
  } finally {
    label.free();
  }
}

/** The files that turn SD scripts off on `camera`, at their paths from the card root. */
export async function scriptsOffCardFiles(camera: string): Promise<CardFile[]> {
  const wasm = await loadWasm();
  return wasm.scriptsOffFiles(camera).map((file) => {
    const item = { path: file.path, bytes: file.bytes };
    file.free();
    return item;
  });
}

/** The ZIP that turns SD scripts off on `camera`: its ModeSet file with the disable command. */
export async function buildScriptsOff(camera: string): Promise<Blob> {
  const module = await loadWasm();
  return new Blob([module.scriptsOffPackage(camera) as Uint8Array<ArrayBuffer>], {
    type: 'application/zip',
  });
}

/** Starts the download of a blob as a file. */
export function save(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  link.click();
  // Some browsers read the URL after the click returns.
  setTimeout(() => URL.revokeObjectURL(url), 60_000);
}
