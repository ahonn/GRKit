/* GR III 2.20: the RTOS calls of the fast-boot loader. The file calls are those
   of the file-system task, which the RTOS serves for every task. */
#ifndef GRKIT_FASTBOOT_TARGET_H
#define GRKIT_FASTBOOT_TARGET_H

#define RT_PRINTF 0x536b8240  /* printf to the serial console */
#define RT_LOG 0x536b83c4  /* the RTOS log: (0, 0, 0, 0, format, ...) */
#define RT_BF_OPEN 0x5383cca8  /* BF_Fs_If_Open(name, mode) -> fNo, 0 on failure */
#define RT_BF_READ 0x5383cd14  /* BF_Fs_If_Read(fNo, address, count, &length) -> 0 */
#define RT_BF_CLOSE 0x5383cce4  /* BF_Fs_If_Close(fNo) */
#define RT_FS_STAT_SIZE 0x5383d7ac  /* BF_Fs_If_Stat(name, out) -> 0; out[2] is the size */
#define RT_FS_OPEN 0x53841474  /* open(name, mode, 0) -> fNo >= 0 */
#define RT_FS_CLOSE 0x53841528  /* close(fNo) */
#define RT_FS_READ 0x53841590  /* read(fNo, address, count, &length) -> 0 */
#define RT_FS_WRITE 0x53841604  /* write(fNo, address, count, &length) -> 0 */
#define RT_FS_CREATE 0x53841678  /* create(name, 0) -> fNo >= 0 */
#define RT_FS_REMOVE 0x53841724  /* remove(name) -> 0 */
#define RT_FS_MKDIR 0x538417c0  /* mkdir(name) -> 0 */
#define RT_FS_OPENDIR 0x538418f8  /* opendir(name, &dNo) -> 0 */
#define RT_FS_READDIR 0x5384199c  /* readdir(dNo, entry) -> 0, 1 after the last entry */
#define RT_FS_CLOSEDIR 0x53841d08  /* closedir(dNo) */
#define RT_FS_RENAME 0x53841dec  /* rename(source, destination) -> 0 */

#endif
