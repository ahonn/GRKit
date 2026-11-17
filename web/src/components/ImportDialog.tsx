import {
  useCallback,
  useId,
  useImperativeHandle,
  useRef,
  useState,
  type ChangeEvent,
  type FormEvent,
  type Ref,
} from 'react';
import { MAX_EFFECTS, type ImportedEffect, type LutImport } from '../catalog';
import type { Strings } from '../i18n';
import { trackHaldImage, trackImport } from '../analytics';
import { guessInput, importLut, saveHaldImage } from '../import/lut';
import {
  coverFromFile,
  INPUT_NAMES,
  INPUTS,
  isPreset,
  lutAccept,
  nameFromFile,
  validCover,
  validName,
  type CoverFields,
  type Input,
} from '../import/names';
import { importCoverLook } from '../coverPalette';
import { presetRecipePlan } from '../import/presetRecipe';
import { TaskSlot } from '../import/task';
import { loadWasm } from '../package';
import { newRecipeId, onCount, type Family, type Recipe } from '../recipes';
import { CoverFieldset, useCoverEditor } from './CoverEditor';
import { CoverIcon } from './EffectIcon';

interface Props {
  strings: Strings;
  baseGamma: Uint16Array;
  /** The IDs of the catalog effects and of the earlier imports. */
  taken: ReadonlySet<string>;
  /** Adds the effect to the page. Returns true when the effect stays unchecked because the
   * package already has `MAX_EFFECTS` checked effects. */
  onImport: (effect: ImportedEffect) => boolean;
  /** The camera line of the page, for a recipe from a preset. */
  family: Family;
  /** The recipe library of that camera line. */
  recipes: readonly Recipe[];
  /** Adds a recipe to that library. */
  onAddRecipe: (recipe: Recipe) => void;
  ref?: Ref<ImportDialogHandle>;
}

/** Opens the dialog from a button elsewhere on the page. */
export interface ImportDialogHandle {
  open: () => void;
}

type Status =
  | { state: 'idle' }
  /** `fraction`: the progress of the fit, from 0 to 1. */
  | { state: 'importing'; fraction: number }
  | { state: 'failed'; message: string }
  | {
      state: 'done';
      effect: ImportedEffect<LutImport>;
      full: boolean;
      /** A message about a recipe from the preset, or null when none was considered. */
      recipeNote: string | null;
    };

/** `auto` uses the input that the LUT data suggests (`guessInput`). */
type InputChoice = 'auto' | Input;

const EMPTY_COVER: CoverFields = {
  template: 'box',
  look: 'gray',
  brand: 'CUSTOM',
  name: 'LOOK',
  number: '',
  band: false,
};

/** Any touch pointer, not only the primary one, so an iPad with a trackpad still counts. */
const hasTouch = () =>
  typeof window !== 'undefined' && (window.matchMedia?.('(any-pointer: coarse)').matches ?? false);

/**
 * The LUT import dialog. `ref.open()` opens it, so several buttons on the page can open the
 * same dialog. The dialog fits the chosen file into an effect of the page, with a film cover
 * icon that the user can edit, then describes how closely the effect follows the LUT and shows
 * the warnings.
 */
export function ImportDialog({
  strings: t,
  baseGamma,
  taken,
  onImport,
  family,
  recipes,
  onAddRecipe,
  ref,
}: Props) {
  const ids = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const [file, setFile] = useState<File | null>(null);
  const [name, setName] = useState('');
  const [choice, setChoice] = useState<InputChoice>('auto');
  // The input that the LUT data suggests, or null while the worker reads the file.
  const [guessed, setGuessed] = useState<Input | null>('standard');
  const coverEditor = useCoverEditor(EMPTY_COVER);
  const { cover } = coverEditor;
  const [status, setStatus] = useState<Status>({ state: 'idle' });
  // Checked by default: add a recipe with the exposure, grain, and white balance of the preset.
  const [addRecipe, setAddRecipe] = useState(true);
  // A new key empties the file input for the next import.
  const [fileKey, setFileKey] = useState(0);
  // Only the newest guess, fit, and cover check change the dialog.
  const [guess] = useState(() => new TaskSlot());
  const [fit] = useState(() => new TaskSlot());
  // An unmount stops a running guess and fit, as a new photo stops a RAW decode.
  const ownDialog = useCallback(
    (element: HTMLDialogElement | null) => {
      dialog.current = element;
      return () => {
        fit.cancel();
        guess.cancel();
        dialog.current = null;
      };
    },
    [fit, guess],
  );

  const importing = status.state === 'importing';
  const preset = file !== null && isPreset(file.name);
  // A preset has no input encoding: the fit always uses Standard.
  const reading = !preset && choice === 'auto' && guessed === null;
  const input: Input = preset ? 'standard' : choice === 'auto' ? (guessed ?? 'standard') : choice;
  const badName = file !== null && !validName(name);
  const ready =
    file !== null &&
    validName(name) &&
    validCover(cover) &&
    coverEditor.error === null &&
    !reading &&
    !importing;

  function reset() {
    setFile(null);
    setName('');
    setChoice('auto');
    guess.cancel();
    setGuessed('standard');
    coverEditor.cancel();
    void coverEditor.show(EMPTY_COVER);
    setStatus({ state: 'idle' });
    setAddRecipe(true);
    setFileKey((key) => key + 1);
  }

  function open() {
    if (status.state === 'done' || status.state === 'failed') reset();
    dialog.current?.showModal();
  }

  useImperativeHandle(ref, () => ({ open }));

  function close() {
    fit.cancel();
    if (importing) setStatus({ state: 'idle' });
    dialog.current?.close();
  }

  async function chooseFile(event: ChangeEvent<HTMLInputElement>) {
    const chosen = event.target.files?.[0] ?? null;
    setFile(chosen);
    setStatus({ state: 'idle' });
    if (!chosen) {
      guess.cancel();
      setGuessed('standard');
      return;
    }
    setName(nameFromFile(chosen.name));
    void fillCover({ ...coverFromFile(chosen.name), look: importCoverLook() });
    // A preset has no input encoding; skip the guess and import as Standard.
    if (isPreset(chosen.name)) {
      guess.cancel();
      setGuessed('standard');
      return;
    }
    setGuessed(null);
    try {
      const found = await guess.run((signal) => guessInput(chosen, baseGamma, signal));
      if (found.current) setGuessed(found.value);
    } catch (error) {
      // A file that cannot be read fails again at the import, with the reason.
      setGuessed('standard');
      console.error(error);
    }
  }

  /**
   * Shows a cover from a file name. A number that the camera cannot draw is left out, or it
   * becomes the film name when the name is only the placeholder.
   */
  async function fillCover(fields: CoverFields) {
    const checked = await coverEditor.show(fields);
    if (fields.number === '' || !checked.current || checked.value === null) return;
    const withoutNumber = { ...fields, number: '' };
    if (fields.name !== EMPTY_COVER.name) {
      void coverEditor.show(withoutNumber);
      return;
    }
    const numberAsName = { ...fields, name: fields.number, number: '' };
    const fits = await coverEditor.probe(numberAsName);
    if (fits.current) void coverEditor.show(fits.value === null ? numberAsName : withoutNumber);
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!ready || !file) return;
    setStatus({ state: 'importing', fraction: 0 });
    try {
      const imported = await fit.run((signal) =>
        importLut(file, name, input, cover, baseGamma, taken, signal, (fraction) => {
          if (!signal.aborted) setStatus({ state: 'importing', fraction });
        }),
      );
      if (!imported.current) return;
      const { grade, warnings } = imported.value.imported;
      trackImport(file.name, input, choice !== 'auto', {
        result: 'ok',
        grade,
        warnings: warnings.length,
      });
      const full = onImport(imported.value);
      const recipeNote = await addPresetRecipe(imported.value, addRecipe);
      setStatus({ state: 'done', effect: imported.value, full, recipeNote });
    } catch (error) {
      console.error(error);
      trackImport(file.name, input, choice !== 'auto', { result: 'failed' });
      setStatus({
        state: 'failed',
        message: error instanceof Error ? error.message : String(error),
      });
    }
  }

  function inputLabel(item: Input): string {
    if (item === 'standard') return t.inputStandard;
    if (item === 'srgb') return t.inputSrgb;
    return INPUT_NAMES[item];
  }

  /**
   * Builds a recipe from the preset hints when the checkbox is on, adds it through the library
   * hook, and returns the done-view note. Without hints, or with the box off, returns null.
   */
  async function addPresetRecipe(
    effect: ImportedEffect<LutImport>,
    wantsRecipe: boolean,
  ): Promise<string | null> {
    const hints = effect.imported.hints;
    if (!wantsRecipe || !hints || !isPreset(effect.imported.source)) return null;
    try {
      const { recipeFromPreset } = await loadWasm();
      const text = recipeFromPreset(hints, family, effect.name, effect.id);
      if (text == null) return null;
      const recipe = JSON.parse(text) as Recipe;
      const plan = presetRecipePlan(true, true, recipes.length, onCount(recipes));
      if (plan === 'full') return t.importRecipeFull;
      const taken = new Set(recipes.map((item) => item.id));
      onAddRecipe({
        ...recipe,
        id: newRecipeId(taken),
        enabled: plan === 'add',
        hidden: false,
        name: effect.name,
      });
      return t.importRecipeAdded(effect.name);
    } catch (error) {
      console.error(error);
      return null;
    }
  }

  const done = status.state === 'done' ? status : null;

  return (
    <dialog
      ref={ownDialog}
      className="dialog"
      aria-labelledby={`${ids}-title`}
      onCancel={(event) => {
        event.preventDefault();
        close();
      }}
    >
      <div className="dialog-head">
        <h3 id={`${ids}-title`} className="dialog-title">
          {t.importTitle}
        </h3>
        <button type="button" className="button small" onClick={close}>
          {t.close}
        </button>
      </div>

      {done ? (
        <div className="dialog-body">
          <div className="import-done">
            <CoverIcon cover={done.effect.cover} />
            <div>
              <p className="import-done-name">{done.effect.name}</p>
              <p className="status">{t.fitText[done.effect.imported.grade]}</p>
              {!isPreset(done.effect.imported.source) && (
                <p className="status">{t.importUsed(INPUT_NAMES[done.effect.imported.input])}</p>
              )}
            </div>
          </div>
          {done.full && <p className="status">{t.importUnchecked(MAX_EFFECTS)}</p>}
          {done.recipeNote && <p className="status">{done.recipeNote}</p>}
          {done.effect.imported.warnings.length > 0 && (
            <div className="import-result">
              <p className="status error">{t.importWarnings}</p>
              <ul className="import-warnings">
                {done.effect.imported.warnings.map((warning) => (
                  <li key={warning} lang="en">
                    {warning}
                  </li>
                ))}
              </ul>
            </div>
          )}
          <div className="dialog-actions">
            <button type="button" className="button" onClick={reset}>
              {t.importAnother}
            </button>
            <button type="button" className="button primary" onClick={close}>
              {t.done}
            </button>
          </div>
        </div>
      ) : (
        <form className="dialog-body" onSubmit={submit}>
          <p className="note">{t.importNote}</p>

          <label htmlFor={`${ids}-file`} className="button upload" aria-disabled={importing}>
            <span className="import-file-name">{file?.name ?? t.importFile}</span>
          </label>
          <input
            key={fileKey}
            id={`${ids}-file`}
            className="visually-hidden"
            type="file"
            accept={lutAccept(hasTouch())}
            disabled={importing}
            onChange={chooseFile}
          />
          {preset && <p className="note">{t.importPresetNote}</p>}
          {preset && (
            <label className="check">
              <input
                type="checkbox"
                checked={addRecipe}
                disabled={importing}
                onChange={(event) => setAddRecipe(event.target.checked)}
              />
              <span>{t.importPresetRecipe}</span>
            </label>
          )}

          <div className="import-fields">
            <label className="field">
              <span className="field-label">{t.importName}</span>
              <input
                className="import-text"
                type="text"
                value={name}
                maxLength={16}
                disabled={importing}
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
            {!preset && (
              <label className="field">
                <span className="field-label">{t.importInput}</span>
                <select
                  className="import-text"
                  value={choice}
                  disabled={importing}
                  onChange={(event) => setChoice(event.target.value as InputChoice)}
                >
                  <option value="auto">
                    {guessed === null ? t.inputAutoReading : t.inputAuto(INPUT_NAMES[guessed])}
                  </option>
                  {INPUTS.map((item) => (
                    <option key={item} value={item}>
                      {inputLabel(item)}
                    </option>
                  ))}
                </select>
                {choice === 'auto' && guessed !== null && (
                  <span className="note">
                    {guessed === 'standard' ? t.inputAutoStandard : t.inputAutoLog}
                  </span>
                )}
              </label>
            )}
          </div>

          <CoverFieldset strings={t} editor={coverEditor} disabled={importing} />

          <details className="hald">
            <summary>{t.haldSummary}</summary>
            <p className="note">{t.haldNote}</p>
            <button
              type="button"
              className="button small"
              onClick={() => void saveHaldImage().then(trackHaldImage)}
            >
              {t.haldDownload}
            </button>
          </details>

          <div aria-live="polite">
            {status.state === 'importing' && (
              <div className="import-progress">
                <p className="status">{t.importing}</p>
                <progress max={1} value={status.fraction} aria-label={t.importing} />
                <span className="import-percent">{Math.round(status.fraction * 100)}%</span>
              </div>
            )}
            {status.state === 'failed' && (
              <p className="status error">
                {t.importError} {status.message}
              </p>
            )}
          </div>

          <div className="dialog-actions">
            <button type="submit" className="button primary" disabled={!ready} aria-busy={importing}>
              {t.importButton}
            </button>
          </div>

          <div className="import-about">
            <p className="note">
              <strong>{t.importHowTitle}</strong> {t.importHow}
            </p>
            <p className="note">
              <strong>{t.importLimitsTitle}</strong> {t.importLimits}
            </p>
          </div>
        </form>
      )}
    </dialog>
  );
}
