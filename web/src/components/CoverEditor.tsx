import { useState } from 'react';
import {
  COVER_LAYOUTS,
  COVER_LOOK_IDS,
  colorsFromLook,
  coverLayout,
  isCoverEditorLookId,
  layoutFields,
  randomCoverLayout,
  randomCoverLook,
  rgbCss,
  type CoverLayout,
  type CoverLookId,
} from '../coverPalette';
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

const layoutLabel = (t: Strings, layout: CoverLayout): string =>
  ({
    box: t.iconTemplateBox,
    box2: t.iconTemplateBox2,
    cine: t.iconTemplateCine,
    stack: t.iconTemplateStack,
  })[layout];

const lookLabel = (t: Strings, look: CoverLookId): string =>
  (isCoverEditorLookId(look) ? t.iconLooks[look] : undefined) ?? look;

/** Looks to show: the short editor set, plus the current look when it is a stock-only look. */
function editorLooks(current: CoverLookId): readonly CoverLookId[] {
  return isCoverEditorLookId(current) ? COVER_LOOK_IDS : [current, ...COVER_LOOK_IDS];
}

/** The film cover icon of an effect, with layout, colors, brand, film name, number, and band. */
export function CoverFieldset({ strings: t, editor, disabled }: Props) {
  const { cover, error } = editor;
  const edit = <K extends keyof CoverFields>(key: K, value: CoverFields[K]) =>
    void editor.show({ ...cover, [key]: value });
  const layout = coverLayout(cover.template, cover.band);
  return (
    <fieldset className="import-icon" disabled={disabled}>
      <legend className="field-label">{t.iconHeading}</legend>
      <div className="import-icon-preview">
        <CoverIcon cover={coverIcon(cover)} />
        <button
          type="button"
          className="button small"
          disabled={disabled}
          onClick={() =>
            void editor.show({
              ...cover,
              look: randomCoverLook(cover.look),
              ...layoutFields(randomCoverLayout(layout)),
            })
          }
        >
          {t.iconRandomColors}
        </button>
      </div>
      <details className="import-icon-edit" open>
        <summary>{t.iconEdit}</summary>
        <div className="import-icon-fields">
          <label className="field">
            <span className="field-label">{t.iconTemplate}</span>
            <select
              className="import-text"
              value={layout}
              onChange={(event) =>
                void editor.show({ ...cover, ...layoutFields(event.target.value as CoverLayout) })
              }
            >
              {COVER_LAYOUTS.map((option) => (
                <option key={option} value={option}>
                  {layoutLabel(t, option)}
                </option>
              ))}
            </select>
          </label>
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
          <details className="import-cover-looks">
            <summary className="field-label">{t.iconColors}</summary>
            <div className="import-cover-swatches" role="radiogroup" aria-label={t.iconColors}>
              {editorLooks(cover.look).map((look) => {
                const colors = colorsFromLook(look);
                return (
                  <label key={look} className="import-cover-swatch" title={lookLabel(t, look)}>
                    <input
                      type="radio"
                      name="cover-look"
                      value={look}
                      checked={cover.look === look}
                      onChange={() => edit('look', look)}
                    />
                    <span className="import-cover-chip" aria-hidden="true">
                      {(['band', 'brand', 'body', 'text', 'accent'] as const).map((slot) => (
                        <span key={slot} style={{ background: rgbCss(colors[slot]) }} />
                      ))}
                    </span>
                    <span className="visually-hidden">{lookLabel(t, look)}</span>
                  </label>
                );
              })}
            </div>
          </details>
          {!validCover(cover) && <p className="note error">{t.iconNote}</p>}
        </div>
      </details>
      {error && <p className="note error import-icon-error">{t.iconTooWide}</p>}
    </fieldset>
  );
}
