import type { ModuleKey } from './modules';

export interface Camera {
  id: string;
  name: string;
  /**
   * The supported firmware version, the site target that holds its runtime, and the ModeSet file
   * that enables SD scripts. A camera without it is planned and cannot be selected.
   */
  support?: {
    firmware: string;
    target: string;
    modeset: string;
    /** True until the runtime has been tested on a camera. */
    experimental: boolean;
    /** The modules that the runtime has, in menu order (`camera.rs`). */
    modules: readonly ModuleKey[];
    /** Framing offers only XPan, as a crop of native 16:9. */
    xpanOnly?: boolean;
  };
}

const ALL: readonly ModuleKey[] = ['imageControl', 'framing', 'guides', 'dateStamp', 'grain'];

export const CAMERAS: readonly Camera[] = [
  {
    id: 'gr3',
    name: 'GR III',
    support: {
      firmware: '2.10',
      target: 'gr3-2.10',
      modeset: '00078350.588',
      experimental: false,
      modules: ALL,
    },
  },
  // GR IIIx 1.60 runs the GR III 2.10 code, so it uses the same runtime.
  {
    id: 'gr3x',
    name: 'GR IIIx',
    support: {
      firmware: '1.60',
      target: 'gr3-2.10',
      modeset: '00078490.609',
      experimental: false,
      modules: ALL,
    },
  },
  // GR IV and GR IV HDF run the same firmware.
  {
    id: 'gr4',
    name: 'GR IV / GR IV HDF',
    support: {
      firmware: '1.11',
      target: 'gr4-1.11',
      modeset: '00078560.636',
      experimental: true,
      modules: ALL,
      xpanOnly: true,
    },
  },
];

/** The token in the install steps that stands for the camera's ModeSet file. */
export const MODESET_TOKEN = '`{modeset}';

/** The target whose effects and previews the site shows. */
export const PREVIEW_TARGET = 'gr3-2.10';
