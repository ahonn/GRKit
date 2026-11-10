import { useState } from 'react';
import type { Strings } from '../i18n';

/**
 * A button that copies the address of `path` on this site, so a visitor on a phone can open the
 * installer on a computer. Without clipboard access, the button stays as it is.
 */
export function CopyLink({ strings: t, path }: { strings: Strings; path: string }) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(new URL(path, location.origin).href);
      setCopied(true);
    } catch {
      // The browser denied the clipboard. The address bar still holds the address.
    }
  }

  return (
    <button type="button" className="banner-button" onClick={copy} aria-live="polite">
      {copied ? t.linkCopied : t.copyLink}
    </button>
  );
}
