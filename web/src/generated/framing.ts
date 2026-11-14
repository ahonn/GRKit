// Generated from crates/grkit-core/src/aspect/ratios.rs. Do not edit: change the
// table there, then run
// GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated.

/** The Framing ratios in the order of the camera menu: the row, the
 * summary, and the width over the height. */
export const FRAMING_RATIOS = [
  { label: 'XPan 65:24', name: 'XPan', ratio: 65 / 24 },
  { label: '2.35:1', name: '2.35:1', ratio: 47 / 20 },
  { label: '16:9', name: '16:9', ratio: 16 / 9 },
  { label: '4:3', name: '4:3', ratio: 4 / 3 },
  { label: '5:4', name: '5:4', ratio: 5 / 4 },
  { label: '7:6', name: '7:6', ratio: 7 / 6 },
  { label: '1:1', name: '1:1', ratio: 1 / 1 },
  { label: '3:4', name: '3:4', ratio: 3 / 4 },
] as const;
