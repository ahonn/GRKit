import { afterEach, describe, expect, it, vi } from 'vitest';
import { fetchFile, TransferError, type Progress } from './transfer';

const URL_ = 'https://grkit.test/runtime/gr3-2.10/sd/SCRIPT/GRKIT/CORE.BIN';

function setup(...replies: (() => Promise<Response>)[]) {
  const fetch = vi.fn();
  for (const reply of replies) fetch.mockImplementationOnce(reply);
  vi.stubGlobal('fetch', fetch);
  const progress: Progress = { received: 0, retrying: false };
  const seen: boolean[] = [];
  const waits: number[] = [];
  const run = () =>
    fetchFile(
      URL_,
      progress,
      () => seen.push(progress.retrying),
      async (ms) => {
        waits.push(ms);
      },
    );
  return { fetch, progress, seen, waits, run };
}

const ok = (text: string) => () => Promise.resolve(new Response(text));
const status = (code: number) => () => Promise.resolve(new Response('', { status: code }));
const offline = () => Promise.reject(new TypeError('Failed to fetch'));

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

describe('fetchFile', () => {
  it('tries again after a lost connection, and counts only the bytes of the last attempt', async () => {
    const { fetch, progress, seen, waits, run } = setup(offline, ok('abc'));
    const bytes = await run();
    expect(new TextDecoder().decode(bytes)).toBe('abc');
    expect(fetch).toHaveBeenCalledTimes(2);
    expect(waits).toEqual([1_000]);
    expect(seen[0]).toBe(true);
    expect(progress).toEqual({ received: 3, retrying: false });
  });

  it('gives up after three attempts with a busy server', async () => {
    const { fetch, waits, progress, run } = setup(status(503), status(502), status(503));
    const error = await run().catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(TransferError);
    expect(error).toMatchObject({ reason: 'status', status: 503, url: URL_ });
    expect(fetch).toHaveBeenCalledTimes(3);
    expect(waits).toEqual([1_000, 3_000]);
    expect(progress.retrying).toBe(false);
  });

  it('does not try again for a missing file', async () => {
    const { fetch, waits, run } = setup(status(404));
    await expect(run()).rejects.toMatchObject({ reason: 'status', status: 404 });
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(waits).toEqual([]);
  });

  it('ends a request without data after 20 s as a stall, and tries again', async () => {
    vi.useFakeTimers();
    const stalled = (_url: string, init: RequestInit) =>
      new Promise<Response>((_resolve, reject) => {
        init.signal!.addEventListener('abort', () => reject(new DOMException('Aborted', 'AbortError')));
      });
    const fetch = vi.fn(stalled).mockImplementationOnce(stalled).mockImplementationOnce(() => ok('x')());
    vi.stubGlobal('fetch', fetch);
    const progress: Progress = { received: 0, retrying: false };
    const result = fetchFile(URL_, progress, () => {}, async () => {});
    await vi.advanceTimersByTimeAsync(19_999);
    expect(fetch).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(new TextDecoder().decode(await result)).toBe('x');
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it('reports a stall that lasts each attempt', async () => {
    vi.useFakeTimers();
    const stalled = (_url: string, init: RequestInit) =>
      new Promise<Response>((_resolve, reject) => {
        init.signal!.addEventListener('abort', () => reject(new DOMException('Aborted', 'AbortError')));
      });
    vi.stubGlobal('fetch', vi.fn(stalled));
    const result = fetchFile(URL_, { received: 0, retrying: false }, () => {}, async () => {}).catch(
      (caught: unknown) => caught,
    );
    await vi.advanceTimersByTimeAsync(60_000);
    expect(await result).toMatchObject({ reason: 'stall' });
  });
});
