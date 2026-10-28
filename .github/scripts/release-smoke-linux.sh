#!/usr/bin/env bash
# Smoke-test the release archives on a pristine debian:bookworm-slim.
# Usage: release-smoke-linux.sh <artifact-dir> <version> <platform> [docker-platform]
#   <artifact-dir> holds curl-runner-cli-<version>-<platform>.tar.gz + SHA256SUMS.txt
#
# install.sh needs curl and talks to github.com, so stage 1 runs it in a container
# that has curl plus a shim redirecting the two GitHub URL shapes to <artifact-dir>.
# Stage 2 runs the installed binary in a container with ONLY ca-certificates
# (no curl, no bun, no node) to prove it is static and reads the system CA store.
set -euo pipefail

ART="$(cd "$1" && pwd)"
VERSION="$2"
PLATFORM="$3"
DPLAT="${4:-}"
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT
mkdir -p "$OUT/shim" "$OUT/home"
PFLAG=()
[ -n "$DPLAT" ] && PFLAG=(--platform "$DPLAT")

cat > "$OUT/shim/curl" <<SHIM
#!/bin/bash
# Test shim: map GitHub release URLs onto the local artifact dir.
url=""; out=""
while [ \$# -gt 0 ]; do
  case "\$1" in
    -o) out="\$2"; shift 2 ;;
    -*) shift ;;
    *) url="\$1"; shift ;;
  esac
done
case "\$url" in
  https://api.github.com/repos/*/releases/latest)
    echo '{"tag_name": "@curl-runner/cli@${VERSION}"}' ;;
  https://github.com/*/releases/download/*)
    f="/artifacts/\${url##*/}"
    if [ -n "\$out" ]; then cp "\$f" "\$out"; else cat "\$f"; fi ;;
  *) echo "unexpected URL: \$url" >&2; exit 1 ;;
esac
SHIM
chmod +x "$OUT/shim/curl"

echo "== stage 1: install.sh (shimmed GitHub) =="
docker run --rm "${PFLAG[@]}" \
  -v "$ART:/artifacts:ro" -v "$OUT/shim:/shim:ro" -v "$OUT/home:/home/out" \
  -v "$REPO_ROOT/packages/docs/public/install.sh:/install.sh:ro" \
  -e SHELL=/bin/bash -e HOME=/home/out debian:bookworm-slim bash -c '
    set -euo pipefail
    apt-get update -qq >/dev/null && apt-get install -y -qq --no-install-recommends ca-certificates >/dev/null
    PATH=/shim:$PATH bash /install.sh
    chmod -R a+rX /home/out'

echo "== stage 2: pristine bookworm-slim (ca-certificates only) =="
docker run --rm "${PFLAG[@]}" -v "$OUT/home:/home/out:ro" debian:bookworm-slim bash -c '
  set -euo pipefail
  apt-get update -qq >/dev/null && apt-get install -y -qq --no-install-recommends ca-certificates >/dev/null
  command -v curl && { echo "curl must not be installed"; exit 1; } || true
  BIN=/home/out/.local/bin/curl-runner
  echo "-- version"; "$BIN" --version
  cat > /tmp/gh.yaml <<YAML
request:
  name: github
  url: https://api.github.com/zen
  method: GET
  headers:
    User-Agent: curl-runner-release-smoke
YAML
  echo "-- https request"; "$BIN" /tmp/gh.yaml'
