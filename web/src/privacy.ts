// The privacy policy of the phone app and the site (`/app/privacy/` under each language page).
// The policy is in English on each language page: it is the text that the stores link to.

/** One point of a part: a short name in bold, then the text. */
export interface PrivacyPoint {
  name: string;
  text: string;
}

export interface PrivacyPart {
  id: string;
  title: string;
  /** The paragraphs before the points. */
  text: readonly string[];
  points: readonly PrivacyPoint[];
}

export const PRIVACY_UPDATED = 'November 28, 2026';

export const PRIVACY_INTRO =
  'This policy covers the GRKit phone app for iOS and Android, and the site grkit.app. GRKit has no accounts, no ads, and no tracking. It does not sell or share personal data.';

export const PRIVACY_CONTACT = 'support@grkit.app';

export const PRIVACY_PARTS: readonly PrivacyPart[] = [
  {
    id: 'privacy-app',
    title: 'The phone app',
    text: ['The app collects no personal data. It has no analytics and no crash reporting.'],
    points: [
      {
        name: 'Camera connection.',
        text: 'The app uses Bluetooth to find your camera and turn on its Wi-Fi. It then talks to the camera over the camera’s own Wi-Fi network. On Android 12 and earlier, the system calls this permission “location”. The app does not read your location.',
      },
      {
        name: 'Shutdown picture.',
        text: 'You pick a photo with the system photo picker. The app sees only the photo that you pick. It crops the photo on the phone and sends it to the camera. The photo goes to no other place.',
      },
      {
        name: 'Updates and recipes.',
        text: 'The app downloads GRKit packages, release notes, and recipe presets from grkit.app. These requests carry no personal data. Cloudflare, which hosts grkit.app, receives your IP address with each request, as with any website.',
      },
      {
        name: 'Data on the phone.',
        text: 'The app keeps the cameras that you saved (name, model, and Bluetooth identifier) on the phone. Delete the app to delete them.',
      },
      {
        name: 'Links.',
        text: 'Help opens our feedback board on UserJot. Other links open GitHub, the RICOH support site, and app stores. These sites have their own privacy policies.',
      },
    ],
  },
  {
    id: 'privacy-site',
    title: 'The site',
    text: [],
    points: [
      {
        name: 'Hosting.',
        text: 'Cloudflare hosts grkit.app. On the first visit to the home page, Cloudflare tells the site your country, to choose a language. The site keeps no record of it.',
      },
      {
        name: 'Language cookie.',
        text: 'If you choose a language, the site keeps it in the cookie grkit_language. This is the only cookie.',
      },
      {
        name: 'Analytics.',
        text: 'The site counts visits with Plausible Analytics. Plausible uses no cookies and keeps no personal data. It shows only totals, such as page views and countries.',
      },
      {
        name: 'Package builder.',
        text: 'Your browser builds the GRKit package on your computer. Firmware files and SD card contents do not leave your computer.',
      },
    ],
  },
  {
    id: 'privacy-children',
    title: 'Children',
    text: ['GRKit is not directed at children, and it collects no personal data from anyone.'],
    points: [],
  },
  {
    id: 'privacy-changes',
    title: 'Changes',
    text: ['If this policy changes, we update this page and its date.'],
    points: [],
  },
];
