import { useState, useSyncExternalStore } from 'react';
import { isAndroidDevice, isAppleMobileDevice, type DeviceNavigator } from '../card';
import type { Language, Strings } from '../i18n';
import { LANDING_HERO } from '../landing';
import { grSnapUrl } from '../links';

/** GR Snap on the App Store. GR Snap is an iOS app by the same author as GRKit. */
const APP_URL = 'https://apps.apple.com/app/gr-snap-ricoh-gr-wireless/id6768040383';

/**
 * The device the bar is for: an iPhone or an iPad gets the App Store, an Android device the GR Snap
 * site, where GR Snap looks for Android testers. Each has its own closed state.
 */
type Platform = 'ios' | 'android';
const CLOSED_KEYS: Readonly<Record<Platform, string>> = {
  ios: 'grkit.appPromo',
  android: 'grkit.appPromoAndroid',
};

/** The platform of this device while the user has not closed its bar, or null. */
function initialPlatform(): Platform | null {
  if (typeof navigator === 'undefined') return null;
  const device = navigator as DeviceNavigator;
  const platform = isAppleMobileDevice(device) ? 'ios' : isAndroidDevice(device) ? 'android' : null;
  if (!platform) return null;
  try {
    return localStorage.getItem(CLOSED_KEYS[platform]) === 'closed' ? null : platform;
  } catch {
    return platform;
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
 * A bar at the bottom of the screen that links to GR Snap, on an iPhone, an iPad or an Android
 * device until the user closes it.
 */
export function AppPromo({ strings, language }: { strings: Strings; language: Language }) {
  const [platform, setPlatform] = useState(initialPlatform);
  if (!platform) return null;
  return <PromoBar strings={strings} language={language} platform={platform} onClose={() => setPlatform(null)} />;
}

/**
 * The bar, which waits until the landing hero scrolls away, so it does not compete with the install
 * action or cover the film looks. It follows the scroll only while it can show.
 */
function PromoBar({
  strings,
  language,
  platform,
  onClose,
}: {
  strings: Strings;
  language: Language;
  platform: Platform;
  onClose: () => void;
}) {
  const past = useSyncExternalStore(subscribeScroll, pastHero, () => false);
  if (!past) return null;

  const close = () => {
    onClose();
    try {
      localStorage.setItem(CLOSED_KEYS[platform], 'closed');
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
          <strong>
            GR Snap
            {platform === 'android' && <em className="app-promo-tag">{strings.appPromoBeta}</em>}
          </strong>
          <span>{strings.appPromo}</span>
          <small>{strings.appPromoMaker}</small>
        </div>
        <a className="button small primary" href={platform === 'ios' ? APP_URL : grSnapUrl(language, 'android-bar')}>
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
