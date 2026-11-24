/**
 * The first release with GR II Positive. The Site workflow deploys `web/` of main with the
 * packager of the latest release, so the page must not offer the look before that release.
 */
export const GR2_POSITIVE_SINCE = '1.7.0';

/** True when the release `version` is `since` or later. */
export function isReleased(since: string, version: string = __GRKIT_VERSION__): boolean {
  const parts = (value: string) => value.split('.').map(Number);
  const have = parts(version);
  const need = parts(since);
  for (let i = 0; i < need.length; i += 1) {
    const difference = (have[i] ?? 0) - need[i]!;
    if (difference !== 0) return difference > 0;
  }
  return true;
}
