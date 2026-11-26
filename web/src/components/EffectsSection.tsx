import {
  Suspense,
  use,
  useCallback,
  useId,
  useRef,
  useState,
  useSyncExternalStore,
  useTransition,
  type ChangeEvent,
} from 'react';
import { Handle, HandleRoot, Item, Provider, Root } from 'react-compare-slider/components';
import { useReactCompareSlider } from 'react-compare-slider/hooks';
import { PREVIEW_TARGET } from '../cameras';
import {
  effectUrl,
  loadCatalog,
  MAX_EFFECTS,
  previewTable,
  type CardEffect,
  type CatalogEffect,
  type Effect,
  type ImportedEffect,
} from '../catalog';
import { nativeText, type Language, type Strings } from '../i18n';
import { forget, prefetchTable, preview, type PreviewEffect, type Source } from '../preview/render';
import { DEFAULT_SAMPLE } from '../preview/samples';
import { PhotoLoader } from '../preview/raw';
import { editable } from '../import/edit';
import { missingNativeLooks, NATIVE_LOOKS, type NativeFirmware } from '../import/native';
import { importFit, resolveSelection, selectImport, type Selection } from '../selection';
import type { Family, Recipe } from '../recipes';
import { STOCKS } from '../stocks';
import { type EffectFilter } from '../effectGroups';
import { withCustomNames, withoutCustomCovers } from '../customCovers';
import {
  COVER_SCHEMES,
  individualCover,
  isFilmCover,
  isMonoLook,
  nativeCover,
  nativeLook,
  schemeCover,
  shownCover,
  effectiveCover,
  type CoverScheme,
  type Customization,
  type Customizations,
} from '../packageCovers';
import { CoverIcon, EffectIcon } from './EffectIcon';
import { ErrorBoundary } from './ErrorBoundary';
import { EditDialog, type EditDialogHandle } from './EditDialog';
import { ImportDialog, type ImportDialogHandle } from './ImportDialog';
import { NativeLookDialog, type NativeLookDialogHandle } from './NativeLookDialog';

const DEFAULT_PREVIEW = 'portra-400-endura-v1';

/**
 * Shelf icon: the cover that the camera shows after a write (`shownCover`), drawn as an SVG. A
 * site film look and a native look have their individual cover, else that of `scheme`; a native
 * look that only the card has takes its known cover. An effect without cover fields shows the icon
 * pixels of its file, which a write keeps.
 */
function shelfIcon(
  effect: Effect | CardEffect,
  customs: Customizations,
  scheme: CoverScheme,
): Pick<Effect, 'icon' | 'cover'> {
  const native = NATIVE_LOOKS.find((look) => look.id === effect.id);
  const site = !effect.imported && !effect.card;
  const own = effect.cover ?? native?.cover;
  const source = effect.imported?.kind === 'lut' || effect.imported?.kind === 'restored' ? 'own' : 'site';
  const custom = native
    ? nativeCover(effect.id, effect.imported ? effect.cover : undefined, scheme)
    : site
      ? effectiveCover(effect, customs, scheme)
      : undefined;
  return { icon: effect.icon, cover: shownCover(own, source, custom) };
}

type ListedEffect = Effect | CardEffect;

type NativeLook = (typeof NATIVE_LOOKS)[number];

type ListEntry =
  | { kind: 'header'; key: string; label: string; group?: CoverGroup }
  | { kind: 'effect'; effect: ListedEffect }
  | { kind: 'native'; look: NativeLook }
  | { kind: 'import'; key: 'import-color' | 'import-mono' };

/** A group of the built-in looks whose covers reset together. */
type CoverGroup = 'color' | 'mono';

type ShelfGroup = 'color' | 'mono' | 'imported' | 'card';

/** The film number, then the print film of a cine look: "5219 · 2393 print". */
function stockText(stock: (typeof STOCKS)[string] & {}, t: Strings): string {
  return stock.print ? `${stock.film} · ${t.printFilm(stock.print)}` : stock.film;
}

function groupLabel(group: ShelfGroup, t: Strings): string {
  switch (group) {
    case 'color':
      return t.filterColor;
    case 'mono':
      return t.groupMono;
    case 'imported':
      return t.groupImported;
    case 'card':
      return t.groupCard;
  }
}

function matchesFilter(effect: ListedEffect, filter: EffectFilter): boolean {
  if (filter === 'all') return true;
  if (filter === 'yours') return Boolean(effect.imported || effect.card);
  if (filter === 'mono') return effect.monochrome;
  return !effect.monochrome && !effect.card;
}

function buildList(
  catalogEffects: readonly Effect[],
  imported: readonly ImportedEffect[],
  cardEffects: readonly CardEffect[],
  filter: EffectFilter,
  t: Strings,
  locked: readonly NativeLook[],
  canImport: boolean,
  resettable: ReadonlySet<CoverGroup>,
): ListEntry[] {
  const catalog = catalogEffects.filter((effect) => matchesFilter(effect, filter));
  const imports = imported.filter((effect) => matchesFilter(effect, filter));
  const cards = cardEffects.filter((effect) => matchesFilter(effect, filter));
  const entries: ListEntry[] = [];

  const pushGroup = (key: ShelfGroup, items: readonly ListedEffect[], headed: boolean) => {
    if (items.length === 0) return;
    if (headed) entries.push({ kind: 'header', key: `h-${key}`, label: groupLabel(key, t) });
    for (const effect of items) entries.push({ kind: 'effect', effect });
  };

  if (filter === 'yours') {
    pushGroup('imported', imports, true);
    pushGroup('card', cards, true);
    return entries;
  }

  // All: Color then B&W. Color / B&W filters: one flat shelf, with a header only to hold the reset
  // of the covers of its group.
  // Card-only effects join the shelf so a Cinema look on the SD card is not also an unlock tile.
  // GR IV unlock tiles sit at the end of their group; Import LUT tiles close Color and B&W.
  const showHeader = (group: CoverGroup) => filter === 'all' || resettable.has(group);
  const pool = [...catalog, ...imports, ...cards];
  const color = pool.filter((effect) => !effect.monochrome);
  const mono = pool.filter((effect) => effect.monochrome);
  const lockedColor = filter === 'mono' ? [] : locked.filter((look) => !look.monochrome);
  const lockedMono = filter === 'color' ? [] : locked.filter((look) => look.monochrome);
  const importColor = canImport && filter !== 'mono';
  const importMono = canImport && filter !== 'color';

  if (color.length > 0 || lockedColor.length > 0 || importColor) {
    if (showHeader('color')) {
      entries.push({ kind: 'header', key: 'h-color', label: groupLabel('color', t), group: 'color' });
    }
    for (const effect of color) entries.push({ kind: 'effect', effect });
    for (const look of lockedColor) entries.push({ kind: 'native', look });
    if (importColor) entries.push({ kind: 'import', key: 'import-color' });
  }

  if (mono.length > 0 || lockedMono.length > 0 || importMono) {
    if (showHeader('mono')) {
      entries.push({ kind: 'header', key: 'h-mono', label: groupLabel('mono', t), group: 'mono' });
    }
    for (const effect of mono) entries.push({ kind: 'effect', effect });
    for (const look of lockedMono) entries.push({ kind: 'native', look });
    if (importMono) entries.push({ kind: 'import', key: 'import-mono' });
  }

  return entries;
}

function subscribeFullscreen(change: () => void): () => void {
  document.addEventListener('fullscreenchange', change);
  return () => document.removeEventListener('fullscreenchange', change);
}

/** The browser can show an element in full screen. iPhone Safari cannot. */
function fullscreenEnabled(): boolean {
  return document.fullscreenEnabled === true;
}

const neverChanges = () => () => undefined;

/** Wide layout that may show the floating preview when the main one scrolls away. */
const DESKTOP_PREVIEW = '(min-width: 1024px)';

function subscribeDesktop(change: () => void): () => void {
  const media = matchMedia(DESKTOP_PREVIEW);
  media.addEventListener('change', change);
  return () => media.removeEventListener('change', change);
}

function desktopPreview(): boolean {
  return matchMedia(DESKTOP_PREVIEW).matches;
}

/**
 * Calls `change` when `element`'s visibility against the viewport may have changed.
 * `thresholds` defaults to any intersection; pass e.g. `[0, 0.5, 1]` for half-visible checks.
 */
function subscribeInView(
  element: HTMLElement | null,
  change: () => void,
  thresholds: number | number[] = 0,
): () => void {
  if (!element) return () => undefined;
  const observer = new IntersectionObserver(() => change(), { threshold: thresholds });
  observer.observe(element);
  return () => observer.disconnect();
}

/** True while any part of `element` intersects the viewport. */
function elementInView(element: HTMLElement | null, whenMissing: boolean): boolean {
  if (!element) return whenMissing;
  const rect = element.getBoundingClientRect();
  return rect.bottom > 0 && rect.top < innerHeight && rect.right > 0 && rect.left < innerWidth;
}

/**
 * Visible area of `element` inside the viewport, as a fraction of its size (0–1).
 * Height is what matters for the preview float: sideways clipping is rare.
 */
function elementVisibleRatio(element: HTMLElement | null): number {
  if (!element) return 1;
  const rect = element.getBoundingClientRect();
  const visible = Math.min(rect.bottom, innerHeight) - Math.max(rect.top, 0);
  if (rect.height <= 0 || visible <= 0) return 0;
  return Math.min(1, visible / rect.height);
}

/** The Film Looks `<section>` that wraps this body (labelled by `#effects-label`). */
function effectsSection(): HTMLElement | null {
  return document.getElementById('effects-label')?.closest('section') ?? null;
}

interface Props {
  strings: Strings;
  language: Language;
  selection: Selection;
  onSelectionChange: (selection: ReadonlySet<string>) => void;
  /** The effects that this page imported for the selected camera, after the catalog effects. */
  imported: readonly ImportedEffect[];
  /** Adds imports, each in place of an earlier one with the same ID. */
  onImport: (effects: readonly ImportedEffect[]) => void;
  /** The native looks that the selected camera can use, which the list offers to unlock. */
  nativeLooks: readonly NativeLook[];
  onRemove: (effect: ImportedEffect) => void;
  /** Keeps an import with a new name or icon in its place. */
  onEdit: (effect: ImportedEffect) => void;
  /** The effect files on the connected card that the catalog and the imports do not have. */
  cardEffects: readonly CardEffect[];
  /** The IDs of the effects on the connected card, or null without a card. */
  onCard: ReadonlySet<string> | null;
  /** The IDs of the film looks whose file on the card differs, such as after a new name. */
  changedOnCard?: ReadonlySet<string>;
  /**
   * The names and covers that the user gave site film looks. Without one, a site film look has
   * its site name and the cover of `scheme`.
   */
  customizations: Customizations;
  /** The cover scheme of site film looks without a custom cover. */
  scheme: CoverScheme;
  /** Keeps a new cover scheme. */
  onScheme: (scheme: CoverScheme) => void;
  /** Keeps the customization of a site film look; an empty one removes it. */
  onCustomize: (id: string, custom: Customization) => void;
  /** Keeps new customizations for several site film looks at once. */
  onCustomizations: (customs: Customizations) => void;
  /** The camera line of the page, for a recipe from a preset import. */
  family: Family;
  /** The recipe library of that camera line. */
  recipes: readonly Recipe[];
  /** Adds a recipe from a preset import. */
  onAddRecipe: (recipe: Recipe) => void;
}

/** The change badge of an effect or a module: what a write to the card does with it. */

export function ChangeBadge({
  on,
  checked,
  changed = false,
  strings: t,
}: {
  on: boolean;
  checked: boolean;
  /** The file on the card differs from the effect of the page. */
  changed?: boolean;
  strings: Strings;
}) {
  if (checked && !on) return <span className="change add">{t.changeAdds}</span>;
  if (!checked && on) return <span className="change remove">{t.changeRemoves}</span>;
  if (checked && changed) return <span className="change add">{t.changeUpdates}</span>;
  return null;
}

export function EffectsSection({
  strings: t,
  selection,
  onSelectionChange,
  imported,
  onImport,
  onRemove,
  onEdit,
  nativeLooks,
  cardEffects,
  onCard,
  changedOnCard,
  customizations,
  scheme,
  onScheme,
  onCustomize,
  onCustomizations,
  family,
  recipes,
  onAddRecipe,
}: Props) {
  const catalog = use(loadCatalog(PREVIEW_TARGET));
  // The site looks as the shelf names them: with the custom name of each that has one.
  const siteEffects = withCustomNames(catalog.effects, customizations);
  const effects: readonly Effect[] = [...siteEffects, ...imported];
  const checked = resolveSelection(
    selection,
    catalog.effects.map((effect) => effect.id),
    [...imported, ...cardEffects].map((effect) => effect.id),
  );
  const taken = new Set([...effects, ...cardEffects].map((effect) => effect.id));
  // Whether the checked effects hold a built-in look with film box colors (a site film look or a
  // native look), for the scheme switch.
  const boxLooks = [...catalog.effects, ...NATIVE_LOOKS].some(
    (effect) => checked.has(effect.id) && schemeCover(effect.cover, 'filmBox') !== undefined,
  );
  // The checked site film looks of each group with an individual cover, for its reset button.
  const covered = (mono: boolean) =>
    catalog.effects.filter(
      (effect) =>
        isMonoLook(effect) === mono &&
        checked.has(effect.id) &&
        individualCover(effect.cover, customizations.get(effect.id)?.cover),
    );
  const resets = (
    [
      { key: 'color', label: t.coversResetColor, looks: covered(false) },
      { key: 'mono', label: t.coversResetMono, looks: covered(true) },
    ] as const
  ).filter((reset) => reset.looks.length > 0);
  // The count reads "of" the effects that a package can hold: each listed effect, up to the limit.
  // With more listed effects than the limit, it also gives how many are listed.
  const listed = effects.length + cardEffects.length;
  const selectable = Math.min(listed, MAX_EFFECTS);
  // The effect that the frame shows, which changes in a transition when its preview is ready, and
  // the effect that the user selected last, which the tiles and the text show at once.
  const [previewId, setPreviewId] = useState(DEFAULT_PREVIEW);
  const [requestedId, setRequestedId] = useState(DEFAULT_PREVIEW);
  const [source, setSource] = useState<Source>(DEFAULT_SAMPLE);
  const photo = useRef<Source | null>(null);
  const importer = useRef<ImportDialogHandle>(null);
  const unlocker = useRef<NativeLookDialogHandle>(null);
  const editor = useRef<EditDialogHandle>(null);
  const schemeLabel = useId();
  /** The site film look of `effect`, with its site name, when the user can edit it. */
  const siteLook = (effect: Effect | CardEffect): CatalogEffect | undefined => {
    if (effect.imported || effect.card) return undefined;
    const site = catalog.effects.find((item) => item.id === effect.id);
    return site && isFilmCover(site.cover) ? site : undefined;
  };
  const editSite = (site: CatalogEffect) => editor.current?.openSite(site, customizations.get(site.id), scheme);
  const [loader] = useState(() => new PhotoLoader());
  const [decoding, setDecoding] = useState(false);
  const ownPhoto = useCallback(
    () => () => {
      loader.cancel();
      if (photo.current) forget(photo.current);
      photo.current = null;
    },
    [loader],
  );
  const [photoError, setPhotoError] = useState(false);
  const [previewAttempt, setPreviewAttempt] = useState(0);
  const [isPending, startTransition] = useTransition();
  const photoCount = useRef(0);
  // The viewer, not the frame, goes to full screen: an error replaces the frame, and the removal of
  // the full-screen element would end full screen.
  const [viewer, setViewer] = useState<HTMLDivElement | null>(null);
  const canFullscreen = useSyncExternalStore(neverChanges, fullscreenEnabled, () => false);
  const fullscreenElement = useSyncExternalStore(
    subscribeFullscreen,
    () => document.fullscreenElement,
    () => null,
  );
  const fullscreen = viewer !== null && fullscreenElement === viewer;
  const desktop = useSyncExternalStore(subscribeDesktop, desktopPreview, () => false);
  const subscribeViewer = useCallback(
    (change: () => void) => subscribeInView(viewer, change, [0, 0.5, 1]),
    [viewer],
  );
  // Show the float once half or more of the preview is off-screen. Missing viewer: keep it hidden.
  const viewerMostlyVisible = useSyncExternalStore(
    subscribeViewer,
    () => elementVisibleRatio(viewer) > 0.5,
    () => true,
  );
  // Only after the main preview has been on screen: otherwise a below-the-fold viewer looks
  // "mostly hidden" while the user is still on Camera or Modules, and the float would appear early.
  const [reachedPreview, setReachedPreview] = useState(false);
  if (viewer && viewerMostlyVisible && !reachedPreview) {
    setReachedPreview(true);
  }
  // Hide the float once the whole Film Looks section has scrolled away.
  const subscribeSection = useCallback(
    (change: () => void) => subscribeInView(effectsSection(), change),
    [],
  );
  const sectionInView = useSyncExternalStore(
    subscribeSection,
    () => elementInView(effectsSection(), false),
    () => false,
  );

  const shown = effects.find((effect) => effect.id === previewId) ?? effects[0];
  const requested = effects.find((effect) => effect.id === requestedId) ?? shown;
  const name = requested?.name ?? '';
  const previewOf = (effect: Effect): PreviewEffect => ({
    id: effect.id,
    table: previewTable(catalog.target, effect),
    size: catalog.lutSize,
    baseline: effectUrl(catalog.target, catalog.baseline),
  });
  const previewed = shown ? previewOf(shown) : null;
  // The target look of the selected effect: the catalog text, the LUT and the fit of an import,
  // or the firmware of a native look.
  const look = !requested
    ? ''
    : requested.imported?.kind === 'native'
      ? nativeText(t, nativeLook(requested.id)?.firmware ?? 'gr4').from(requested.imported.version)
      : requested.imported?.kind === 'restored'
        ? t.restoredFrom(requested.imported.source)
        : requested.imported
          ? `${requested.imported.source} · ${t.fitText[requested.imported.grade]}`
          : (t.looks[requested.id] ?? requested.look);
  // Locked native looks wait as tiles of their group until the user unlocks them from firmware, or
  // until the connected card already has that effect file.
  const locked = catalog.baseGamma ? missingNativeLooks([...imported, ...cardEffects], nativeLooks) : [];
  const [filter, setFilter] = useState<EffectFilter>('all');
  // What the frame waits for: a new photo, or the preview of the selected effect.
  const waiting = decoding ? t.decodingPhoto : isPending ? t.previewRendering(name) : null;
  const listEntries = buildList(
    siteEffects,
    imported,
    cardEffects,
    filter,
    t,
    locked,
    Boolean(catalog.baseGamma),
    new Set(resets.map((reset) => reset.key)),
  );
  const visibleEffects = listEntries.flatMap((entry) =>
    entry.kind === 'effect' ? [entry.effect] : [],
  );

  function toggle(effect: Effect | CardEffect) {
    const next = new Set(checked);
    if (next.has(effect.id)) {
      next.delete(effect.id);
    } else if (next.size < MAX_EFFECTS) {
      next.add(effect.id);
    }
    onSelectionChange(next);
  }

  function showEffect(effect: Effect) {
    setRequestedId(effect.id);
    startTransition(() => setPreviewId(effect.id));
  }

  /** Downloads the preview table of an effect that the pointer or the focus reaches. */
  function prefetch(effect: Effect) {
    prefetchTable(previewTable(catalog.target, effect));
  }

  /** Starts the preview at the press, before the click, which comes at the release. */
  function startPreview(effect: Effect) {
    preview(source, previewOf(effect)).catch(() => undefined);
  }

  /** Adds imports of one source and checks them if they follow it closely and the package has
   * room. Returns true when the package had no room for one of them. */
  function importEffects(effects: readonly ImportedEffect[]): boolean {
    const first = effects[0];
    if (!first) return false;
    onImport(effects);
    const { selection: next, full } = selectImport(
      checked,
      effects.map((effect) => effect.id),
      importFit(first.imported),
    );
    if (next !== checked) onSelectionChange(next);
    showEffect(first);
    return full;
  }

  function openImport() {
    importer.current?.open();
  }

  /**
   * Adds the imports of a GRKit.zip. The ones that the ZIP installs are checked again while the
   * package has room, and the others wait for the user.
   */
  function openUnlock(firmware: NativeFirmware) {
    unlocker.current?.open(firmware);
  }

  function showSample() {
    loader.cancel();
    setDecoding(false);
    setPhotoError(false);
    startTransition(() => setSource(DEFAULT_SAMPLE));
    if (photo.current) forget(photo.current);
    photo.current = null;
  }

  async function openPhoto(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = '';
    if (!file) return;
    setPhotoError(false);
    setDecoding(true);
    try {
      const image = await loader.open(file);
      // A replacement, sample selection, or unmount cancelled this request.
      if (!image) return;
      const next: Source = { key: `photo-${++photoCount.current}`, load: async () => image };
      const previous = photo.current;
      photo.current = next;
      startTransition(() => setSource(next));
      if (previous) forget(previous);
      setDecoding(false);
    } catch {
      setPhotoError(true);
      setDecoding(false);
    }
  }

  const sourceLabel = source === DEFAULT_SAMPLE ? t.samplePhoto : t.yourPhoto;

  function retryPreview() {
    forget(source);
    setPreviewAttempt((attempt) => attempt + 1);
  }

  function toggleFullscreen() {
    // A refusal, such as without a user gesture, leaves the page as it is.
    if (fullscreen) document.exitFullscreen().catch(() => undefined);
    else viewer?.requestFullscreen().catch(() => undefined);
  }

  function renderPreview() {
    if (!shown || !previewed) return null;
    return (
      <ErrorBoundary
        resetKey={`${source.key}\n${shown.id}\n${previewAttempt}`}
        fallback={
          <div className="frame">
            <div className="frame-placeholder">
              <p>{t.previewError}</p>
              <button type="button" className="button small" onClick={retryPreview}>
                {t.retryPreview}
              </button>
            </div>
          </div>
        }
      >
        <Suspense fallback={<Placeholder text={sourceLabel} busy />}>
          <PreviewFrame
            source={source}
            effect={previewed}
            compareLabel={t.compare}
            busy={isPending || decoding}
            status={waiting}
            originalLabel={t.original}
            originalAlt={sourceLabel}
            effectName={shown.name}
          />
        </Suspense>
      </ErrorBoundary>
    );
  }

  // Desktop only: a corner copy once the user has reached the main preview and then scrolled it
  // mostly off-screen, while the Film Looks section remains.
  const showFloat =
    desktop &&
    reachedPreview &&
    !viewerMostlyVisible &&
    sectionInView &&
    !fullscreen &&
    Boolean(shown && previewed);

  return (
    <div className="section-body effects-body" ref={ownPhoto}>
      <div className="preview-row">
        <div className="viewer" ref={setViewer}>
          {renderPreview()}
          {canFullscreen && shown && (
            <button
              type="button"
              className="frame-fullscreen"
              aria-label={fullscreen ? t.exitFullScreen : t.fullScreen}
              title={fullscreen ? t.exitFullScreen : t.fullScreen}
              onClick={toggleFullscreen}
            >
              <svg
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                {fullscreen ? (
                  <path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5" />
                ) : (
                  <path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" />
                )}
              </svg>
            </button>
          )}
        </div>

        <div className="preview-side">
          <div className="preview-meta">
            <div className="preview-copy">
              <span className="field-label">{t.previewing}</span>
              <span className="previewing">
                {name}
                {requested && STOCKS[requested.id] && (
                  <span className="effect-stock">{stockText(STOCKS[requested.id]!, t)}</span>
                )}
              </span>
              {look && <span className="previewing-look">{look}</span>}
              {requested && !requested.imported && t.lookHints[requested.id] && (
                <span className="previewing-hint">{t.lookHints[requested.id]}</span>
              )}
              {requested && onCard && (
                <ChangeBadge
                  on={onCard.has(requested.id)}
                  checked={checked.has(requested.id)}
                  changed={changedOnCard?.has(requested.id)}
                  strings={t}
                />
              )}
              {requested && siteLook(requested) && (
                <div className="preview-actions">
                  <button
                    type="button"
                    className="link-button"
                    aria-label={t.editOf(requested.name)}
                    onClick={() => editSite(siteLook(requested)!)}
                  >
                    {t.editEffect}
                  </button>
                </div>
              )}
              {requested?.imported && (
                <div className="preview-actions">
                  {editable(requested) && (
                    <button
                      type="button"
                      className="link-button"
                      onClick={() => editor.current?.open(requested, scheme)}
                    >
                      {t.editEffect}
                    </button>
                  )}
                  <button
                    type="button"
                    className="link-button"
                    onClick={() => onRemove(requested)}
                  >
                    {t.removeEffect}
                  </button>
                </div>
              )}
            </div>
            <div className="preview-controls">
              <p className="note preview-note">
                {/* The landing hero credits the sample photo where it first shows. */}
                {t.previewNote}
              </p>
              <div className="preview-control-row">
                {requested && (
                  <button
                    type="button"
                    className="button small package-toggle"
                    aria-pressed={checked.has(requested.id)}
                    disabled={!checked.has(requested.id) && checked.size >= MAX_EFFECTS}
                    onClick={() => toggle(requested)}
                  >
                    {checked.has(requested.id) ? t.removeFromPackage : t.includeInPackage}
                  </button>
                )}
                <label htmlFor="photo-input" className="button small upload">
                  <svg
                    width="18"
                    height="18"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                  >
                    <path d="M12 16V4" />
                    <path d="M7 9l5-5 5 5" />
                    <path d="M4 20h16" />
                  </svg>
                  <span>{t.upload}</span>
                </label>
                <input
                  id="photo-input"
                  className="visually-hidden"
                  type="file"
                  accept=".dng,image/x-adobe-dng"
                  onChange={openPhoto}
                />
                {source !== DEFAULT_SAMPLE && (
                  <button type="button" className="button small" onClick={showSample}>
                    {t.useSample}
                  </button>
                )}
              </div>
              {decoding && (
                <p role="status" className="status">
                  {t.decodingPhoto}
                </p>
              )}
              {photoError && (
                <p role="alert" className="status error">
                  {t.photoError}
                </p>
              )}
            </div>
          </div>
        </div>
      </div>

      <div className="effects-pick">
        <div className="toolbar effects-toolbar">
          <p className="effects-count">
            {t.count(selectable)[0]}
            <span className="count">{checked.size}</span>
            {t.count(selectable)[1]}
            {listed > MAX_EFFECTS && t.countTotal(listed)}
          </p>
          <div className="effects-filters" role="group" aria-label={t.effects}>
            {(
              [
                ['all', t.filterAll],
                ['color', t.filterColor],
                ['mono', t.filterMono],
                ['yours', t.filterYours],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                className="filter-seg"
                aria-pressed={filter === id}
                onClick={() => setFilter(id)}
              >
                {label}
              </button>
            ))}
          </div>
          <div className="effects-bulk">
            {/* The scheme of the built-in covers without an individual cover. */}
            {boxLooks && (
              <div className="cover-scheme">
                <span id={schemeLabel}>{t.coversFilmBox}</span>
                <div className="effects-filters" role="group" aria-labelledby={schemeLabel}>
                  {COVER_SCHEMES.map((option) => (
                    <button
                      key={option}
                      type="button"
                      className="filter-seg"
                      aria-pressed={scheme === option}
                      onClick={() => onScheme(option)}
                    >
                      {t.coverSchemes[option]}
                    </button>
                  ))}
                </div>
              </div>
            )}
            <button
              type="button"
              className="button small"
              onClick={() => {
                const next = new Set(checked);
                for (const effect of visibleEffects) {
                  if (next.size >= MAX_EFFECTS) break;
                  next.add(effect.id);
                }
                onSelectionChange(next);
              }}
            >
              {t.selectAll}
            </button>
            <button
              type="button"
              className="button small"
              onClick={() => {
                const drop = new Set(visibleEffects.map((effect) => effect.id));
                onSelectionChange(new Set([...checked].filter((id) => !drop.has(id))));
              }}
            >
              {t.clear}
            </button>
          </div>
        </div>
        <div className="note-group">
          <p className="note">{t.countLimit(MAX_EFFECTS)}</p>
          {catalog.baseGamma && (
            <p className="note">
              {t.importHint}{' '}
              <button type="button" className="link-button" onClick={openImport}>
                {t.importOpen}
              </button>
            </p>
          )}
          {onCard && <p className="note">{t.effectsCardNote}</p>}
        </div>

        <div className="effects-shelf" aria-label={t.effects}>
          {listEntries.map((entry) => {
            if (entry.kind === 'header') {
              const reset = resets.find((item) => item.key === entry.group);
              return (
                <div key={entry.key} className="shelf-label">
                  <span>{entry.label}</span>
                  {reset && (
                    <button
                      type="button"
                      className="link-button cover-clear"
                      onClick={() => onCustomizations(withoutCustomCovers(reset.looks, customizations))}
                    >
                      {reset.label}
                    </button>
                  )}
                </div>
              );
            }
            if (entry.kind === 'import') {
              return (
                <button
                  key={entry.key}
                  type="button"
                  className="shelf-tile action-tile import-tile"
                  onClick={openImport}
                >
                  <span className="tile-body">
                    <span className="import-mark" aria-hidden="true">
                      +
                    </span>
                    <span className="tile-name">{t.importOpen}</span>
                  </span>
                </button>
              );
            }
            if (entry.kind === 'native') {
              const { look } = entry;
              return (
                <button
                  key={look.id}
                  type="button"
                  className="shelf-tile action-tile native-tile"
                  aria-label={t.nativeUnlockOf(look.name)}
                  onClick={() => openUnlock(look.firmware)}
                >
                  <span className="tile-body">
                    <CoverIcon cover={shownCover(look.cover, 'site', schemeCover(look.cover, scheme))!} />
                    <span className="tile-name">{look.name}</span>
                    <span className="tile-meta">{nativeText(t, look.firmware).needs}</span>
                  </span>
                </button>
              );
            }
            const effect = entry.effect;
            const isChecked = checked.has(effect.id);
            const previewed = effect.id === requested?.id;
            return (
              <div
                key={effect.id}
                className="shelf-tile"
                data-checked={isChecked || undefined}
                data-previewed={previewed || undefined}
              >
                <input
                  className="tile-check"
                  type="checkbox"
                  checked={isChecked}
                  disabled={!isChecked && checked.size >= MAX_EFFECTS}
                  aria-label={effect.name}
                  onChange={() => toggle(effect)}
                />
                {effect.card ? (
                  <div className="tile-body">
                    <EffectIcon effect={shelfIcon(effect, customizations, scheme)} />
                    <span className="tile-name">{effect.name}</span>
                    <span className="tile-meta">{t.cardOnlyEffect}</span>
                  </div>
                ) : (
                  <button
                    type="button"
                    className="tile-body"
                    aria-pressed={previewed}
                    aria-label={t.previewOf(effect.name)}
                    onPointerEnter={() => prefetch(effect)}
                    onFocus={() => prefetch(effect)}
                    onPointerDown={() => startPreview(effect)}
                    onClick={() => showEffect(effect)}
                  >
                    <EffectIcon effect={shelfIcon(effect, customizations, scheme)} />
                    <span className="tile-name">{effect.name}</span>
                  </button>
                )}
                {/* A film look of the site edits as an import does; only an import can go. */}
                {!effect.imported && siteLook(effect) && (
                  <div className="tile-actions">
                    <button
                      type="button"
                      className="link-button"
                      aria-label={t.editOf(effect.name)}
                      onClick={() => editSite(siteLook(effect)!)}
                    >
                      {t.editEffect}
                    </button>
                  </div>
                )}
                {effect.imported && (
                  <div className="tile-actions">
                    {editable(effect) && (
                      <button
                        type="button"
                        className="link-button"
                        aria-label={t.editOf(effect.name)}
                        onClick={() => editor.current?.open(effect, scheme)}
                      >
                        {t.editEffect}
                      </button>
                    )}
                    <button
                      type="button"
                      className="link-button"
                      aria-label={t.removeOf(effect.name)}
                      onClick={() => onRemove(effect)}
                    >
                      {t.removeEffect}
                    </button>
                  </div>
                )}
              </div>
            );
          })}
        </div>
        {catalog.baseGamma && (
          <ImportDialog
            ref={importer}
            strings={t}
            baseGamma={catalog.baseGamma}
            taken={taken}
            onImport={(effect) => importEffects([effect])}
            family={family}
            recipes={recipes}
            onAddRecipe={onAddRecipe}
          />
        )}
        <EditDialog ref={editor} strings={t} onSave={onEdit} onSaveSite={onCustomize} />
        {nativeLooks.length > 0 && catalog.baseGamma && (
          <NativeLookDialog
            ref={unlocker}
            strings={t}
            baseGamma={catalog.baseGamma}
            onImport={importEffects}
          />
        )}
      </div>
      {showFloat && (
        <aside className="preview-float" aria-hidden="true">
          {renderPreview()}
        </aside>
      )}
    </div>
  );
}

interface FrameProps {
  source: Source;
  effect: PreviewEffect;
  compareLabel: string;
  busy: boolean;
  /** The text over the frame while a new preview is on its way. */
  status: string | null;
  originalLabel: string;
  originalAlt: string;
  effectName: string;
}

/** The baseline development on the left and the effect on the right. */
function PreviewFrame({
  source,
  effect,
  compareLabel,
  busy,
  status,
  originalLabel,
  originalAlt,
  effectName,
}: FrameProps) {
  const result = use(preview(source, effect));
  const comparison = useReactCompareSlider({ clip: 'itemTwo' });
  return (
    <div className="frame" aria-busy={busy}>
      <Provider {...comparison}>
        <Root className="image-comparison">
          <Item item="itemOne">
            <img src={result.original} alt={`${originalAlt} — ${originalLabel}`} />
          </Item>
          <Item item="itemTwo" className="comparison-effect">
            <img src={result.processed} alt={`${originalAlt} — ${effectName}`} />
          </Item>
          <HandleRoot aria-label={compareLabel}>
            <Handle buttonStyle={{ width: 32, height: 32, gap: '0.2rem' }} />
          </HandleRoot>
        </Root>
      </Provider>
      <span className="tag">{effectName}</span>
      {status && (
        <span className="frame-status" role="status">
          <span className="spinner" aria-hidden="true" />
          {status}
        </span>
      )}
    </div>
  );
}

function Placeholder({ text, busy = false }: { text: string; busy?: boolean }) {
  return (
    <div className="frame" aria-busy={busy}>
      <span className="frame-placeholder">
        {busy && <span className="spinner" aria-hidden="true" />}
        {text}
      </span>
    </div>
  );
}
