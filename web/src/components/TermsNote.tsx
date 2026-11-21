import { sentences } from '../i18n';

/**
 * The line next to a button that writes or downloads GRKit: continuing accepts the terms of use.
 * A `lead` sentence, such as the ready status, comes first on the same line. The link opens a new
 * tab, which keeps the connected card and the selection of this page.
 */
export function TermsNote({
  text,
  href,
  lead,
}: {
  text: readonly [string, string, string];
  href: string;
  lead?: string;
}) {
  const [before, link, after] = text;
  return (
    <p className="note">
      {lead ? sentences(lead, before) : before}
      <a href={href} target="_blank" rel="noreferrer">
        {link}
      </a>
      {after}
    </p>
  );
}
