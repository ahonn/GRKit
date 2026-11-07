import { useState } from 'react';
import type { Strings } from '../i18n';
import { coverProblem } from '../import/lut';
import { coverIcon, validCover, validCoverText, type CoverFields } from '../import/names';
import { TaskSlot, type TaskResult } from '../import/task';
import { CoverIcon } from './EffectIcon';

/** The cover fields of an effect and why the camera cannot draw them (`coverProblem`). */
export interface CoverEditor {
  cover: CoverFields;
  /** Why the camera cannot draw the cover, or null while it can or while the check runs. */
  error: string | null;
  /** Sets the cover and checks that the camera can draw it. */
  show: (next: CoverFields) => Promise<TaskResult<string | null>>;
  /** Checks other fields without showing them. A later check or edit replaces this one. */
  probe: (fields: CoverFields) => Promise<TaskResult<string | null>>;
  /** Stops the running check. */
  cancel: () => void;
}

/** The state of a cover editor that starts with `initial`. Only the newest check changes it. */
export function useCoverEditor(initial: CoverFields): CoverEditor {
  const [cover, setCover] = useState<CoverFields>(initial);
  const [error, setError] = useState<string | null>(null);
  const [check] = useState(() => new TaskSlot());

  function show(next: CoverFields): Promise<TaskResult<string | null>> {
    setCover(next);
    setError(null);
    if (!validCover(next)) {
      check.cancel();
      return Promise.resolve({ current: false });
    }
    const checked = check.run(() => coverProblem(next));
    void checked.then((result) => {
      if (result.current) setError(result.value);
    });
    return checked;
  }

  return {
    cover,
    error,
    show,
    probe: (fields) => check.run(() => coverProblem(fields)),
    cancel: () => check.cancel(),
  };
}

interface Props {
  strings: Strings;
  editor: CoverEditor;
  disabled?: boolean;
}

/** The film cover icon of an effect, with its brand, film name, number, and band to edit. */
export function CoverFieldset({ strings: t, editor, disabled }: Props) {
  const { cover, error } = editor;
  const edit = <K extends keyof CoverFields>(key: K, value: CoverFields[K]) =>
    void editor.show({ ...cover, [key]: value });
  return (
    <fieldset className="import-icon" disabled={disabled}>
      <legend className="field-label">{t.iconHeading}</legend>
      <div className="import-icon-preview">
        <CoverIcon cover={coverIcon(cover)} />
      </div>
      <details className="import-icon-edit" open>
        <summary>{t.iconEdit}</summary>
        <div className="import-icon-fields">
          {(
            [
              ['brand', t.iconBrand, true],
              ['name', t.iconFilm, true],
              ['number', t.iconNumber, false],
            ] as const
          ).map(([key, label, required]) => (
            <label key={key} className="field">
              <span className="field-label">{label}</span>
              <input
                className="import-text"
                type="text"
                value={cover[key]}
                maxLength={12}
                aria-invalid={(required || cover[key] !== '') && !validCoverText(cover[key])}
                onChange={(event) => edit(key, event.target.value)}
              />
            </label>
          ))}
          <label className="check">
            <input type="checkbox" checked={cover.band} onChange={(event) => edit('band', event.target.checked)} />
            <span>{t.iconBand}</span>
          </label>
          {!validCover(cover) && <p className="note error">{t.iconNote}</p>}
        </div>
      </details>
      {error && <p className="note error import-icon-error">{t.iconTooWide}</p>}
    </fieldset>
  );
}
