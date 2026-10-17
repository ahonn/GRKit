export interface Camera {
  id: string;
  name: string;
  /**
   * The supported firmware version, the site target that holds its runtime, and the ModeSet file
   * that enables SD scripts. A camera without it is planned and cannot be selected.
   */
  support?: { firmware: string; target: string; modeset: string };
}

export const CAMERAS: readonly Camera[] = [
  {
    id: 'gr3',
    name: 'GR III',
    support: { firmware: '2.10', target: 'gr3-2.10', modeset: '00078350.588' },
  },
  // GR IIIx 1.60 runs the GR III 2.10 code, so it uses the same runtime.
  {
    id: 'gr3x',
    name: 'GR IIIx',
    support: { firmware: '1.60', target: 'gr3-2.10', modeset: '00078490.609' },
  },
  { id: 'gr4', name: 'GR IV' },
];

/** The token in the install steps that stands for the camera's ModeSet file. */
export const MODESET_TOKEN = '`{modeset}';

/** The target whose effects and previews the site shows. */
export const PREVIEW_TARGET = 'gr3-2.10';
