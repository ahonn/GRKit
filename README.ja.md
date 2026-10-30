# GRKit

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | 日本語 | [한국어](README.ko.md)

[![Tests](https://github.com/ahonn/grkit/actions/workflows/tests.yml/badge.svg)](https://github.com/ahonn/grkit/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

GRKit は、RICOH GR III・GR IIIx・GR IV に、フィルム風の色、XPan などの比率、日付写し込み、粒状感を追加するツールです。SD カードから動作し、ファームウェアは書き換えません。カードを抜けば、カメラは元の状態に戻ります。

<p>
  <img src="https://github.com/ahonn/grkit/raw/main/resources/images/grkit-root.ja.jpg" alt="ADJ モードの GRKit ページ" width="49%">
  <img src="https://github.com/ahonn/grkit/raw/main/resources/images/grkit-menu.ja.jpg" alt="MENU のカスタマイズタブにある GRKit" width="49%">
</p>

写真：nagi usano「[Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/)」、[CC BY 2.0](https://creativecommons.org/licenses/by/2.0/)

## 機能

撮影中に ADJ レバーを押し、6 番目の項目「GRK」を選びます。

- **フィルムルック**：Portra 400、Velvia 100、Tri-X 400 など。撮影画面でそのまま確認できます。[grkit.app](https://grkit.app) で手持ちの `.cube` LUT や Lightroom プリセットも読み込めます。各ルックはオープンソースのフィルム LUT をもとに近似したもので、元の LUT とまったく同じではありません（[出典](effects/NOTICE.md)）。
- **フレーミング**：XPan、2.35:1、16:9、4:3、5:4、7:6、1:1。撮影画面にマスクか枠線で範囲を表示します。
- **構図ガイド**：9 分割、画面中央、黄金比、対角線。
- **日付入れ撮影**：クラシック、GR1s のドット、GR II の 3 種類の書体で写真に日付を入れます。
- **粒状感**：強さは弱・中・強・特強、粒の大きさは自動・小・中・大から選べます。
- **レシピ**：今のルックをカメラに保存するか、[grkit.app](https://grkit.app) でレシピを作り、フィルムルックかイメージコントロール、粒状感、ホワイトバランスをまとめて切り替えます。

GRKit の読み込みには、電源を入れてから約 8〜10 秒かかります。GR IV や、インストール・アップデート後の最初の起動ではもう少しかかります。それまではカメラ本来の設定で撮影されます。すべての設定は MENU のカスタマイズタブの最後にある GRKit にあります。モジュールを自作する場合は [Writing a module](docs/modules.md)（英語）をご覧ください。

## 対応カメラ

| カメラ | 実機で確認済み | 対応 |
| --- | --- | --- |
| GR III、GR III HDF | 2.10、2.20 | — |
| GR IIIx、GR IIIx HDF | 1.60、1.70 | — |
| GR IV、GR IV HDF | 1.11、1.20 | — |
| GR IV Monochrome | 1.00 | 1.10 |

GR IV Monochrome ではフィルムルックは使えません。ほかの機種やファームウェアには対応していません。対応カメラで GRKit が読み込まれない場合は、[ファームウェア書き出しガイド](resources/DUMP.ja.md)をご覧ください。

> [!WARNING]
> GRKit は試験的なプロジェクトです。ご利用は自己責任でお願いします。
> インストールするとカメラの SD スクリプト設定がオンになり、オフにするまでそのままです。GRKit に問題があるとカメラが起動中に止まることがあるため、インストール前に[復旧ガイド](resources/RECOVERY.ja.md)をお読みください。

## インストール

[grkit.app](https://grkit.app) でカメラ、モジュール、エフェクトを選び、次のどちらかでインストールします。

- **SD カードに直接書き込む**：パソコンの Chrome か Edge で行います。アップデートやアンインストール、モジュールやエフェクトの追加・削除もサイトからできます。
- **GRKit.zip をダウンロード**：中の PDF ガイドの手順に従います（[GR III・GR IIIx](resources/INSTALLATION.ja.md)、[GR IV](resources/INSTALLATION-GR4.ja.md)）。

MENU ボタンを押しながら電源を入れるのは初回だけです。ZIP でアップデートするときは、カードの `SCRIPT` フォルダーを入れ替えてください。設定は `GRKIT` に残ります。カードをフォーマットすると GRKit も消えるので、先に `GRKIT` をパソコンにコピーしておいてください。

GRK が表示されないときは、まず [grkit.app](https://grkit.app) の FAQ をご覧ください。それでも解決しない場合は、`GRKIT/REPORT.TXT`、`GRKIT/BOOT.TXT`、`SCRIPT/GRKIT/INSTALL.ERR`（ある場合）を添えて[デバイスレポート](https://github.com/ahonn/grkit/issues/new?template=device-report.yml)を送ってください。

## ソースからのビルド

Rust 1.92 以降、ARM 対応の Clang と LLD、リコー公式のファームウェアファイルが必要です（ファームウェアはリポジトリに含まれていません）。

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

ほかのカメラ、一部のモジュールだけのパッケージ、ほかの言語のガイドについては [Build packages](CONTRIBUTING.md#build-packages)（英語）をご覧ください。

## コントリビューション

[CONTRIBUTING.md](CONTRIBUTING.md)（英語）をご覧ください。動いた場合も動かなかった場合も、デバイスレポートを歓迎します。特にまだ実機で確認できていないファームウェアの報告は助かります。不具合、機能の要望、質問は[フィードバックフォーム](https://tally.so/r/Z9jRgB)からお送りください。プロジェクトを応援したい方は [Buy Me a Coffee](https://buymeacoffee.com/yuexunjiang) からどうぞ。

## ドキュメント（英語）

- [アーキテクチャ](docs/architecture.md)
- [モジュールの作り方](docs/modules.md)と[モジュールリファレンス](docs/module-reference.md)
- [リリース手順](docs/releasing.md)
- [エフェクト](effects/README.md)
- [ウェブサイト](web/README.md)

## ライセンス

コードは [MIT ライセンス](LICENSE)です。各パッケージには、収録ファイルのライセンスをまとめた `LICENSES.md` が入っています。

- エフェクトのライセンスは CC BY-SA 4.0 または MIT です。詳しくは [effects/NOTICE.md](effects/NOTICE.md) をご覧ください。
- メニューと撮影画面の文字には [Roboto Condensed](https://github.com/googlefonts/roboto-classic) を [SIL Open Font License 1.1](crates/grkit-core/assets/RobotoCondensed-LICENSE.txt) のもとで使用しています。
- エフェクトのフィッティング手法は [gr3-film-mods](https://github.com/idontknownihil/gr3-film-mods) から [MIT ライセンス](crates/grkit-core/GR3_FILM_MODS_LICENSE.txt)のもとで移植しました。

RICOH および GR は Ricoh Company, Ltd. の商標です。フィルム名はそれぞれの所有者の商標です。GRKit は独立したプロジェクトであり、リコーやフィルムメーカーとは関係ありません。
