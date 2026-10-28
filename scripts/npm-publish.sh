#!/usr/bin/env bash
# Release publish step (changesets/action `publish:` input). Runs only once a
# version PR has been merged. Order matters: the five platform packages go
# first, then `changeset publish` publishes @curl-runner/cli, whose staged
# optionalDependencies pin them to the same version. Re-runnable: versions
# already on npm are skipped, @curl-runner/cli included (checked here, not by
# changesets).
# Usage: npm-publish.sh <bin-dir>   (the downloaded bin-* build artifacts)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$1"
VERSION="$(jq -r .version "$REPO_ROOT/packages/cli/package.json")"

mkdir -p "$BIN_DIR"
"$REPO_ROOT/scripts/npm-stage.sh" "$REPO_ROOT" "$BIN_DIR"

for p in linux-x64 linux-arm64 darwin-x64 darwin-arm64 win32-x64; do
  name="@curl-runner/cli-$p"
  # 0 published, 1 not published; anything else aborts (set -e) instead of
  # being mistaken for "unpublished".
  status=0
  "$REPO_ROOT/scripts/npm-published.sh" "$name" "$VERSION" || status=$?
  case "$status" in
    0)
      echo "$name@$VERSION already published, skipping"
      continue
      ;;
    1) ;;
    *)
      echo "error: npm registry lookup for $name@$VERSION failed; not publishing" >&2
      exit 1
      ;;
  esac
  exe=curl-runner
  [ "$p" = win32-x64 ] && exe=curl-runner.exe
  if [ ! -f "$REPO_ROOT/npm/cli-$p/bin/$exe" ]; then
    echo "error: $name@$VERSION is not on npm and its binary was not built" >&2
    exit 1
  fi
  (cd "$REPO_ROOT/npm/cli-$p" && npm publish --access public)
done

# `changeset publish` misreads already-published versions here (it retried
# @curl-runner/cli@2.0.0 and the redirect package), so decide ourselves.
status=0
"$REPO_ROOT/scripts/npm-published.sh" @curl-runner/cli "$VERSION" || status=$?
case "$status" in
  0)
    echo "@curl-runner/cli@$VERSION already published, skipping"
    exit 0
    ;;
  1) ;;
  *)
    echo "error: npm registry lookup for @curl-runner/cli@$VERSION failed; not publishing" >&2
    exit 1
    ;;
esac

cd "$REPO_ROOT"
pnpm release
