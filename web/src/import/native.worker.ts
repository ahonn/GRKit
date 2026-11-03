import init, { nativeLooks } from '../wasm/grkit_wasm.js';
import wasmUrl from '../wasm/grkit_wasm_bg.wasm?url';

export interface NativeRequest {
  /** An official GR IV ZIP or the firmware file in it. */
  bytes: ArrayBuffer;
  baseGamma: Uint16Array;
}

/** One look of the firmware file, as `nativeLooks` gives it. */
export interface NativeLook {
  id: string;
  name: string;
  version: string;
  blob: Uint8Array;
  table: Uint8Array;
  icon: Uint8Array;
  monochrome: boolean;
}

export type NativeResponse =
  | { kind: 'looks'; looks: NativeLook[] }
  | { kind: 'error'; message: string };

/**
 * Reads the native looks of one firmware file. It decodes about 40 MB, so it runs in a worker.
 * The page starts a new worker for each file and terminates it after.
 */
self.onmessage = async ({ data }: MessageEvent<NativeRequest>) => {
  try {
    await init({ module_or_path: wasmUrl });
    const looks = nativeLooks(new Uint8Array(data.bytes), data.baseGamma).map((effect) => {
      try {
        return {
          id: effect.id,
          name: effect.name,
          version: effect.version,
          blob: effect.blob,
          table: effect.preview,
          icon: effect.icon,
          monochrome: effect.monochrome,
        };
      } finally {
        effect.free();
      }
    });
    self.postMessage({ kind: 'looks', looks } satisfies NativeResponse, {
      transfer: looks.flatMap((look) => [look.blob.buffer, look.table.buffer, look.icon.buffer]),
    });
  } catch (error) {
    self.postMessage({
      kind: 'error',
      message: error instanceof Error ? error.message : 'The firmware file cannot be read',
    } satisfies NativeResponse);
  }
};
