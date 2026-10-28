#!/usr/bin/env bash
# shellcheck disable=SC2016 # jq filters and container scripts expand later, on purpose
# Smoke-test the npm packaging end to end in Docker, without the public
# registry: stage + `npm pack` every package, publish the tarballs to a
# throwaway local verdaccio, then install globally and run `curl-runner
# --version` in pristine containers:
#   bun   oven/bun:debian  (node fallback removed, asserts no node)  bun i -g
#   npm   node:22-slim     (asserts no bun)   npm i -g
#   pnpm  node:22-slim     (asserts no bun)   pnpm add -g
# plus the launcher's failure path (platform package deleted -> clear error).
# Usage: npm-smoke.sh <bin-dir> [docker-platform...]
#   <bin-dir>  holds curl-runner-linux-{x64,arm64} (release build artifacts)
#   platforms  default: linux/<host arch>; e.g. linux/arm64 linux/amd64
# Needs docker and jq. Writes nothing to any public registry.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$(cd "$1" && pwd)"
shift
if [ $# -gt 0 ]; then
  DPLATS=("$@")
else
  case "$(uname -m)" in arm64 | aarch64) DPLATS=(linux/arm64) ;; *) DPLATS=(linux/amd64) ;; esac
fi
VERSION="$(jq -r .version "$REPO_ROOT/packages/cli/package.json")"
ID="crnpm-$$"
WORK="$(mktemp -d)"
NET="$ID-net"
REG="http://$ID-registry:4873"

cleanup() {
  docker rm -f "$ID-registry" >/dev/null 2>&1 || true
  docker network rm "$NET" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

echo "== stage + pack @curl-runner/cli@$VERSION"
mkdir -p "$WORK/root/packages/cli" "$WORK/tgz"
cp -R "$REPO_ROOT/packages/cli/package.json" "$REPO_ROOT/packages/cli/README.md" \
  "$REPO_ROOT/packages/cli/bin" "$WORK/root/packages/cli/"
cp -R "$REPO_ROOT/npm" "$WORK/root/npm"
rm -rf "$WORK"/root/npm/*/bin
"$REPO_ROOT/scripts/npm-stage.sh" "$WORK/root" "$BIN_DIR"
# Pack only what has a binary (the smoke never needs darwin/win32).
PACK_DIRS="packages/cli"
for d in "$WORK"/root/npm/cli-*; do
  [ -d "$d/bin" ] && PACK_DIRS="$PACK_DIRS npm/$(basename "$d")"
done
docker run --rm -v "$WORK:/w" -w /w/root node:22-slim sh -ec "
  for d in $PACK_DIRS; do (cd \$d && npm pack --silent --pack-destination /w/tgz); done
  for t in /w/tgz/*.tgz; do echo \"-- \$(basename \$t)\"; tar -tvzf \$t | awk '{print \$1, \$NF}'; done"

echo "== local registry (verdaccio, no uplinks)"
mkdir -p "$WORK/verdaccio"
cat > "$WORK/verdaccio/config.yaml" <<'YAML'
storage: /verdaccio/storage/data
auth:
  htpasswd:
    file: /verdaccio/storage/htpasswd
    max_users: 10
packages:
  "**":
    access: $all
    publish: $all
log: { type: stdout, format: pretty, level: warn }
YAML
docker network create "$NET" >/dev/null
docker run -d --name "$ID-registry" --network "$NET" \
  -v "$WORK/verdaccio/config.yaml:/verdaccio/conf/config.yaml:ro" verdaccio/verdaccio:6 >/dev/null
docker run --rm --network "$NET" -v "$WORK/tgz:/tgz:ro" node:22-slim sh -ec "
  for i in \$(seq 1 60); do node -e \"fetch('$REG/-/ping').then(r=>process.exit(r.ok?0:1),()=>process.exit(1))\" && break; sleep 1; done
  TOKEN=\$(node -e \"fetch('$REG/-/user/org.couchdb.user:smoke',{method:'PUT',headers:{'content-type':'application/json'},body:JSON.stringify({name:'smoke',password:'smoke-pass'})}).then(r=>r.json()).then(j=>console.log(j.token))\")
  echo '//$ID-registry:4873/:_authToken='\$TOKEN > ~/.npmrc
  for t in /tgz/*.tgz; do npm publish --registry $REG \$t >/dev/null && echo \"published \$(basename \$t)\"; done"

check() { # <label> <image> <docker-platform> <script>
  echo "== $1 ($3)"
  if docker run --rm --platform "$3" --network "$NET" -e REG="$REG" -e VERSION="$VERSION" "$2" sh -ec "$4"; then
    echo "PASS $1 ($3)"
  else
    echo "FAIL $1 ($3)"
    FAILED=1
  fi
}

# Shared tail: version check, then delete the platform package and expect a
# fast, clear failure instead of a hang.
VERIFY='
  echo "which: $(command -v curl-runner)"
  out=$(curl-runner --version); echo "curl-runner --version: $out"
  [ "$out" = "$VERSION" ]
  # Informational: install.rs detection as seen through the launcher.
  curl-runner upgrade --dry-run 2>&1 | grep -a "Installation" || true
  # os/cpu selection: exactly one platform package installed.
  pkgs=$(find / -type d -path "*node_modules/@curl-runner/cli-*" -prune 2>/dev/null)
  echo "platform packages installed:"; echo "$pkgs"
  [ "$(echo "$pkgs" | wc -l)" -eq 1 ]
  pkg=$pkgs
  rm -rf "$pkg"
  if err=$(curl-runner --version 2>&1); then echo "expected failure"; exit 1; fi
  echo "after removing it: $err"
  echo "$err" | grep -q "platform package @curl-runner/cli-linux-.* is missing"
'

FAILED=0
for plat in "${DPLATS[@]}"; do
  check "bun i -g (no node)" oven/bun:debian "$plat" '
    # The image symlinks node -> bun; a bun.sh install has no such fallback.
    rm -rf /usr/local/bun-node-fallback-bin
    if command -v node; then echo "node must not exist"; exit 1; fi
    bun --version
    bun i -g --registry "$REG" "@curl-runner/cli@$VERSION"
    '"$VERIFY"
  check "npm i -g (no bun)" node:22-slim "$plat" '
    if command -v bun; then echo "bun must not exist"; exit 1; fi
    node --version
    npm i -g --registry "$REG" "@curl-runner/cli@$VERSION"
    '"$VERIFY"
  check "pnpm add -g (no bun)" node:22-slim "$plat" '
    if command -v bun; then echo "bun must not exist"; exit 1; fi
    npm i -g pnpm@10 >/dev/null 2>&1
    export PNPM_HOME=/pnpm PATH="/pnpm:$PATH"
    pnpm --version
    pnpm add -g --registry "$REG" "@curl-runner/cli@$VERSION"
    '"$VERIFY"
done

if [ "$FAILED" -ne 0 ]; then
  echo "npm smoke: FAIL"
  exit 1
fi
echo "npm smoke: PASS"
