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

# sha256 of a body; fails unless the response is 200 with a non-empty body.
fetch_sum() { # url [curl args]
  local url="$1" f code
  shift
  f="$(mktemp)"
  code="$(curl -s -o "$f" -w '%{http_code}' --max-time 60 "$@" "$url")"
  if [ "$code" = 200 ] && [ -s "$f" ]; then shasum -a 256 "$f" | cut -d' ' -f1; rm -f "$f"; return 0; fi
  rm -f "$f"
  return 1
}

expect_status() { # url expected [curl flags], e.g. -L to follow redirects
  CHECKS=$((CHECKS + 1))
  local got out final
  out="$(get '%{http_code} %{url_effective}' "$1" ${3:-})"
  got="${out%% *}"
  final="${out#* }"
  [ "$got" = "$2" ] || fail "$1 -> $got (want $2)"
  if [ -n "${3:-}" ] && [ "$(echo "$final" | cut -d/ -f3)" != "$(echo "$1" | cut -d/ -f3)" ]; then
    fail "$1 redirected off-host to $final"
  fi
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
  a="$(fetch_sum "$BASE$f" ${EXTRA[@]+"${EXTRA[@]}"})" || { fail "$f on $BASE: not 200 or empty"; continue; }
  b="$(fetch_sum "$REF$f")" || { fail "$f on $REF: not 200 or empty"; continue; }
  [ "$a" = "$b" ] || fail "$f differs from $REF$f"
done

expect_status "$BASE/api/test/echo" 200
expect_status "$BASE/api/test/status/404" 404
CHECKS=$((CHECKS + 1))
out="$(get '%{http_code} %{time_total}' "$BASE/api/test/delay/1")"
[ "${out%% *}" = 200 ] || fail "/api/test/delay/1 -> ${out%% *} (want 200)"
t="${out#* }"
awk -v t="$t" 'BEGIN { exit !(t >= 1.0) }' || fail "/api/test/delay/1 took ${t}s (want >= 1s)"

# Canonical-host redirects. Production only: other deployments have their own hostname.
# One permanent hop (301/308) straight to the canonical URL; redirects are not followed.
expect_redirect() { # url expected-location
  CHECKS=$((CHECKS + 1))
  local out code loc
  out="$(get '%{http_code} %{redirect_url}' "$1")"
  code="${out%% *}"
  loc="${out#* }"
  { [ "$code" = 301 ] || [ "$code" = 308 ]; } || fail "$1 -> $code (want 301 or 308)"
  [ "$loc" = "$2" ] || fail "$1 -> Location '$loc' (want $2)"
}

if [ "$BASE" = "https://www.curl-runner.com" ]; then
  for u in http://curl-runner.com/ http://www.curl-runner.com/ https://curl-runner.com/; do
    expect_redirect "$u" "https://www.curl-runner.com/"
  done
else
  echo "skip: canonical-host redirect checks run only against https://www.curl-runner.com"
fi

echo "$CHECKS checks, $FAILS failures"
[ "$FAILS" -eq 0 ]
