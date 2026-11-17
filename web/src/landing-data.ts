// The landing data that the page (landing.ts) and scripts/build-landing.mjs share. It has no
// asset imports, so Node can load it.
import type { Cover } from './catalog';
import { packagingColors } from './coverPalette';

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

const look = (name: string, image: string, cover: Omit<Cover, 'colors'>): LandingLook => ({
  name,
  image,
  cover: {
    ...cover,
    colors: packagingColors(cover.brand, cover.name, cover.template, cover.number ?? ''),
  },
});

export const LANDING_LOOKS = [
  look('Portra 400', 'portra-400', {
    band: true,
    brand: 'Kodak',
    name: 'PORTRA',
    number: '400',
    template: 'box',
  }),
  look('Kodachrome 64', 'kodachrome-64', {
    band: true,
    brand: 'Kodak',
    name: 'KODACHROME',
    number: '64',
    template: 'box',
  }),
  look('Velvia 100', 'velvia-100', {
    band: true,
    brand: 'FUJIFILM',
    name: 'VELVIA',
    number: '100',
    template: 'box',
  }),
  look('Eterna 500T', 'eterna-500t', {
    brand: 'FUJIFILM',
    name: '8573',
    number: '500T',
    template: 'cine',
  }),
  look('Tri-X 400', 'tri-x-400', {
    band: true,
    brand: 'Kodak',
    name: 'TRI-X',
    number: '400',
    template: 'box',
  }),
] as const satisfies readonly LandingLook[];

/** The film looks that each package can hold (effects/ in the repository), so the strip can say
    how many more there are than it shows. landing.test.ts checks it against effects/. */
export const BUILT_IN_LOOKS = 30;

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
  takeaway: {
    author: 'Tony Barrett',
    license: 'CC BY-SA 4.0',
    camera: 'RICOH GR III',
    source: 'https://discuss.pixls.us/t/40918',
  },
  dunes: {
    author: 'raw.pixls.us',
    license: 'CC0 1.0',
    camera: 'RICOH GR III',
    source: 'https://raw.pixls.us/',
  },
  corner: {
    author: 'raw.pixls.us',
    license: 'CC0 1.0',
    camera: 'RICOH GR IIIx',
    source: 'https://raw.pixls.us/',
  },
  platform: {
    author: 'Tim Wood',
    license: 'CC BY-SA 4.0',
    camera: 'FUJIFILM X100F',
    source: 'https://discuss.pixls.us/t/50453',
  },
  alley: {
    author: 'sbij',
    license: 'CC BY-SA 4.0',
    camera: 'Sony RX100 III',
    source: 'https://discuss.pixls.us/t/34269',
  },
  cat: {
    author: 'Yannic Meyer',
    license: 'CC BY-SA 4.0',
    camera: 'Sony A7 III',
    source: 'https://discuss.pixls.us/t/57758',
  },
  azaleas: {
    author: 'Bao',
    license: 'CC BY-SA 4.0',
    camera: 'Sony A6000',
    source: 'https://discuss.pixls.us/t/12686',
  },
  coast: {
    author: 'Nacho Rodríguez',
    license: 'CC BY-SA 4.0',
    camera: 'Sony A6000',
    source: 'https://discuss.pixls.us/t/27097',
  },
  havana: {
    author: 'Franklin Aguilar Matos',
    license: 'CC BY-SA 4.0',
    camera: 'FUJIFILM X-H1',
    source: 'https://discuss.pixls.us/t/39870',
  },
} as const;
