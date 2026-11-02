#!/usr/bin/env bash
# Build a local macOS app bundle of curl-runner studio, so the system can
# hand it `curl-runner://` share links: {target}/studio/curl-runner studio.app,
# with an Info.plist that registers the `curl-runner` URL scheme. {target} is
# $CARGO_TARGET_DIR when set, else target/ in the repo.
#
# Nothing is installed: the bundle stays in the target dir. Opening it once
# registers it with LaunchServices, which then sends it the scheme's links.
# `--server` builds with the server-backed UI (and the mock behind it); a
# distributed build never passes it. `--universal` builds arm64 and x86_64 and
# joins them with lipo. The icon comes from assets/AppIcon.icns
# (scripts/studio-icon.sh renders it).
# Usage: studio-bundle.sh [--release] [--universal] [--server] [repo-root]
set -euo pipefail

PROFILE=debug
SERVER=
UNIVERSAL=
while [[ "${1:-}" == --* ]]; do
  case "$1" in
    --release) PROFILE=release ;;
    --universal) UNIVERSAL=1 ;;
    --server) SERVER=1 ;;
    *)
      echo "error: unknown option $1" >&2
      exit 1
      ;;
  esac
  shift
done
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

ICON="$ROOT/crates/curl-runner-studio/assets/AppIcon.icns"
[[ -f "$ICON" ]] || {
  echo "error: $ICON not found (run scripts/studio-icon.sh)" >&2
  exit 1
}

cargo_flags=(--manifest-path "$ROOT/Cargo.toml" -p curl-runner-studio)
[[ "$PROFILE" == release ]] && cargo_flags+=(--release)
[[ -n "$SERVER" ]] && cargo_flags+=(--features server)

APP="$TARGET/studio/curl-runner studio.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
if [[ -n "$UNIVERSAL" ]]; then
  archs=(aarch64-apple-darwin x86_64-apple-darwin)
  bins=()
  for arch in "${archs[@]}"; do
    cargo build --locked "${cargo_flags[@]}" --target "$arch"
    bins+=("$TARGET/$arch/$PROFILE/curl-runner-studio")
  done
  lipo -create "${bins[@]}" -output "$APP/Contents/MacOS/curl-runner-studio"
else
  cargo build --locked "${cargo_flags[@]}"
  cp "$TARGET/$PROFILE/curl-runner-studio" "$APP/Contents/MacOS/curl-runner-studio"
fi
cp "$ICON" "$APP/Contents/Resources/AppIcon.icns"

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
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
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
