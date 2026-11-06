/* GR IV 1.11: the RTOS calls of the fast-boot loader. The file calls are those
   of the file-system task, which the RTOS serves for every task. */
#ifndef GRKIT_FASTBOOT_TARGET_H
#define GRKIT_FASTBOOT_TARGET_H

#define RT_PRINTF 0x538ec6b4  /* printf to the serial console */
#define RT_LOG 0x538ec838  /* the RTOS log: (0, 0, 0, 0, format, ...) */
#define RT_BF_OPEN 0x53a95bf4  /* BF_Fs_If_Open(name, mode) -> fNo, 0 on failure */
#define RT_BF_READ 0x53a95c60  /* BF_Fs_If_Read(fNo, address, count, &length) -> 0 */
#define RT_BF_CLOSE 0x53a95c30  /* BF_Fs_If_Close(fNo) */
#define RT_FS_STAT_SIZE 0x53a966f8  /* BF_Fs_If_Stat(name, out) -> 0; out[2] is the size */
#define RT_FS_OPEN 0x53a9a3c0  /* open(name, mode, 0) -> fNo >= 0 */
#define RT_FS_CLOSE 0x53a9a474  /* close(fNo) */
#define RT_FS_READ 0x53a9a4dc  /* read(fNo, address, count, &length) -> 0 */
#define RT_FS_WRITE 0x53a9a550  /* write(fNo, address, count, &length) -> 0 */
#define RT_FS_CREATE 0x53a9a5c4  /* create(name, 0) -> fNo >= 0 */
#define RT_FS_REMOVE 0x53a9a670  /* remove(name) -> 0 */
#define RT_FS_MKDIR 0x53a9a70c  /* mkdir(name) -> 0 */
#define RT_FS_OPENDIR 0x53a9a844  /* opendir(name, &dNo) -> 0 */
#define RT_FS_READDIR 0x53a9a8e8  /* readdir(dNo, entry) -> 0, 1 after the last entry */
#define RT_FS_CLOSEDIR 0x53a9ac54  /* closedir(dNo) */
#define RT_FS_RENAME 0x53a9ad38  /* rename(source, destination) -> 0 */

#endif
