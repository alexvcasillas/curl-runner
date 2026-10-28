#!/usr/bin/env bash
# shellcheck disable=SC2016 # jq filters and container scripts expand later, on purpose
# Stage the npm packages for publishing (in place, under <root>):
#   - stamp the @curl-runner/cli version into every npm/cli-<platform> package
#   - copy the release binaries into npm/cli-<platform>/bin/
#   - add the platform packages as exact-version optionalDependencies of
#     packages/cli (not committed: keeps pnpm-lock.yaml free of unpublished versions)
# Usage: npm-stage.sh <root> <bin-dir>
#   <root>    holds packages/cli and npm/ (the repo checkout in CI)
#   <bin-dir> holds the build job's curl-runner-<platform>[.exe] artifacts
# A platform without a binary is left unstaged (warning); npm-publish.sh
# refuses to publish it. Needs jq.
set -euo pipefail

ROOT="$(cd "$1" && pwd)"
BIN_DIR="$(cd "$2" && pwd)"
CLI="$ROOT/packages/cli/package.json"
VERSION="$(jq -r .version "$CLI")"
PLATFORMS="linux-x64 linux-arm64 darwin-x64 darwin-arm64 win32-x64"

stamp() { # <file> <jq filter> [jq args...]
  local f="$1" filter="$2"
  shift 2
  jq "$@" "$filter" "$f" > "$f.tmp" && mv "$f.tmp" "$f"
}

for p in $PLATFORMS; do
  pkg="$ROOT/npm/cli-$p"
  # The release build names the Windows target windows-x64; npm calls it win32.
  case "$p" in
    win32-x64) src="$BIN_DIR/curl-runner-windows-x64.exe" dst="$pkg/bin/curl-runner.exe" ;;
    *) src="$BIN_DIR/curl-runner-$p" dst="$pkg/bin/curl-runner" ;;
  esac
  stamp "$pkg/package.json" '.version = $v' --arg v "$VERSION"
  if [ -f "$src" ]; then
    mkdir -p "$pkg/bin"
    cp "$src" "$dst"
    chmod 755 "$dst"
    echo "staged @curl-runner/cli-$p@$VERSION"
  else
    echo "warning: no binary for $p ($src); @curl-runner/cli-$p not staged" >&2
  fi
done

stamp "$CLI" '.optionalDependencies = ($p | split(" ") | map({key: "@curl-runner/cli-\(.)", value: $v}) | from_entries)' \
  --arg p "$PLATFORMS" --arg v "$VERSION"
echo "staged @curl-runner/cli@$VERSION optionalDependencies"
