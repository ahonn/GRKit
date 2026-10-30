# GRKit

[English](README.md) | 简体中文 | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md) | [한국어](README.ko.md)

[![Tests](https://github.com/ahonn/grkit/actions/workflows/tests.yml/badge.svg)](https://github.com/ahonn/grkit/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

GRKit 给理光 GR III、GR IIIx 和 GR IV 加上胶片滤镜、XPan 等画幅、日期水印和颗粒，从存储卡运行，无需刷机。拔掉存储卡，相机就恢复原样。

<p>
  <img src="https://github.com/ahonn/grkit/raw/main/resources/images/grkit-root.zh-Hans.jpg" alt="ADJ 模式中的 GRKit 页面" width="49%">
  <img src="https://github.com/ahonn/grkit/raw/main/resources/images/grkit-menu.zh-Hans.jpg" alt="MENU 自定义页中的 GRKit 小节" width="49%">
</p>

照片：nagi usano 拍摄的 [Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/)，[CC BY 2.0](https://creativecommons.org/licenses/by/2.0/)。

## 功能

拍摄时按下 ADJ 拨杆，选择第 6 项 GRK。

- **胶片风格**：Portra 400、Velvia 100、Tri-X 400 等，取景时实时预览。也可以在 [grkit.app](https://grkit.app) 导入自己的 `.cube` LUT 或 Lightroom 预设。风格由开源胶片 LUT 拟合而来，效果接近原 LUT，但不完全一样（[来源](effects/NOTICE.md)）。
- **画幅**：XPan、2.35:1、16:9、4:3、5:4、7:6 和 1:1，取景时用遮罩或框线标出范围。
- **辅助线**：三分法、中心、黄金分割和对角线。
- **加印日期**：在照片上印日期，有经典、GR1s 点阵和 GR II 三种样式。
- **颗粒感**：弱、中、强、特强四档，颗粒大小可选自动、小、中、大。
- **配方**：在相机上保存当前风格，或在 [grkit.app](https://grkit.app) 制作配方，一步切换胶片风格或图像控制、颗粒感和白平衡。

开机后 GRKit 需要约 8 到 10 秒加载，GR IV 以及安装或更新后的第一次开机会更久。在此之前相机使用原本的设置。完整设置在 MENU 自定义页最后的 GRKit 小节。想自己写模块，见 [Writing a module](docs/modules.md)（英文）。

## 支持的机型

| 机型 | 真机测试 | 已支持 |
| --- | --- | --- |
| GR III、GR III HDF | 2.10、2.20 | — |
| GR IIIx、GR IIIx HDF | 1.60、1.70 | — |
| GR IV、GR IV HDF | 1.11、1.20 | — |
| GR IV Monochrome | 1.00 | 1.10 |

GR IV Monochrome 不含胶片风格。其他机型和固件版本暂不支持。如果 GRKit 在受支持的相机上无法加载，请看[固件导出指南](resources/DUMP.zh-Hans.md)。

> [!WARNING]
> GRKit 仍是实验性项目，使用风险自负。
> 安装时会打开相机的 SD 脚本设置，之后需要手动关闭。如果 GRKit 出错，相机可能卡在开机阶段，安装前请先读[恢复指南](resources/RECOVERY.zh-Hans.md)。

## 安装

在 [grkit.app](https://grkit.app) 选好相机、模块和效果，然后任选一种方式：

- **直接写入 SD 卡**：在电脑上用 Chrome 或 Edge 操作。以后更新、卸载或增删模块和效果，也都在网站上完成。
- **下载 GRKit.zip**：按包里的 PDF 指南操作（[GR III 和 GR IIIx](resources/INSTALLATION.zh-Hans.md)、[GR IV](resources/INSTALLATION-GR4.zh-Hans.md)）。

只有第一次需要按住 MENU 开机。用 ZIP 更新时，替换卡里的 `SCRIPT` 文件夹即可，设置保存在 `GRKIT` 里。格式化存储卡会删掉 GRKit，格式化前先把 `GRKIT` 备份到电脑。

GRK 没有出现时，先看 [grkit.app](https://grkit.app) 上的常见问题。还是不行的话，提交[设备报告](https://github.com/ahonn/grkit/issues/new?template=device-report.yml)，附上 `GRKIT/REPORT.TXT`、`GRKIT/BOOT.TXT` 和 `SCRIPT/GRKIT/INSTALL.ERR`（如果有）。

## 从源码构建

需要 Rust 1.92 及以上、支持 ARM 的 Clang 和 LLD，以及理光官方固件文件（仓库不包含固件）。

```sh
cargo install --path crates/grkit-cli --locked
grkit effects build --source effects --out build/effects
grkit build --target gr3-2.10 --firmware firmware/fwdc239b.bin --out build/runtime
for module in modules/*/; do grkit module build --dir "$module" --out "build/modules/$(basename "$module").GRM"; done
sh .github/scripts/install-guide-tools.sh build/guide-tools
sh .github/scripts/build-guides.sh grkit build/guide-tools build/guides
grkit package --build build/runtime --effects build/effects $(for m in build/modules/*.GRM; do printf -- '--module %s ' "$m"; done) \
  --guide build/guides/gr3/en.pdf --out dist/GRKit.zip
grkit verify dist/GRKit.zip
```

其他机型、部分模块或其他语言的指南，见 [Build packages](CONTRIBUTING.md#build-packages)（英文）。

## 参与贡献

见 [CONTRIBUTING.md](CONTRIBUTING.md)（英文）。不管能不能用，都欢迎提交设备报告，尤其是还没经过真机测试的固件。遇到问题、有功能想法或疑问，可以填写[反馈表单](https://tally.so/r/Z9jRgB)，也欢迎[请我喝杯咖啡](https://buymeacoffee.com/yuexunjiang)支持这个项目。

## 文档（英文）

- [架构](docs/architecture.md)
- [编写模块](docs/modules.md)和[模块参考](docs/module-reference.md)
- [发布流程](docs/releasing.md)
- [效果](effects/README.md)
- [网站](web/README.md)

## 许可

代码采用 [MIT 许可证](LICENSE)。每个安装包都带有 `LICENSES.md`，列出包内文件的许可。

- 效果采用 CC BY-SA 4.0 或 MIT 许可，详见 [effects/NOTICE.md](effects/NOTICE.md)。
- 菜单和取景画面的文字使用 [Roboto Condensed](https://github.com/googlefonts/roboto-classic) 字体，采用 [SIL Open Font License 1.1](crates/grkit-core/assets/RobotoCondensed-LICENSE.txt)。
- 效果拟合方法移植自 [gr3-film-mods](https://github.com/idontknownihil/gr3-film-mods)，采用 [MIT 许可证](crates/grkit-core/GR3_FILM_MODS_LICENSE.txt)。

RICOH 和 GR 是 Ricoh Company, Ltd. 的商标，胶片名称是各自所有者的商标。GRKit 是独立项目，与理光及任何胶片厂商均无关联。
