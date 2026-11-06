/* GR IV 1.20: the RTOS calls of the fast-boot loader. The file calls are those
   of the file-system task, which the RTOS serves for every task. */
#ifndef GRKIT_FASTBOOT_TARGET_H
#define GRKIT_FASTBOOT_TARGET_H

#define RT_PRINTF 0x5391427c  /* printf to the serial console */
#define RT_LOG 0x53914400  /* the RTOS log: (0, 0, 0, 0, format, ...) */
#define RT_BF_OPEN 0x53abdf38  /* BF_Fs_If_Open(name, mode) -> fNo, 0 on failure */
#define RT_BF_READ 0x53abdfa4  /* BF_Fs_If_Read(fNo, address, count, &length) -> 0 */
#define RT_BF_CLOSE 0x53abdf74  /* BF_Fs_If_Close(fNo) */
#define RT_FS_STAT_SIZE 0x53abea3c  /* BF_Fs_If_Stat(name, out) -> 0; out[2] is the size */
#define RT_FS_OPEN 0x53ac2704  /* open(name, mode, 0) -> fNo >= 0 */
#define RT_FS_CLOSE 0x53ac27b8  /* close(fNo) */
#define RT_FS_READ 0x53ac2820  /* read(fNo, address, count, &length) -> 0 */
#define RT_FS_WRITE 0x53ac2894  /* write(fNo, address, count, &length) -> 0 */
#define RT_FS_CREATE 0x53ac2908  /* create(name, 0) -> fNo >= 0 */
#define RT_FS_REMOVE 0x53ac29b4  /* remove(name) -> 0 */
#define RT_FS_MKDIR 0x53ac2a50  /* mkdir(name) -> 0 */
#define RT_FS_OPENDIR 0x53ac2b88  /* opendir(name, &dNo) -> 0 */
#define RT_FS_READDIR 0x53ac2c2c  /* readdir(dNo, entry) -> 0, 1 after the last entry */
#define RT_FS_CLOSEDIR 0x53ac2f98  /* closedir(dNo) */
#define RT_FS_RENAME 0x53ac307c  /* rename(source, destination) -> 0 */

#endif
