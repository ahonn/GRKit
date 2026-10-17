import { Suspense, use, useRef, useState, useTransition, type ChangeEvent } from 'react';
import { PREVIEW_TARGET } from '../cameras';
import { effectUrl, loadCatalog, MAX_EFFECTS, type Effect } from '../catalog';
import type { Language, Strings } from '../i18n';
import { forget, picture, preview, type PreviewEffect, type Source } from '../preview/render';
import { SAMPLES, type Sample } from '../preview/samples';
import { EffectIcon } from './EffectIcon';
import { ErrorBoundary } from './ErrorBoundary';

const DEFAULT_PREVIEW = 'portra-400-endura-v1';

interface Props {
  strings: Strings;
  language: Language;
  selection: ReadonlySet<string>;
  onSelectionChange: (selection: ReadonlySet<string>) => void;
}

export function EffectsSection({ strings: t, language, selection, onSelectionChange }: Props) {
  const catalog = use(loadCatalog(PREVIEW_TARGET));
  const effects = catalog.effects;
  const [previewId, setPreviewId] = useState(DEFAULT_PREVIEW);
  const [source, setSource] = useState<Source>(SAMPLES[0]!);
  const [photo, setPhoto] = useState<Source | null>(null);
  const [photoError, setPhotoError] = useState(false);
  const [split, setSplit] = useState(50);
  const [isPending, startTransition] = useTransition();
  const photoCount = useRef(0);

  const shown = effects.find((effect) => effect.id === previewId) ?? effects[0];
  const name = shown?.name ?? '';

  function toggle(effect: Effect) {
    const next = new Set(selection);
    if (next.has(effect.id)) {
      next.delete(effect.id);
    } else if (next.size < MAX_EFFECTS) {
      next.add(effect.id);
    }
    onSelectionChange(next);
  }

  function showEffect(effect: Effect) {
    startTransition(() => setPreviewId(effect.id));
  }

  function showSample(sample: Sample) {
    setPhotoError(false);
    startTransition(() => {
      setSource(sample);
      setPhoto(null);
    });
    if (photo) {
      forget(photo);
    }
  }

  async function openPhoto(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    // Clear the input, so the same file can be opened again.
    event.target.value = '';
    if (!file) {
      return;
    }
    photoCount.current += 1;
    const next: Source = { key: `photo-${photoCount.current}`, load: async () => file };
    try {
      await picture(next);
    } catch {
      setPhotoError(true);
      return;
    }
    setPhotoError(false);
    const previous = photo;
    startTransition(() => {
      setPhoto(next);
      setSource(next);
    });
    if (previous) {
      forget(previous);
    }
  }

  const sourceLabel =
    photo && source === photo
      ? t.yourPhoto
      : (SAMPLES.find((sample) => sample === source)?.label(t, language) ?? '');

  return (
    <div className="section-body">
      <div className="preview-row">
        <div className="viewer">
          {shown && (
            <ErrorBoundary
              resetKey={`${source.key}\n${shown.id}`}
              fallback={<Placeholder text={t.previewError} />}
            >
              <Suspense fallback={<Placeholder text={sourceLabel} busy />}>
                <PreviewFrame
                  source={source}
                  effect={{
                    id: shown.id,
                    table: effectUrl(catalog.target, shown.preview),
                    size: catalog.lutSize,
                  }}
                  split={split}
                  busy={isPending}
                  originalLabel={t.original}
                  originalAlt={sourceLabel}
                  effectName={name}
                />
              </Suspense>
            </ErrorBoundary>
          )}
          <label className="compare">
            <span>{t.compare}</span>
            <input
              type="range"
              min="0"
              max="100"
              value={split}
              onChange={(event) => setSplit(Number(event.target.value))}
            />
          </label>
        </div>

        <div className="preview-side">
          <div className="field">
            <span className="field-label">{t.previewing}</span>
            <span className="previewing">{name}</span>
          </div>
          <div className="field">
            <span className="field-label">{t.samples}</span>
            <div className="choices">
              {SAMPLES.map((sample) => (
                <button
                  key={sample.key}
                  type="button"
                  className="button small"
                  aria-pressed={source === sample}
                  onClick={() => showSample(sample)}
                >
                  {sample.label(t, language)}
                </button>
              ))}
            </div>
          </div>
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
            accept="image/jpeg,image/png"
            onChange={openPhoto}
          />
          {photoError && (
            <p role="alert" className="status error">
              {t.photoError}
            </p>
          )}
          <p className="note">{t.uploadNote}</p>
          <p className="note">{t.previewNote}</p>
        </div>
      </div>

      <div className="toolbar">
        <p>
          {t.count(MAX_EFFECTS)[0]}
          <span className="count">{selection.size}</span>
          {t.count(MAX_EFFECTS)[1]}
        </p>
        <div>
          <button
            type="button"
            className="button"
            onClick={() =>
              onSelectionChange(new Set(effects.slice(0, MAX_EFFECTS).map((effect) => effect.id)))
            }
          >
            {t.selectAll}
          </button>
          <button type="button" className="button" onClick={() => onSelectionChange(new Set())}>
            {t.clear}
          </button>
        </div>
      </div>

      <div className="table-box">
        <table>
          <thead>
            <tr>
              <th scope="col" style={{ width: 44 }}>
                <span className="visually-hidden">{t.include}</span>
              </th>
              <th scope="col" style={{ width: 56 }}>
                {t.icon}
              </th>
              <th scope="col">{t.effect}</th>
              <th scope="col">{t.look}</th>
              <th scope="col">{t.type}</th>
              <th scope="col" style={{ width: 88 }}>
                <span className="visually-hidden">{t.preview}</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {effects.map((effect) => (
              <tr key={effect.id}>
                <td>
                  <input
                    id={`effect-${effect.id}`}
                    type="checkbox"
                    checked={selection.has(effect.id)}
                    disabled={!selection.has(effect.id) && selection.size >= MAX_EFFECTS}
                    onChange={() => toggle(effect)}
                  />
                </td>
                <td>
                  <EffectIcon effect={effect} />
                </td>
                <td>
                  <label htmlFor={`effect-${effect.id}`} className="effect-name">
                    {effect.name}
                  </label>
                </td>
                <td>{t.looks[effect.id] ?? effect.look}</td>
                <td>{effect.monochrome ? t.mono : t.color}</td>
                <td>
                  <button
                    type="button"
                    className="button small"
                    aria-pressed={effect.id === shown?.id}
                    aria-label={t.previewOf(effect.name)}
                    onClick={() => showEffect(effect)}
                  >
                    {t.preview}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

interface FrameProps {
  source: Source;
  effect: PreviewEffect;
  split: number;
  busy: boolean;
  originalLabel: string;
  originalAlt: string;
  effectName: string;
}

/** The original on the left of the divider and the effect on the right. */
function PreviewFrame({ source, effect, split, busy, originalLabel, originalAlt, effectName }: FrameProps) {
  const result = use(preview(source, effect));
  return (
    <div className="frame" aria-busy={busy}>
      <img src={result.original} alt={originalAlt} />
      <div className="after" style={{ clipPath: `inset(0 0 0 ${split}%)` }}>
        <img src={result.processed} alt="" />
      </div>
      <div className="divider" style={{ left: `${split}%` }} />
      <span className="tag left">{originalLabel}</span>
      <span className="tag right">{effectName}</span>
    </div>
  );
}

function Placeholder({ text, busy = false }: { text: string; busy?: boolean }) {
  return (
    <div className="frame" aria-busy={busy}>
      <span className="frame-placeholder">{text}</span>
    </div>
  );
}
