/** A request fails when no data arrives for this long, so a stalled connection ends in an error. */
const STALL_MS = 20_000;
/** The attempts of one file, and the wait before each attempt after the first. */
const ATTEMPTS = 3;
const RETRY_WAIT_MS = [1_000, 3_000];

/** Why a site file did not arrive: no data for `STALL_MS`, an error status, or no connection. */
export type TransferFailure = 'stall' | 'status' | 'network';

/** A site file that did not arrive. `status` is the HTTP status of a `status` failure. */
export class TransferError extends Error {
  constructor(
    readonly reason: TransferFailure,
    readonly url: string,
    readonly status = 0,
  ) {
    super(reason === 'status' ? `Request failed: ${status} ${url}` : `Request failed (${reason}): ${url}`);
    this.name = 'TransferError';
  }
}

/** The bytes of one file as they arrive. A new attempt starts `received` again from 0. */
export interface Progress {
  received: number;
  /** An attempt failed, and a new one waits or runs. */
  retrying: boolean;
}

/** A failure that a new attempt can fix: a stall, no connection, or a status of a busy server. */
function retryable(error: unknown): boolean {
  if (!(error instanceof TransferError)) return false;
  if (error.reason !== 'status') return true;
  return error.status === 408 || error.status === 429 || error.status >= 500;
}

async function attempt(url: string, progress: Progress, notify: () => void): Promise<Uint8Array> {
  const controller = new AbortController();
  let timer: ReturnType<typeof setTimeout> | undefined;
  const arm = () => {
    clearTimeout(timer);
    timer = setTimeout(() => controller.abort(), STALL_MS);
  };
  arm();
  try {
    const response = await fetch(url, { cache: 'no-cache', signal: controller.signal });
    if (!response.ok) {
      throw new TransferError('status', url, response.status);
    }
    const chunks: Uint8Array[] = [];
    let length = 0;
    const reader = response.body!.getReader();
    for (;;) {
      arm();
      const { done, value } = await reader.read();
      if (done) break;
      chunks.push(value);
      length += value.length;
      progress.received += value.length;
      notify();
    }
    const bytes = new Uint8Array(length);
    let offset = 0;
    for (const chunk of chunks) {
      bytes.set(chunk, offset);
      offset += chunk.length;
    }
    return bytes;
  } catch (error) {
    if (error instanceof TransferError) throw error;
    // The stall timer aborts the request. Any other error is the connection's.
    throw new TransferError(controller.signal.aborted ? 'stall' : 'network', url);
  } finally {
    clearTimeout(timer);
  }
}

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

/**
 * Downloads `url`. A stall, a lost connection, or a busy server gets up to `ATTEMPTS` attempts
 * in all, which a slow or unstable connection needs. `notify` runs on each change of `progress`.
 */
export async function fetchFile(
  url: string,
  progress: Progress,
  notify: () => void,
  wait: (ms: number) => Promise<void> = sleep,
): Promise<Uint8Array> {
  for (let number = 1; ; number += 1) {
    try {
      const bytes = await attempt(url, progress, notify);
      progress.retrying = false;
      notify();
      return bytes;
    } catch (error) {
      if (number >= ATTEMPTS || !retryable(error)) {
        progress.retrying = false;
        notify();
        throw error;
      }
      progress.received = 0;
      progress.retrying = true;
      notify();
      await wait(RETRY_WAIT_MS[number - 1]!);
    }
  }
}
