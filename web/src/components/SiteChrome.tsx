import { siteUrl } from '../catalog';
import { LANGUAGES, type Language, type Strings } from '../i18n';
import { fastbootPath, installPath, LANGUAGE_PATHS, type Page } from '../languagePage';
import { guideUrl, REPOSITORY } from '../links';

/** The UserJot board for feature ideas and votes. A plain link, so the page loads nothing from UserJot. */
export const FEATURE_BOARD = 'https://grkit.userjot.com';
/** The Tally form for problems and questions, with the report files. A plain link, so the page loads nothing from Tally. */
export const SUPPORT_FORM = 'https://tally.so/r/Z9jRgB';
/** The Tally form for photos taken with GRKit, which the site may show with credit. */
export const PHOTO_FORM = 'https://tally.so/r/RWd74Q';
export const DONATE = 'https://buymeacoffee.com/yuexunjiang';
/** The release notes of the GRKit version that the site packages, on the releases page. */
export const RELEASE_URL = `${REPOSITORY}/releases#release-v${__GRKIT_VERSION__}`;

interface MastheadProps {
  strings: Strings;
  language: Language;
  page: Page;
  onLanguage: (language: Language) => void;
}

/**
 * The top of every page, in one row: the name with the version on the left, then the links to
 * the film looks, the installer, and the fast-boot firmware page, GitHub, and the language.
 */
export function Masthead({ strings: t, language, page, onLanguage }: MastheadProps) {
  const install = installPath(language);
  return (
    <header className="masthead">
      <div className="masthead-top">
        <div className="masthead-title">
          {/* The name is not the heading of the page: each page has its own h1. */}
          <a href={LANGUAGE_PATHS[language]} className="masthead-name">
            GRK<span className="dot">i</span>t
          </a>
          <a className="masthead-version" href={RELEASE_URL}>
            v{__GRKIT_VERSION__}
          </a>
        </div>
        <div className="languages">
          <nav className="masthead-nav" aria-label="GRKit">
            <a href={`${install}#effects-label`}>{t.effects}</a>
            <a href={install} aria-current={page === 'install' ? 'page' : undefined}>
              {t.installLink}
            </a>
            <a href={fastbootPath(language)} aria-current={page === 'fastboot' ? 'page' : undefined}>
              {t.fastbootLink}
            </a>
          </nav>
          <a className="github" href={REPOSITORY} aria-label="GitHub" title="GitHub">
            <svg viewBox="0 0 16 16" width="22" height="22" aria-hidden="true" focusable="false">
              <path
                fill="currentColor"
                d="M8 0c4.42 0 8 3.58 8 8a8.013 8.013 0 0 1-5.45 7.59c-.4.08-.55-.17-.55-.38 0-.27.01-1.13.01-2.2 0-.75-.25-1.23-.54-1.48 1.78-.2 3.65-.88 3.65-3.95 0-.88-.31-1.59-.82-2.15.08-.2.36-1.02-.08-2.12 0 0-.67-.22-2.2.82-.64-.18-1.32-.27-2-.27-.68 0-1.36.09-2 .27-1.53-1.03-2.2-.82-2.2-.82-.44 1.1-.16 1.92-.08 2.12-.51.56-.82 1.28-.82 2.15 0 3.06 1.86 3.75 3.64 3.95-.23.2-.44.55-.51 1.07-.46.21-1.61.55-2.33-.66-.15-.24-.6-.83-1.23-.82-.67.01-.27.38.01.53.34.19.73.9.82 1.13.16.45.68 1.31 2.69.94 0 .67.01 1.3.01 1.49 0 .21-.15.45-.55.38A7.995 7.995 0 0 1 0 8c0-4.42 3.58-8 8-8Z"
              />
            </svg>
          </a>
          <span className="language-select">
            <select
              className="button"
              aria-label={t.language}
              value={language}
              onChange={(event) => onLanguage(event.target.value as Language)}
            >
              {LANGUAGES.map((item) => (
                <option key={item.code} value={item.code} lang={item.code}>
                  {item.label}
                </option>
              ))}
            </select>
          </span>
        </div>
      </div>
    </header>
  );
}

/** A group of footer links under a heading. */
interface LinkGroup {
  title: string;
  links: readonly { href: string; label: string }[];
}

/** The links at the end of every page, in groups: GRKit, help, community, and legal. */
export function Footer({ strings: t, language }: { strings: Strings; language: Language }) {
  const install = installPath(language);
  const groups: readonly LinkGroup[] = [
    {
      title: 'GRKit',
      links: [
        { href: install, label: t.installLink },
        { href: `${install}#effects-label`, label: t.effects },
        { href: `${install}#modules-label`, label: t.modulesLink },
        { href: fastbootPath(language), label: t.fastbootLink },
      ],
    },
    {
      title: t.footerHelp,
      links: [
        { href: SUPPORT_FORM, label: t.reportProblem },
        { href: `${install}#faq-label`, label: t.faqTitle },
        { href: guideUrl('RECOVERY', language), label: t.recovery },
        { href: guideUrl('DUMP', language), label: t.firmwareDump },
      ],
    },
    {
      title: t.footerCommunity,
      links: [
        { href: FEATURE_BOARD, label: t.suggestFeature },
        { href: PHOTO_FORM, label: t.sharePhotos },
        { href: REPOSITORY, label: 'GitHub' },
        { href: DONATE, label: t.donate },
      ],
    },
    {
      title: t.footerLegal,
      links: [
        { href: `${REPOSITORY}/blob/main/effects/NOTICE.md`, label: t.licenses },
        { href: siteUrl('THIRD_PARTY_LICENSES.txt'), label: t.thirdParty },
        { href: guideUrl('TERMS', language), label: t.terms },
      ],
    },
  ];
  return (
    <footer className="footer">
      <div className="footer-links">
        {groups.map((group) => (
          <div key={group.title} className="footer-group">
            <p className="footer-heading">{group.title}</p>
            <ul>
              {group.links.map((link) => (
                <li key={link.href}>
                  <a href={link.href}>{link.label}</a>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
      <div className="footer-meta">
        <p>{t.trademarks}</p>
        <a href={RELEASE_URL}>GRKit {__GRKIT_VERSION__}</a>
      </div>
    </footer>
  );
}
