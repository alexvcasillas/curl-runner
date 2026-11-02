#!/usr/bin/env bash
# Render the studio app icon: the site's TerminalMark (SiteHeader.tsx) in white
# on a dark rounded tile (packages/docs/public/apple-icon-dark.png), on the
# macOS icon grid (1024px canvas, 824px tile, 100px inset, 185px radius).
# Writes crates/curl-runner-studio/assets/AppIcon-1024.png and AppIcon.icns.
# Needs rsvg-convert (brew install librsvg), sips and iconutil.
# Usage: studio-icon.sh [repo-root]
set -euo pipefail

ROOT="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
OUT="$ROOT/crates/curl-runner-studio/assets"

for tool in rsvg-convert sips iconutil; do
  command -v "$tool" >/dev/null || {
    echo "error: $tool not found (rsvg-convert: brew install librsvg)" >&2
    exit 1
  }
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# TerminalMark's 24x24 viewBox, scaled 27x and centred on the 1024 canvas.
cat > "$WORK/icon.svg" <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
  <rect x="100" y="100" width="824" height="824" rx="185" fill="#262626"/>
  <g transform="translate(188 188) scale(27)" fill="none" stroke="#ffffff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <polyline points="4 17 10 11 4 5"/>
    <line x1="12" y1="19" x2="20" y2="19"/>
  </g>
</svg>
SVG

mkdir -p "$OUT"
rsvg-convert -w 1024 -h 1024 "$WORK/icon.svg" -o "$OUT/AppIcon-1024.png"

SET="$WORK/AppIcon.iconset"
mkdir "$SET"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$OUT/AppIcon-1024.png" --out "$SET/icon_${size}x${size}.png" >/dev/null
  sips -z "$((size * 2))" "$((size * 2))" "$OUT/AppIcon-1024.png" --out "$SET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$SET" -o "$OUT/AppIcon.icns"
echo "Wrote $OUT/AppIcon-1024.png and AppIcon.icns"
