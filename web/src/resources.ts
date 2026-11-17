/** A video or article about GRKit that the Resources page lists. */
export interface ResourceLink {
  id: string;
  kind: 'video' | 'article';
  href: string;
  /** The title as the publisher shows it. */
  title: string;
  /** The channel or site name. */
  source: string;
}

/**
 * Videos and articles that help people install or understand GRKit. The Resources page lists
 * them; Fast start points at the full tutorial. Keep the list short: only links that help a user.
 */
export const RESOURCE_LINKS = [
  {
    id: 'drnick-full',
    kind: 'video',
    href: 'https://www.youtube.com/watch?v=DBF2UyQg8jY',
    title: 'Ricoh GR IV GRKit FULL Tutorial: Fast Start Install, Bloom, Halation & 70mm Crop',
    source: 'DRNICKDANGER',
  },
  {
    id: 'drnick-intro',
    kind: 'video',
    href: 'https://www.youtube.com/watch?v=447B_-sNfoA',
    title: 'GRKit for Ricoh GR IV! New Film Simulations, Aspect Ratios, Grain & More!',
    source: 'DRNICKDANGER',
  },
  {
    id: 'pentax-rumors',
    kind: 'article',
    href: 'https://pentaxrumors.com/2026/11/07/grkit-offers-film-looks-and-xpan-for-ricoh-gr-cameras-no-firmware-flashing-required/',
    title: 'GRKit offers film looks and XPan for Ricoh GR cameras, no firmware flashing required',
    source: 'Pentax & Ricoh Rumors',
  },
  {
    id: 'diy-photography',
    kind: 'article',
    href: 'https://www.diyphotography.net/grkit-ricoh-gr-film-looks/',
    title: 'GRKit Brings Film Looks and XPan Framing to Ricoh GR Cameras Without Firmware Changes',
    source: 'DIY Photography',
  },
  {
    id: '852-camera',
    kind: 'article',
    href: 'https://852-camera.com/blog/grkit-ricoh-gr-film-looks-xpan-sd-card',
    title: 'GRKit 介紹｜Ricoh GR 底片風格、XPan 比例與 SD 卡新玩法',
    source: '852.CAMERA',
  },
  {
    id: 'abduzeedo',
    kind: 'article',
    href: 'https://abduzeedo.com/grkit-adds-film-looks-and-xpan-crops-ricoh-gr-cameras',
    title: 'GRKit Adds Film Looks and XPan Crops to Ricoh GR Cameras',
    source: 'Abduzeedo',
  },
] as const satisfies readonly ResourceLink[];

export type ResourceId = (typeof RESOURCE_LINKS)[number]['id'];

/** The full install and Fast start walkthrough that Fast start and Resources highlight. */
export const PRIMARY_VIDEO = RESOURCE_LINKS[0];
