import {
  useCallback,
  useId,
  useImperativeHandle,
  useRef,
  useState,
  type ChangeEvent,
  type DragEvent,
  type Ref,
} from 'react';
import { MAX_EFFECTS, type ImportedEffect, type NativeImport } from '../catalog';
import type { Strings } from '../i18n';
import { trackNativeLooks } from '../analytics';
import {
  FIRMWARE_PAGE,
  nativeProblem,
  unlockNativeLooks,
  type NativeProblem,
} from '../import/native';
import { TaskSlot } from '../import/task';
import { CoverIcon } from './EffectIcon';

interface Props {
  strings: Strings;
  baseGamma: Uint16Array;
  /** Adds the looks to the page, in place of earlier copies with the same IDs. Returns true when
   * a look stays unchecked because the package already has `MAX_EFFECTS` checked effects. */
  onImport: (effects: readonly ImportedEffect[]) => boolean;
  ref?: Ref<NativeLookDialogHandle>;
}

/** Opens the dialog from a button elsewhere on the page. */
export interface NativeLookDialogHandle {
  open: () => void;
}

type Status =
  | { state: 'idle' }
  | { state: 'reading' }
  | { state: 'failed'; problem: NativeProblem }
  | { state: 'done'; effects: readonly ImportedEffect<NativeImport>[]; full: boolean };

/** The File System Access picker, which Chromium browsers have. It opens in Downloads. */
interface PickerHost {
  showOpenFilePicker?: (options: {
    id: string;
    startIn: 'downloads';
    types: { description: string; accept: Record<string, string[]> }[];
  }) => Promise<{ getFile(): Promise<File> }[]>;
}

const ACCEPT = '.zip,.bin';

/**
 * The dialog that unlocks the GR IV Cinema looks: a link to Ricoh's firmware page, a file choice
 * or a drop of the downloaded ZIP, then the read in a worker. The file stays in the browser.
 */
export function NativeLookDialog({ strings: t, baseGamma, onImport, ref }: Props) {
  const ids = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const [status, setStatus] = useState<Status>({ state: 'idle' });
  const [dragging, setDragging] = useState(false);
  // A new key empties the file input, so the same file can be chosen again.
  const [fileKey, setFileKey] = useState(0);
  // Only the newest read changes the dialog.
  const [task] = useState(() => new TaskSlot());
  // An unmount stops a running read.
  const ownDialog = useCallback(
    (element: HTMLDialogElement | null) => {
      dialog.current = element;
      return () => {
        task.cancel();
        dialog.current = null;
      };
    },
    [task],
  );

  const reading = status.state === 'reading';

  function open() {
    if (status.state !== 'reading') setStatus({ state: 'idle' });
    dialog.current?.showModal();
  }

  useImperativeHandle(ref, () => ({ open }));

  function cancel() {
    task.cancel();
    setStatus({ state: 'idle' });
  }

  function close() {
    cancel();
    setDragging(false);
    dialog.current?.close();
  }

  async function read(file: File) {
    setStatus({ state: 'reading' });
    try {
      const result = await task.run((signal) => unlockNativeLooks(file, baseGamma, signal));
      if (!result.current) return;
      const effects = result.value;
      trackNativeLooks({ result: 'ok', version: effects[0]?.imported.version ?? '' });
      const full = onImport(effects);
      setStatus({ state: 'done', effects, full });
    } catch (error) {
      console.error(error);
      const problem = nativeProblem(error);
      trackNativeLooks({ result: 'failed', problem });
      setStatus({ state: 'failed', problem });
    }
  }

  /** Opens the picker in Downloads where the browser has it, or else the file input. */
  async function pick() {
    const host = window as unknown as PickerHost;
    if (!host.showOpenFilePicker) {
      input.current?.click();
      return;
    }
    let file: File;
    try {
      const [handle] = await host.showOpenFilePicker({
        id: 'grkit-firmware',
        startIn: 'downloads',
        types: [
          {
            description: t.nativeFileType,
            accept: { 'application/zip': ['.zip'], 'application/octet-stream': ['.bin'] },
          },
        ],
      });
      if (!handle) return;
      file = await handle.getFile();
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') return;
      // A picker that the page cannot use, such as in a frame, leaves the file input.
      console.error(error);
      input.current?.click();
      return;
    }
    void read(file);
  }

  function chooseFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    setFileKey((key) => key + 1);
    if (file) void read(file);
  }

  const carriesFile = (event: DragEvent) => event.dataTransfer.types.includes('Files');

  function dragOver(event: DragEvent<HTMLDialogElement>) {
    if (!carriesFile(event) || reading) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = 'copy';
    setDragging(true);
  }

  function dragLeave(event: DragEvent<HTMLDialogElement>) {
    if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDragging(false);
  }

  function drop(event: DragEvent<HTMLDialogElement>) {
    if (!carriesFile(event)) return;
    event.preventDefault();
    setDragging(false);
    const file = event.dataTransfer.files[0];
    if (file && !reading) void read(file);
  }

  const done = status.state === 'done' ? status : null;

  return (
    <dialog
      ref={ownDialog}
      className="dialog native-dialog"
      aria-labelledby={`${ids}-title`}
      data-dragging={dragging || undefined}
      onCancel={(event) => {
        event.preventDefault();
        close();
      }}
      onDragOver={dragOver}
      onDragLeave={dragLeave}
      onDrop={drop}
    >
      <div className="dialog-head">
        <h3 id={`${ids}-title`} className="dialog-title">
          {t.nativeTitle}
        </h3>
        <button type="button" className="button small" onClick={close}>
          {t.close}
        </button>
      </div>

      <div className="dialog-body">
        <p className="note">{t.nativeIntro}</p>

        <ol className="native-steps">
          <li>
            <p className="status">{t.nativeStepDownload}</p>
            <a className="button small" href={FIRMWARE_PAGE} target="_blank" rel="noreferrer">
              {t.nativeOpenRicoh}
            </a>
          </li>
          <li>
            <p className="status">{t.nativeStepChoose}</p>
            <button type="button" className="upload" disabled={reading} onClick={() => void pick()}>
              {t.nativeChoose}
            </button>
            <input
              key={fileKey}
              ref={input}
              type="file"
              accept={ACCEPT}
              hidden
              onChange={chooseFile}
            />
          </li>
          <li>
            <p className="status">{t.nativeStepRead}</p>
            <div className="native-status" aria-live="polite">
              {reading && (
                <div className="native-reading">
                  <span className="spinner" aria-hidden="true" />
                  <span className="status">{t.nativeReading}</span>
                  <button type="button" className="button small" onClick={cancel}>
                    {t.cancel}
                  </button>
                </div>
              )}
              {status.state === 'failed' && (
                <p className="status error">{t.nativeErrors[status.problem]}</p>
              )}
              {done && (
                <>
                  <ul className="native-done">
                    {done.effects.map((effect) => (
                      <li key={effect.id}>
                        <CoverIcon cover={effect.cover} />
                        <span className="import-done-name">{effect.name}</span>
                      </li>
                    ))}
                  </ul>
                  <p className="status">{t.nativeDone(done.effects[0]?.imported.version ?? '')}</p>
                  {done.full && <p className="status">{t.nativeUnchecked(MAX_EFFECTS)}</p>}
                </>
              )}
            </div>
          </li>
        </ol>

        {done && (
          <div className="dialog-actions">
            <button type="button" className="button primary" onClick={close}>
              {t.done}
            </button>
          </div>
        )}

        <div className="import-about">
          <p className="note">{t.nativePrivacy}</p>
          <p className="note">{t.nativeLicense}</p>
        </div>
      </div>
    </dialog>
  );
}
