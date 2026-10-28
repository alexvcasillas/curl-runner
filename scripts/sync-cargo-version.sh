#!/usr/bin/env bash
# Stamp the version of packages/cli/package.json (bumped by `changeset version`)
# into the Cargo workspace: [workspace.package] version, the pinned
# curl-runner-core dependency and the workspace entries of Cargo.lock.
# Runs as part of `pnpm run version`, so the version PR carries both bumps.
# Usage: sync-cargo-version.sh [repo-root]
set -euo pipefail

ROOT="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
VERSION="$(jq -r .version "$ROOT/packages/cli/package.json")"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || {
  echo "error: unexpected version '$VERSION' in packages/cli/package.json" >&2
  exit 1
}

V="$VERSION" perl -0pi -e '
  s/(\[workspace\.package\]\n(?:#[^\n]*\n)*version = )"[^"]*"/$1"$ENV{V}"/ or die "workspace version not found\n";
  s/(curl-runner-core = \{ path = "crates\/curl-runner-core", version = )"=[^"]*"/$1"=$ENV{V}"/ or die "core dependency not found\n";
' "$ROOT/Cargo.toml"

V="$VERSION" perl -0pi -e '
  for my $c (qw(curl-runner curl-runner-core testkit)) {
    s/(\[\[package\]\]\nname = "\Q$c\E"\nversion = )"[^"]*"/$1"$ENV{V}"/ or die "Cargo.lock entry $c not found\n";
  }
' "$ROOT/Cargo.lock"

echo "Cargo workspace set to $VERSION"
