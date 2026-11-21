import { useId, useMemo, useState } from 'react';
import type { Strings } from '../i18n';
import { save } from '../package';
import { makePicture, PICTURE_HEIGHT, PICTURE_NAME, PICTURE_WIDTH, type PictureChoice } from '../shutdownPicture';

interface Props {
  strings: Strings;
  /** The page writes to a card. Else the user copies the file to the card. */
  toCard: boolean;
  /** The file of the connected card, or null. */
  onCard: Uint8Array | null;
  choice: PictureChoice;
  onChoice: (choice: PictureChoice) => void;
  /** The firmware versions of the camera, such as `2.21`, for the version text of the preview. */
  firmware: string;
}

/**
 * The text that the camera draws over the picture: the bar of the shot counts and the firmware
 * version, at their places on the screen of 720 by 480. The preview shows where the picture stays
 * free of them.
 */
function CameraText({ firmware }: { firmware: string }) {
  const version = /\d+\.\d+/.exec(firmware)?.[0];
  return (
    <svg className="shutdown-text" viewBox={`0 0 ${PICTURE_WIDTH} ${PICTURE_HEIGHT}`} aria-hidden="true">
      <rect x="20" y="340" width="680" height="40" fill="#000" fillOpacity="0.45" />
      <text x="360" y="371" textAnchor="middle" fontSize="30" textLength="545" lengthAdjust="spacingAndGlyphs">
        Today : 0000000 &gt; Total: 0000000
      </text>
      {version && (
        <text x="710" y="462" textAnchor="end" fontSize="30" textLength="176" lengthAdjust="spacingAndGlyphs">
          FW Ver.{version}
        </text>
      )}
    </svg>
  );
}

function dataUrl(bytes: Uint8Array): string {
  let text = '';
  for (const byte of bytes) text += String.fromCharCode(byte);
  return `data:image/jpeg;base64,${btoa(text)}`;
}

/**
 * The picture that the camera shows when it turns off (Shutdown Screen): the user selects
 * a picture, and the page shows the file that the camera will show.
 */
export function ShutdownSection({ strings, toCard, onCard, choice, onChoice, firmware }: Props) {
  const t = strings.shutdown;
  const id = useId();
  const [failed, setFailed] = useState(false);
  const [fileKey, setFileKey] = useState(0);
  const shown = choice === 'remove' ? null : (choice ?? onCard);
  const url = useMemo(() => (shown ? dataUrl(shown) : null), [shown]);

  async function choose(file: File | undefined) {
    // A new input, so the same file can be selected again.
    setFileKey((key) => key + 1);
    if (!file) return;
    try {
      onChoice(await makePicture(file));
      setFailed(false);
    } catch (error) {
      console.error(error);
      setFailed(true);
    }
  }

  return (
    <div className="section-body shutdown">
      <p className="note">{t.note}</p>
      {url && (
        <figure className="shutdown-preview">
          <div className="shutdown-screen">
            <img src={url} width={PICTURE_WIDTH} height={PICTURE_HEIGHT} alt={t.alt} />
            <CameraText firmware={firmware} />
          </div>
          <figcaption className="note">{t.textNote}</figcaption>
        </figure>
      )}
      <div className="shutdown-actions">
        <label htmlFor={`${id}-file`} className="button small upload">
          {t.choose}
        </label>
        <input
          key={fileKey}
          id={`${id}-file`}
          className="visually-hidden"
          type="file"
          accept="image/*"
          onChange={(event) => void choose(event.target.files?.[0])}
        />
        {choice instanceof Uint8Array && !toCard && (
          <button
            type="button"
            className="button small"
            onClick={() => save(new Blob([choice as Uint8Array<ArrayBuffer>], { type: 'image/jpeg' }), PICTURE_NAME)}
          >
            {t.download}
          </button>
        )}
        {shown && (
          <button
            type="button"
            className="button small"
            onClick={() => onChoice(choice === null && toCard ? 'remove' : null)}
          >
            {t.remove}
          </button>
        )}
        {choice === 'remove' && (
          <button type="button" className="button small" onClick={() => onChoice(null)}>
            {t.undo}
          </button>
        )}
      </div>
      {failed && <p className="status error">{t.failed}</p>}
      {choice instanceof Uint8Array && !toCard && <p className="note">{t.downloadNote}</p>}
    </div>
  );
}
