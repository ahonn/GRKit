import { useState } from 'react';
import { isAppleMobileDevice, type DeviceNavigator } from '../card';
import type { Strings } from '../i18n';

/** GR Snap on the App Store. GR Snap is an iOS app by the same author as GRKit. */
const APP_URL = 'https://apps.apple.com/app/gr-snap-ricoh-gr-wireless/id6768040383';
const CLOSED_KEY = 'grkit.appPromo';

/** Shown on an iPhone or an iPad until the user closes it. */
function initiallyShown(): boolean {
  if (typeof navigator === 'undefined' || !isAppleMobileDevice(navigator as DeviceNavigator)) return false;
  try {
    return localStorage.getItem(CLOSED_KEY) !== 'closed';
  } catch {
    return true;
  }
}

/** A bar at the bottom of the screen that links to GR Snap on the App Store. */
export function AppPromo({ strings }: { strings: Strings }) {
  const [shown, setShown] = useState(initiallyShown);
  if (!shown) return null;

  const close = () => {
    setShown(false);
    try {
      localStorage.setItem(CLOSED_KEY, 'closed');
    } catch {
      // Without storage, the bar comes back on the next visit.
    }
  };

  return (
    <>
      {/* Keeps the end of the page clear of the fixed bar. */}
      <div className="app-promo-space" aria-hidden="true" />
      <aside className="app-promo" aria-label="GR Snap">
        <img src="/gr-snap.png" alt="" width="40" height="40" />
        <div className="app-promo-text">
          <strong>GR Snap</strong>
          <span>{strings.appPromo}</span>
          <small>{strings.appPromoMaker}</small>
        </div>
        <a className="button small primary" href={APP_URL}>
          {strings.appPromoOpen}
        </a>
        <button type="button" className="app-promo-close" aria-label={strings.appPromoClose} onClick={close}>
          <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
            <path d="M3 3l10 10M13 3L3 13" stroke="currentColor" strokeWidth="1.5" />
          </svg>
        </button>
      </aside>
    </>
  );
}
