import { useId, useRef, useState, type ReactNode } from 'react';
import { cardStarted, cardVersion, hasChanges, type CardChanges } from '../card';
import type { CardSession } from '../cardSession';
import type { Camera } from '../cameras';
import { sentences, type Strings } from '../i18n';

/** What the card panels are doing. Each task disables the card buttons until it ends. */
export type CardTask =
  | { kind: 'idle' }
  | { kind: 'reading' }
  | { kind: 'preparing'; received: number }
  | { kind: 'writing'; written: number; total: number }
  | { kind: 'finishing' };

export type CardMessage = { tone: 'ok' | 'error'; lines: readonly string[] } | null;

const megabytes = (bytes: number) => (bytes / 1_000_000).toFixed(1);


function taskText(task: CardTask, t: Strings): string | null {
  switch (task.kind) {
    case 'reading':
      return t.cardReading;
    case 'preparing':
      return task.received > 0 ? t.cardDownloading(megabytes(task.received)) : t.cardPreparing;
    case 'writing':
      return t.cardWriting(Math.min(task.written + 1, Math.max(task.total, 1)), task.total);
    case 'finishing':
      return t.cardFinishing;
    default:
      return null;
  }
}

function Status({ task, message, strings: t }: { task: CardTask; message: CardMessage; strings: Strings }) {
  const text = taskText(task, t);
  return (
    <div aria-live="polite" className="card-status">
      {text && <p className="status">{text}</p>}
      {message?.lines.map((line) => (
        <p key={line} className={message.tone === 'error' ? 'status error' : 'status'}>
          {line}
        </p>
      ))}
    </div>
  );
}

/** How the user installs GRKit: write to a connected card, or download GRKit.zip. */
export type InstallMode = 'card' | 'zip';

interface StartProps {
  strings: Strings;
  mode: InstallMode;
  onMode: (mode: InstallMode) => void;
  session: CardSession | null;
  /** The card's GRKit is for another camera than the selected one. */
  otherCamera: boolean;
  /**
   * The camera of the card's GRKit, which the page then shows here instead of the camera section.
   * `options` has two entries when one build runs on both, such as GR IV and GR IV Monochrome.
   */
  cardCamera: { options: readonly Camera[]; selected: Camera; firmware: string } | null;
  onCamera: (camera: Camera) => void;
  /** Any card task runs. */
  busy: boolean;
  reading: boolean;
  message: CardMessage;
  /** The name of the card of an earlier visit, which `onReconnect` opens again, or null. */
  lastCard: string | null;
  onConnect: () => void;
  onReconnect: () => void;
  onDisconnect: () => void;
}

/**
 * The first section: the install mode, and in card mode the connected card. The page shows it only
 * in a browser that can write to a card.
 */
export function StartSection({
  strings: t,
  mode,
  onMode,
  session,
  otherCamera,
  cardCamera,
  onCamera,
  busy,
  reading,
  message,
  lastCard,
  onConnect,
  onReconnect,
  onDisconnect,
}: StartProps) {
  const state = session?.state;
  return (
    <section className="section" aria-labelledby="start-label">
      <h2 id="start-label">{t.card}</h2>
      <div className="section-body card">
        <div role="radiogroup" aria-labelledby="start-label" className="cameras modes">
          <label className="camera">
            <input
              type="radio"
              name="mode"
              checked={mode === 'card'}
              disabled={busy}
              onChange={() => onMode('card')}
            />
            <span className="camera-text">
              <span className="camera-name">{t.modeCard}</span>
              <span className="camera-status">
                <span className="badge supported">{t.recommended}</span>
              </span>
            </span>
          </label>
          <label className="camera">
            <input
              type="radio"
              name="mode"
              checked={mode === 'zip'}
              disabled={busy}
              onChange={() => onMode('zip')}
            />
            <span className="camera-text">
              <span className="camera-name">{t.modeZip}</span>
              <span className="mode-text">{t.modeZipText}</span>
            </span>
          </label>
        </div>

        {mode === 'card' &&
          (!session || !state ? (
            <>
              <div className={lastCard ? 'card-actions pair' : 'card-actions'}>
                {lastCard ? (
                  <>
                    <button type="button" className="download" disabled={busy} onClick={onReconnect}>
                      {t.cardReconnect(lastCard)}
                    </button>
                    <button type="button" className="download outline" disabled={busy} onClick={onConnect}>
                      {t.cardConnectAnother}
                    </button>
                  </>
                ) : (
                  <button type="button" className="download" disabled={busy} onClick={onConnect}>
                    {t.cardConnect}
                  </button>
                )}
              </div>
              <p className="note">
                {lastCard ? sentences(t.cardReconnectNote, t.cardReader) : sentences(t.cardReader, t.cardPickNote)}
              </p>
            </>
          ) : (
            <>
              <dl className="card-facts">
                <dt>{t.cardName}</dt>
                <dd>{state.name || '—'}</dd>
                {cardCamera && (
                  <>
                    <dt>{t.camera}</dt>
                    <dd>
                      {cardCamera.options.length > 1 ? (
                        <span className="card-cameras" role="group" aria-label={t.camera}>
                          {cardCamera.options.map((option) => (
                            <button
                              key={option.key}
                              type="button"
                              className="button small"
                              aria-pressed={option === cardCamera.selected}
                              disabled={busy}
                              onClick={() => onCamera(option)}
                            >
                              {option.name}
                            </button>
                          ))}
                        </span>
                      ) : (
                        cardCamera.selected.name
                      )}
                    </dd>
                    <dt>{t.firmware}</dt>
                    <dd>{cardCamera.firmware}</dd>
                  </>
                )}
                <dt>{t.cardGrkit}</dt>
                <dd>
                  {state.install === 'none'
                    ? t.cardNone
                    : state.install === 'legacy'
                      ? t.cardLegacy
                      : [cardVersion(state), !cardStarted(state) && t.cardNotStarted]
                          .filter(Boolean)
                          .join(' · ')}
                </dd>
              </dl>
              {!state.isCardRoot && <p className="note warning">{t.cardNotRoot}</p>}
              {otherCamera && <p className="note warning">{t.cardOtherCamera}</p>}
              {state.install === 'legacy' && <p className="note">{t.cardLegacyNote}</p>}
              {/* Right after a write, the result of the write shows this step. */}
              {state.install === 'current' && !cardStarted(state) && !session.written && (
                <p className="note">{t.cardDoneFirst}</p>
              )}
              {state.userModules.length > 0 && (
                <p className="note">
                  {t.cardUserModules} {state.userModules.map((name) => name.replace(/\.GRM$/i, '')).join(t.listSeparator)}
                </p>
              )}
              <div className="card-actions">
                <button type="button" className="button" disabled={busy} onClick={onConnect}>
                  {t.cardConnectOther}
                </button>
                <button type="button" className="button" disabled={busy} onClick={onDisconnect}>
                  {t.cardDisconnect}
                </button>
              </div>
            </>
          ))}
        {mode === 'card' && (
          <Status task={reading ? { kind: 'reading' } : { kind: 'idle' }} message={message} strings={t} />
        )}
      </div>
    </section>
  );
}

interface ChangesProps {
  strings: Strings;
  session: CardSession;
  changes: CardChanges;
  version: string;
  /** The names of the modules of the package, in menu order. */
  modules: readonly string[];
  /** The number of effects of the package. */
  effectCount: number;
  /** The release page of `version`. */
  releaseUrl: string;
  /** The name of a module by ID, and of an effect by ID. */
  moduleName: (id: string) => string;
  effectName: (id: string) => string;
  /** True when the selection makes a package. */
  canWrite: boolean;
  /** Any card task runs. */
  busy: boolean;
  /** The task of this section, for its progress text. */
  task: CardTask;
  message: CardMessage;
  onApply: () => void;
  /** The links that the success state shows, as the download has them. */
  links: ReactNode;
  /** The label and the text of the change of the recipe library, or null without one. */
  recipes?: { label: string; text: string } | null;
}

function Change({ label, items, separator }: { label: string; items: readonly string[]; separator: string }) {
  if (items.length === 0) return null;
  return (
    <>
      <dt>{label}</dt>
      <dd>{items.join(separator)}</dd>
    </>
  );
}

/** The last section with a card: the changes, and the button that writes them. */
export function CardChangesView({
  strings: t,
  session,
  changes,
  version,
  releaseUrl,
  modules,
  effectCount,
  moduleName,
  effectName,
  canWrite,
  busy,
  task,
  message,
  onApply,
  links,
  recipes = null,
}: ChangesProps) {
  const fresh = changes.grkit !== 'same';
  const any = hasChanges(changes);
  const state = session.state;
  // After a write, until the selection changes again, the page shows the next steps.
  const doneLines = message?.tone === 'ok' && !any && !busy ? message.lines : null;
  return (
    <div className="card-changes">
      <dl className="card-facts changes">
        <dt>{t.cardGrkit}</dt>
        <dd>
          {changes.grkit === 'install'
            ? t.changeInstall(version)
            : changes.grkit === 'update'
              ? t.changeUpdate(version)
              : t.changeSame(version)}
          {fresh && (
            <>
              {' · '}
              {/* A new tab keeps the connected card and the selection of this page. */}
              <a href={releaseUrl} target="_blank" rel="noreferrer">
                {t.releaseNotes}
              </a>
            </>
          )}
        </dd>
        {state.install === 'none' ? (
          <>
            <Change label={t.changeModules} items={modules} separator={t.listSeparator} />
            <dt>{t.changeEffects}</dt>
            <dd>{effectCount}</dd>
          </>
        ) : (
          <>
            <Change label={t.changeAddModules} items={changes.addModules.map(moduleName)} separator={t.listSeparator} />
            <Change label={t.changeRemoveModules} items={changes.removeModules.map(moduleName)} separator={t.listSeparator} />
            <Change label={t.changeAddEffects} items={changes.addEffects.map(effectName)} separator={t.listSeparator} />
            <Change label={t.changeRemoveEffects} items={changes.removeEffects.map(effectName)} separator={t.listSeparator} />
          </>
        )}
        {recipes && (
          <>
            <dt>{recipes.label}</dt>
            <dd>{recipes.text}</dd>
          </>
        )}
        {changes.scriptsOn && (
          <>
            <dt>{t.changeScripts}</dt>
            <dd>{t.changeScriptsOn}</dd>
          </>
        )}
        {changes.cleanLegacy && (
          <>
            <dt>{t.changeLegacy}</dt>
            <dd>{t.changeRemoves}</dd>
          </>
        )}
      </dl>
      {doneLines ? (
        <div className="card-done" role="status">
          <p className="card-done-title">{t.cardDoneTitle}</p>
          <ol className="steps">
            {doneLines.map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ol>
          {links}
        </div>
      ) : (
        <>
          {!any && <p className="status">{t.noChanges}</p>}
          <div className="package-action">
            <button
              type="button"
              className="download"
              disabled={busy || !any || !canWrite || !state.isCardRoot}
              aria-busy={busy}
              onClick={onApply}
            >
              {changes.grkit === 'install'
                ? t.cardApplyInstall
                : changes.grkit === 'update'
                  ? t.cardApplyUpdate
                  : t.cardApply}
            </button>
            <Status task={task} message={message} strings={t} />
          </div>
        </>
      )}
    </div>
  );
}

interface RemoveProps {
  strings: Strings;
  session: CardSession;
  /** Any card task runs. */
  busy: boolean;
  /** The task of this section, for its progress text. */
  task: CardTask;
  message: CardMessage;
  /**
   * Uninstalls GRKit: with `data` also the settings and the effects, and with `scriptsOff` it then
   * writes the files that turn SD scripts off.
   */
  onUninstall: (data: boolean, scriptsOff: boolean) => void;
  onScriptsOff: () => void;
  onCleanModeset: () => void;
}

/** The Uninstall section with a card. */
export function UninstallSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="section" aria-labelledby="uninstall-label">
      <h2 id="uninstall-label">{title}</h2>
      <div className="section-body">{children}</div>
    </section>
  );
}

/**
 * The card actions of Uninstall GRKit: uninstall GRKit, and turn SD scripts off. Without GRKit on
 * the card, only the cleanup of the files that turned SD scripts off stays.
 */
export function CardUninstall({
  strings: t,
  session,
  busy,
  task,
  message,
  onUninstall,
  onScriptsOff,
  onCleanModeset,
}: RemoveProps) {
  const [data, setData] = useState(false);
  const [scriptsOff, setScriptsOff] = useState(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  const state = session.state;
  const off = state.modeset?.command === 'off';

  function confirm() {
    setData(false);
    setScriptsOff(false);
    dialog.current?.showModal();
  }

  function uninstall() {
    dialog.current?.close();
    onUninstall(data, scriptsOff);
  }

  return (
    <div className="card">
      {state.install !== 'none' && (
        <div className="card-block">
          <p className="card-lead">{t.uninstallText}</p>
          <div className="card-actions">
            <button type="button" className="button" disabled={busy} onClick={confirm}>
              {t.uninstallButton}
            </button>
          </div>
          <dialog ref={dialog} className="dialog confirm" aria-labelledby={titleId}>
            <div className="dialog-head">
              <h3 id={titleId} className="dialog-title">
                {t.uninstallButton}
              </h3>
            </div>
            <div className="dialog-body">
              <p className="card-lead">{t.uninstallConfirm}</p>
              <label className="card-check">
                <input type="checkbox" checked={data} onChange={() => setData(!data)} />
                <span>{t.uninstallData}</span>
              </label>
              <label className="card-check">
                <input type="checkbox" checked={scriptsOff} onChange={() => setScriptsOff(!scriptsOff)} />
                <span>{t.uninstallScriptsOff}</span>
              </label>
              <div className="dialog-actions">
                <button type="button" className="button" onClick={() => dialog.current?.close()}>
                  {t.cancel}
                </button>
                <button type="button" className="button danger" onClick={uninstall}>
                  {t.uninstallConfirmButton}
                </button>
              </div>
            </div>
          </dialog>
        </div>
      )}

      <Status task={task} message={message} strings={t} />

      <div className="card-block">
        {state.install !== 'none' && (
          <>
            <h3 className="card-heading">{t.scriptsOffTitle}</h3>
            <p className="note">{t.scriptsOffText}</p>
            <div className="card-actions">
              <button type="button" className="button" disabled={busy} onClick={onScriptsOff}>
                {t.scriptsOffWrite}
              </button>
            </div>
          </>
        )}
        {off && (
          <div className="card-actions">
            <p className="note">{t.modesetClean}</p>
            <button type="button" className="button small" disabled={busy} onClick={onCleanModeset}>
              {t.modesetCleanButton}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
