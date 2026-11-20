#!/bin/sh
# Prints the release notes for the release index as a JSON list of
# {"version", "body"}: the bodies of the latest published GitHub releases, and
# for VERSION, when it is not published yet, its section of CHANGELOG.md.
# `grkit release-index --notes` keeps their bullet items as plain text.
# Needs gh with GH_TOKEN, and jq.
# Usage: sh .github/scripts/release-notes.sh <version>
set -eu

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <version>" >&2
  exit 2
fi
version=$1
scripts=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Published releases only: a draft is not public yet.
gh release list --exclude-drafts --exclude-pre-releases --limit 10 \
  --json tagName --jq '.[].tagName' | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' > "$work/tags" || true

: > "$work/notes.jsonl"
if ! grep -qx "v$version" "$work/tags"; then
  if sh "$scripts/changelog-section.sh" "$version" > "$work/body"; then
    jq -n --arg version "$version" --rawfile body "$work/body" \
      '{version: $version, body: $body}' >> "$work/notes.jsonl"
  fi
fi
while read -r tag; do
  gh release view "$tag" --json tagName,body \
    --jq '{version: (.tagName | ltrimstr("v")), body: .body}' >> "$work/notes.jsonl"
done < "$work/tags"
jq -s . "$work/notes.jsonl"
