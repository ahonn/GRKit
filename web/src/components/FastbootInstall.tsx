import { LATEST, latestCamera } from '../fastboot';
import { FASTBOOT_SCREENS, type FastbootScreen } from '../fastbootScreens';
import type { Strings } from '../i18n';

/** The step of `installSteps` that copies the file to the card: the file names follow its text. */
const COPY_STEP = 1;

/** The camera screen that follows the text of a step of `installSteps`. */
const STEP_SCREENS: Readonly<Record<number, FastbootScreen>> = { 3: 'confirm', 4: 'updating', 5: 'done' };

function Screen({ screen, strings }: { screen: FastbootScreen; strings: Strings }) {
  const caption = strings.fastboot.screens[screen];
  return (
    <figure className="fastboot-screen">
      <img src={FASTBOOT_SCREENS[screen]} alt={caption} width="720" height="480" loading="lazy" />
      <figcaption>{caption}</figcaption>
    </figure>
  );
}

/**
 * How to install the fast-boot firmware with the camera screens of each step, and what to check
 * when the camera does not show the update screen. The update file name is the most common error.
 */
export function FastbootInstall({ strings }: { strings: Strings }) {
  const t = strings.fastboot;
  // GR IV and GR IV Monochrome share the file, so each file name is one row.
  const files = [...new Set(LATEST.map((item) => item.file))].map((file) => ({
    file,
    cameras: LATEST.filter((item) => item.file === file).map(latestCamera),
  }));

  return (
    <>
      <section className="section fastboot" aria-labelledby="fastboot-install">
        <h2 id="fastboot-install">{t.install}</h2>
        <div className="section-body">
          <p className="fastboot-warning" role="note">
            {t.warning}
          </p>
          <ol className="steps">
            {t.installSteps.map((step, index) => (
              <li key={step}>
                {step}
                {index === COPY_STEP && (
                  <div className="fastboot-names">
                    <p>{t.fileNames}</p>
                    <ul className="fastboot-files">
                      {files.map(({ file, cameras }) => (
                        <li key={file}>
                          <span>{cameras.join(' / ')}</span>
                          <code>{file}</code>
                        </li>
                      ))}
                    </ul>
                    <p>{t.renamed}</p>
                    <p>{t.extensions}</p>
                  </div>
                )}
                {STEP_SCREENS[index] && <Screen screen={STEP_SCREENS[index]} strings={strings} />}
              </li>
            ))}
          </ol>
          <p className="fastboot-text">{t.installDone}</p>
        </div>
      </section>
      <section className="section fastboot" aria-labelledby="fastboot-help">
        <h2 id="fastboot-help">{t.help}</h2>
        <div className="section-body">
          <ul className="steps fastboot-help">
            {t.helpItems.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </div>
      </section>
    </>
  );
}
