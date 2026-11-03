---
default: patch
---

On Windows, `grkit module build` and `grkit module test` find the headers of a module, because the build no longer gives Clang paths with the `\?\` prefix.
