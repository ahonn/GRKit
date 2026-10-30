import type { Camera } from './cameras';
import type { Grade } from './catalog';
import type { Input } from './import/names';
import type { NativeProblem } from './import/native';
import type { Language } from './i18n';
import type { ModuleKey, ModulePlan } from './modules';

type Props = Record<string, string>;

declare global {
  interface Window {
    /** Set only when the build adds Plausible (PLAUSIBLE_HOST). */
    plausible?: (event: string, options: { props: Props }) => void;
  }
}

/** Sends a custom event to Plausible. Without Plausible, it does nothing. */
function track(event: string, props: Props): void {
  window.plausible?.(event, { props });
}

/**
 * The properties of a GRKit download: the camera, the effect count, the page language, and
 * `on` or `off` for each module that the site offers for the camera. A module is `on` only
 * when the package installs it, so Image Control without effects is `off`.
 */
export function downloadProps(
  camera: Camera,
  plan: ModulePlan,
  effectCount: number,
  language: Language,
): Props {
  const props: Props = { camera: camera.key, effects: String(effectCount), language };
  for (const key of camera.support?.modules ?? ([] as readonly ModuleKey[])) {
    props[key] = plan.visible.includes(key) ? 'on' : 'off';
  }
  return props;
}

/** Records a GRKit package that the page saved. */
export function trackDownload(
  camera: Camera,
  plan: ModulePlan,
  effectCount: number,
  language: Language,
): void {
  track('Download', downloadProps(camera, plan, effectCount, language));
}

/** Records a write to a connected card: a first install, an update, or a change of the selection. */
export function trackCardWrite(
  camera: Camera,
  plan: ModulePlan,
  effectCount: number,
  language: Language,
  kind: 'install' | 'update' | 'change',
): void {
  track('Card Write', { ...downloadProps(camera, plan, effectCount, language), kind });
}

/** Records an uninstall from a connected card. */
export function trackCardUninstall(camera: Camera, data: boolean, scriptsOff: boolean): void {
  track('Card Uninstall', { camera: camera.key, data: data ? 'yes' : 'no', scriptsOff: scriptsOff ? 'yes' : 'no' });
}

/** Records a saved package that turns SD scripts off. */
export function trackScriptsOff(camera: Camera): void {
  track('Scripts Off', { camera: camera.key });
}

/** A LUT import of the page: its result, and the grade of an effect that it made. */
export type ImportOutcome = { result: 'ok'; grade: Grade; warnings: number } | { result: 'failed' };

/**
 * The properties of a LUT import: the file format by its extension, the input that the fit used,
 * whether the user chose it or kept the guess, and the outcome. They hold no file name and no
 * LUT data.
 */
export function importProps(
  file: string,
  input: Input,
  chosen: boolean,
  outcome: ImportOutcome,
): Props {
  const extension = file.includes('.') ? file.slice(file.lastIndexOf('.') + 1).toLowerCase() : '';
  const format =
    extension === 'cube' || extension === 'png'
      ? extension
      : extension === 'tif' || extension === 'tiff'
        ? 'tiff'
        : 'other';
  const props: Props = { format, input, choice: chosen ? 'manual' : 'auto', result: outcome.result };
  if (outcome.result === 'ok') {
    props.grade = outcome.grade;
    props.warnings = outcome.warnings > 0 ? 'yes' : 'no';
  }
  return props;
}

/** Records a LUT import that ended, with an effect or with an error. */
export function trackImport(
  file: string,
  input: Input,
  chosen: boolean,
  outcome: ImportOutcome,
): void {
  track('LUT Import', importProps(file, input, chosen, outcome));
}

/** A read of native looks from a firmware file: its GR IV version, or why it failed. */
export type NativeOutcome = { result: 'ok'; version: string } | { result: 'failed'; problem: NativeProblem };

/** The properties of a read of native looks. They hold no file name and no file data. */
export function nativeLooksProps(outcome: NativeOutcome): Props {
  return outcome.result === 'ok'
    ? { result: 'ok', version: outcome.version }
    : { result: 'failed', problem: outcome.problem };
}

/** Records a read of the Cinema looks from a GR IV firmware file that ended. */
export function trackNativeLooks(outcome: NativeOutcome): void {
  track('Cinema Unlock', nativeLooksProps(outcome));
}

/** Records a saved identity HALD image. */
export function trackHaldImage(): void {
  track('HALD Image', {});
}
