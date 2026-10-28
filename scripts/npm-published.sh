#!/usr/bin/env bash
# Is <name>@<version> on the registry? Exit 0 = published, 1 = not published
# (E404 for the package, or the package exists without that version: npm view
# then exits 0 with empty output), 2 = lookup failed after retries (network,
# auth, 5xx). Callers must not treat 2 as "unpublished".
# Usage: npm-published.sh <name> <version>
set -uo pipefail

err="$(mktemp)"
trap 'rm -f "$err"' EXIT
for attempt in 1 2 3; do
  if out="$(npm view "$1@$2" version 2>"$err")"; then
    [ "$out" = "$2" ] && exit 0
    [ -z "$out" ] && exit 1
    echo "unexpected npm view output for $1@$2: $out" >&2
    exit 2
  fi
  grep -q "E404" "$err" && exit 1
  echo "npm view $1@$2 failed (attempt $attempt/3)" >&2
  if [ "$attempt" -lt 3 ]; then sleep $((attempt * 5)); fi
done
echo "error: could not determine whether $1@$2 is published:" >&2
cat "$err" >&2
exit 2
