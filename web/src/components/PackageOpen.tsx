import { useRef } from 'react';
import { sentences, type Strings } from '../i18n';

/** The read of a GRKit.zip of an earlier download. */
export type PackageStatus =
  | { state: 'idle' | 'reading' | 'error' }
  /** `added` imports came back, and `locked` is true when native looks need an unlock again. */
  | { state: 'done'; added: number; locked: boolean };

/**
 * Opens a GRKit.zip of an earlier download, so the page shows what it holds: its camera, its
 * modules, its film looks, and the imports of the browser that made it.
 */
export function PackageOpen({
  strings: t,
  status,
  onOpen,
}: {
  strings: Strings;
  status: PackageStatus;
  onOpen: (file: File) => void;
}) {
  const input = useRef<HTMLInputElement>(null);
  return (
    <div className="note-group">
      <p className="note">
        {t.packageHint}{' '}
        <button
          type="button"
          className="link-button"
          disabled={status.state === 'reading'}
          onClick={() => input.current?.click()}
        >
          {t.packageOpen}
        </button>
        <input
          ref={input}
          className="visually-hidden"
          type="file"
          accept=".zip,application/zip"
          tabIndex={-1}
          aria-hidden="true"
          onChange={(event) => {
            const file = event.target.files?.[0];
            event.target.value = '';
            if (file) onOpen(file);
          }}
        />
      </p>
      <div aria-live="polite">
        {status.state === 'reading' && <p className="status">{t.packageReading}</p>}
        {status.state === 'error' && <p className="status error">{t.packageError}</p>}
        {status.state === 'done' && (
          <p className="status">
            {sentences(
              t.packageDone,
              ...(status.added > 0 ? [t.packageImports(status.added)] : []),
              ...(status.locked ? [t.packageLocked] : []),
            )}
          </p>
        )}
      </div>
    </div>
  );
}
