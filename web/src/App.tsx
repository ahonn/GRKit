import { Suspense, use, useCallback, useRef, useState, type ReactNode } from 'react';
import {
  trackCardUninstall,
  trackCardWrite,
  trackDownload,
  trackDownloadError,
  trackScriptsOff,
} from './analytics';
import { suggestOwnBrowser } from './browser';
import {
  applyChanges,
  canWriteCards,
  suggestCardBrowser,
  cardBuilds,
  cardModules,
  pickCard,
  planChanges,
  removeGrkit,
  removeModesetFiles,
  writeFiles,
  type CardChanges,
  type CardDirectory,
} from './card';
import { allowCard, forgetLastCard, saveLastCard } from './cardStore';
import { libraryChanges, openSession, packageEffects as cardPackageEffects, type CardSession } from './cardSession';
import {
  cameraForCard,
  CAMERAS,
  camerasForCard,
  firmwareForTarget,
  MODESET_TOKEN,
  PREVIEW_TARGET,
  type Camera,
} from './cameras';
import { loadCatalog, siteUrl, type ImportedEffect, type PackageEffect } from './catalog';
import {
  CardChangesView,
  CardUninstall,
  StartSection,
  UninstallSection,
  type CardMessage,
  type CardTask,
  type InstallMode,
} from './components/CardPanel';
import { ChangeBadge, EffectsSection } from './components/EffectsSection';
import { ErrorBoundary } from './components/ErrorBoundary';
import { RichText } from './components/RichText';
import { loadFonts } from './fonts';
import { deleteImport, saveImport } from './import/store';
import { applyLanguage, initialLanguage, LANGUAGES, STRINGS, type Language, type Strings } from './i18n';
import { ALL_MODULES, MODULES, planModules, type ModuleKey } from './modules';
import {
  buildCardFiles,
  buildPackage,
  buildScriptsOff,
  prefetchPackage,
  save,
  scriptsOffCardFiles,
} from './package';
import { loadSelection, resolveSelection, saveSelection, type Selection } from './selection';
import { TransferError } from './transfer';

const REPOSITORY = 'https://github.com/ahonn/grkit';
/** The Tally form for problems, feature ideas, and questions. A plain link, so the page loads nothing from Tally. */
const FEEDBACK_FORM = 'https://tally.so/r/Z9jRgB';
const DONATE = 'https://buymeacoffee.com/yuexunjiang';
/** The release page of the GRKit version that the site packages. */
const RELEASE_URL = `${REPOSITORY}/releases/tag/v${__GRKIT_VERSION__}`;

/** A guide in `resources/` in the page language, or in English. */
function guideUrl(name: 'RECOVERY' | 'DUMP', language: Language): string {
  return `${REPOSITORY}/blob/main/resources/${name}${language === 'en' ? '' : `.${language}`}.md`;
}

type Build =
  | { state: 'idle' }
  | { state: 'building'; received: number; retrying: boolean }
  | { state: 'ready' }
  | { state: 'failed' };

/** Megabytes with one decimal, as the progress text shows them. */
const megabytes = (bytes: number) => (bytes / 1_000_000).toFixed(1);

export function App({
  initialImports = [],
  initialCard = null,
}: {
  initialImports?: readonly ImportedEffect[];
  /** The card connected on an earlier visit, which a click reconnects. */
  initialCard?: CardDirectory | null;
}) {
  const [language, setLanguage] = useState<Language>(initialLanguage);
  const [cameraKey, setCameraKey] = useState(CAMERAS[0]!.key);
  // The ZIP download has no card to read the effects from, so the choice of the last visit
  // comes back.
  const [selection, setSelection] = useState<Selection>(() => loadSelection(browserStorage()));
  const [enabled, setEnabled] = useState<ReadonlySet<ModuleKey>>(ALL_MODULES);
  // Effects fitted from LUTs on this page. This browser keeps them for the next visit.
  const [imported, setImported] = useState<readonly ImportedEffect[]>(initialImports);
  const [build, setBuild] = useState<Build>({ state: 'idle' });
  // A retry of a failed catalog request clears the error boundaries that use the catalog.
  const [catalogAttempt, setCatalogAttempt] = useState(0);
  // A newer build or a changed selection makes the result of an older build stale.
  const buildRun = useRef(0);
  const [mode, setMode] = useState<InstallMode>(initialMode);
  // The connected SD card. The page changes it only when the user confirms the changes.
  const [card, setCard] = useState<CardSession | null>(null);
  const [lastCard, setLastCard] = useState<CardDirectory | null>(initialCard);
  const [cardTask, setCardTask] = useState<CardTask>({ kind: 'idle' });
  const [connectMessage, setConnectMessage] = useState<CardMessage>(null);
  const [applyMessage, setApplyMessage] = useState<CardMessage>(null);
  const [removeMessage, setRemoveMessage] = useState<CardMessage>(null);
  // The section whose action runs, so that only it shows the progress.
  const [taskArea, setTaskArea] = useState<'apply' | 'remove'>('apply');
  // A newer connection makes the result of an older card task stale.
  const cardRun = useRef(0);

  const t = STRINGS[language];
  // An app browser, such as WeChat's, often saves no GRKit.zip, so the page asks for a browser.
  const appBrowser = suggestOwnBrowser();
  const camera = CAMERAS.find((item) => item.key === cameraKey) ?? CAMERAS[0]!;
  const support = camera.support;
  const targets = support?.targets;
  const available = support?.modules ?? [];
  // A card with GRKit fixes the camera: an update or a change keeps the camera of the card. One
  // build runs on two entries, such as GR III and GR IIIx, so the user can still pick between them.
  // Only the card mode uses the card.
  const active = mode === 'card' ? card : null;
  // Uninstall shows for a card with GRKit, for the files that turned SD scripts off, and for the
  // result of the last uninstall.
  const showUninstall =
    !!active &&
    (active.state.install !== 'none' || active.state.modeset?.command === 'off' || !!removeMessage);
  const cardCameras =
    active && active.state.install !== 'none'
      ? camerasForCard(cardBuilds(active.state), active.state.rootNames)
      : null;
  const lockedCameras = cardCameras && cardCameras.length > 0 ? cardCameras : null;
  // With a card whose GRKit tells its camera, the card facts show the camera instead of the
  // camera section.
  const cardCamera =
    active && lockedCameras
      ? {
          options: lockedCameras,
          selected: camera,
          firmware: firmwareForTarget(camera, oneBuild(cardBuilds(active.state))),
        }
      : null;
  // Before the first change, every catalog effect is checked, so the count is not 0.
  const plan = planModules(enabled, selection?.size ?? 1, available);

  function prefetch() {
    if (targets) prefetchPackage(targets, camera.id, language);
  }

  // When the effects come into view, the package files start to download, so Download is quick
  // on a slow connection. A change of camera or language observes again for the new files.
  const prefetchInView = useCallback(
    (section: HTMLElement | null) => {
      if (!section || !targets) return;
      const observer = new IntersectionObserver(
        (entries) => {
          if (entries.some((entry) => entry.isIntersecting)) {
            observer.disconnect();
            prefetchPackage(targets, camera.id, language);
          }
        },
        { rootMargin: '200px 0px' },
      );
      observer.observe(section);
      return () => observer.disconnect();
    },
    [targets, camera.id, language],
  );

  function chooseLanguage(next: Language) {
    loadFonts(next);
    applyLanguage(next);
    setLanguage(next);
  }

  /** A change of the selection makes the result of a download or a card write stale. */
  function resetBuild() {
    buildRun.current += 1;
    setBuild({ state: 'idle' });
    setApplyMessage(null);
  }

  function chooseCamera(key: Camera['key']) {
    setCameraKey(key);
    resetBuild();
  }

  function keepSelection(next: ReadonlySet<string>) {
    setSelection(next);
    saveSelection(browserStorage(), next);
  }

  function changeSelection(next: ReadonlySet<string>) {
    keepSelection(next);
    resetBuild();
  }

  // The effects section checks an import (`selectImport`).
  function addImported(effect: ImportedEffect) {
    setImported((current) => [...current, effect]);
    void saveImport(effect);
    resetBuild();
  }

  function removeImported(effect: ImportedEffect) {
    setImported((current) => current.filter((item) => item !== effect));
    void deleteImport(effect.id);
    if (selection?.has(effect.id)) {
      const next = new Set(selection);
      next.delete(effect.id);
      changeSelection(next);
    } else {
      resetBuild();
    }
  }

  function toggleModule(key: ModuleKey) {
    const next = new Set(enabled);
    if (!next.delete(key)) {
      next.add(key);
    }
    setEnabled(next);
    resetBuild();
  }

  const [scriptsOff, setScriptsOff] = useState<'idle' | 'building' | 'failed'>('idle');

  async function downloadScriptsOff() {
    setScriptsOff('building');
    try {
      save(await buildScriptsOff(camera.id), 'GRKit-scripts-off.zip');
      trackScriptsOff(camera);
      setScriptsOff('idle');
    } catch (error) {
      console.error(error);
      setScriptsOff('failed');
    }
  }

  /**
   * The checked effects of the table. The builds of a camera have the same effects, so the first
   * build's catalog serves the package. Imported effects do not depend on the target.
   */
  async function checkedEffects(): Promise<PackageEffect[]> {
    const catalog = (await loadCatalog(targets![0]!)).effects;
    const others = [...imported, ...(card?.extra ?? [])];
    const checked = resolveSelection(
      selection,
      catalog.map((effect) => effect.id),
      others.map((effect) => effect.id),
    );
    return [...catalog, ...others].filter((effect) => checked.has(effect.id));
  }

  /** The effects of the package: the checked ones, or none while Film Look is off. */
  const packageEffects = () => (plan.includeEffects ? checkedEffects() : Promise.resolve([]));

  /** Opens the card picker, then reads the card that the user picks. */
  async function connectCard() {
    let root;
    try {
      root = await pickCard();
    } catch (error) {
      if (errorName(error) !== 'AbortError') {
        console.error(error);
        setConnectMessage({
          tone: 'error',
          lines: [errorName(error) === 'NotAllowedError' ? t.cardPermission : t.cardConnectError],
        });
      }
      return;
    }
    await openCard(root);
  }

  /** Reads the card of an earlier visit again, after the user allows editing it. */
  async function reconnectCard() {
    if (!lastCard) return;
    try {
      if (!(await allowCard(lastCard))) {
        setConnectMessage({ tone: 'error', lines: [t.cardPermission] });
        return;
      }
    } catch (error) {
      console.error(error);
      setConnectMessage({ tone: 'error', lines: [t.cardNotInserted] });
      return;
    }
    await openCard(lastCard, t.cardNotInserted);
  }

  /** Reads the card and shows what it holds as the selection. `failure` is the error text. */
  async function openCard(root: CardDirectory, failure = t.cardConnectError) {
    const run = (cardRun.current += 1);
    setCardTask({ kind: 'reading' });
    setConnectMessage(null);
    setApplyMessage(null);
    setRemoveMessage(null);
    try {
      const catalog = await loadCatalog(PREVIEW_TARGET);
      const session = await openSession(root, [...catalog.effects, ...imported]);
      if (run !== cardRun.current) return;
      const state = session.state;
      const fit = cameraForCard(camera, cardBuilds(state), state.rootNames) ?? camera;
      setCameraKey(fit.key);
      if (state.install !== 'none') keepSelection(new Set(session.effects.keys()));
      if (state.install !== 'none') {
        const ids = new Set(cardModules(state));
        setEnabled(new Set(MODULES.filter((module) => ids.has(module.id)).map((module) => module.key)));
      }
      setCard(session);
      setLastCard(root);
      void saveLastCard(root);
      resetBuild();
      if (fit.support) prefetchPackage(fit.support.targets, fit.id, language);
    } catch (error) {
      console.error(error);
      if (run === cardRun.current) setConnectMessage({ tone: 'error', lines: [failure] });
    } finally {
      if (run === cardRun.current) setCardTask({ kind: 'idle' });
    }
  }

  /** Runs one action of Remove GRKit on the connected card, then reads the card again. */
  async function removeAction(
    task: CardTask,
    action: (session: CardSession) => Promise<void>,
    done: readonly string[],
  ) {
    if (!card) return;
    const session = card;
    const run = cardRun.current;
    setApplyMessage(null);
    setRemoveMessage(null);
    setTaskArea('remove');
    setCardTask(task);
    try {
      await action(session);
      const catalog = await loadCatalog(PREVIEW_TARGET);
      const next = await openSession(session.root, [...catalog.effects, ...imported]);
      if (run !== cardRun.current) return;
      setCard(next);
      setRemoveMessage({ tone: 'ok', lines: done });
    } catch (error) {
      console.error(error);
      if (run === cardRun.current) setRemoveMessage({ tone: 'error', lines: cardErrorLines(error, t, false) });
    } finally {
      if (run === cardRun.current) setCardTask({ kind: 'idle' });
    }
  }

  const uninstallCard = (data: boolean, scriptsOff: boolean) =>
    removeAction(
      { kind: 'finishing' },
      async (session) => {
        await removeGrkit(session.root, data);
        if (scriptsOff) await writeFiles(session.root, await scriptsOffCardFiles(camera.id));
        trackCardUninstall(camera, data, scriptsOff);
      },
      scriptsOff ? [t.uninstallDone, t.scriptsOffNext] : [t.uninstallDone],
    );

  const scriptsOffCard = () =>
    removeAction(
      { kind: 'writing', written: 0, total: 2 },
      async (session) => {
        await writeFiles(session.root, await scriptsOffCardFiles(camera.id));
        trackScriptsOff(camera);
      },
      [t.scriptsOffDone],
    );

  const cleanModeset = () =>
    removeAction(
      { kind: 'finishing' },
      (session) => removeModesetFiles(session.root, session.state),
      [t.modesetCleaned],
    );

  function chooseMode(next: InstallMode) {
    setMode(next);
    saveMode(next);
    setApplyMessage(null);
    setRemoveMessage(null);
  }

  function disconnectCard() {
    cardRun.current += 1;
    setCard(null);
    setLastCard(null);
    void forgetLastCard();
    setConnectMessage(null);
    setApplyMessage(null);
    setRemoveMessage(null);
  }

  /** The changes from the card to the selection: `checked` gives the checked effect IDs. */
  function cardChanges(session: CardSession, checked: readonly string[]): CardChanges {
    const changes = planChanges(
      session.state,
      __GRKIT_VERSION__,
      plan.modules,
      plan.includeEffects ? { before: [...session.effects.keys()], after: checked } : null,
    );
    return session.written && changes.grkit === 'update' ? { ...changes, grkit: 'same' } : changes;
  }

  /** Writes the changes that the user confirmed, then reads the card again. */
  async function applyCard() {
    if (!card || !targets) return;
    const session = card;
    const run = cardRun.current;
    setApplyMessage(null);
    setRemoveMessage(null);
    setTaskArea('apply');
    setCardTask({ kind: 'preparing', received: 0 });
    try {
      const checked = await checkedEffects();
      const effects = plan.includeEffects ? cardPackageEffects(checked) : [];
      const files = await buildCardFiles(targets, camera.id, effects, plan.modules, language, (received) => {
        if (run === cardRun.current) setCardTask({ kind: 'preparing', received });
      });
      const changes = cardChanges(
        session,
        checked.map((effect) => effect.id),
      );
      const library = await libraryChanges(session, files, checked, changes.addEffects, changes.removeEffects);
      await applyChanges(session.root, session.state, files, library, ({ written, total }) => {
        if (run !== cardRun.current) return;
        setCardTask(written < total ? { kind: 'writing', written, total } : { kind: 'finishing' });
      });
      const catalog = await loadCatalog(PREVIEW_TARGET);
      const next = await openSession(session.root, [...catalog.effects, ...imported]);
      if (run !== cardRun.current) return;
      setCard({ ...next, written: true });
      setApplyMessage({
        tone: 'ok',
        // The camera steps after the write: the MENU step for a first install and for SD scripts
        // that were off, then GRK in ADJ.
        lines: [
          t.cardDone,
          t.cardStepInsert,
          ...(changes.grkit === 'install' ? [t.cardDoneFirst] : changes.scriptsOn ? [t.cardEnable] : []),
          t.steps[3]!.join(''),
        ],
      });
      trackCardWrite(
        camera,
        plan,
        plan.includeEffects ? checked.length : 0,
        language,
        changes.grkit === 'same' ? 'change' : changes.grkit,
      );
    } catch (error) {
      console.error(error);
      if (run === cardRun.current) {
        // Only a site file that did not arrive is a download error. Other errors are the card's.
        if (error instanceof TransferError) trackDownloadError(camera, 'card', appBrowser, error);
        setApplyMessage({ tone: 'error', lines: cardErrorLines(error, t, true) });
      }
    } finally {
      if (run === cardRun.current) setCardTask({ kind: 'idle' });
    }
  }

  async function download() {
    if (!targets || plan.visible.length === 0) {
      return;
    }
    buildRun.current += 1;
    const run = buildRun.current;
    setBuild({ state: 'building', received: 0, retrying: false });
    try {
      const effects = await packageEffects();
      const blob = await buildPackage(
        targets,
        camera.id,
        effects,
        plan.modules,
        language,
        (received, retrying) => {
          if (run === buildRun.current) setBuild({ state: 'building', received, retrying });
        },
      );
      if (run === buildRun.current) {
        save(blob, 'GRKit.zip');
        trackDownload(camera, plan, effects.length, language);
        setBuild({ state: 'ready' });
      }
    } catch (error) {
      console.error(error);
      if (run === buildRun.current) {
        trackDownloadError(camera, 'zip', appBrowser, error);
        setBuild({ state: 'failed' });
      }
    }
  }

  const packageLinks = (
    <ul className="package-links">
      <li>
        <a className="button" href={FEEDBACK_FORM}>
          {t.feedback}
        </a>
      </li>
      <li>
        <a className="button" href={DONATE}>
          {t.donate}
        </a>
      </li>
    </ul>
  );

  return (
    <>
      {appBrowser ? (
        <div className="browser-banner" role="note">
          {t.appBrowser}
        </div>
      ) : (
        suggestCardBrowser() && (
          <div className="browser-banner" role="note">
            {t.cardBrowser}
          </div>
        )
      )}
    <div className="page">
      <div className="languages">
        <a className="github" href={REPOSITORY} aria-label="GitHub" title="GitHub">
          <svg viewBox="0 0 16 16" width="22" height="22" aria-hidden="true" focusable="false">
            <path
              fill="currentColor"
              d="M8 0c4.42 0 8 3.58 8 8a8.013 8.013 0 0 1-5.45 7.59c-.4.08-.55-.17-.55-.38 0-.27.01-1.13.01-2.2 0-.75-.25-1.23-.54-1.48 1.78-.2 3.65-.88 3.65-3.95 0-.88-.31-1.59-.82-2.15.08-.2.36-1.02-.08-2.12 0 0-.67-.22-2.2.82-.64-.18-1.32-.27-2-.27-.68 0-1.36.09-2 .27-1.53-1.03-2.2-.82-2.2-.82-.44 1.1-.16 1.92-.08 2.12-.51.56-.82 1.28-.82 2.15 0 3.06 1.86 3.75 3.64 3.95-.23.2-.44.55-.51 1.07-.46.21-1.61.55-2.33-.66-.15-.24-.6-.83-1.23-.82-.67.01-.27.38.01.53.34.19.73.9.82 1.13.16.45.68 1.31 2.69.94 0 .67.01 1.3.01 1.49 0 .21-.15.45-.55.38A7.995 7.995 0 0 1 0 8c0-4.42 3.58-8 8-8Z"
            />
          </svg>
        </a>
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
        <span className="language-select">
          <select
            className="button"
            aria-label={t.language}
            value={language}
            onChange={(event) => chooseLanguage(event.target.value as Language)}
          >
            {LANGUAGES.map((item) => (
              <option key={item.code} value={item.code} lang={item.code}>
                {item.label}
              </option>
            ))}
          </select>
        </span>
      </div>

      <header className="masthead">
        <div className="masthead-title">
          <h1>
            GRK<span className="dot">i</span>t
          </h1>
          <a className="masthead-version" href={RELEASE_URL}>
            v{__GRKIT_VERSION__}
          </a>
        </div>
        <p className="lead">
          {t.lead.map((line) => (
            <span key={line} className="lead-line">
              {line}
            </span>
          ))}
        </p>
      </header>

      <main>
        {canWriteCards() && (
          <StartSection
            strings={t}
            mode={mode}
            onMode={chooseMode}
            session={card}
            cardCamera={cardCamera}
          onCamera={(next) => chooseCamera(next.key)}
          otherCamera={
              !!card && !!support && cardBuilds(card.state).some((build) => !support.targets.includes(build))
            }
            busy={cardTask.kind !== 'idle'}
            reading={cardTask.kind === 'reading'}
            message={connectMessage}
            lastCard={lastCard?.name ?? null}
            onConnect={connectCard}
            onReconnect={reconnectCard}
            onDisconnect={disconnectCard}
          />
        )}

        {!cardCamera && (
          <section className="section" aria-labelledby="camera-label">
            <h2 id="camera-label">{t.camera}</h2>
            <div className="section-body">
              <div role="radiogroup" aria-labelledby="camera-label" className="cameras">
                {CAMERAS.map((item) => (
                  <label key={item.key} className="camera">
                    <input
                      type="radio"
                      name="camera"
                      checked={item.key === camera.key}
                      disabled={!item.support || (!!lockedCameras && !lockedCameras.includes(item))}
                      onChange={() => chooseCamera(item.key)}
                    />
                    <span className="camera-text">
                      <span className="camera-name">{item.name}</span>
                      {item.support && (
                        <span className="camera-firmware">
                          {t.firmware} {item.support.firmware}
                        </span>
                      )}
                      <span className="camera-status">
                        <span className={item.support ? 'badge supported' : 'badge'}>
                          {item.support ? t.supported : t.planned}
                        </span>
                        {item.support?.noEffects && (
                          <span className="camera-firmware">{t.noEffects}</span>
                        )}
                      </span>
                    </span>
                  </label>
                ))}
              </div>
              <p className="note">{t.firmwareNote}</p>
            </div>
          </section>
        )}

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
                    <span className="module-name">
                      {t.moduleNames[module.key]}
                      {active && active.state.install !== 'none' && (
                        <ChangeBadge
                          on={cardModules(active.state).includes(module.id)}
                          checked={plan.modules.includes(module.id)}
                          strings={t}
                        />
                      )}
                    </span>
                    <span className="module-description">
                      {t.modulesText[module.key]}
                    </span>
                    {module.key === 'filmLook' &&
                      enabled.has('filmLook') &&
                      selection?.size === 0 && (
                        <span className="module-warning">{t.filmLookEmpty}</span>
                      )}
                  </span>
                </label>
              ))}
            </div>
          </div>
        </section>

        <section className="section" aria-labelledby="effects-label" ref={prefetchInView}>
          <h2 id="effects-label">{t.effects}</h2>
          {!available.includes('filmLook') ? (
            <p className="status section-body">{t.effectsUnavailable(camera.name)}</p>
          ) : plan.includeEffects ? (
            <ErrorBoundary
              resetKey={String(catalogAttempt)}
              fallback={
                <div className="section-body load-error">
                  <p className="status error">{t.loadError}</p>
                  <button
                    type="button"
                    className="button small"
                    onClick={() => setCatalogAttempt((attempt) => attempt + 1)}
                  >
                    {t.retryLoad}
                  </button>
                </div>
              }
            >
              <Suspense fallback={<p className="status section-body">{t.loading}</p>}>
                <EffectsSection
                  strings={t}
                  language={language}
                  selection={selection}
                  onSelectionChange={changeSelection}
                  imported={imported}
                  onImport={addImported}
                  onRemove={removeImported}
                  cardEffects={active?.extra ?? []}
                  onCard={active && active.state.install !== 'none' ? new Set(active.effects.keys()) : null}
                />
              </Suspense>
            </ErrorBoundary>
          ) : (
            <p className="status section-body">{t.effectsOff}</p>
          )}
        </section>

        <section className="section" aria-labelledby="package-label">
          <h2 id="package-label">{mode === 'card' ? t.install : t.package}</h2>
          {mode === 'card' && !card ? (
            <div className="section-body card">
              <div className="card-actions">
                <button
                  type="button"
                  className="download"
                  disabled={cardTask.kind !== 'idle'}
                  onClick={lastCard ? reconnectCard : connectCard}
                >
                  {lastCard ? t.cardReconnect(lastCard.name) : t.cardConnect}
                </button>
              </div>
            </div>
          ) : mode === 'card' && card ? (
            <div className="section-body">
              <ErrorBoundary resetKey={String(catalogAttempt)} fallback={<p className="status error">{t.loadError}</p>}>
                <Suspense fallback={<p className="status">{t.loading}</p>}>
                  <CardChangesSection
                    strings={t}
                    session={card}
                    selection={selection}
                    imported={imported}
                    changes={cardChanges}
                    modules={MODULES.filter((module) => plan.visible.includes(module.key)).map(
                      (module) => t.moduleNames[module.key],
                    )}
                    canWrite={!!targets && plan.visible.length > 0}
                    busy={cardTask.kind !== 'idle'}
                    task={taskArea === 'apply' ? cardTask : IDLE}
                    message={applyMessage}
                    onApply={applyCard}
                    links={packageLinks}
                  />
                </Suspense>
              </ErrorBoundary>
              {plan.visible.length === 0 && <p className="status error">{t.noModules}</p>}
            </div>
          ) : (
          <div className="package">
              <dl>
                <dt>{t.camera}</dt>
                <dd>{camera.name}</dd>
                <dt>{t.firmware}</dt>
                <dd>{support?.firmware ?? '—'}</dd>
                <dt>{t.effectCount}</dt>
                <dd>
                  {plan.includeEffects ? (
                    <ErrorBoundary resetKey={String(catalogAttempt)} fallback="—">
                      <Suspense fallback="—">
                        <CheckedCount
                          selection={selection}
                          others={[...imported, ...(active?.extra ?? [])].map((effect) => effect.id)}
                        />
                      </Suspense>
                    </ErrorBoundary>
                  ) : (
                    0
                  )}
                </dd>
                <dt>{t.modules}</dt>
                <dd>
                  {plan.visible.length > 0
                    ? MODULES.filter((module) => plan.visible.includes(module.key))
                        .map((module) => t.moduleNames[module.key])
                        .join(t.listSeparator)
                    : '—'}
                </dd>
              </dl>
              <div className="package-action">
                <button
                  type="button"
                  className="download"
                  disabled={!targets || plan.visible.length === 0 || build.state === 'building'}
                  aria-busy={build.state === 'building'}
                  onClick={download}
                  onPointerEnter={prefetch}
                  onFocus={prefetch}
                >
                  {t.download}
                </button>
                <div aria-live="polite">
                  {plan.visible.length === 0 && <p className="status error">{t.noModules}</p>}
                  {build.state === 'building' && (
                    <p className="status">
                      {build.retrying
                        ? t.retrying
                        : build.received > 0
                          ? t.downloading(megabytes(build.received))
                          : t.building}
                    </p>
                  )}
                  {build.state === 'ready' && <p className="status">{t.ready}</p>}
                  {build.state === 'failed' && <p className="status error">{t.buildError}</p>}
                </div>
                {build.state === 'ready' && packageLinks}
              </div>
            </div>
          )}
        </section>

        {mode === 'zip' && (
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
        )}

        {showUninstall && (
          <UninstallSection title={t.remove}>
            <CardUninstall
              strings={t}
              session={card!}
              busy={cardTask.kind !== 'idle'}
              task={taskArea === 'remove' ? cardTask : IDLE}
              message={removeMessage}
              onUninstall={uninstallCard}
              onScriptsOff={scriptsOffCard}
              onCleanModeset={cleanModeset}
            />
          </UninstallSection>
        )}

        {mode === 'zip' && (
          <section className="section" aria-labelledby="update-label">
            <h2 id="update-label">{t.update}</h2>
            <ol className="steps">
              {t.updateSteps.map((parts, index) => (
                <li key={index}>
                  <RichText parts={parts} />
                </li>
              ))}
            </ol>
          </section>
        )}

        {mode === 'zip' && (
          <section className="section" aria-labelledby="remove-label">
            <h2 id="remove-label">{t.remove}</h2>
            <ol className="steps">
              {t.removeSteps.map((parts, index) => (
                <li key={index}>
                  <RichText
                    parts={parts.map((part) =>
                      part === MODESET_TOKEN ? `\`${support?.modeset ?? ''}` : part,
                    )}
                  />
                </li>
              ))}
            </ol>
            <div className="package-action">
              <button
                type="button"
                className="download secondary"
                disabled={!support || scriptsOff === 'building'}
                aria-busy={scriptsOff === 'building'}
                onClick={downloadScriptsOff}
              >
                {t.scriptsOff(camera.name)}
              </button>
              <div aria-live="polite">
                {scriptsOff === 'failed' && <p className="status error">{t.scriptsOffError}</p>}
              </div>
            </div>
          </section>
        )}

        <section className="section" aria-labelledby="faq-label">
          <h2 id="faq-label">{t.faqTitle}</h2>
          <div className="section-body faq">
            {t.faq.map((item) => (
              <details key={item.q}>
                <summary>{item.q}</summary>
                <p>
                  <RichText parts={item.a} />
                </p>
              </details>
            ))}
          </div>
        </section>
      </main>

      <footer className="footer">
        <div className="footer-links">
          <ul>
            <li>
              <a href={guideUrl('RECOVERY', language)}>{t.recovery}</a>
            </li>
            <li>
              <a href={guideUrl('DUMP', language)}>{t.firmwareDump}</a>
            </li>
          </ul>
          <ul>
            <li>
              <a href={FEEDBACK_FORM}>{t.feedback}</a>
            </li>
            <li>
              <a href={DONATE}>{t.donate}</a>
            </li>
          </ul>
          <ul>
            <li>
              <a href={`${REPOSITORY}/blob/main/effects/NOTICE.md`}>{t.licenses}</a>
            </li>
            <li>
              <a href={siteUrl('THIRD_PARTY_LICENSES.txt')}>{t.thirdParty}</a>
            </li>
          </ul>
        </div>
        <div className="footer-meta">
          <p>{t.trademarks}</p>
          <a href={RELEASE_URL}>GRKit {__GRKIT_VERSION__}</a>
        </div>
      </footer>
    </div>
    </>
  );
}

const errorName = (error: unknown) => (error as { name?: string } | null)?.name;

/**
 * The lines of a failed card task. `partial` tells that the task wrote GRKit files, so the card may
 * hold a part of them and should not go into the camera yet.
 */
function cardErrorLines(error: unknown, t: Strings, partial: boolean): string[] {
  switch (errorName(error)) {
    case 'NoModificationAllowedError':
      return [t.cardLocked];
    case 'NotAllowedError':
    case 'SecurityError':
      return [t.cardPermission];
  }
  const reason =
    errorName(error) === 'QuotaExceededError'
      ? t.cardFull
      : errorName(error) === 'NotFoundError'
        ? t.cardGone
        : t.cardWriteError;
  return [reason, partial ? t.cardPartial : t.cardTryAgain];
}

/** The build when there is exactly one, else null. */
const oneBuild = (builds: readonly string[]) => (builds.length === 1 ? builds[0]! : null);

const MODE_KEY = 'grkit.installMode';

/** The storage of this site, or null when the browser blocks it. */
function browserStorage(): Storage | null {
  try {
    return localStorage;
  } catch {
    return null;
  }
}

/** The card mode where the browser can write to a card, unless the user chose the ZIP before. */
function initialMode(): InstallMode {
  if (!canWriteCards()) return 'zip';
  try {
    return localStorage.getItem(MODE_KEY) === 'zip' ? 'zip' : 'card';
  } catch {
    return 'card';
  }
}

function saveMode(mode: InstallMode) {
  try {
    localStorage.setItem(MODE_KEY, mode);
  } catch {
    // The choice is a convenience. Without storage, the page asks again next time.
  }
}

const IDLE: CardTask = { kind: 'idle' };

interface ChangesSectionProps {
  strings: Strings;
  session: CardSession;
  selection: Selection;
  imported: readonly ImportedEffect[];
  changes: (session: CardSession, checked: readonly string[]) => CardChanges;
  modules: readonly string[];
  canWrite: boolean;
  busy: boolean;
  task: CardTask;
  message: CardMessage;
  onApply: () => void;
  links: ReactNode;
}

/** The changes from the card to the selection. Before the first change, it needs the catalog. */
function CardChangesSection({ strings: t, session, selection, imported, changes, ...rest }: ChangesSectionProps) {
  const catalog = use(loadCatalog(PREVIEW_TARGET));
  const checked = resolveSelection(
    selection,
    catalog.effects.map((effect) => effect.id),
    [...imported, ...session.extra].map((effect) => effect.id),
  );
  const effects = [...catalog.effects, ...imported, ...session.extra];
  const effectName = (id: string) => effects.find((effect) => effect.id === id)?.name ?? id;
  const moduleName = (id: string) => {
    const module = MODULES.find((item) => item.id === id);
    return module ? t.moduleNames[module.key] : id;
  };
  return (
    <CardChangesView
      strings={t}
      session={session}
      changes={changes(session, [...checked])}
      version={__GRKIT_VERSION__}
      releaseUrl={RELEASE_URL}
      effectCount={checked.size}
      moduleName={moduleName}
      effectName={effectName}
      {...rest}
    />
  );
}

/** The number of checked effects. Before the first change, it needs the catalog. */
function CheckedCount({ selection, others }: { selection: Selection; others: readonly string[] }) {
  const catalog = use(loadCatalog(PREVIEW_TARGET));
  return resolveSelection(
    selection,
    catalog.effects.map((effect) => effect.id),
    others,
  ).size;
}
