#!/usr/bin/env bash
# Stamp the version of packages/cli/package.json (bumped by `changeset version`)
# into the Cargo workspace: [workspace.package] version, every pinned
# workspace dependency (`{ path = "crates/…", version = "=…" }`) and every
# workspace crate in Cargo.lock. Not a fixed list: a new crate is picked up.
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
  my $n = s/(\{ path = "crates\/[^"]+", version = )"=[^"]*"/$1"=$ENV{V}"/g;
  $n or die "no pinned workspace dependencies found\n";
' "$ROOT/Cargo.toml"

# Every workspace crate, not a fixed list: in Cargo.lock they are the
# packages with no `source = ` line (registry and git crates have one).
V="$VERSION" perl -0pi -e '
  my $n = s/(\[\[package\]\]\nname = "[^"]+"\nversion = )"[^"]*"(?=\n(?!source = ))/$1"$ENV{V}"/g;
  $n or die "no workspace entries found in Cargo.lock\n";
' "$ROOT/Cargo.lock"

echo "Cargo workspace set to $VERSION"
