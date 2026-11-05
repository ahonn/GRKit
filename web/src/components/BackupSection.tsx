import { useRef, useState, type ChangeEvent } from 'react';
import { createBackup, readBackup, type BackupContents, type BackupSettings } from '../backup';
import type { ImportedEffect } from '../catalog';
import type { Strings } from '../i18n';
import { save } from '../package';

interface Props {
  strings: Strings;
  /** Every imported effect, for every camera, in the order that the page shows them. */
  imports: readonly ImportedEffect[];
  settings: BackupSettings;
  /** Restores the imported effects and the settings of `backup`. */
  onRestore: (backup: BackupContents) => void;
}

type Status = 'idle' | 'saved' | 'done' | 'error';

/**
 * Saves the imported effects and the settings of this browser in one file, and restores them
 * in another browser. Both stores belong to one browser, so a file is the way to move them.
 */
export function BackupSection({ strings: t, imports, settings, onRestore }: Props) {
  const input = useRef<HTMLInputElement>(null);
  const [status, setStatus] = useState<Status>('idle');

  function download() {
    const date = new Date().toISOString().slice(0, 10);
    save(createBackup(imports, settings), `grkit-backup-${date}.json`);
    setStatus('saved');
  }

  async function restore(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = '';
    if (!file) return;
    setStatus('idle');
    try {
      const backup = readBackup(await file.text());
      if (!backup) {
        setStatus('error');
        return;
      }
      onRestore(backup);
      setStatus('done');
    } catch {
      setStatus('error');
    }
  }

  return (
    <section className="section" aria-labelledby="backup-label">
      <h2 id="backup-label">{t.backupTitle}</h2>
      <div className="section-body">
        <p className="note">{t.backupNote}</p>
        <div className="card-actions">
          <button type="button" className="button" onClick={download}>
            {t.backupExport}
          </button>
          <button type="button" className="button" onClick={() => input.current?.click()}>
            {t.backupImport}
          </button>
          <input
            ref={input}
            className="visually-hidden"
            type="file"
            accept=".json,application/json"
            onChange={restore}
          />
        </div>
        <div aria-live="polite">
          {status === 'saved' && <p className="status">{t.backupSaved}</p>}
          {status === 'done' && <p className="status">{t.backupDone}</p>}
          {status === 'error' && <p className="status error">{t.backupError}</p>}
        </div>
      </div>
    </section>
  );
}
