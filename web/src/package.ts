import { effectUrl, siteUrl, type Effect } from './catalog';

async function fetchBytes(url: string): Promise<Uint8Array> {
  const response = await fetch(url, { cache: 'no-cache' });
  if (!response.ok) {
    throw new Error(`Request failed: ${response.status} ${url}`);
  }
  return new Uint8Array(await response.arrayBuffer());
}

async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', bytes as Uint8Array<ArrayBuffer>);
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Builds GRKit.zip in the browser from the target's published runtime and the selected effects.
 * `camera` selects the ModeSet file. `hidden` names the modules that the camera menu hides.
 * grkit-wasm checks each runtime file against `build.json` and each effect's format.
 */
export async function buildPackage(
  target: string,
  camera: string,
  effects: readonly Effect[],
  hidden: readonly string[],
): Promise<Blob> {
  const { default: init, Package } = await import('./wasm/grkit_wasm.js');
  await init();
  const runtime = `runtime/${target}/`;
  const builder = new Package(await fetchBytes(siteUrl(`${runtime}build.json`)));
  try {
    builder.setCamera(camera);
    const files = await Promise.all(
      builder.runtimePaths().map(async (path) => {
        return [path, await fetchBytes(siteUrl(`${runtime}sd/${path}`))] as const;
      }),
    );
    for (const [path, bytes] of files) {
      builder.addRuntimeFile(path, bytes);
    }
    const blobs = await Promise.all(effects.map((effect) => fetchBytes(effectUrl(target, effect.file))));
    for (const [index, blob] of blobs.entries()) {
      const effect = effects[index]!;
      if ((await sha256(blob)) !== effect.sha256) {
        throw new Error(`Effect checksum mismatch: ${effect.id}`);
      }
      builder.addEffect(blob);
    }
    for (const name of hidden) {
      builder.hideModule(name);
    }
    const zip = builder.build();
    return new Blob([zip as Uint8Array<ArrayBuffer>], { type: 'application/zip' });
  } finally {
    builder.free();
  }
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
