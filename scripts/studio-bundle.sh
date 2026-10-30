#!/usr/bin/env bash
# Build a local macOS app bundle of curl-runner studio, so the system can
# hand it `curl-runner://` share links: {target}/studio/curl-runner studio.app,
# with an Info.plist that registers the `curl-runner` URL scheme. {target} is
# $CARGO_TARGET_DIR when set, else target/ in the repo.
#
# Nothing is installed: the bundle stays in the target dir. Opening it once
# registers it with LaunchServices, which then sends it the scheme's links.
# Usage: studio-bundle.sh [--release] [repo-root]
set -euo pipefail

PROFILE=debug
if [[ "${1:-}" == "--release" ]]; then
  PROFILE=release
  shift
fi
ROOT="${1:-$(cd "$(dirname "$0")/.." && pwd)}"

[[ "$(uname -s)" == Darwin ]] || {
  echo "error: the studio bundle is macOS only" >&2
  exit 1
}

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)"
[[ -n "$VERSION" ]] || {
  echo "error: workspace version not found in Cargo.toml" >&2
  exit 1
}

TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
# Cargo reads a relative CARGO_TARGET_DIR from where it runs: here.
[[ "$TARGET" == /* ]] || TARGET="$PWD/$TARGET"

cargo_flags=(--manifest-path "$ROOT/Cargo.toml" -p curl-runner-studio)
[[ "$PROFILE" == release ]] && cargo_flags+=(--release)
cargo build "${cargo_flags[@]}"

BIN="$TARGET/$PROFILE/curl-runner-studio"
APP="$TARGET/studio/curl-runner studio.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp "$BIN" "$APP/Contents/MacOS/curl-runner-studio"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>com.curl-runner.studio</string>
  <key>CFBundleName</key>
  <string>curl-runner studio</string>
  <key>CFBundleDisplayName</key>
  <string>curl-runner studio</string>
  <key>CFBundleExecutable</key>
  <string>curl-runner-studio</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$VERSION</string>
  <key>CFBundleVersion</key>
  <string>$VERSION</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>CFBundleURLTypes</key>
  <array>
    <dict>
      <key>CFBundleURLName</key>
      <string>curl-runner share link</string>
      <key>CFBundleURLSchemes</key>
      <array>
        <string>curl-runner</string>
      </array>
    </dict>
  </array>
</dict>
</plist>
PLIST

plutil -lint "$APP/Contents/Info.plist" >/dev/null
echo "Built $APP ($PROFILE, $VERSION)"
echo "Open it once (open \"$APP\") to register it with LaunchServices."
