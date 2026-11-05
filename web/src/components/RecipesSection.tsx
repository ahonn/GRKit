import { useRef, useState } from 'react';
import { Handle, HandleRoot, Item, Provider, Root } from 'react-compare-slider/components';
import { useReactCompareSlider } from 'react-compare-slider/hooks';
import { siteUrl } from '../catalog';
import { CORRECTION_VALUES, LEVEL_KEYS, PRESETS, TYPE_NAMES, WHITE_BALANCE_MODES } from '../generated/recipes';
import type { Language, Strings } from '../i18n';
import { LANDING_PHOTOS } from '../landing-data';
import { RECIPE_STRINGS, type RecipeStrings } from '../recipeStrings';
import {
  automaticName,
  cameraOrder,
  LIBRARY_MAX,
  libraryWarnings,
  ON_MAX,
  onCount,
  presetOf,
  presetRows,
  type Family,
  type Preset,
  type Recipe,
} from '../recipes';

interface Props {
  strings: Strings;
  language: Language;
  family: Family;
  library: readonly Recipe[];
  onChange: (library: Recipe[]) => void;
  /** The recipes of the connected card while they differ from `library`, else null. */
  cardRecipes: readonly Recipe[] | null;
  /** The choice between the recipes of the card (true) and those of the site (false). */
  onCardChoice: (useCard: boolean) => void;
  /** The names of the effects of the catalog and of the imports, by their recipe code. */
  effectNames: ReadonlyMap<number, string>;
  /** The recipe codes of the effects of the package. */
  packageEffects: ReadonlySet<number>;
  /** The package has the Grain module. */
  grainModule: boolean;
  /** The package has the Glow module. */
  glowModule: boolean;
  /** Adds the recipe of a preset to the library, and its effect to the package. */
  onAddPreset: (recipe: Recipe, effect: string | null) => void;
}

/**
 * The pictures of each preset: the camera developed a RAW photo with the recipe and without it
 * (scripts/build-landing.mjs). Presets that share a photo share its picture without a recipe.
 */
const PRESET_PICTURES: Readonly<Record<string, { recipe: string; original: string; photo: keyof typeof LANDING_PHOTOS }>> = {
  'cinestill-800t': { recipe: siteUrl('recipes/cinestill-800t.jpg'), original: siteUrl('recipes/takeaway.jpg'), photo: 'takeaway' },
  'bennet-800': { recipe: siteUrl('recipes/bennet-800.jpg'), original: siteUrl('recipes/dunes.jpg'), photo: 'dunes' },
  'reggie-negative': { recipe: siteUrl('recipes/reggie-negative.jpg'), original: siteUrl('recipes/dunes.jpg'), photo: 'dunes' },
};

/** The count of recipes that are on shows from this many below `ON_MAX`. */
const COUNT_NEAR = 5;

/** The name of the look of a recipe: its effect, its native type, Film Look Off, or the camera's. */
function lookName(recipe: Recipe, r: RecipeStrings, language: Language, effects: Props['effectNames']): string {
  const look = recipe.look;
  if (look.kind === 'effect') return effects.get(look.code) ?? r.lookEffect;
  if (look.kind === 'native') return TYPE_NAMES[language]?.[look.imageType] ?? r.lookNative;
  return look.kind === 'off' ? r.lookOff : r.lookKept;
}

/** A level as the camera shows it: stored 0 to 8 is -4 to +4. */
const level = (stored: number) => (stored === 4 ? '0' : stored > 4 ? `+${stored - 4}` : `${stored - 4}`);

/** A white balance shift as the camera shows it, such as A14 or G2, or nothing for none. */
const shift = (stored: number, minus: string, plus: string) =>
  stored === 14 ? '' : stored < 14 ? `${minus}${14 - stored}` : `${plus}${stored - 14}`;

/** An exposure compensation in thirds of a stop, such as +1 1/3 EV. */
function compensation(thirds: number): string {
  const sign = thirds > 0 ? '+' : thirds < 0 ? '-' : '±';
  const whole = Math.floor(Math.abs(thirds) / 3);
  const rest = Math.abs(thirds) % 3;
  const value = [whole || (rest ? '' : '0'), rest ? `${rest}/3` : ''].filter(Boolean).join(' ');
  return `${sign}${value} EV`;
}

const ND_INDEX = { off: 0, on: 1, auto: 2 } as const;
const METERING_INDEX = { multi: 0, center: 1, spot: 2, highlight: 3 } as const;

/**
 * The settings that a recipe holds, as a name and a value each, in the order of the camera
 * menus: the look and the levels of a native look that are not 0, Grain, Glow, the white balance
 * with its shifts, each correction, and the exposure settings.
 */
function details(recipe: Recipe, r: RecipeStrings, language: Language, effects: Props['effectNames']): [string, string][] {
  const rows: [string, string][] = [[r.look, lookName(recipe, r, language, effects)]];
  const look = recipe.look;
  if (look.kind === 'native') {
    look.levels.forEach((stored, index) => {
      const key = LEVEL_KEYS[index];
      if (key && stored !== 4) rows.push([r.levels[key] ?? key, level(stored)]);
    });
  }
  if (recipe.grain) rows.push([r.grain, r.grainLevels[recipe.grain.level] ?? '']);
  const glow = recipe.glow;
  if (glow?.bloom) rows.push([r.bloom, r.glowLevels[glow.bloom] ?? '']);
  if (glow?.halation) {
    const color = glow.halationColor === null ? '' : ` ${r.halationColors[glow.halationColor] ?? ''}`;
    rows.push([r.halation, `${r.glowLevels[glow.halation] ?? ''}${color}`]);
  }
  const balance = recipe.whiteBalance;
  if (balance) {
    const label = WHITE_BALANCE_MODES[balance.mode]?.[recipe.family === 'gr3' ? 0 : 1] ?? '';
    const shifts = [shift(balance.shiftBa, 'B', 'A'), shift(balance.shiftGm, 'G', 'M')].filter(Boolean).join(' ');
    const kelvin = label === 'ColorTemp1' ? `${balance.kelvin} K` : '';
    rows.push([r.whiteBalance, [r.modes[label] ?? label, kelvin, shifts].filter(Boolean).join(' · ')]);
  }
  recipe.corrections.forEach((stored, index) => {
    if (stored === null) return;
    const value = CORRECTION_VALUES[index]?.[stored] ?? '';
    rows.push([r.correctionNames[index] ?? '', r.values[value] ?? value]);
  });
  const exposure = recipe.exposure;
  if (exposure.compensation !== null) rows.push([r.compensation, compensation(exposure.compensation)]);
  if (exposure.isoLimit !== null) rows.push([r.isoLimit, `ISO ${exposure.isoLimit}`]);
  if (exposure.metering !== null) rows.push([r.metering, r.meterings[METERING_INDEX[exposure.metering]] ?? '']);
  if (exposure.nd !== null) rows.push([r.nd, r.nds[ND_INDEX[exposure.nd]] ?? '']);
  return rows;
}

/** The settings of a recipe in one line: the look, Grain, and Glow, then the names of the others. */
function summary(recipe: Recipe, r: RecipeStrings, language: Language, effects: Props['effectNames']): string {
  const parts = [lookName(recipe, r, language, effects)];
  if (recipe.grain) parts.push(`${r.grain} ${r.grainLevels[recipe.grain.level] ?? ''}`);
  const glow = recipe.glow;
  if (glow?.bloom) parts.push(`${r.bloom} ${r.glowLevels[glow.bloom] ?? ''}`);
  if (glow?.halation) parts.push(`${r.halation} ${r.glowLevels[glow.halation] ?? ''}`);
  if (recipe.whiteBalance) parts.push(r.whiteBalance);
  if (recipe.corrections.some((value) => value !== null)) parts.push(r.corrections);
  if (Object.values(recipe.exposure).some((value) => value !== null)) parts.push(r.exposure);
  return parts.join(' · ');
}

/**
 * The recipes of the selected camera line, in the order of the camera, then the presets that the
 * library does not hold, off. The camera makes and edits recipes; the site turns them on and off
 * and deletes them, but not a preset, and a preset joins the library when the person turns it on.
 * As with the film looks, at most `ON_MAX` recipes are on: the camera lists them. The others stay
 * on the card for the site. The name of a recipe opens its settings, with the pictures of a
 * preset.
 */
export function RecipesSection({
  strings: t,
  language,
  family,
  library,
  onChange,
  cardRecipes,
  onCardChoice,
  effectNames,
  packageEffects,
  grainModule,
  glowModule,
  onAddPreset,
}: Props) {
  const r = RECIPE_STRINGS[language];
  const on = onCount(library);
  const presets = presetRows(PRESETS, family, library);
  const [shown, setShown] = useState<Recipe | null>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  function show(recipe: Recipe) {
    setShown(recipe);
    dialog.current?.showModal();
  }
  // Only the recipes that are on apply on the camera.
  const warnings = libraryWarnings(
    library.filter((recipe) => recipe.enabled),
    { grain: grainModule, glow: glowModule },
    packageEffects,
  );
  const nameOf = (recipe: Recipe) => recipe.name || automaticName(lookName(recipe, r, language, effectNames), recipe.number);

  /** Turns a recipe on or off. It moves to the end of the recipes that are on, or to the start of the others. */
  function toggle(index: number) {
    onChange(cameraOrder(library.map((recipe, other) => (other === index ? { ...recipe, enabled: !recipe.enabled } : recipe))));
  }

  return (
    <div className="section-body recipes">
      {cardRecipes && (
        <div className="note-group" aria-live="polite">
          <p className="note warning">{r.cardDiffers}</p>
          <div className="recipe-tools">
            <button type="button" className="button" onClick={() => onCardChoice(true)}>
              {r.useCard}
            </button>
            <button type="button" className="button" onClick={() => onCardChoice(false)}>
              {r.keepSite}
            </button>
          </div>
        </div>
      )}
      {on >= ON_MAX - COUNT_NEAR && (
        <p>
          {r.onCount(ON_MAX)[0]}
          <span className="count">{on}</span>
          {r.onCount(ON_MAX)[1]}
        </p>
      )}
      {library.length + presets.length === 0 ? (
        <p className="status">{r.empty}</p>
      ) : (
        <ol className="recipe-list">
          {library.map((recipe, index) => (
            <li key={`${index}-${recipe.name}`} className="recipe-row" data-off={!recipe.enabled || undefined}>
              <input
                className="row-check recipe-check"
                type="checkbox"
                checked={recipe.enabled}
                disabled={!recipe.enabled && on >= ON_MAX}
                aria-label={nameOf(recipe)}
                onChange={() => toggle(index)}
              />
              <button type="button" className="recipe-name link-button" onClick={() => show(recipe)}>
                {nameOf(recipe)}
              </button>
              <span className="recipe-look">{summary(recipe, r, language, effectNames)}</span>
              <span className="recipe-actions">
                {/* A preset of the site stays: turning it off keeps it out of the camera. */}
                {!presetOf(PRESETS, recipe) && (
                  <button
                    type="button"
                    className="button small"
                    onClick={() => onChange(library.filter((_, other) => other !== index))}
                  >
                    {r.remove}
                  </button>
                )}
              </span>
            </li>
          ))}
          {presets.map(({ preset, recipe }) => (
            <li key={preset.id} className="recipe-row" data-off>
              <input
                className="row-check recipe-check"
                type="checkbox"
                checked={false}
                disabled={on >= ON_MAX || library.length >= LIBRARY_MAX}
                aria-label={recipe.name}
                onChange={() => onAddPreset(recipe, preset.effect)}
              />
              <button type="button" className="recipe-name link-button" onClick={() => show(recipe)}>
                {recipe.name}
              </button>
              <span className="recipe-look">
                {r.presetTag} · {summary(recipe, r, language, effectNames)}
              </span>
            </li>
          ))}
        </ol>
      )}
      {warnings.grain.length > 0 && <p className="note warning">{r.warnGrain(warnings.grain.join(t.listSeparator))}</p>}
      {warnings.glow.length > 0 && <p className="note warning">{r.warnGlow(warnings.glow.join(t.listSeparator))}</p>}
      {warnings.missing.length > 0 && (
        <p className="note warning">{r.warnMissing(warnings.missing.join(t.listSeparator))}</p>
      )}
      <div aria-live="polite">{library.length >= LIBRARY_MAX && <p className="status">{r.full(LIBRARY_MAX)}</p>}</div>
      <dialog
        ref={dialog}
        className="dialog"
        aria-labelledby="recipe-title"
        onClose={() => setShown(null)}
      >
        {shown && (
          <RecipeDetails
            t={t}
            r={r}
            language={language}
            recipe={shown}
            name={nameOf(shown)}
            preset={presetOf(PRESETS, shown)}
            effectNames={effectNames}
            onClose={() => dialog.current?.close()}
          />
        )}
      </dialog>
    </div>
  );
}

/** The settings of a recipe, and for a preset its pictures with the recipe and without it. */
function RecipeDetails({
  t,
  r,
  language,
  recipe,
  name,
  preset,
  effectNames,
  onClose,
}: {
  t: Strings;
  r: RecipeStrings;
  language: Language;
  recipe: Recipe;
  name: string;
  preset: Preset | undefined;
  effectNames: Props['effectNames'];
  onClose: () => void;
}) {
  const comparison = useReactCompareSlider({ clip: 'itemTwo' });
  const picture = preset ? PRESET_PICTURES[preset.id] : undefined;
  const photo = picture ? LANDING_PHOTOS[picture.photo] : undefined;
  return (
    <>
      <div className="dialog-head">
        <h3 id="recipe-title" className="dialog-title">
          {name}
        </h3>
        <button type="button" className="button small" onClick={onClose}>
          {t.close}
        </button>
      </div>
      <div className="dialog-body">
        {picture && photo && (
          <figure className="preset-picture">
            <div className="frame preset-frame">
              <Provider {...comparison}>
                <Root className="image-comparison">
                  <Item item="itemOne">
                    <img src={picture.original} alt={`${name} — ${r.presetBefore}`} />
                  </Item>
                  <Item item="itemTwo" className="comparison-effect">
                    <img src={picture.recipe} alt={`${name} — ${r.presetAfter}`} />
                  </Item>
                  <HandleRoot aria-label={t.compare}>
                    <Handle buttonStyle={{ width: 32, height: 32, gap: '0.2rem' }} />
                  </HandleRoot>
                </Root>
              </Provider>
              <span className="tag">{name}</span>
            </div>
            <figcaption className="preset-caption">
              <a className="note" href={photo.source} target="_blank" rel="noreferrer">
                {r.photo(photo.author, photo.license)}
              </a>
            </figcaption>
          </figure>
        )}
        {preset && <p className="note">{r.presetText[preset.id]}</p>}
        {preset?.author && preset.source && (
          <p className="note">
            {r.by(preset.author)} ·{' '}
            <a href={preset.source} target="_blank" rel="noreferrer">
              {r.original}
            </a>
          </p>
        )}
        <dl className="recipe-details">
          {details(recipe, r, language, effectNames).map(([label, value]) => (
            <div key={label}>
              <dt>{label}</dt>
              <dd>{value}</dd>
            </div>
          ))}
        </dl>
      </div>
    </>
  );
}
