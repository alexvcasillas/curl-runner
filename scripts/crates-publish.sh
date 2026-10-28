#!/usr/bin/env bash
# Publish curl-runner-core, then curl-runner, to crates.io at the workspace
# version. Re-runnable: versions already on crates.io are skipped. A failed
# registry lookup aborts instead of being mistaken for "unpublished".
# Needs CARGO_REGISTRY_TOKEN (CI: crates.io trusted publishing).
# The binaries were already built and tested by the release matrix, so the
# per-crate verify build is skipped; the release smoke runs the full
# `cargo publish --workspace --dry-run` instead.
# Usage: crates-publish.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
VERSION="$(cargo metadata --format-version 1 --no-deps --locked |
  jq -r '.packages[] | select(.name == "curl-runner") | .version')"

# 0 published, 1 not published, 2 lookup failed.
published() {
  local code
  for attempt in 1 2 3; do
    code="$(curl -s -o /dev/null -w '%{http_code}' \
      -A "curl-runner-release (https://github.com/alexvcasillas/curl-runner)" \
      "https://crates.io/api/v1/crates/$1/$2")" || code=000
    case "$code" in
      200) return 0 ;;
      404) return 1 ;;
    esac
    echo "crates.io lookup for $1@$2 returned $code (attempt $attempt/3)" >&2
    if [ "$attempt" -lt 3 ]; then sleep $((attempt * 5)); fi
  done
  return 2
}

for crate in curl-runner-core curl-runner; do
  status=0
  published "$crate" "$VERSION" || status=$?
  case "$status" in
    0)
      echo "$crate@$VERSION already on crates.io, skipping"
      continue
      ;;
    1) ;;
    *)
      echo "error: crates.io lookup for $crate@$VERSION failed; not publishing" >&2
      exit 1
      ;;
  esac
  cargo publish --locked --no-verify -p "$crate"
done
