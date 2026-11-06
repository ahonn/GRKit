/** The line that points a camera with fast-boot releases to the fast-boot firmware page. */
export function FastbootHint({ text, href }: { text: readonly [string, string, string]; href: string }) {
  const [before, link, after] = text;
  return (
    <p className="note">
      {before}
      <a href={href}>{link}</a>
      {after}
    </p>
  );
}
