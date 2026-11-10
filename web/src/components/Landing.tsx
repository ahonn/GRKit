import { useState } from 'react';
import { Handle, HandleRoot, Item, Provider, Root } from 'react-compare-slider/components';
import { useReactCompareSlider } from 'react-compare-slider/hooks';
import { sentences, type Language, type Strings } from '../i18n';
import { fastbootPath, installPath } from '../languagePage';
import { REPOSITORY } from '../links';
import {
  CAMERA_SCREENS,
  cropWidth,
  guideLines,
  LANDING_GUIDES,
  LANDING_HERO,
  LANDING_LOOKS,
  LANDING_PHOTOS,
  LANDING_RATIOS,
  landingImage,
  type LandingGuide,
} from '../landing';
import { EffectIcon } from './EffectIcon';
import { RichText } from './RichText';

type Photo = keyof typeof LANDING_PHOTOS;
type Look = (typeof LANDING_LOOKS)[number];

interface Props {
  strings: Strings;
  language: Language;
  /** Whether this browser writes to a card. Without it (a phone, a tablet, Safari, Firefox), the
      installer only downloads GRKit.zip. */
  canWrite: boolean;
}

/** The home page: what GRKit does, with a way into the installer. */
export function Landing({ strings, language, canWrite }: Props) {
  const t = strings.landing;
  return (
    <>
      <section className="landing-hero" id={LANDING_HERO} aria-label={t.looks}>
        <div className="landing-copy">
          <h1 className="lead">
            {strings.lead.map((line) => (
              <span key={line} className="lead-line">
                {line}
              </span>
            ))}
          </h1>
          <InstallAction strings={strings} language={language} canWrite={canWrite} />
        </div>
        <Looks strings={strings} />
      </section>

      <section className="section" aria-labelledby="landing-framing">
        <h2 id="landing-framing">{strings.moduleNames.framing}</h2>
        <div className="section-body">
          <p className="landing-text">{t.framingText}</p>
          <Framing strings={strings} />
        </div>
      </section>

      <section className="section" aria-labelledby="landing-details">
        <h2 id="landing-details">{t.details}</h2>
        <div className="section-body">
          <div className="landing-details">
            <DateDetail strings={strings} />
            <GlowDetail strings={strings} />
            <GrainDetail strings={strings} />
            <LutDetail strings={strings} language={language} />
          </div>
        </div>
      </section>

      <section className="section" aria-labelledby="landing-camera">
        <h2 id="landing-camera">{t.inCamera}</h2>
        <div className="section-body">
          <div className="landing-screens">
            <figure>
              <img src={CAMERA_SCREENS[language].adj} alt={t.adjCaption} width="720" height="480" loading="lazy" />
              <figcaption>{t.adjCaption}</figcaption>
            </figure>
            <figure>
              <img src={CAMERA_SCREENS[language].menu} alt={t.menuCaption} width="720" height="480" loading="lazy" />
              <figcaption>{t.menuCaption}</figcaption>
            </figure>
          </div>
        </div>
      </section>

      <section className="section" aria-labelledby="landing-faq">
        <h2 id="landing-faq">{t.faqTitle}</h2>
        <div className="section-body faq">
          {t.faq.map((item) => (
            <details key={item.q}>
              <summary>{item.q}</summary>
              <p>
                <RichText parts={item.a} link={REPOSITORY} />
              </p>
            </details>
          ))}
        </div>
      </section>

      {/* The end of the page: what keeps the camera safe, then the way into the installer. */}
      <section className="section" aria-labelledby="landing-safe">
        <h2 id="landing-safe">{t.safe}</h2>
        <div className="section-body">
          <ul className="landing-facts">
            {t.facts.map(([title, text]) => (
              <li key={title}>
                <h3>{title}</h3>
                <p>{text}</p>
              </li>
            ))}
            <li>
              <h3>{t.fastFact[0]}</h3>
              <p>
                {t.fastFact[1]} <a href={fastbootPath(language)}>{t.fastFact[2]}</a>
              </p>
            </li>
          </ul>
          <a className="download landing-install landing-safe-install" href={installPath(language)}>
            {t.install}
          </a>
        </div>
      </section>
    </>
  );
}

/** The author and the license of a photo, as a link to its source page. */
function Credit({ photo, camera = false }: { photo: Photo; camera?: boolean }) {
  const { author, license, source } = LANDING_PHOTOS[photo];
  return (
    <a href={source} target="_blank" rel="noreferrer">
      {author} · {license}
      {camera && ` · ${LANDING_PHOTOS[photo].camera}`}
    </a>
  );
}

/**
 * The main action: the installer page, which writes the card or downloads GRKit.zip, the cameras
 * it supports, and what it does on this device.
 */
function InstallAction({ strings, language, canWrite }: { strings: Strings; language: Language; canWrite: boolean }) {
  const t = strings.landing;
  return (
    <div className="landing-action">
      <a className="download landing-install" href={installPath(language)}>
        {t.install}
      </a>
      <p className="note">{sentences(t.supports, canWrite ? t.installNote : t.installNoteZip)}</p>
    </div>
  );
}

/**
 * The hero: the Standard development against the picked film look, which the handle compares,
 * and the looks to pick from. Only the look is labeled.
 */
function Looks({ strings }: { strings: Strings }) {
  const t = strings.landing;
  const [look, setLook] = useState<Look>(LANDING_LOOKS[0]);
  const comparison = useReactCompareSlider({ clip: 'itemTwo' });
  return (
    <div className="landing-looks">
      <div className="frame">
        <Provider {...comparison}>
          <Root className="image-comparison">
            <Item item="itemOne">
              <img src={landingImage('looks/standard.jpg')} alt={`${strings.samplePhoto} — ${strings.original}`} />
            </Item>
            <Item item="itemTwo" className="comparison-effect">
              <img src={landingImage(`looks/${look.image}.jpg`)} alt={`${strings.samplePhoto} — ${look.name}`} />
            </Item>
            <HandleRoot aria-label={strings.compare}>
              <Handle buttonStyle={{ width: 32, height: 32, gap: '0.2rem' }} />
            </HandleRoot>
          </Root>
        </Provider>
        <span className="tag">{look.name}</span>
      </div>
      <div className="landing-looks-meta">
        <p className="landing-look-name">{look.name}</p>
        <p className="note">
          <Credit photo="kitchen" camera />
        </p>
      </div>
      <div className="landing-strip" role="group" aria-label={t.looks}>
        {LANDING_LOOKS.map((item) => (
          <button
            key={item.name}
            type="button"
            className="landing-film"
            aria-pressed={item === look}
            onClick={() => setLook(item)}
          >
            <EffectIcon effect={{ icon: '', cover: item.cover }} />
            <span>{item.name}</span>
          </button>
        ))}
      </div>
      {/* The other looks load while the visitor reads, so a pick shows at once. */}
      {LANDING_LOOKS.filter((item) => item !== look).map((item) => (
        <link key={item.image} rel="prefetch" as="image" href={landingImage(`looks/${item.image}.jpg`)} />
      ))}
    </div>
  );
}

/** Framing: the 3:2 display with the frame and guides, and the photo that the camera records. */
function Framing({ strings }: { strings: Strings }) {
  const t = strings.landing;
  const [ratio, setRatio] = useState<(typeof LANDING_RATIOS)[number]>(LANDING_RATIOS[0]);
  const [guide, setGuide] = useState<LandingGuide>('thirds');
  const image = landingImage('looks/portra-400.jpg');
  return (
    <div className="landing-framing">
      <div className="landing-choices" role="group" aria-label={strings.moduleNames.framing}>
        {LANDING_RATIOS.map((item) => (
          <button
            key={item.label}
            type="button"
            className="button small landing-ratio"
            aria-pressed={item === ratio}
            onClick={() => setRatio(item)}
          >
            {item.label}
          </button>
        ))}
      </div>
      <div className="landing-framing-views">
        <figure>
          <div className="landing-display">
            <img src={image} alt="" />
            <div className="landing-crop" style={{ width: `${cropWidth(ratio.ratio)}%`, aspectRatio: ratio.ratio }}>
              <svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
                {guideLines(guide, ratio.ratio).map((points, index) => (
                  <polyline key={index} points={points.map(([x, y]) => `${x},${y}`).join(' ')} />
                ))}
              </svg>
            </div>
          </div>
          <figcaption>
            <b>{t.display}</b> 3:2
          </figcaption>
          {/* The guides show on the display, so their choices sit under it. */}
          <div className="landing-choices" role="group" aria-label={strings.moduleNames.guides}>
            <span className="landing-choices-label">{strings.moduleNames.guides}</span>
            {LANDING_GUIDES.map((item, index) => (
              <button
                key={item}
                type="button"
                className="button small"
                aria-pressed={item === guide}
                onClick={() => setGuide(item)}
              >
                {t.guideRows[index]}
              </button>
            ))}
          </div>
        </figure>
        <figure>
          <div className="landing-shot" style={{ aspectRatio: ratio.ratio }}>
            <img src={image} alt={`${strings.samplePhoto} — ${ratio.label}`} />
          </div>
          <figcaption>
            <b>{t.photo}</b> {ratio.label}
          </figcaption>
        </figure>
      </div>
    </div>
  );
}

const DATE_LOOKS = ['classic', 'gr1s', 'gr2'] as const;

function DateDetail({ strings }: { strings: Strings }) {
  const t = strings.landing;
  const [look, setLook] = useState(0);
  return (
    <figure className="landing-detail">
      <div className="landing-zoom">
        <img
          src={landingImage(`date-${DATE_LOOKS[look]}.jpg`)}
          alt={`${strings.moduleNames.dateStamp} — ${t.dateLooks[look]}`}
          width="800"
          height="600"
          loading="lazy"
        />
      </div>
      <figcaption>
        <h3>{strings.moduleNames.dateStamp}</h3>
        <p>{t.dateText}</p>
        <Choices labels={t.dateLooks} value={look} onChange={setLook} label={strings.moduleNames.dateStamp} />
        <p className="note"><Credit photo="tram" /></p>
      </figcaption>
    </figure>
  );
}

function GlowDetail({ strings }: { strings: Strings }) {
  const t = strings.landing;
  // 0 Off, 1 Bloom, 2 Halation, each at the Medium strength.
  const [kind, setKind] = useState(1);
  const image = ['glow-off.jpg', 'bloom.jpg', 'halation.jpg'][kind]!;
  return (
    <figure className="landing-detail">
      <div className="landing-zoom">
        <img
          src={landingImage(image)}
          alt={`${strings.moduleNames.glow} — ${t.glowKinds[kind]}`}
          width="720"
          height="540"
          loading="lazy"
        />
      </div>
      <figcaption>
        <h3>{strings.moduleNames.glow}</h3>
        <p>{t.glowText}</p>
        <Choices labels={t.glowKinds} value={kind} onChange={setKind} label={strings.moduleNames.glow} />
        <p className="note"><Credit photo="street" /></p>
      </figcaption>
    </figure>
  );
}

function GrainDetail({ strings }: { strings: Strings }) {
  const t = strings.landing;
  // 1 Weak to 4 Extra Strong.
  const [level, setLevel] = useState(2);
  return (
    <figure className="landing-detail">
      <div className="landing-zoom">
        <img
          src={landingImage(`grain-${level}.jpg`)}
          alt={`${strings.moduleNames.grain} — ${t.grainLevels[level - 1]}`}
          width="720"
          height="540"
          loading="lazy"
        />
      </div>
      <figcaption>
        <h3>{strings.moduleNames.grain}</h3>
        <p>{t.grainText}</p>
        <Choices
          labels={t.grainLevels}
          value={level - 1}
          onChange={(value) => setLevel(value + 1)}
          label={strings.moduleNames.grain}
        />
        <p className="note"><Credit photo="tram" /></p>
      </figcaption>
    </figure>
  );
}

function Choices({
  labels,
  value,
  onChange,
  label,
}: {
  labels: readonly string[];
  value: number;
  onChange: (value: number) => void;
  label: string;
}) {
  return (
    <div className="landing-choices" role="group" aria-label={label}>
      {labels.map((text, index) => (
        <button
          key={text}
          type="button"
          className="button small"
          aria-pressed={index === value}
          onClick={() => onChange(index)}
        >
          {text}
        </button>
      ))}
    </div>
  );
}

/**
 * LUT import: the sample photo in Standard on the left and with a film look on the right, as an
 * imported LUT changes it, and a link to the import on the install page.
 */
function LutDetail({ strings, language }: { strings: Strings; language: Language }) {
  const [title, text, link] = strings.landing.lut;
  return (
    <figure className="landing-detail">
      <div className="landing-zoom landing-lut">
        <img src={landingImage('looks/standard.jpg')} alt="" width="720" height="480" loading="lazy" />
        <img
          className="landing-lut-after"
          src={landingImage('looks/velvia-100.jpg')}
          alt={`${strings.samplePhoto} — ${title}`}
          width="720"
          height="480"
          loading="lazy"
        />
        <span className="tag landing-lut-before-tag">{strings.original}</span>
        <span className="tag">LUT</span>
      </div>
      <figcaption>
        <h3>{title}</h3>
        <p>{text}</p>
        {/* A link in the place where the other cards have their choices, styled as one of them. */}
        <div className="landing-choices">
          <a className="button small landing-link-button" href={`${installPath(language)}#effects-label`}>
            {link}
          </a>
        </div>
        <p className="note">
          <Credit photo="kitchen" />
        </p>
      </figcaption>
    </figure>
  );
}
