# GRKit

仅适用于 RICOH GR IV 和 GR IV HDF 固件 1.11。

GRKit 是实验性软件，不提供任何担保。如果 GRKit 出错，相机可能在启动时停止响应。请在安装前阅读[恢复指南](https://github.com/ahonn/grkit/blob/main/resources/RECOVERY.zh-Hans.md)。安装 GRKit 即表示你接受[使用条款](https://github.com/ahonn/grkit/blob/main/resources/TERMS.zh-Hans.md)。

## 安装

在电脑上的 Chrome 或 Edge 中打开 [grkit.app](https://grkit.app/zh-hans/)，可以通过读卡器直接把 GRKit 写入存储卡，代替第 2 步。

1. 关闭相机。
2. 把 `SD_ROOT` 中的内容（不是这个文件夹本身）复制到存储卡根目录。复制后，根目录中应有 `SCRIPT` 文件夹。
3. 插入存储卡。按住 MENU 按钮开机，等待相机自动关机。
4. 正常开机。

第 3 步会打开相机的 SD 脚本功能：相机显示 Mode Set Command 和 Complete，然后自动关机。如果 SD 脚本功能已经打开，跳过这一步。

![Mode Set Command 画面显示 Script - Enable 和 Complete。](images/modeset.png)

安装后，可以从存储卡根目录删除 `00078560.636` 和 `DEVELOP.MOD`。

## 使用

开机几秒后，拍摄画面顶部显示 **GRKit**，表示加载完成。安装或更新后的第一次开机会久一些。加载完成前拍的照片没有 GRKit 效果。在拍摄模式下按 ADJ 杆，选择第 6 项 GRK。

- **胶片风格：** 高亮一个效果即可在实时显示中预览，按 OK 保留。按 Fn 调整这个效果。不会改变相机的影像控制设置。这些效果是为 GR III 制作的，在 GR IV 上颜色可能不同。
- **画幅：** XPan、2.35:1、16:9 等比例。默认裁切 JPEG 照片，DNG 保持 3:2。按 Fn 可以改为仅辅助构图，或设置遮罩。
- **辅助线：** 只显示在屏幕上，不会出现在照片中。
- **加印日期：** 在照片右下角加印日期。按 Fn 选择日期样式和字体。
- **光晕：** 柔光和胶片光晕，只作用于 JPEG 照片，拍摄画面不预览。
- **颗粒感：** 给照片加颗粒。按 Fn 选择颗粒大小。

全部设置也在 MENU 自定义页（C）最后的 **GRKit** 小节中。每个模块一行，可以启用或关闭模块，在**可选项**中勾选 ADJ 里显示的效果、比例或样式，在 **ADJ 菜单顺序**中调整顺序。关机后设置会保留。

## 更新

在电脑上的 Chrome 或 Edge 中打开 [grkit.app](https://grkit.app/zh-hans/)，可以通过读卡器直接更新存储卡，代替第 2、3 步。

1. 关闭相机。
2. 从存储卡删除 `SCRIPT` 文件夹。
3. 把新安装包 `SD_ROOT` 中的 `SCRIPT` 文件夹复制到存储卡根目录。
4. 正常开机，不需要按住 MENU。

设置和效果保存在 `GRKIT` 文件夹中，更新后仍在。从 GRKit 1.2 更新时，请用 grkit.app 更新，以保留设置和效果。

## 快速启动（可选）

在官方固件上，GRKit 要等相机读完存储卡，开机后十秒左右才加载。装上快速启动固件后，开机后一到两秒就能加载。同一个安装包两种固件都能用。不装快速启动固件，GRKit 的功能也一样。

快速启动固件在 grkit.app 的[快速启动页面](https://grkit.app/zh-hans/fastboot/)制作：选择从 Ricoh 官网下载的官方固件，页面会生成快速启动固件，然后像更新官方固件一样安装。安装需要更新相机固件，更新中断时相机可能无法开机。要恢复，安装同一版本的官方固件即可。不支持 GR IV Monochrome。

## 故障排除

### 按住 MENU 开机时，相机开始更新固件

存储卡根目录中有固件文件。关闭相机，删除根目录中所有 `.bin` 文件，然后重新执行安装第 3 步。

### 相机没有显示 Mode Set Command

- 先按住 MENU 按钮，再开机。
- 把 `SD_ROOT` 中的内容复制到存储卡根目录，不要复制到 `DCIM` 等文件夹中。

### ADJ 中没有 GRK

1. 开机后等待几秒，直到拍摄画面顶部显示 **GRKit**。
2. 确认存储卡根目录中有 `SCRIPT` 文件夹。
3. 如果存储卡中有 `SCRIPT/GRKIT/INSTALL.ERR`，在相机菜单中确认相机显示的是受支持的固件版本。版本受支持时，请提交[设备报告](https://github.com/ahonn/grkit/issues/new?template=device-report.yml)，并附上 `INSTALL.ERR`、`GRKIT/REPORT.TXT` 和 `GRKIT/BOOT.TXT`（如果有）。
4. GRKit 只能在 Ricoh 官方固件和 grkit.app 的快速启动固件上运行。

### 格式化存储卡后 GRKit 不工作

格式化会删除 GRKit 和它的设置。格式化之前，把 `GRKIT` 文件夹复制到电脑。格式化之后，重新复制 `SD_ROOT` 中的内容，再把 `GRKIT` 文件夹复制回存储卡根目录。

## 移除 GRKit

在电脑上的 Chrome 或 Edge 中打开 [grkit.app](https://grkit.app/zh-hans/)，可以通过读卡器直接移除 GRKit，并可同时关闭 SD 脚本功能。

关闭相机，从存储卡删除 `SCRIPT` 和 `GRKIT` 文件夹。

SD 脚本功能会保持打开。要关闭它：

1. 在 https://grkit.app 下载你的相机对应的关闭 SD 脚本的文件。
2. 把 `SD_ROOT` 中的内容复制到存储卡根目录。
3. 插入存储卡。按住 MENU 按钮开机，等待相机自动关机。
4. 从存储卡根目录删除 `00078560.636` 和 `DEVELOP.MOD`。

相机在启动时停止响应时，请参阅[恢复指南](https://github.com/ahonn/grkit/blob/main/resources/RECOVERY.zh-Hans.md)。

GRKit 与 Ricoh 无关。源代码和许可：https://github.com/ahonn/grkit
