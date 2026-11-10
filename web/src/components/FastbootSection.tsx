import { useState } from 'react';
import { firmwareName, LATEST, LATEST_NAMES, latestName, makeFastbootFirmware, releaseName } from '../fastboot';
import type { Strings } from '../i18n';
import { FastbootInstall } from './FastbootInstall';
import { TermsNote } from './TermsNote';

type State =
  | { state: 'idle' }
  | { state: 'working' }
  | { state: 'ready'; release: string; fileName: string }
  | { state: 'outdated'; release: string; latest: string }
  | { state: 'unknown' }
  | { state: 'failed' };

/**
 * The optional fast-boot firmware: how it compares with the official firmware, the patch of an
 * official update file in the browser (`grkit_firmware`), how to install it (`FastbootInstall`), and
 * how to remove it.
 */
export function FastbootSection({ strings, terms }: { strings: Strings; terms: string }) {
  const t = strings.fastboot;
  const [state, setState] = useState<State>({ state: 'idle' });

  async function choose(file: File | undefined) {
    if (!file) return;
    setState({ state: 'working' });
    try {
      setState(await makeFastbootFirmware(file));
    } catch (error) {
      console.error(error);
      setState({ state: 'failed' });
    }
  }

  return (
    <>
      <section className="section fastboot" aria-labelledby="fastboot-label">
        <h1 id="fastboot-label">{t.title}</h1>
        <div className="section-body">
          <p className="fastboot-lead">{t.lead}</p>
          <table className="compare">
            <thead>
              <tr>
                <td />
                <th scope="col">{t.compareHead[0]}</th>
                <th scope="col">{t.compareHead[1]}</th>
              </tr>
            </thead>
            <tbody>
              {t.compare.map(([name, official, fast]) => (
                <tr key={name}>
                  <th scope="row">{name}</th>
                  <td data-label={t.compareHead[0]}>{official}</td>
                  <td data-label={t.compareHead[1]}>{fast ?? LATEST_NAMES.join(t.versionSeparator)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
      <section className="section fastboot" id="fastboot" aria-labelledby="fastboot-make">
        <h2 id="fastboot-make">{t.make}</h2>
        <div className="section-body">
          <ol className="steps">
            <li>
              {t.makeSteps[0]}
              <ul className="ricoh-links">
                {LATEST.map((item) => (
                  <li key={item.page}>
                    <a
                      className="button small"
                      href={item.page}
                      target="_blank"
                      rel="noreferrer"
                      aria-label={t.ricohPage(latestName(item))}
                    >
                      {latestName(item)}
                    </a>
                  </li>
                ))}
              </ul>
            </li>
            <li>
              {t.makeSteps[1]}
              <div className="package-action">
                <label htmlFor="fastboot-input" className="download" aria-disabled={state.state === 'working'}>
                  {t.choose}
                </label>
                <input
                  id="fastboot-input"
                  className="visually-hidden"
                  type="file"
                  accept=".zip,.bin"
                  disabled={state.state === 'working'}
                  onChange={(event) => {
                    void choose(event.currentTarget.files?.[0]);
                    event.currentTarget.value = '';
                  }}
                />
                <div className="note-group">
                  <p className="note">{t.note}</p>
                  <TermsNote text={strings.termsNote} href={terms} />
                </div>
                <div aria-live="polite">
                  {state.state === 'working' && (
                    <p className="status fastboot-working">
                      <span className="spinner" aria-hidden="true" />
                      {t.working}
                    </p>
                  )}
                  {state.state === 'ready' && <p className="status">{t.ready(firmwareName(state.release), state.fileName)}</p>}
                  {state.state === 'outdated' && (
                    <p className="status error">{t.outdated(releaseName(state.release), firmwareName(state.latest))}</p>
                  )}
                  {state.state === 'unknown' && <p className="status error">{t.unknown}</p>}
                  {state.state === 'failed' && <p className="status error">{t.error}</p>}
                </div>
              </div>
            </li>
          </ol>
        </div>
      </section>
      <FastbootInstall strings={strings} />
      <section className="section fastboot" aria-labelledby="fastboot-restore">
        <h2 id="fastboot-restore">{t.restore}</h2>
        <div className="section-body">
          <p className="fastboot-text">{t.restoreText}</p>
        </div>
      </section>
    </>
  );
}
