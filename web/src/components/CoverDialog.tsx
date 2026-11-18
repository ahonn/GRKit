import { useId, useImperativeHandle, useRef, useState, type FormEvent, type Ref } from 'react';
import type { CatalogEffect, Cover } from '../catalog';
import { customCoverOf, siteCoverFields } from '../customCovers';
import type { Strings } from '../i18n';
import { validCover, type CoverFields } from '../import/names';
import { CoverFieldset, useCoverEditor } from './CoverEditor';

interface Props {
  strings: Strings;
  /** Keeps the custom cover of a site film look, or removes it for null. */
  onSave: (id: string, cover: Cover | null) => void;
  ref?: Ref<CoverDialogHandle>;
}

/** Opens the dialog for a site film look. */
export interface CoverDialogHandle {
  open: (effect: CatalogEffect, colorful: boolean, custom: Cover | undefined) => void;
}

const NO_COVER: CoverFields = { template: 'box', look: 'gray', brand: '', name: '', number: '', band: false };

interface Opened {
  effect: CatalogEffect & { cover: Cover };
  colorful: boolean;
  custom: boolean;
}

/**
 * Changes the film cover icon of a site film look. The name and the effect stay. Fields that make
 * the default cover remove the custom cover, so the look follows the colorful switch again.
 */
export function CoverDialog({ strings: t, onSave, ref }: Props) {
  const ids = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const [opened, setOpened] = useState<Opened | null>(null);
  const coverEditor = useCoverEditor(NO_COVER);

  useImperativeHandle(ref, () => ({
    open(effect, colorful, custom) {
      const fields = siteCoverFields(effect.cover, colorful, custom);
      if (!fields || !effect.cover) return;
      setOpened({ effect: { ...effect, cover: effect.cover }, colorful, custom: custom !== undefined });
      coverEditor.cancel();
      void coverEditor.show(fields);
      dialog.current?.showModal();
    },
  }));

  const cover = coverEditor.cover;
  const ready = opened !== null && validCover(cover) && coverEditor.error === null;

  function close() {
    coverEditor.cancel();
    dialog.current?.close();
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    if (!ready || !opened) return;
    onSave(opened.effect.id, customCoverOf(opened.effect.cover, opened.colorful, cover));
    close();
  }

  function reset() {
    if (!opened) return;
    onSave(opened.effect.id, null);
    close();
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
          {opened ? t.coverTitleOf(opened.effect.name) : t.coverTitle}
        </h3>
        <button type="button" className="button small" onClick={close}>
          {t.close}
        </button>
      </div>
      {opened && (
        <form className="dialog-body" onSubmit={submit}>
          <CoverFieldset strings={t} editor={coverEditor} />
          <p className="note">{t.coverNote}</p>
          <div className="dialog-actions">
            {opened.custom && (
              <button type="button" className="button" onClick={reset}>
                {t.coverReset}
              </button>
            )}
            <button type="submit" className="button primary" disabled={!ready}>
              {t.editSave}
            </button>
          </div>
        </form>
      )}
    </dialog>
  );
}
