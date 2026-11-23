#!/bin/sh
# Installs the tools that build the installation guides into DIR: Typst in
# DIR/bin and the Noto Sans SC, TC, JP, and KR fonts in DIR/fonts.
# Each download must match its pinned SHA-256.
# Usage: sh .github/scripts/install-guide-tools.sh DIR
set -eu

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 DIR" >&2
  exit 2
fi
dir=$1

typst_version=0.15.1
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    typst=typst-x86_64-unknown-linux-musl
    typst_sha256=a6d077d0a95eed5a2eba715b2dae06be954f624ccbf85758a03f389ded33118c
    ;;
  Linux-aarch64)
    typst=typst-aarch64-unknown-linux-musl
    typst_sha256=5aa8d74a3d906e60ea12a66ac2f37f8eef1b14cbad7182a745e393a10c23dcee
    ;;
  Darwin-arm64)
    typst=typst-aarch64-apple-darwin
    typst_sha256=48f62ed034aa3a7978309579ac6ca00045e2ef0da73114e8af27cfd8e74dc05a
    ;;
  *)
    echo "No pinned Typst build for $(uname -s) $(uname -m)." >&2
    exit 1
    ;;
esac
# The subset OTF fonts of a Noto Sans CJK release.
fonts_url=https://github.com/notofonts/noto-cjk/raw/Sans2.004/Sans/SubsetOTF

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# fetch URL FILE SHA256: downloads URL to FILE and checks its SHA-256.
fetch() {
  curl --fail --location --silent --show-error --retry 3 -o "$2" "$1"
  if command -v shasum >/dev/null; then
    actual=$(shasum -a 256 "$2")
  else
    actual=$(sha256sum "$2")
  fi
  if [ "${actual%% *}" != "$3" ]; then
    echo "The SHA-256 of $1 differs from the pinned value." >&2
    exit 1
  fi
}

mkdir -p "$dir/bin" "$dir/fonts"
fetch "https://github.com/typst/typst/releases/download/v$typst_version/$typst.tar.xz" \
  "$work/typst.tar.xz" "$typst_sha256"
tar -xJf "$work/typst.tar.xz" -C "$work"
cp "$work/$typst/typst" "$dir/bin/typst"

fetch "$fonts_url/SC/NotoSansSC-Regular.otf" "$dir/fonts/NotoSansSC-Regular.otf" \
  faa6c9df652116dde789d351359f3d7e5d2285a2b2a1f04a2d7244df706d5ea9
fetch "$fonts_url/SC/NotoSansSC-Bold.otf" "$dir/fonts/NotoSansSC-Bold.otf" \
  c6cb5a93abaa9edc8ee7463b7ebb7f42d618d40e6ed2f7a5371c97b0b64767c0
fetch "$fonts_url/TC/NotoSansTC-Regular.otf" "$dir/fonts/NotoSansTC-Regular.otf" \
  5bab0cb3c1cf89dde07c4a95a4054b195afbcfe784d69d75c340780712237537
fetch "$fonts_url/TC/NotoSansTC-Bold.otf" "$dir/fonts/NotoSansTC-Bold.otf" \
  55420b259eb119bf5f2a0aadba10cf9d736c12d64ab93e78546d69ef5f43558b
fetch "$fonts_url/JP/NotoSansJP-Regular.otf" "$dir/fonts/NotoSansJP-Regular.otf" \
  dff723ba59d57d136764a04b9b2d03205544f7cd785a711442d6d2d085ac5073
fetch "$fonts_url/JP/NotoSansJP-Bold.otf" "$dir/fonts/NotoSansJP-Bold.otf" \
  1b0edfb500b73a4fa8a4fcaae1bbbd403994e08e73e3e0da37e70d3853f42c5f
fetch "$fonts_url/KR/NotoSansKR-Regular.otf" "$dir/fonts/NotoSansKR-Regular.otf" \
  69975a0ac8472717870aefeab0a4d52739308d90856b9955313b2ad5e0148d68
fetch "$fonts_url/KR/NotoSansKR-Bold.otf" "$dir/fonts/NotoSansKR-Bold.otf" \
  5a6ceb287ed2fc6cfc6213144ebea68cbd94b20fc9eb873d8486493bf02d9bda
