import type { Effect } from '../catalog';

/** The ADJ icon of an effect: a ring for a filter, a film frame, or a plain tile. */
export function EffectIcon({ effect }: { effect: Effect }) {
  return (
    <span className={`effect-icon ${effect.shape}`} aria-hidden="true">
      {effect.label}
    </span>
  );
}
