#!/usr/bin/env bash
# Pack a built curl-runner studio app bundle into a compressed disk image
# (UDZO) with the app and an /Applications link to drag it onto. The volume
# is named "curl-runner studio". Signing and notarizing are left to the
# caller (the release workflow signs the app first and the dmg after).
# Usage: studio-dmg.sh <app> <out.dmg>
set -euo pipefail

[[ $# -eq 2 ]] || {
  echo "usage: studio-dmg.sh <app> <out.dmg>" >&2
  exit 1
}
APP="${1%/}"
OUT="$2"

[[ "$(uname -s)" == Darwin ]] || {
  echo "error: the studio dmg is macOS only" >&2
  exit 1
}
[[ -d "$APP" && "$APP" == *.app ]] || {
  echo "error: $APP is not an app bundle" >&2
  exit 1
}

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

# ditto keeps the bundle's signature, extended attributes and symlinks intact.
ditto "$APP" "$STAGE/$(basename "$APP")"
ln -s /Applications "$STAGE/Applications"

mkdir -p "$(dirname "$OUT")"
hdiutil create -volname "curl-runner studio" -srcfolder "$STAGE" \
  -fs HFS+ -format UDZO -ov "$OUT" >/dev/null
echo "Built $OUT"
