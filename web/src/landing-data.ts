// The landing data that the page (landing.ts) and scripts/build-landing.mjs share. It has no
// asset imports, so Node can load it.
import type { Cover } from './catalog';

/**
 * The film looks of the landing hero: the catalog name, the image that scripts/build-landing.mjs
 * renders on the Kitchen Grizzly photo, and the cover of the catalog effect, so the hero draws its
 * icons without the catalog.
 */
export interface LandingLook {
  name: string;
  image: string;
  cover: Cover;
}

export const LANDING_LOOKS = [
  { name: 'Portra 400', image: 'portra-400', cover: { band: true, brand: 'Kodak', name: 'PORTRA', number: '400', template: 'box' } },
  { name: 'Kodachrome 64', image: 'kodachrome-64', cover: { band: true, brand: 'Kodak', name: 'KODACHROME', number: '64', template: 'box' } },
  { name: 'Velvia 100', image: 'velvia-100', cover: { band: true, brand: 'FUJIFILM', name: 'VELVIA', number: '100', template: 'box' } },
  { name: 'Eterna 500T', image: 'eterna-500t', cover: { brand: 'FUJIFILM', name: '8573', number: '500T', template: 'cine' } },
  { name: 'Tri-X 400', image: 'tri-x-400', cover: { band: true, brand: 'Kodak', name: 'TRI-X', number: '400', template: 'box' } },
] as const satisfies readonly LandingLook[];

/** The photos of the landing page, credited where they show, with a link to their source page. */
export const LANDING_PHOTOS = {
  kitchen: {
    author: 'MIKI Yoshihito',
    license: 'CC BY 2.0',
    camera: 'RICOH GR IIIx',
    source: 'https://www.flickr.com/photos/mujitra/53311001084/',
  },
  tram: {
    author: 'MIKI Yoshihito',
    license: 'CC BY 2.0',
    camera: 'RICOH GR IIIx',
    source: 'https://www.flickr.com/photos/mujitra/53731862585/',
  },
  street: {
    author: 'Jules Verne Times Two',
    license: 'CC BY-SA 4.0',
    camera: 'RICOH GR III',
    source: 'https://commons.wikimedia.org/wiki/File:People_dining_outside,_Rue_des_Marronniers,_Lyon,_France_julesvernex2.jpg',
  },
} as const;
