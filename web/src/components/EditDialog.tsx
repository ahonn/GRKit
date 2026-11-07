import { useId, useImperativeHandle, useRef, useState, type FormEvent, type Ref } from 'react';
import type { ImportedEffect } from '../catalog';
import type { Strings } from '../i18n';
import { coverFields, relabelImport } from '../import/edit';
import { validCover, validName, type CoverFields } from '../import/names';
import { CoverFieldset, useCoverEditor } from './CoverEditor';
import { EffectIcon } from './EffectIcon';

interface Props {
  strings: Strings;
  /** Keeps the changed import in the place of the one before. */
  onSave: (effect: ImportedEffect) => void;
  ref?: Ref<EditDialogHandle>;
}

/** Opens the dialog for an import from a button of its row. */
export interface EditDialogHandle {
  open: (effect: ImportedEffect) => void;
}

const NO_COVER: CoverFields = { brand: '', name: '', number: '', band: false };

/**
 * Changes the name of a LUT import, and the text of its film cover icon when it has one. The
 * effect keeps its ID, so the camera keeps its Fn adjustments.
 */
export function EditDialog({ strings: t, onSave, ref }: Props) {
  const ids = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const [effect, setEffect] = useState<ImportedEffect | null>(null);
  const [name, setName] = useState('');
  // The cover fields of an import with a film box cover, else null: its icon stays.
  const [fields, setFields] = useState<CoverFields | null>(null);
  const coverEditor = useCoverEditor(NO_COVER);
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState(false);

  useImperativeHandle(ref, () => ({
    open(next) {
      const cover = coverFields(next.cover);
      setEffect(next);
      setName(next.name);
      setFields(cover);
      coverEditor.cancel();
      if (cover) void coverEditor.show(cover);
      setSaving(false);
      setFailed(false);
      dialog.current?.showModal();
    },
  }));

  const cover = fields && coverEditor.cover;
  const badName = !validName(name);
  const ready = effect !== null && !badName && (!cover || (validCover(cover) && coverEditor.error === null)) && !saving;

  function close() {
    coverEditor.cancel();
    dialog.current?.close();
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!ready || !effect) return;
    setSaving(true);
    setFailed(false);
    try {
      onSave(await relabelImport(effect, name, cover));
      close();
    } catch (error) {
      console.error(error);
      setFailed(true);
    } finally {
      setSaving(false);
    }
  }

  return (
    <dialog
      ref={dialog}
      className="dialog"
      aria-labelledby={`${ids}-title`}
      onCancel={(event) => {
        event.preventDefault();
        close();
      }}
    >
      <div className="dialog-head">
        <h3 id={`${ids}-title`} className="dialog-title">
          {t.editTitle}
        </h3>
        <button type="button" className="button small" onClick={close}>
          {t.close}
        </button>
      </div>
      {effect && (
        <form className="dialog-body" onSubmit={submit}>
          <div className="import-fields">
            <label className="field">
              <span className="field-label">{t.importName}</span>
              <input
                className="import-text"
                type="text"
                value={name}
                maxLength={16}
                disabled={saving}
                aria-invalid={badName}
                aria-describedby={badName ? `${ids}-name-note` : undefined}
                onChange={(event) => setName(event.target.value)}
              />
              {badName && (
                <span id={`${ids}-name-note`} className="note error">
                  {t.importNameNote}
                </span>
              )}
            </label>
          </div>
          {cover ? (
            <CoverFieldset strings={t} editor={coverEditor} disabled={saving} />
          ) : (
            <div className="import-icon">
              <span className="field-label">{t.iconHeading}</span>
              <div className="import-icon-preview">
                <EffectIcon effect={effect} />
              </div>
            </div>
          )}
          <div aria-live="polite">{failed && <p className="status error">{t.editError}</p>}</div>
          <div className="dialog-actions">
            <button type="submit" className="button primary" disabled={!ready} aria-busy={saving}>
              {t.editSave}
            </button>
          </div>
        </form>
      )}
    </dialog>
  );
}
