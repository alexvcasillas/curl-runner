#!/usr/bin/env bash
# Checks that a deployment of the website serves every URL the current
# production site serves (taken from the reference sitemap), plus the markdown
# mirrors, feeds, installers and the /api/test endpoints.
#
# Usage: scripts/check-site-urls.sh <base-url> [reference-url]
#   reference-url default: https://www.curl-runner.com
# Env: CURL_EXTRA  extra curl arguments (e.g. a protection-bypass header).
# Exits non-zero, listing every failure.
set -uo pipefail

BASE="${1:?usage: check-site-urls.sh <base-url> [reference-url]}"
REF="${2:-https://www.curl-runner.com}"
BASE="${BASE%/}"
REF="${REF%/}"
# shellcheck disable=SC2206
EXTRA=(${CURL_EXTRA:-})
FAILS=0
CHECKS=0

fail() { echo "FAIL $*"; FAILS=$((FAILS + 1)); }
get() { curl -s -o /dev/null -w "$1" --max-time 60 ${3:-} ${EXTRA[@]+"${EXTRA[@]}"} "$2"; }

expect_status() { # url expected [curl flags], e.g. -L to follow redirects
  CHECKS=$((CHECKS + 1))
  local got
  got="$(get '%{http_code}' "$1" ${3:-})"
  [ "$got" = "$2" ] || fail "$1 -> $got (want $2)"
}

expect_type() { # url regex
  CHECKS=$((CHECKS + 1))
  local out code type
  out="$(get '%{http_code} %{content_type}' "$1")"
  code="${out%% *}"
  type="${out#* }"
  { [ "$code" = 200 ] && echo "$type" | grep -Eq "$2"; } || fail "$1 -> $code '$type' (want 200 $2)"
}

SITEMAP="$(curl -s --max-time 60 "$REF/sitemap.xml")"
PATHS="$(echo "$SITEMAP" | grep -o '<loc>[^<]*</loc>' | sed -E 's#</?loc>##g' \
  | sed -E 's#^https?://[^/]+##' | sed -E 's#^$#/#' | sort -u)"
[ -n "$PATHS" ] || { echo "FAIL could not read $REF/sitemap.xml"; exit 1; }
echo "base:      $BASE"
echo "reference: $REF ($(echo "$PATHS" | wc -l | tr -d ' ') sitemap paths)"

while IFS= read -r p; do
  expect_status "$BASE$p" 200
  [ "$p" = "/" ] || expect_status "$BASE${p%/}/" 200 -L # production 308s to the slashless URL too
  case "$p" in
    /docs/*) expect_type "$BASE$p.md" 'text/(markdown|plain)' ;;
  esac
done <<< "$PATHS"

expect_type "$BASE/docs.md" 'text/(markdown|plain)'
for f in /llms.txt /rss.xml /sitemap.xml; do expect_status "$BASE$f" 200; done

for f in /install.sh /install.ps1; do
  CHECKS=$((CHECKS + 1))
  a="$(curl -s --max-time 60 ${EXTRA[@]+"${EXTRA[@]}"} "$BASE$f" | shasum -a 256 | cut -d' ' -f1)"
  b="$(curl -s --max-time 60 "$REF$f" | shasum -a 256 | cut -d' ' -f1)"
  [ "$a" = "$b" ] || fail "$f differs from $REF$f"
done

expect_status "$BASE/api/test/echo" 200
expect_status "$BASE/api/test/status/404" 404
CHECKS=$((CHECKS + 1))
t="$(get '%{time_total}' "$BASE/api/test/delay/1")"
awk -v t="$t" 'BEGIN { exit !(t >= 1.0) }' || fail "/api/test/delay/1 took ${t}s (want >= 1s)"

echo "$CHECKS checks, $FAILS failures"
[ "$FAILS" -eq 0 ]
