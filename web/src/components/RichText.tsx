import type { RichText as Parts } from '../i18n';

/** Renders text parts. A part that starts with a backtick is code. */
export function RichText({ parts }: { parts: Parts }) {
  return parts.map((part, index) =>
    part.startsWith('`') ? <code key={index}>{part.slice(1)}</code> : <span key={index}>{part}</span>,
  );
}
