import type { RichText as Parts } from '../i18n';

/**
 * Renders text parts. A part that starts with a backtick is code, and a part that starts with `[`
 * links to `link`.
 */
export function RichText({ parts, link }: { parts: Parts; link?: string }) {
  return parts.map((part, index) =>
    part.startsWith('`') ? (
      <code key={index}>{part.slice(1)}</code>
    ) : part.startsWith('[') && link ? (
      <a key={index} href={link}>
        {part.slice(1)}
      </a>
    ) : (
      <span key={index}>{part.startsWith('[') ? part.slice(1) : part}</span>
    ),
  );
}
