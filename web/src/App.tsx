import { Suspense, useRef, useState } from 'react';
import { CAMERAS, MODESET_TOKEN } from './cameras';
import { loadCatalog } from './catalog';
import { EffectsSection } from './components/EffectsSection';
import { ErrorBoundary } from './components/ErrorBoundary';
import { RichText } from './components/RichText';
import { applyLanguage, initialLanguage, LANGUAGES, STRINGS, type Language } from './i18n';
import { ALL_MODULES, MODULES, planModules, type ModuleKey } from './modules';
import { buildPackage, save } from './package';

const DEFAULT_SELECTION = ['portra-400-endura-v1', 'vision3-250d-2383-v1'];
const REPOSITORY = 'https://github.com/ahonn/GRKit';

type Build = { state: 'idle' } | { state: 'building' } | { state: 'ready' } | { state: 'failed' };

export function App() {
  const [language, setLanguage] = useState<Language>(initialLanguage);
  const [cameraId, setCameraId] = useState(CAMERAS[0]!.id);
  const [selection, setSelection] = useState<ReadonlySet<string>>(() => new Set(DEFAULT_SELECTION));
  const [enabled, setEnabled] = useState<ReadonlySet<ModuleKey>>(ALL_MODULES);
  const [build, setBuild] = useState<Build>({ state: 'idle' });
  // A newer build or a changed selection makes the result of an older build stale.
  const buildRun = useRef(0);

  const t = STRINGS[language];
  const camera = CAMERAS.find((item) => item.id === cameraId) ?? CAMERAS[0]!;
  const support = camera.support;
  const available = support?.modules ?? [];
  const plan = planModules(enabled, selection.size, available);
  const effectCount = plan.includeEffects ? selection.size : 0;

  function chooseLanguage(next: Language) {
    applyLanguage(next);
    setLanguage(next);
  }

  function resetBuild() {
    buildRun.current += 1;
    setBuild({ state: 'idle' });
  }

  function chooseCamera(id: string) {
    setCameraId(id);
    resetBuild();
  }

  function changeSelection(next: ReadonlySet<string>) {
    setSelection(next);
    resetBuild();
  }

  function toggleModule(key: ModuleKey) {
    const next = new Set(enabled);
    if (!next.delete(key)) {
      next.add(key);
    }
    setEnabled(next);
    resetBuild();
  }

  async function download() {
    if (!support || plan.visible.length === 0) {
      return;
    }
    buildRun.current += 1;
    const run = buildRun.current;
    setBuild({ state: 'building' });
    try {
      const effects = plan.includeEffects
        ? (await loadCatalog(support.target)).effects.filter((effect) => selection.has(effect.id))
        : [];
      const blob = await buildPackage(support.target, camera.id, effects, plan.hidden);
      if (run === buildRun.current) {
        save(blob, 'GRKit.zip');
        setBuild({ state: 'ready' });
      }
    } catch (error) {
      console.error(error);
      if (run === buildRun.current) {
        setBuild({ state: 'failed' });
      }
    }
  }

  return (
    <div className="page">
      <div className="languages">
        <div role="group" aria-label={t.language}>
          {LANGUAGES.map((item) => (
            <button
              key={item.code}
              type="button"
              lang={item.code}
              className="button"
              aria-pressed={item.code === language}
              onClick={() => chooseLanguage(item.code)}
            >
              {item.label}
            </button>
          ))}
        </div>
      </div>

      <header className="masthead">
        <h1>GRKit</h1>
        <p>{t.tagline}</p>
      </header>

      <main>
        <section className="section" aria-labelledby="camera-label">
          <h2 id="camera-label">{t.camera}</h2>
          <div className="section-body">
            <div role="radiogroup" aria-labelledby="camera-label" className="cameras">
              {CAMERAS.map((item) => (
                <label key={item.id} className="camera">
                  <input
                    type="radio"
                    name="camera"
                    checked={item.id === camera.id}
                    disabled={!item.support}
                    onChange={() => chooseCamera(item.id)}
                  />
                  <span className="camera-text">
                    <span className="camera-name">{item.name}</span>
                    {item.support && (
                      <span className="camera-firmware">
                        {t.firmware} {item.support.firmware}
                      </span>
                    )}
                    <span
                      className={
                        !item.support
                          ? 'badge'
                          : item.support.experimental
                            ? 'badge experimental'
                            : 'badge supported'
                      }
                    >
                      {!item.support
                        ? t.planned
                        : item.support.experimental
                          ? t.experimental
                          : t.supported}
                    </span>
                  </span>
                </label>
              ))}
            </div>
            {support?.experimental && (
              <p className="note warning">{t.experimentalNote(camera.name)}</p>
            )}
          </div>
        </section>

        <section className="section" aria-labelledby="modules-label">
          <h2 id="modules-label">{t.modules}</h2>
          <div className="section-body">
            <p className="note">{t.modulesNote}</p>
            <div role="group" aria-labelledby="modules-label" className="modules">
              {MODULES.filter((module) => available.includes(module.key)).map((module) => (
                <label key={module.key} className="module">
                  <input
                    type="checkbox"
                    checked={enabled.has(module.key)}
                    onChange={() => toggleModule(module.key)}
                  />
                  <span className="module-text">
                    <span className="module-name">{module.name}</span>
                    <span className="module-description">
                      {module.key === 'framing' && support?.xpanOnly
                        ? t.framingXpanOnly
                        : t.modulesText[module.key]}
                    </span>
                    {module.key === 'imageControl' &&
                      enabled.has('imageControl') &&
                      selection.size === 0 && (
                        <span className="module-warning">{t.imageControlEmpty}</span>
                      )}
                  </span>
                </label>
              ))}
            </div>
          </div>
        </section>

        <section className="section" aria-labelledby="effects-label">
          <h2 id="effects-label">{t.effects}</h2>
          {!available.includes('imageControl') ? (
            <p className="status section-body">{t.effectsUnavailable(camera.name)}</p>
          ) : plan.includeEffects ? (
            <ErrorBoundary fallback={<p className="status error section-body">{t.loadError}</p>}>
              <Suspense fallback={<p className="status section-body">{t.loading}</p>}>
                <EffectsSection
                  strings={t}
                  language={language}
                  selection={selection}
                  onSelectionChange={changeSelection}
                />
              </Suspense>
            </ErrorBoundary>
          ) : (
            <p className="status section-body">{t.effectsOff}</p>
          )}
        </section>

        <section className="section" aria-labelledby="package-label">
          <h2 id="package-label">{t.package}</h2>
          <div className="package">
            <dl>
              <dt>{t.camera}</dt>
              <dd>{camera.name}</dd>
              <dt>{t.firmware}</dt>
              <dd>{support?.firmware ?? '—'}</dd>
              <dt>{t.effectCount}</dt>
              <dd>{effectCount}</dd>
              <dt>{t.modules}</dt>
              <dd>
                {plan.visible.length > 0
                  ? MODULES.filter((module) => plan.visible.includes(module.key))
                      .map((module) => module.name)
                      .join(', ')
                  : '—'}
              </dd>
              <dd className="package-note">{t.firmwareNote}</dd>
            </dl>
            <div className="package-action">
              <button
                type="button"
                className="download"
                disabled={!support || plan.visible.length === 0 || build.state === 'building'}
                aria-busy={build.state === 'building'}
                onClick={download}
              >
                {t.download}
              </button>
              <div aria-live="polite">
                {plan.visible.length === 0 && <p className="status error">{t.noModules}</p>}
                {build.state === 'building' && <p className="status">{t.building}</p>}
                {build.state === 'ready' && <p className="status">{t.ready}</p>}
                {build.state === 'failed' && (
                  <p className="status error">{t.buildError}</p>
                )}
              </div>
            </div>
          </div>
        </section>

        <section className="section" aria-labelledby="install-label">
          <h2 id="install-label">{t.install}</h2>
          <ol className="steps">
            {t.steps.map((parts, index) => (
              <li key={index}>
                <RichText
                  parts={parts.map((part) =>
                    part === MODESET_TOKEN ? `\`${support?.modeset ?? ''}` : part,
                  )}
                />
              </li>
            ))}
          </ol>
        </section>
      </main>

      <footer className="footer">
        <p>{t.footer}</p>
        <div>
          <a href={`${REPOSITORY}/blob/main/effects/NOTICE.md`}>{t.licenses}</a>
          <a href={`${REPOSITORY}/blob/main/resources/RECOVERY.md`}>{t.recovery}</a>
          <a href={REPOSITORY}>{t.source}</a>
        </div>
      </footer>
    </div>
  );
}
