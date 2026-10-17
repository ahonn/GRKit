export interface Camera {
  id: string;
  name: string;
  /** The supported firmware version, and the site target that holds its runtime. A camera without it is planned and cannot be selected. */
  support?: { firmware: string; target: string };
}

export const CAMERAS: readonly Camera[] = [
  { id: 'gr3', name: 'GR III', support: { firmware: '2.10', target: 'gr3-2.10' } },
  { id: 'gr3x', name: 'GR IIIx' },
  { id: 'gr4', name: 'GR IV' },
];

/** The target whose effects and previews the site shows. */
export const PREVIEW_TARGET = 'gr3-2.10';
