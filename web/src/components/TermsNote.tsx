/**
 * The line next to a button that writes or downloads GRKit: continuing accepts the terms of use.
 * The link opens a new tab, which keeps the connected card and the selection of this page.
 */
export function TermsNote({ text, href }: { text: readonly [string, string, string]; href: string }) {
  const [before, link, after] = text;
  return (
    <p className="note">
      {before}
      <a href={href} target="_blank" rel="noreferrer">
        {link}
      </a>
      {after}
    </p>
  );
}
