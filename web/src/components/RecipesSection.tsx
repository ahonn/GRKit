import { use, useRef, useState, type ReactNode } from 'react';
import type { Language, Strings } from '../i18n';
import { RECIPE_STRINGS, type RecipeStrings } from '../recipeStrings';
import { copyRecipe } from '../recipeCopy';
import { parseRecipeText, recipeText } from '../recipeText';
import {
  effectCode,
  FAMILIES,
  ISO_LIMITS,
  LIBRARY_MAX,
  libraryWarnings,
  loadModel,
  loadTypeNames,
  NAME_MAX,
  nameFits,
  nativeLook,
  NEUTRAL,
  newRecipe,
  NO_SHIFT,
  recipeProblem,
  type Family,
  type LibraryRecipe,
  type Look,
  type Model,
  type ModelType,
  type Recipe,
} from '../recipes';

interface Props {
  strings: Strings;
  language: Language;
  family: Family;
  library: readonly LibraryRecipe[];
  onChange: (library: LibraryRecipe[]) => void;
  /** The effects of the package, which an effect recipe can use. Empty without Film Look. */
  effects: readonly { id: string; name: string }[];
  /** The package has the Grain module. */
  grainModule: boolean;
  /**
   * Adds a recipe to the library of another camera line: false when that library is full.
   */
  onCopy: (family: Family, item: LibraryRecipe) => boolean;
}

/** A level as the camera shows it: stored 0 to 8 is -4 to +4. */
const level = (stored: number) => (stored === NEUTRAL ? '0' : stored > NEUTRAL ? `+${stored - NEUTRAL}` : `${stored - NEUTRAL}`);
/** A shift as the camera shows it: B14 ... 0 ... A14, or G14 ... 0 ... M14. */
const shift = (stored: number, minus: string, plus: string) =>
  stored === NO_SHIFT ? '0' : stored < NO_SHIFT ? `${minus}${NO_SHIFT - stored}` : `${plus}${stored - NO_SHIFT}`;
/** Exposure compensation in thirds of a stop: -2/3, +1 1/3, +2. */
export function exposureText(thirds: number): string {
  if (thirds === 0) return '0';
  const sign = thirds > 0 ? '+' : '-';
  const whole = Math.floor(Math.abs(thirds) / 3);
  const rest = Math.abs(thirds) % 3;
  return `${sign}${whole ? whole : ''}${whole && rest ? ' ' : ''}${rest ? `${rest}/3` : ''}`;
}
const EXPOSURES = Array.from({ length: 31 }, (_, index) => index - 15);
const LEVEL_KEYS = [
  'Saturation',
  'ColorHue',
  'ImageKey',
  'Contrast',
  'ContrastHighLight',
  'ContrastShadow',
  'Sharpness',
  'Shading',
  'ClarityControl',
];

function Select({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: readonly (readonly [string, string])[];
  onChange: (value: string) => void;
}) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      <select className="import-text" value={value} onChange={(event) => onChange(event.target.value)}>
        {options.map(([key, text]) => (
          <option key={key} value={key}>
            {text}
          </option>
        ))}
      </select>
    </label>
  );
}

/** The name of a value label of the camera in the site language. */
const valueName = (r: RecipeStrings, label: string) => r.values[label] ?? label;

/** The look of a recipe in a few words: the effect or the type. */
function lookText(recipe: Recipe, model: Model, effects: Props['effects'], r: RecipeStrings): string {
  const look = recipe.look;
  if (look.kind === 'effect') {
    return effects.find((effect) => effectCode(effect.id) === look.code)?.name ?? r.lookEffect;
  }
  if (look.kind === 'native') return model.types.find((type) => type.index === look.imageType)?.name ?? r.lookNative;
  return r.none;
}

/**
 * The recipe library of the selected camera line: the recipes that the site writes to the card,
 * in the order of the camera. Each recipe opens in an editor that offers only the values of the
 * camera line.
 */
/** The name of a flagged field in the site language. */
const fieldName = (r: RecipeStrings, field: string) =>
  r.fields[field] ?? r.parameters[field] ?? (field.startsWith('correction') ? r.corrections[Number(field.slice(10))] : field) ?? field;

export function RecipesSection({
  strings: t,
  language,
  family,
  library,
  onChange,
  effects,
  grainModule,
  onCopy,
}: Props) {
  const r = RECIPE_STRINGS[language];
  const model = use(loadModel(family, language));
  const names = use(loadTypeNames());
  const dialog = useRef<HTMLDialogElement>(null);
  const importDialog = useRef<HTMLDialogElement>(null);
  const [pasted, setPasted] = useState('');
  const [status, setStatus] = useState<string | null>(null);
  const [draft, setDraft] = useState<LibraryRecipe | null>(null);
  const [editing, setEditing] = useState(-1);
  const [problem, setProblem] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const warnings = libraryWarnings(
    library.map((item) => item.recipe),
    grainModule,
    new Map(effects.map((effect) => [effectCode(effect.id), effect.id])),
  );

  function open(index: number) {
    const name = `Recipe ${library.length + 1}`;
    setDraft(index < 0 ? { recipe: newRecipe(family, model, name), author: '', source: '' } : structuredClone(library[index]!));
    setEditing(index);
    setProblem(null);
    dialog.current?.showModal();
  }

  function close() {
    dialog.current?.close();
    setDraft(null);
  }

  async function save() {
    if (!draft) return;
    setSaving(true);
    try {
      const found = nameFits(draft.recipe.name) ? await recipeProblem(draft.recipe) : r.nameHint;
      if (found) {
        setProblem(found);
        return;
      }
      const next = [...library];
      const saved = { recipe: draft.recipe, author: draft.author, source: draft.source };
      if (editing < 0) next.push(saved);
      else next[editing] = saved;
      onChange(next);
      close();
    } finally {
      setSaving(false);
    }
  }

  /** Reads the pasted text and opens it in the editor with its flags. */
  function readText() {
    const parsed = parseRecipeText(pasted, family, model, effects, names);
    importDialog.current?.close();
    setPasted('');
    setDraft({ recipe: parsed.recipe, author: parsed.author, source: parsed.source, flags: parsed.flags });
    setEditing(-1);
    setProblem(null);
    dialog.current?.showModal();
  }

  async function copyText(item: LibraryRecipe) {
    try {
      await navigator.clipboard.writeText(recipeText(item, model, language, effects));
      setStatus(r.copied);
    } catch {
      setStatus(null);
    }
  }

  async function copyTo(item: LibraryRecipe, to: Family) {
    const target = await loadModel(to, language);
    const copied = copyRecipe(item.recipe, model, to, target);
    const name = FAMILIES.find((entry) => entry.family === to)!.name;
    setStatus(
      onCopy(to, { ...item, recipe: copied.recipe, flags: [...(item.flags ?? []), ...copied.flags] })
        ? r.copiedTo(name)
        : r.full(LIBRARY_MAX),
    );
  }

  function move(index: number, by: number) {
    const next = [...library];
    const [item] = next.splice(index, 1);
    next.splice(index + by, 0, item!);
    onChange(next);
  }

  return (
    <div className="section-body recipes">
      <p className="note">{r.note}</p>
      {library.length === 0 ? (
        <p className="status">{r.empty}</p>
      ) : (
        <ol className="recipe-list">
          {library.map((item, index) => (
            <li key={`${index}-${item.recipe.name}`} className="recipe-row">
              <span className="recipe-name">{item.recipe.name}</span>
              <span className="recipe-look">
                {lookText(item.recipe, model, effects, r)}
                {item.flags && item.flags.length > 0 && (
                  <span className="recipe-flags">
                    {' · '}
                    {r.check([...new Set(item.flags.map((flag) => fieldName(r, flag.field)))].join(t.listSeparator))}
                  </span>
                )}
              </span>
              <span className="recipe-actions">
                <button type="button" className="button small" onClick={() => open(index)}>
                  {r.edit}
                </button>
                <button
                  type="button"
                  className="button small"
                  aria-label={`${r.up}: ${item.recipe.name}`}
                  disabled={index === 0}
                  onClick={() => move(index, -1)}
                >
                  ↑
                </button>
                <button
                  type="button"
                  className="button small"
                  aria-label={`${r.down}: ${item.recipe.name}`}
                  disabled={index === library.length - 1}
                  onClick={() => move(index, 1)}
                >
                  ↓
                </button>
                <button type="button" className="button small" onClick={() => void copyText(item)}>
                  {r.copyText}
                </button>
                <select
                  className="button small"
                  aria-label={`${r.copyTo}: ${item.recipe.name}`}
                  value=""
                  onChange={(event) => void copyTo(item, event.target.value as Family)}
                >
                  <option value="">{r.copyTo}</option>
                  {FAMILIES.filter((entry) => entry.family !== family).map((entry) => (
                    <option key={entry.family} value={entry.family}>
                      {entry.name}
                    </option>
                  ))}
                </select>
                <button
                  type="button"
                  className="button small"
                  onClick={() => onChange(library.filter((_, other) => other !== index))}
                >
                  {r.remove}
                </button>
              </span>
            </li>
          ))}
        </ol>
      )}
      {warnings.grain.length > 0 && (
        <p className="note warning">{r.warnGrain(warnings.grain.join(t.listSeparator))}</p>
      )}
      {warnings.missing.length > 0 && (
        <p className="note warning">{r.warnMissing(warnings.missing.join(t.listSeparator))}</p>
      )}
      <div className="recipe-tools">
        <button type="button" className="button" disabled={library.length >= LIBRARY_MAX} onClick={() => open(-1)}>
          {r.add}
        </button>
        <button
          type="button"
          className="button"
          disabled={library.length >= LIBRARY_MAX}
          onClick={() => importDialog.current?.showModal()}
        >
          {r.importText}
        </button>
      </div>
      <div aria-live="polite">
        {library.length >= LIBRARY_MAX && <p className="status">{r.full(LIBRARY_MAX)}</p>}
        {status && <p className="status">{status}</p>}
      </div>
      <dialog ref={importDialog} className="dialog" aria-labelledby="recipe-import-title">
        <div className="dialog-head">
          <h2 id="recipe-import-title" className="dialog-title">
            {r.importTitle}
          </h2>
        </div>
        <div className="dialog-body">
          <p className="note">{r.importHint}</p>
          <textarea
            className="import-text recipe-paste"
            rows={12}
            value={pasted}
            aria-label={r.importTitle}
            onChange={(event) => setPasted(event.target.value)}
          />
          <div className="dialog-actions">
            <button type="button" className="button" onClick={() => importDialog.current?.close()}>
              {r.cancel}
            </button>
            <button type="button" className="button primary" disabled={!pasted.trim()} onClick={readText}>
              {r.read}
            </button>
          </div>
        </div>
      </dialog>
      <dialog ref={dialog} className="dialog" aria-labelledby="recipe-title" onClose={() => setDraft(null)}>
        {draft && (
          <RecipeEditor
            r={r}
            model={model}
            effects={effects}
            draft={draft}
            onDraft={(next) => {
              setDraft(next);
              setProblem(null);
            }}
            problem={problem}
            saving={saving}
            onSave={save}
            onCancel={close}
          />
        )}
      </dialog>
    </div>
  );
}

function RecipeEditor({
  r,
  model,
  effects,
  draft,
  onDraft,
  problem,
  saving,
  onSave,
  onCancel,
}: {
  r: RecipeStrings;
  model: Model;
  effects: Props['effects'];
  draft: LibraryRecipe;
  onDraft: (draft: LibraryRecipe) => void;
  problem: string | null;
  saving: boolean;
  onSave: () => void;
  onCancel: () => void;
}) {
  const recipe = draft.recipe;
  const set = (change: Partial<Recipe>) => onDraft({ ...draft, recipe: { ...recipe, ...change } });
  const look = recipe.look;
  const type = look.kind === 'native' ? model.types.find((item) => item.index === look.imageType) : undefined;
  const setLook = (change: Partial<Look>) => set({ look: { ...look, ...change } as Look });
  const effectChoices = effects.map((effect) => [String(effectCode(effect.id)), effect.name] as const);
  const levelOptions = Array.from({ length: 9 }, (_, value) => [String(value), level(value)] as const);

  const nativeFields = (current: ModelType & {}): ReactNode => {
    if (look.kind !== 'native') return null;
    return (
      <>
        {LEVEL_KEYS.map((key, index) =>
          current.parameters.some((parameter) => parameter.key === key) ? (
            <Select
              key={key}
              label={r.parameters[key] ?? key}
              value={String(look.levels[index])}
              options={levelOptions}
              onChange={(value) => {
                const levels = [...look.levels];
                levels[index] = Number(value);
                setLook({ levels });
              }}
            />
          ) : null,
        )}
        {current.parameters
          .filter((parameter) => !LEVEL_KEYS.includes(parameter.key))
          .map((parameter) => {
            const field = parameter.key.startsWith('ColorTone')
              ? 'tone'
              : parameter.key === 'HdrIntense'
                ? 'intensity'
                : parameter.key === 'FilterEffectCustom'
                  ? 'filter'
                  : 'solarization';
            // Retro toning is a level.
            const options =
              parameter.key === 'ColorToneRetro'
                ? levelOptions
                : parameter.choices.map((choice) => [String(choice.value), valueName(r, choice.label)] as const);
            return (
              <Select
                key={parameter.key}
                label={r.parameters[parameter.key] ?? parameter.key}
                value={String(look[field])}
                options={options}
                onChange={(value) => setLook({ [field]: Number(value) })}
              />
            );
          })}
        {current.grain && (
          <Select
            label={r.nativeGrain}
            value={String(look.grainStrength)}
            options={r.nativeGrainStrengths.map((text, value) => [String(value), text] as const)}
            onChange={(value) => setLook({ grainStrength: Number(value), grainSize: Number(value) ? look.grainSize : 0 })}
          />
        )}
        {current.grain && current.grainSizes && look.grainStrength > 0 && (
          <Select
            label={`${r.nativeGrain} · ${r.size}`}
            value={String(look.grainSize)}
            options={r.nativeGrainSizes.map((text, value) => [String(value), text] as const)}
            onChange={(value) => setLook({ grainSize: Number(value) })}
          />
        )}
      </>
    );
  };

  const balance = recipe.whiteBalance;
  const mode = balance ? model.modes.find((item) => item.index === balance.mode) : undefined;

  return (
    <div className="recipe-editor">
      <div className="dialog-head">
        <h2 id="recipe-title" className="dialog-title">
          {r.editTitle}
        </h2>
      </div>
      <div className="dialog-body">
        {draft.flags && draft.flags.length > 0 && (
          <div className="note warning" role="status">
            <p>{r.flagged}</p>
            <ul>
              {draft.flags.map((flag, index) => (
                <li key={index}>
                  {fieldName(r, flag.field)}
                  {flag.text ? `: ${flag.text}` : ''}
                </li>
              ))}
            </ul>
          </div>
        )}
        <div className="import-fields">
          <label className="field">
            <span className="field-label">{r.name}</span>
            <input
              className="import-text"
              value={recipe.name}
              maxLength={NAME_MAX}
              aria-invalid={!nameFits(recipe.name)}
              onChange={(event) => set({ name: event.target.value })}
            />
            <span className="note">{r.nameHint}</span>
          </label>
          <label className="field">
            <span className="field-label">{r.author}</span>
            <input
              className="import-text"
              value={draft.author}
              onChange={(event) => onDraft({ ...draft, author: event.target.value })}
            />
          </label>
          <label className="field">
            <span className="field-label">{r.source}</span>
            <input
              className="import-text"
              type="url"
              value={draft.source}
              onChange={(event) => onDraft({ ...draft, source: event.target.value })}
            />
          </label>
        </div>

        <fieldset className="recipe-group">
          <legend>{r.look}</legend>
          <div className="import-fields">
            <Select
              label={r.look}
              value={look.kind === 'effect' ? 'effect' : 'native'}
              options={[
                ['native', r.lookNative],
                ...(effects.length > 0 ? ([['effect', r.lookEffect]] as const) : []),
              ]}
              onChange={(value) =>
                set({
                  look:
                    value === 'effect'
                      ? { kind: 'effect', code: effectCode(effects[0]!.id), adjustments: Array(9).fill(NEUTRAL) }
                      : nativeLook(model.types[0]!),
                })
              }
            />
            {look.kind === 'effect' && (
              <Select
                label={r.effect}
                value={String(look.code)}
                options={
                  effectChoices.some(([code]) => code === String(look.code))
                    ? effectChoices
                    : [[String(look.code), r.lookEffect], ...effectChoices]
                }
                onChange={(value) => setLook({ code: Number(value) })}
              />
            )}
            {look.kind === 'effect' &&
              LEVEL_KEYS.map((key, index) => (
                <Select
                  key={key}
                  label={r.parameters[key] ?? key}
                  value={String(look.adjustments[index])}
                  options={levelOptions}
                  onChange={(value) => {
                    const adjustments = [...look.adjustments];
                    adjustments[index] = Number(value);
                    setLook({ adjustments });
                  }}
                />
              ))}
            {look.kind === 'native' && (
              <Select
                label={r.type}
                value={String(look.imageType)}
                options={model.types.map((item) => [String(item.index), item.name] as const)}
                onChange={(value) => set({ look: nativeLook(model.types.find((item) => item.index === Number(value))!) })}
              />
            )}
            {type && nativeFields(type)}
          </div>
          {effects.length === 0 && <p className="note">{r.noEffect}</p>}
        </fieldset>

        <fieldset className="recipe-group">
          <legend>{r.grain}</legend>
          <div className="import-fields">
            <Select
              label={r.grain}
              value={recipe.grain ? String(recipe.grain.level) : 'none'}
              options={[['none', r.grainNone], ...r.grainLevels.map((text, value) => [String(value), text] as const)]}
              onChange={(value) =>
                set({ grain: value === 'none' ? null : { level: Number(value), size: recipe.grain?.size ?? 0 } })
              }
            />
            {recipe.grain && (
              <Select
                label={`${r.grain} · ${r.size}`}
                value={String(recipe.grain.size)}
                options={r.grainSizes.map((text, value) => [String(value), text] as const)}
                onChange={(value) => set({ grain: { ...recipe.grain!, size: Number(value) } })}
              />
            )}
          </div>
        </fieldset>

        {model.modes.length > 0 && (
          <fieldset className="recipe-group">
            <legend>{r.whiteBalance}</legend>
            <div className="import-fields">
              <Select
                label={r.whiteBalance}
                value={balance ? String(balance.mode) : 'none'}
                options={[
                  ['none', r.keep],
                  ...model.modes.map((item) => [String(item.index), r.modes[item.label] ?? item.label] as const),
                ]}
                onChange={(value) =>
                  set({
                    whiteBalance:
                      value === 'none'
                        ? null
                        : {
                            mode: Number(value),
                            shiftBa: balance?.shiftBa ?? NO_SHIFT,
                            shiftGm: balance?.shiftGm ?? NO_SHIFT,
                            kelvin: balance?.kelvin ?? 5500,
                          },
                  })
                }
              />
              {balance && mode?.shifts && (
                <>
                  <Select
                    label={r.shiftBa}
                    value={String(balance.shiftBa)}
                    options={Array.from({ length: 29 }, (_, value) => [String(value), shift(value, 'B', 'A')] as const)}
                    onChange={(value) => set({ whiteBalance: { ...balance, shiftBa: Number(value) } })}
                  />
                  <Select
                    label={r.shiftGm}
                    value={String(balance.shiftGm)}
                    options={Array.from({ length: 29 }, (_, value) => [String(value), shift(value, 'G', 'M')] as const)}
                    onChange={(value) => set({ whiteBalance: { ...balance, shiftGm: Number(value) } })}
                  />
                </>
              )}
              {balance && mode?.temperature && (
                <label className="field">
                  <span className="field-label">{r.kelvin}</span>
                  <input
                    className="import-text"
                    type="number"
                    min={2500}
                    max={10000}
                    step={10}
                    value={balance.kelvin}
                    onChange={(event) => set({ whiteBalance: { ...balance, kelvin: Number(event.target.value) } })}
                  />
                </label>
              )}
            </div>
          </fieldset>
        )}

        <fieldset className="recipe-group">
          <legend>{r.corrections.join(' · ')}</legend>
          <div className="import-fields">
            {model.corrections.map((correction, index) => (
              <Select
                key={correction.key}
                label={r.corrections[index]!}
                value={recipe.corrections[index] === null ? 'none' : String(recipe.corrections[index])}
                options={[
                  ['none', r.unset],
                  ...correction.choices.map((choice) => [String(choice.value), valueName(r, choice.label)] as const),
                ]}
                onChange={(value) => {
                  const corrections = [...recipe.corrections];
                  corrections[index] = value === 'none' ? null : Number(value);
                  set({ corrections });
                }}
              />
            ))}
          </div>
        </fieldset>

        <fieldset className="recipe-group">
          <legend>{r.hints}</legend>
          <p className="note">{r.hintsNote}</p>
          <div className="import-fields">
            <Select
              label={r.exposure}
              value={recipe.hints.exposure === null ? 'none' : String(recipe.hints.exposure)}
              options={[['none', r.none], ...EXPOSURES.map((thirds) => [String(thirds), exposureText(thirds)] as const)]}
              onChange={(value) => set({ hints: { ...recipe.hints, exposure: value === 'none' ? null : Number(value) } })}
            />
            <Select
              label={r.isoLimit}
              value={recipe.hints.isoLimit === null ? 'none' : String(recipe.hints.isoLimit)}
              options={[['none', r.none], ...ISO_LIMITS.map((iso) => [String(iso), String(iso)] as const)]}
              onChange={(value) => set({ hints: { ...recipe.hints, isoLimit: value === 'none' ? null : Number(value) } })}
            />
            <Select
              label={r.metering}
              value={recipe.hints.metering ?? 'none'}
              options={[
                ['none', r.none],
                ...(['multi', 'center', 'spot', 'highlight'] as const).map(
                  (key, index) => [key, r.meterings[index]!] as const,
                ),
              ]}
              onChange={(value) =>
                set({
                  hints: {
                    ...recipe.hints,
                    metering: value === 'none' ? null : (value as Recipe['hints']['metering']),
                  },
                })
              }
            />
            <Select
              label={r.nd}
              value={recipe.hints.nd ?? 'none'}
              options={[
                ['none', r.none],
                ...(['off', 'on', 'auto'] as const).map((key, index) => [key, r.nds[index]!] as const),
              ]}
              onChange={(value) =>
                set({ hints: { ...recipe.hints, nd: value === 'none' ? null : (value as Recipe['hints']['nd']) } })
              }
            />
          </div>
        </fieldset>

        {problem && (
          <p className="status error" role="alert">
            {r.problem(problem)}
          </p>
        )}
        <div className="dialog-actions">
          <button type="button" className="button" onClick={onCancel}>
            {r.cancel}
          </button>
          <button type="button" className="button primary" disabled={saving} onClick={onSave}>
            {r.save}
          </button>
        </div>
      </div>
    </div>
  );
}
