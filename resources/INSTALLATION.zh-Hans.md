# GRKit

仅适用于 RICOH GR III 和 GR III HDF 固件 2.10。

## 安装

1. 关闭相机。
2. 把 `SD_ROOT` 中的内容复制到存储卡根目录。
3. 插入存储卡。按住 MENU 按钮开机。等待相机自动关机。
4. 正常开机。

第 3 步会打开相机的 SD 脚本功能。相机先显示 Mode Set Command，然后显示 Complete，之后自动关机。

![Mode Set Command 画面显示 Script - Enable 和 Complete。](images/modeset.png)

如果 SD 脚本功能已经打开，跳过第 3 步。
安装完成后，你可以从存储卡根目录删除 `00078350.588` 和 `DEVELOP.MOD`。这一步是可选的。

## 更新

1. 关闭相机。
2. 从存储卡删除 `SCRIPT` 文件夹。
3. 把新安装包 `SD_ROOT` 中的 `SCRIPT` 文件夹复制到存储卡根目录。
4. 正常开机，不需要按住 MENU。

GRKit 把设置、效果和你加入的模块保存在 `GRKIT` 文件夹中，所以更新后它们仍在。

删除 `SCRIPT` 也会删除 GRKit 1.2 的设置和效果。要保留它们，请在电脑上的 Chrome 或 Edge 中打开 [grkit.app](https://grkit.app)，通过读卡器连接存储卡后更新。

## 使用

在拍摄模式下按下 ADJ 杆进入 ADJ 模式，然后选择第 6 个功能 GRK。GRKit 在开机几秒后才加载，所以 GRK 可能稍后才出现。加载完成后，拍摄画面顶部会显示 **GRKit** 几秒。这个提示需要画幅或辅助线模块。

![GRKit 页面列出模块，并显示当前效果。](images/grkit-root.jpg)

照片：nagi usano 拍摄的 [Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/)，[CC BY 2.0](https://creativecommons.org/licenses/by/2.0/)。

- **胶片风格（Film Look）：** 高亮一个效果，即可在实时显示中预览。按 OK 按钮保留这个效果。按 Fn 按钮调整这个效果的 [标准] 参数。在 GRK 菜单的胶片风格一行按 Fn 按钮，可以调整当前选择的效果。每个效果单独保存自己的调整，不会改变相机的影像控制设置。
- **画幅（Framing）：** 选择 XPan、2.35:1、16:9、4:3、5:4、7:6 或 1:1。选择任何画幅时，相机的 [长宽比] 都会设为 3:2，显示屏上照片以外的区域会变暗。在相机菜单中把 [长宽比] 设为其他值时，画幅暂时不起作用。DNG 文件保留完整的传感器图像，并记录裁切范围。应用这个裁切范围的 RAW 软件只显示照片区域。
- **辅助线（Guides）：** 选择一种辅助线样式。按 Fn 按钮设置线条颜色、不透明度，以及 DISP 隐藏拍摄信息时是否显示辅助线。辅助线只显示在显示屏上，不会出现在照片中。
- **加印日期（Date Imprint）：** 选择一种日期样式。按 Fn 按钮设置 **日期样式（Date Style）** 和数字的 **日期字体（Date Font）**：经典、GR1s 或 GR II。照片右下角会加印日期。开启期间，显示全部拍摄信息时显示屏上会显示 DATE。
- **颗粒感（Grain）：** 选择弱、中、强或特强。按 Fn 设置 **大小**：自动（跟随 ISO）、小、中或大。颗粒感只加在照片上，不影响影片。

按 MENU 按钮返回上一级。关机后 GRKit 会保留你的设定。

GRKit 的全部设置也在 MENU 自定义页（C）最后的 **GRKit** 小节中：

- 每个模块一行。第一项设置**启用**立即开启或关闭这个模块。关闭的模块不在 ADJ 中显示；而模块自己的“关闭”值（例如画幅中的“关闭”）只关闭效果，模块仍在 ADJ 中。后面是它的其他设置。胶片风格中有**胶卷**，用来选择效果，还有当前效果的**效果参数**。
- 胶片风格、画幅、辅助线和加印日期的设置中，**启用**之后有**可选项**一行，列出效果、比例、辅助线样式或日期样式。每一项前有一个复选框，在一项上按 OK 勾选或取消勾选。ADJ 只显示开启的项。
- **ADJ 菜单顺序**：设置模块在 GRK 中的顺序。选择一个模块，再选择它的新位置。新的顺序立即生效。
- **关于 GRKit**：显示 GRKit 的版本和项目地址。

`GRKIT/REPORT.TXT` 中有 GRKit 的版本号。

GRKit 把效果保存在 `GRKIT/EFFECTS` 文件夹中。新的安装包会加入它的效果，并替换这些效果的旧版本，但不会移除其他效果。要加入效果，把它的文件复制到 `GRKIT/EFFECTS`。要加入另一个安装包的效果，把那个安装包 `SD_ROOT/SCRIPT/GRKIT/EFFECTS` 中的文件复制到 `GRKIT/EFFECTS`。不要把效果复制到 `SCRIPT/GRKIT/EFFECTS`：GRKit 只在安装或更新后第一次开机时读取这个文件夹。要移除一个效果，先关闭相机，再删除它的文件。只有当之后的安装包中这个效果的文件有变化时，它才会被重新加入。要加入模块，把它的文件复制到 `GRKIT/MODULES`。MENU GRKit 小节中胶片风格的**可选项**列出每个效果文件，并立即开启或关闭它在胶片风格中的显示。GRKit 最多安装 40 个效果，优先安装开启的效果。如果开启一个没有安装的效果，它在下次开机后出现。`GRKIT/REPORT.TXT` 列出上次开机时的模块和效果，以及模块没有加载的原因。

## 故障排除

### 按住 MENU 按钮开机时，相机开始更新固件

存储卡根目录中有固件更新文件。关闭相机。从存储卡根目录删除所有 `.bin` 文件。然后重新执行第 3 步。

### 相机没有显示 Mode Set Command

- 先按住 MENU 按钮，再开机。如果开机后才按 MENU 按钮，SD 脚本功能不会打开。
- 把 `SD_ROOT` 中的内容复制到存储卡根目录，不要复制到 `DCIM` 或其他文件夹中。

### ADJ 模式中没有 GRK

1. 开机后等待几秒。GRKit 在相机启动后才加载。加载完成后，拍摄画面顶部会显示 **GRKit** 几秒。
2. 在 ADJ 模式中找 GRK。GRKit 的设置也在 MENU 自定义页的 GRKit 小节中。
3. 如果第 3 步中相机没有显示 Mode Set Command，重新执行第 3 步。
4. 确认存储卡根目录中有 `SCRIPT` 文件夹。复制 `SD_ROOT` 中的全部内容，包括 `SCRIPT`。
5. 如果存储卡中有 `SCRIPT/GRKIT/INSTALL.ERR`，表示 GRKit 没有安装成功。确认包与相机的固件版本对应。在相机菜单中可以查看固件版本。如果版本正确，请提交[设备报告](https://github.com/ahonn/grkit/issues/new?template=device-report.yml)，并附上 `INSTALL.ERR`、`GRKIT/REPORT.TXT` 和 `GRKIT/BOOT.TXT` 中的全部内容。
6. GRKit 只能在 Ricoh 官方固件上运行，不能在修改过的固件上运行。

### 格式化存储卡后 GRKit 不工作

格式化存储卡会删除 `SCRIPT` 和 `GRKIT` 文件夹，无论在相机上还是在电脑上格式化。GRKit、GRKit 的设置和效果都会被删除。

1. 格式化之前，把 `GRKIT` 文件夹复制到电脑。
2. 格式化之后，再次把 `SD_ROOT` 中的内容复制到存储卡根目录。
3. 把备份的 `GRKIT` 文件夹复制回存储卡根目录。

### GRKit 以安全模式启动

在安全模式下，GRKit 不加载任何模块，`GRKIT/REPORT.TXT` 会说明原因。`GRKIT` 文件夹中有名为 `SAFE` 或 `SAFE.TXT` 的文件时，GRKit 进入安全模式。删除这个文件，即可重新加载模块。

如果加入一个模块后相机停止响应，在 MENU 自定义页的 GRKit 小节中把这个模块的**启用**设为关闭。如果相机在打开 MENU 之前就停止响应，取出存储卡，在电脑上从 `GRKIT/MODULES` 删除这个模块，或在 `GRKIT` 文件夹中放一个名为 `SAFE` 或 `SAFE.TXT` 的空文件。

## 移除或恢复

要移除 GRKit，先关闭相机。然后从存储卡删除 `SCRIPT` 和 `GRKIT` 文件夹。`GRKIT` 保存 GRKit 的设置。

SD 脚本设置会保持打开。要关闭它：

1. 在 https://grkit.app 下载你的相机对应的关闭 SD 脚本的文件。按钮在安装步骤下方。
2. 把 `SD_ROOT` 中的内容复制到存储卡根目录。
3. 插入存储卡。按住 MENU 按钮开机，等待相机自动关机。
4. 从存储卡根目录删除 `00078350.588` 和 `DEVELOP.MOD`。

如果相机在启动时停止响应，取出电池和存储卡。然后不插存储卡开机。如果仍然无法启动，请参阅[恢复指南](https://github.com/ahonn/grkit/blob/main/resources/RECOVERY.zh-Hans.md)。

GRKit 与 Ricoh 无关。源代码和许可：https://github.com/ahonn/grkit
