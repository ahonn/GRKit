#!/bin/sh
# Use the existing C test harnesses with fatal memory and undefined-behavior checks.
set -eu
exec "${GRKIT_SANITIZER_CC:-clang}" -fsanitize=address,undefined \
  -fno-sanitize-recover=all -fno-omit-frame-pointer "$@"
