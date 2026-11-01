import {
  Suspense,
  use,
  useCallback,
  useRef,
  useState,
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
  type Effect,
  type ImportedEffect,
} from '../catalog';
import { sentences, type Language, type Strings } from '../i18n';
import { forget, prefetchTable, preview, type PreviewEffect, type Source } from '../preview/render';
import { DEFAULT_SAMPLE } from '../preview/samples';
import { PhotoLoader } from '../preview/raw';
import { resolveSelection, selectImport, type Selection } from '../selection';
import { STOCKS } from '../stocks';
import { EffectIcon } from './EffectIcon';
import { ErrorBoundary } from './ErrorBoundary';
import { ImportDialog, type ImportDialogHandle } from './ImportDialog';

const DEFAULT_PREVIEW = 'portra-400-endura-v1';

/** The film number, then the print film of a cine look: "5219 · 2393 print". */
function stockText(stock: (typeof STOCKS)[string] & {}, t: Strings): string {
  return stock.print ? `${stock.film} · ${t.printFilm(stock.print)}` : stock.film;
}

interface Props {
  strings: Strings;
  language: Language;
  selection: Selection;
  onSelectionChange: (selection: ReadonlySet<string>) => void;
  /** The effects that this page fitted from LUTs, after the catalog effects. */
  imported: readonly ImportedEffect[];
  onImport: (effect: ImportedEffect) => void;
  onRemove: (effect: ImportedEffect) => void;
  /** The effect files on the connected card that the catalog and the imports do not have. */
  cardEffects: readonly CardEffect[];
  /** The IDs of the effects on the connected card, or null without a card. */
  onCard: ReadonlySet<string> | null;
}

/** The change badge of an effect or a module: what a write to the card does with it. */
export function ChangeBadge({ on, checked, strings: t }: { on: boolean; checked: boolean; strings: Strings }) {
  if (checked && !on) return <span className="change add">{t.changeAdds}</span>;
  if (!checked && on) return <span className="change remove">{t.changeRemoves}</span>;
  return null;
}

export function EffectsSection({
  strings: t,
  selection,
  onSelectionChange,
  imported,
  onImport,
  onRemove,
  cardEffects,
  onCard,
}: Props) {
  const catalog = use(loadCatalog(PREVIEW_TARGET));
  const effects: readonly Effect[] = [...catalog.effects, ...imported];
  const checked = resolveSelection(
    selection,
    catalog.effects.map((effect) => effect.id),
    [...imported, ...cardEffects].map((effect) => effect.id),
  );
  const taken = new Set([...effects, ...cardEffects].map((effect) => effect.id));
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
  // The target look of the selected effect: the catalog text, or the LUT and the fit of an import.
  const look = !requested
    ? ''
    : requested.imported
      ? `${requested.imported.source} · ${t.fitText[requested.imported.grade]}`
      : (t.looks[requested.id] ?? requested.look);
  // What the frame waits for: a new photo, or the preview of the selected effect.
  const waiting = decoding ? t.decodingPhoto : isPending ? t.previewRendering(name) : null;

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

  /** Adds an import and checks it if it fits closely and the package has room. Returns true
   * when the package had no room. */
  function importEffect(effect: ImportedEffect): boolean {
    onImport(effect);
    const { selection: next, full } = selectImport(checked, effect.id, effect.imported.grade);
    if (next !== checked) onSelectionChange(next);
    showEffect(effect);
    return full;
  }

  function openImport() {
    importer.current?.open();
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

  return (
    <div className="section-body effects-body" ref={ownPhoto}>
      <div className="preview-row">
        <div className="viewer">
          {shown && previewed && (
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
          )}
        </div>

        <div className="preview-side">
          <div className="field">
            <span className="field-label">{t.previewing}</span>
            <span className="previewing">{name}</span>
            {requested && STOCKS[requested.id] && (
              <span className="effect-stock">{stockText(STOCKS[requested.id]!, t)}</span>
            )}
            {look && <span className="previewing-look">{look}</span>}
          </div>

          <div className="field photo-field">
            <span className="field-label">{t.photoSource(sourceLabel)}</span>
            <label htmlFor="photo-input" className="upload">
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
            {source !== DEFAULT_SAMPLE && (
              <button type="button" className="button small" onClick={showSample}>
                {t.useSample}
              </button>
            )}
            <p className="note">
              {sentences(t.uploadNote, t.previewNote)}
              {source === DEFAULT_SAMPLE && (
                <>
                  {' '}
                  <a
                    href="https://www.flickr.com/photos/cafetricot/51803682755/"
                    target="_blank"
                    rel="noreferrer"
                  >
                    {t.sampleCredit}
                  </a>
                </>
              )}
            </p>
          </div>
        </div>
      </div>

      <div className="effects-pick">
        <div className="toolbar">
          <p>
            {t.count(selectable)[0]}
            <span className="count">{checked.size}</span>
            {t.count(selectable)[1]}
            {listed > MAX_EFFECTS && t.countTotal(listed)}
          </p>
          <div>
            <button
              type="button"
              className="button"
              onClick={() =>
                onSelectionChange(
                  new Set(
                    [...effects, ...cardEffects].map((effect) => effect.id).slice(0, MAX_EFFECTS),
                  ),
                )
              }
            >
              {t.selectAll}
            </button>
            <button type="button" className="button" onClick={() => onSelectionChange(new Set())}>
              {t.clear}
            </button>
          </div>
        </div>
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

        <ul className="effects-list" aria-label={t.effects}>
          {[...effects, ...cardEffects].map((effect) => {
            const isChecked = checked.has(effect.id);
            const previewed = effect.id === requested?.id;
            const stock = STOCKS[effect.id];
            const body = (
              <>
                <EffectIcon effect={effect} />
                <span className="row-text">
                  <span className="row-name">{effect.name}</span>
                  <span className="row-meta">
                    {effect.card ? t.cardOnlyEffect : effect.monochrome ? t.mono : t.color}
                    {stock && ` · ${stockText(stock, t)}`}
                  </span>
                </span>
              </>
            );
            return (
              <li
                key={effect.id}
                className="effect-row"
                data-checked={isChecked || undefined}
                data-previewed={previewed || undefined}
              >
                <input
                  className="row-check"
                  type="checkbox"
                  checked={isChecked}
                  disabled={!isChecked && checked.size >= MAX_EFFECTS}
                  aria-label={effect.name}
                  onChange={() => toggle(effect)}
                />
                {effect.card ? (
                  <div className="row-body">{body}</div>
                ) : (
                  <button
                    type="button"
                    className="row-body"
                    aria-pressed={previewed}
                    aria-label={t.previewOf(effect.name)}
                    onPointerEnter={() => prefetch(effect)}
                    onFocus={() => prefetch(effect)}
                    onPointerDown={() => startPreview(effect)}
                    onClick={() => showEffect(effect)}
                  >
                    {body}
                  </button>
                )}
                {onCard && <ChangeBadge on={onCard.has(effect.id)} checked={isChecked} strings={t} />}
                {effect.imported && (
                  <button
                    type="button"
                    className="row-remove"
                    aria-label={t.removeOf(effect.name)}
                    onClick={() => onRemove(effect)}
                  >
                    {t.removeEffect}
                  </button>
                )}
              </li>
            );
          })}
          {catalog.baseGamma && (
            <li className="effect-row import-row">
              <button type="button" className="row-body" onClick={openImport}>
                <span className="import-mark" aria-hidden="true">
                  +
                </span>
                <span className="row-name">{t.importOpen}</span>
              </button>
            </li>
          )}
        </ul>
        {catalog.baseGamma && (
          <ImportDialog
            ref={importer}
            strings={t}
            baseGamma={catalog.baseGamma}
            taken={taken}
            onImport={importEffect}
          />
        )}
      </div>
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
            <Handle buttonStyle={{ width: 44, height: 44 }} />
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
