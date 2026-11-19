import { useId, useImperativeHandle, useRef, useState, type FormEvent, type Ref } from 'react';
import type { CatalogEffect, Cover, ImportedEffect } from '../catalog';
import { customCoverOf, customizationOf, siteCoverFields } from '../customCovers';
import type { Strings } from '../i18n';
import { editableCoverFields, relabelImport, relabelNative, sameCoverFields } from '../import/edit';
import { validCover, validName, type CoverFields } from '../import/names';
import { individualCover, isMonoLook, nativeLook, type CoverScheme, type Customization } from '../packageCovers';
import { CoverFieldset, useCoverEditor } from './CoverEditor';

interface Props {
  strings: Strings;
  /** Keeps the changed import in the place of the one before. */
  onSave: (effect: ImportedEffect) => void;
  /** Keeps the customization of a site film look; an empty one makes it the site's again. */
  onSaveSite: (id: string, custom: Customization) => void;
  ref?: Ref<EditDialogHandle>;
}

/** Opens the dialog from a button of a row: for an import, or for a site film look. */
export interface EditDialogHandle {
  /** `scheme` gives the cover of a native look without an individual cover. */
  open: (effect: ImportedEffect, scheme: CoverScheme) => void;
  /** `scheme` gives the cover of the look without a custom cover. */
  openSite: (effect: CatalogEffect, custom: Customization | undefined, scheme: CoverScheme) => void;
}

/** What the dialog edits: an import, or a site film look with its customization. */
type Target =
  | { kind: 'import'; effect: ImportedEffect }
  | { kind: 'native'; effect: ImportedEffect; base: Cover }
  | {
      kind: 'site';
      effect: CatalogEffect & { cover: Cover };
      custom: Customization | undefined;
      scheme: CoverScheme;
    };

const NO_COVER: CoverFields = {
  template: 'box',
  look: 'gray',
  brand: '',
  name: '',
  number: '',
  band: false,
};

/**
 * Changes the name of a LUT import or a site film look, and its film cover icon. Restored imports
 * without cover metadata get a cover deduced from the name so the user can set layout and colors.
 * A rename that leaves the cover fields as opened keeps the GRF icon pixels. A site film look
 * keeps its name and cover as a customization: its own name and its gray and film box covers are
 * none, and "Use the default cover" gives it the cover of the scheme again. A native look keeps its
 * name and cover in its import, by the same rule of covers.
 */
export function EditDialog({ strings: t, onSave, onSaveSite, ref }: Props) {
  const ids = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const [target, setTarget] = useState<Target | null>(null);
  const [name, setName] = useState('');
  const [openedCover, setOpenedCover] = useState<CoverFields>(NO_COVER);
  const coverEditor = useCoverEditor(NO_COVER);
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState(false);

  function show(next: Target, shownName: string, cover: CoverFields) {
    setTarget(next);
    setName(shownName);
    setOpenedCover(cover);
    coverEditor.cancel();
    void coverEditor.show(cover);
    setSaving(false);
    setFailed(false);
    dialog.current?.showModal();
  }

  useImperativeHandle(ref, () => ({
    open(effect, scheme) {
      const base = nativeLook(effect.id)?.cover;
      const fields = base && effect.imported.kind === 'native' && siteCoverFields(base, effect.cover, scheme);
      if (base && fields) show({ kind: 'native', effect, base }, effect.name, fields);
      else show({ kind: 'import', effect }, effect.name, editableCoverFields(effect) ?? NO_COVER);
    },
    openSite(effect, custom, scheme) {
      const fields = siteCoverFields(effect.cover, custom?.cover, scheme, isMonoLook(effect));
      if (!fields || !effect.cover) return;
      show(
        { kind: 'site', effect: { ...effect, cover: effect.cover }, custom, scheme },
        custom?.name ?? effect.name,
        fields,
      );
    },
  }));

  const cover = coverEditor.cover;
  const badName = !validName(name);
  const ready =
    target !== null && !badName && validCover(cover) && coverEditor.error === null && !saving;

  function close() {
    coverEditor.cancel();
    dialog.current?.close();
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!ready || !target) return;
    if (target.kind === 'site') {
      const { effect } = target;
      onSaveSite(effect.id, customizationOf(effect, name, customCoverOf(effect.cover, cover)));
      close();
      return;
    }
    const { effect } = target;
    setSaving(true);
    setFailed(false);
    try {
      if (target.kind === 'native') {
        onSave(await relabelNative(target.effect, target.base, name, customCoverOf(target.base, cover)));
        close();
        return;
      }
      // Unchanged cover fields → rename only; do not rewrite camera icon pixels.
      const nextCover = sameCoverFields(cover, openedCover) ? null : cover;
      onSave(await relabelImport(effect, name, nextCover));
      close();
    } catch (error) {
      console.error(error);
      setFailed(true);
    } finally {
      setSaving(false);
    }
  }

  /** Keeps the name as typed and gives the built-in look the cover of the scheme again. */
  async function resetCover() {
    if (!target || target.kind === 'import' || badName) return;
    if (target.kind === 'site') {
      onSaveSite(target.effect.id, customizationOf(target.effect, name, null));
      close();
      return;
    }
    setSaving(true);
    setFailed(false);
    try {
      onSave(await relabelNative(target.effect, target.base, name, null));
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
      {target && (
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
          <CoverFieldset strings={t} editor={coverEditor} disabled={saving} />
          <div aria-live="polite">{failed && <p className="status error">{t.editError}</p>}</div>
          <div className="dialog-actions">
            {(target.kind === 'site'
              ? individualCover(target.effect.cover, target.custom?.cover)
              : target.kind === 'native' && individualCover(target.base, target.effect.cover)) && (
              <button type="button" className="button" disabled={badName || saving} onClick={() => void resetCover()}>
                {t.coverReset}
              </button>
            )}
            <button type="submit" className="button primary" disabled={!ready} aria-busy={saving}>
              {t.editSave}
            </button>
          </div>
        </form>
      )}
    </dialog>
  );
}
