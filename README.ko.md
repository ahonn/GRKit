# GRKit

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md) | 한국어

[![Tests](https://github.com/ahonn/grkit/actions/workflows/tests.yml/badge.svg)](https://github.com/ahonn/grkit/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

GRKit은 RICOH GR III, GR IIIx, GR IV에 필름 느낌의 색감, XPan 같은 화면 비율, 날짜 각인, 입자감을 더해 주는 도구입니다. SD 카드에서 실행되며 펌웨어는 건드리지 않습니다. 카드를 빼면 카메라는 원래 상태로 돌아갑니다.

<p>
  <img src="https://github.com/ahonn/grkit/raw/main/resources/images/grkit-root.ko.jpg" alt="ADJ 모드의 GRKit 페이지" width="49%">
  <img src="https://github.com/ahonn/grkit/raw/main/resources/images/grkit-menu.ko.jpg" alt="MENU 사용자 설정 탭의 GRKit 항목" width="49%">
</p>

사진: nagi usano, [Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/), [CC BY 2.0](https://creativecommons.org/licenses/by/2.0/)

## 기능

촬영 중에 ADJ 레버를 누르고 여섯 번째 항목인 GRK를 선택합니다.

- **필름 룩**: Portra 400, Velvia 100, Tri-X 400 등. 촬영 화면에서 바로 확인할 수 있습니다. [grkit.app](https://grkit.app)에서 갖고 있는 `.cube` LUT나 Lightroom 프리셋도 가져올 수 있습니다. 각 룩은 오픈 소스 필름 LUT를 근사한 것이라 원본 LUT와 완전히 같지는 않습니다([출처](effects/NOTICE.md)).
- **프레이밍**: XPan, 2.35:1, 16:9, 4:3, 5:4, 7:6, 1:1. 촬영 화면에 마스크나 프레임 선으로 범위를 표시합니다.
- **구도 가이드**: 9분할, 화면 중앙, 황금비, 대각선.
- **날짜 각인**: 클래식, GR1s 도트, GR II의 세 가지 글꼴로 사진에 날짜를 넣습니다.
- **입자감**: 강도는 약·중·강·매우 강, 입자 크기는 자동·소·중·대에서 고릅니다.
- **레시피**: 현재 룩을 카메라에 저장하거나 [grkit.app](https://grkit.app)에서 레시피를 만들어, 필름 룩이나 이미지 컨트롤, 입자감, 화이트 밸런스를 한 번에 바꿉니다.

GRKit은 전원을 켜고 8~10초쯤 뒤에 불러와집니다. GR IV나 설치·업데이트 후 처음 켤 때는 더 걸립니다. 그전까지는 카메라 본래 설정으로 촬영됩니다. 모든 설정은 MENU 사용자 설정 탭 마지막의 GRKit 항목에 있습니다. 모듈을 직접 만들려면 [Writing a module](docs/modules.md)(영어)을 참고하세요.

## 지원 카메라

| 카메라 | 실제 카메라에서 확인 | 지원 |
| --- | --- | --- |
| GR III, GR III HDF | 2.10, 2.20 | — |
| GR IIIx, GR IIIx HDF | 1.60, 1.70 | — |
| GR IV, GR IV HDF | 1.11, 1.20 | — |
| GR IV Monochrome | 1.00 | 1.10 |

GR IV Monochrome에서는 필름 룩을 쓸 수 없습니다. 다른 기종이나 펌웨어는 지원하지 않습니다. 지원 카메라에서 GRKit이 불러와지지 않으면 [펌웨어 덤프 가이드](resources/DUMP.ko.md)를 참고하세요.

> [!WARNING]
> GRKit은 실험적인 프로젝트입니다. 사용에 따른 책임은 본인에게 있습니다.
> 설치하면 카메라의 SD 스크립트 설정이 켜지고, 끌 때까지 그대로 유지됩니다. GRKit에 문제가 있으면 카메라가 시작 중에 멈출 수 있으니 설치 전에 [복구 가이드](resources/RECOVERY.ko.md)를 읽어 주세요.

## 설치

[grkit.app](https://grkit.app)에서 카메라, 모듈, 효과를 고른 다음 아래 방법 중 하나로 설치합니다.

- **카드에 기록**: 컴퓨터의 Chrome이나 Edge에서 합니다. 업데이트와 제거, 모듈과 효과의 추가·삭제도 사이트에서 할 수 있습니다.
- **GRKit.zip 다운로드**: 안에 든 PDF 가이드를 따릅니다([GR III·GR IIIx](resources/INSTALLATION.ko.md), [GR IV](resources/INSTALLATION-GR4.ko.md)).

MENU 버튼을 누른 채 전원을 켜는 것은 처음 한 번뿐입니다. ZIP으로 업데이트할 때는 카드의 `SCRIPT` 폴더를 바꿔 넣으세요. 설정은 `GRKIT`에 남습니다. 카드를 포맷하면 GRKit도 지워지므로 먼저 `GRKIT`을 컴퓨터에 복사해 두세요.

GRK가 보이지 않으면 먼저 [grkit.app](https://grkit.app)의 FAQ를 확인하세요. 그래도 해결되지 않으면 `GRKIT/REPORT.TXT`, `GRKIT/BOOT.TXT`, `SCRIPT/GRKIT/INSTALL.ERR`(있는 경우)을 첨부해 [기기 리포트](https://github.com/ahonn/grkit/issues/new?template=device-report.yml)를 보내 주세요.

## 소스에서 빌드

Rust 1.92 이상, ARM을 지원하는 Clang과 LLD, 리코 공식 펌웨어 파일이 필요합니다(펌웨어는 저장소에 들어 있지 않습니다).

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

다른 카메라, 일부 모듈만 담은 패키지, 다른 언어의 가이드는 [Build packages](CONTRIBUTING.md#build-packages)(영어)를 참고하세요.

## 기여

[CONTRIBUTING.md](CONTRIBUTING.md)(영어)를 참고하세요. 잘 작동했든 안 됐든 기기 리포트는 언제나 환영합니다. 특히 아직 실제 카메라에서 확인하지 못한 펌웨어의 리포트가 큰 도움이 됩니다. 버그, 기능 제안, 질문은 [피드백 양식](https://tally.so/r/Z9jRgB)으로 보내 주세요. 프로젝트를 응원하고 싶다면 [Buy Me a Coffee](https://buymeacoffee.com/yuexunjiang)를 이용해 주세요.

## 문서(영어)

- [아키텍처](docs/architecture.md)
- [모듈 만들기](docs/modules.md)와 [모듈 레퍼런스](docs/module-reference.md)
- [릴리스 절차](docs/releasing.md)
- [효과](effects/README.md)
- [웹사이트](web/README.md)

## 라이선스

코드는 [MIT 라이선스](LICENSE)입니다. 각 패키지에는 포함된 파일의 라이선스를 정리한 `LICENSES.md`가 들어 있습니다.

- 효과의 라이선스는 CC BY-SA 4.0 또는 MIT입니다. 자세한 내용은 [effects/NOTICE.md](effects/NOTICE.md)를 참고하세요.
- 메뉴와 촬영 화면의 글자에는 [Roboto Condensed](https://github.com/googlefonts/roboto-classic)를 [SIL Open Font License 1.1](crates/grkit-core/assets/RobotoCondensed-LICENSE.txt)에 따라 사용합니다.
- 효과의 피팅 방법은 [gr3-film-mods](https://github.com/idontknownihil/gr3-film-mods)에서 [MIT 라이선스](crates/grkit-core/GR3_FILM_MODS_LICENSE.txt)에 따라 가져왔습니다.

RICOH와 GR은 Ricoh Company, Ltd.의 상표입니다. 필름 이름은 각 소유자의 상표입니다. GRKit은 독립 프로젝트이며 리코나 필름 제조사와 관련이 없습니다.
