import { useCallback } from 'react';
import type { Cover, Effect } from '../catalog';
import { resolveColors, rgbCss } from '../coverPalette';

const SIZE = 40;

/** Paints base64 40 x 40 RGBA pixels. A malformed icon leaves the canvas empty. */
function paint(canvas: HTMLCanvasElement | null, icon: string) {
  const context = canvas?.getContext('2d');
  if (!context) {
    return;
  }
  const pixels = Uint8ClampedArray.from(atob(icon), (character) => character.charCodeAt(0));
  if (pixels.length === SIZE * SIZE * 4) {
    context.putImageData(new ImageData(pixels, SIZE, SIZE), 0, 0);
  }
}

/**
 * Compresses a text horizontally when it is wider than its `data-width`. The width is measured
 * again when the web fonts load, because the first measurement can use a fallback font.
 */
function fit(text: SVGTextElement | null) {
  if (!text) {
    return;
  }
  const measure = () => {
    text.removeAttribute('textLength');
    const width = Number(text.dataset.width);
    if (text.getComputedTextLength() > width) {
      text.setAttribute('textLength', String(width));
      text.setAttribute('lengthAdjust', 'spacingAndGlyphs');
    }
  };
  measure();
  void document.fonts?.ready.then(measure);
}

/** The smallest size of a large name, about the 4-pixel capitals of `artwork.rs`. */
const MIN_LARGE_SIZE = 5.5;
/** The capital height of the cover font as a share of its size. */
const CAP_HEIGHT = 0.7;

/**
 * Makes a large name smaller until it fits its `data-width`, and keeps its capitals centered on
 * the `data-center` row, as `artwork.rs` shrinks a name without a number. At the smallest size, a
 * name that is still too wide compresses horizontally.
 */
function fitLarge(text: SVGTextElement | null) {
  if (!text) {
    return;
  }
  const measure = () => {
    text.removeAttribute('textLength');
    text.style.removeProperty('font-size');
    const width = Number(text.dataset.width);
    const base = parseFloat(getComputedStyle(text).fontSize);
    const length = text.getComputedTextLength();
    const size = length > width ? Math.max(MIN_LARGE_SIZE, (base * width) / length) : base;
    text.style.fontSize = `${size}px`;
    text.setAttribute('y', String(Number(text.dataset.center) + (size * CAP_HEIGHT) / 2));
    if (text.getComputedTextLength() > width) {
      text.setAttribute('textLength', String(width));
      text.setAttribute('lengthAdjust', 'spacingAndGlyphs');
    }
  };
  measure();
  void document.fonts?.ready.then(measure);
}

/**
 * A film cover in the layout of the camera icon. Colors come from `cover.colors`, or packaging
 * hues from the film stock when omitted.
 *
 * Templates: `box` top brand band; `cine` top band + bottom stripes; `stack` bottom brand band.
 */
export function CoverIcon({ cover }: { cover: Cover }) {
  const colors = resolveColors(cover);
  const stripes = cover.template === 'cine';
  const stack = cover.template === 'stack';
  const band = cover.template === 'box' && cover.band;
  if (stack) {
    const footer = 28;
    const center = (3 + footer) / 2;
    return (
      <svg className="effect-icon" viewBox="0 0 40 40" aria-hidden="true">
        <rect x="0" y="3" width="40" height="34" fill={rgbCss(colors.body)} />
        <rect x="0" y={footer} width="40" height={37 - footer} fill={rgbCss(colors.band)} />
        <text
          key={cover.brand}
          ref={fit}
          data-width="34"
          x="20"
          y="34.8"
          className="cover-brand"
          fill={rgbCss(colors.brand)}
        >
          {cover.brand}
        </text>
        {cover.number ? (
          <>
            <text
              key={cover.name}
              ref={fit}
              data-width="38"
              x="20"
              y="13"
              className="cover-name"
              fill={rgbCss(colors.text)}
            >
              {cover.name}
            </text>
            <text
              key={cover.number}
              ref={fit}
              data-width="36"
              x="20"
              y="24.5"
              className="cover-number"
              fill={rgbCss(colors.text)}
            >
              {cover.number}
            </text>
          </>
        ) : (
          <text
            key={`large ${cover.name} ${center}`}
            ref={fitLarge}
            data-width="38"
            data-center={center}
            x="20"
            y={center + 4}
            className="cover-name-large"
            fill={rgbCss(colors.text)}
          >
            {cover.name}
          </text>
        )}
      </svg>
    );
  }
  const headerEnd = 12;
  const brandY = 10.4;
  const baseline = stripes ? 30.2 : band ? 32 : 33;
  const center = (headerEnd + (stripes ? 32 : band ? 34 : 37)) / 2;
  const nameY = 19;
  return (
    <svg className="effect-icon" viewBox="0 0 40 40" aria-hidden="true">
      <rect x="0" y="3" width="40" height="34" fill={rgbCss(colors.body)} />
      <rect x="0" y="3" width="40" height={headerEnd - 3} fill={rgbCss(colors.band)} />
      {band && <rect x="0" y="34" width="40" height="3" fill={rgbCss(colors.accent)} />}
      {stripes &&
        [32, 34, 36].map((y) => (
          <rect key={y} x="0" y={y} width="40" height="1" fill={rgbCss(colors.accent)} />
        ))}
      <text
        key={cover.brand}
        ref={fit}
        data-width="34"
        x="20"
        y={brandY}
        className="cover-brand"
        fill={rgbCss(colors.brand)}
      >
        {cover.brand}
      </text>
      {cover.number ? (
        <>
          <text
            key={cover.name}
            ref={fit}
            data-width="38"
            x="20"
            y={nameY}
            className="cover-name"
            fill={rgbCss(colors.text)}
          >
            {cover.name}
          </text>
          <text
            key={cover.number}
            ref={fit}
            data-width="36"
            x="20"
            y={baseline}
            className="cover-number"
            fill={rgbCss(colors.text)}
          >
            {cover.number}
          </text>
        </>
      ) : (
        <text
          key={`large ${cover.name} ${center}`}
          ref={fitLarge}
          data-width="38"
          data-center={center}
          x="20"
          y={center + 4}
          className="cover-name-large"
          fill={rgbCss(colors.text)}
        >
          {cover.name}
        </text>
      )}
    </svg>
  );
}

/**
 * The ADJ icon of an effect. A film cover draws as SVG. Effects with no cover metadata use the
 * baked 40×40 camera pixels.
 */
export function EffectIcon({ effect }: { effect: Pick<Effect, 'icon' | 'cover'> }) {
  const { icon, cover } = effect;
  const draw = useCallback((canvas: HTMLCanvasElement | null) => paint(canvas, icon), [icon]);
  if (cover) {
    return <CoverIcon cover={cover} />;
  }
  return <canvas ref={draw} className="effect-icon pixels" width={SIZE} height={SIZE} aria-hidden="true" />;
}
