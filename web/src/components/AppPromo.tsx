import { useState, useSyncExternalStore } from 'react';
import { isAppleMobileDevice, type DeviceNavigator } from '../card';
import type { Strings } from '../i18n';
import { LANDING_HERO } from '../landing';

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

/** Calls `change` when the page scrolls or resizes. */
function subscribeScroll(change: () => void): () => void {
  addEventListener('scroll', change, { passive: true });
  addEventListener('resize', change);
  return () => {
    removeEventListener('scroll', change);
    removeEventListener('resize', change);
  };
}

/**
 * Whether the landing hero has scrolled out of view. Before the hero is in the page (the first
 * render), it has not: the bar never shows over the hero, even for a frame.
 */
function pastHero(): boolean {
  const hero = document.getElementById(LANDING_HERO);
  return !!hero && hero.getBoundingClientRect().bottom <= 0;
}

/**
 * A bar at the bottom of the screen that links to GR Snap on the App Store, on an iPhone or an iPad
 * until the user closes it.
 */
export function AppPromo({ strings }: { strings: Strings }) {
  const [shown, setShown] = useState(initiallyShown);
  if (!shown) return null;
  return <PromoBar strings={strings} onClose={() => setShown(false)} />;
}

/**
 * The bar, which waits until the landing hero scrolls away, so it does not compete with the install
 * action or cover the film looks. It follows the scroll only while it can show.
 */
function PromoBar({ strings, onClose }: { strings: Strings; onClose: () => void }) {
  const past = useSyncExternalStore(subscribeScroll, pastHero, () => false);
  if (!past) return null;

  const close = () => {
    onClose();
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
