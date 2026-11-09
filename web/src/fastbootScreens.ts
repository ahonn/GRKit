import confirm from '../../resources/images/fastboot-confirm.jpg';
import done from '../../resources/images/fastboot-done.jpg';
import updating from '../../resources/images/fastboot-updating.jpg';

/** The camera screens of the firmware update in the install guide of the fast-boot page. */
export type FastbootScreen = 'confirm' | 'updating' | 'done';

/** The English camera screens of a GR III update, shown for each camera and page language. */
export const FASTBOOT_SCREENS: Readonly<Record<FastbootScreen, string>> = { confirm, updating, done };
