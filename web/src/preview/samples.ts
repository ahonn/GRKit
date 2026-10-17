import type { Language, Strings } from '../i18n';
import { siteUrl } from '../catalog';
import { drawChart } from './chart';
import type { Source } from './render';

export interface Sample extends Source {
  label: (strings: Strings, language: Language) => string;
}

/** A photo under `public/samples/`. Use Standard JPEGs from the camera, so the preview matches. */
export function photoSample(key: string, file: string, label: Record<Language, string>): Sample {
  return {
    key,
    label: (_, language) => label[language],
    load: async () => {
      const response = await fetch(siteUrl(`samples/${file}`));
      if (!response.ok) {
        throw new Error(`Sample request failed: ${response.status}`);
      }
      return response.blob();
    },
  };
}

export const SAMPLES: readonly Sample[] = [
  { key: 'chart', label: (strings) => strings.chart, load: drawChart },
];
