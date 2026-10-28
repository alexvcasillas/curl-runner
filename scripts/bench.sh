#!/usr/bin/env bash
# Startup, request overhead and peak memory of a curl-runner binary, against the
# local testkit server (so it measures curl-runner, not the network). The
# website's numbers come from this script.
#
# Usage: scripts/bench.sh [binary] [other-binary-to-compare]
#   binary  default: target/release/curl-runner (built if missing)
# Env: RUNS (startup samples, default 50), REQ_RUNS (suite samples, default 10).
# Needs perl (timing) and /usr/bin/time (macOS `-l` or GNU `-v` for memory).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
RUNS="${RUNS:-50}"
REQ_RUNS="${REQ_RUNS:-10}"
BIN="${1:-$ROOT/target/release/curl-runner}"
OTHER="${2:-}"
SERVER="$ROOT/target/release/testkit-server"

[ -x "$BIN" ] || cargo build -q --release -p curl-runner
[ -x "$SERVER" ] || cargo build -q --release -p testkit --bin testkit-server

WORK="$(mktemp -d)"
"$SERVER" --port 0 > "$WORK/server.log" 2>&1 &
SPID=$!
trap 'kill $SPID 2>/dev/null; wait $SPID 2>/dev/null; rm -rf "$WORK"' EXIT
for _ in $(seq 50); do grep -q '^listening ' "$WORK/server.log" && break; sleep 0.1; done
BASE="$(sed -n 's/^listening //p' "$WORK/server.log")"
[ -n "$BASE" ] || { echo "error: testkit server did not start" >&2; exit 1; }

# 100 requests to a fast JSON endpoint, each with a status check.
{
  echo "requests:"
  for i in $(seq 100); do
    printf '  - name: r%s\n    url: %s/json/user\n    expect:\n      status: 200\n' "$i" "$BASE"
  done
} > "$WORK/hundred.yaml"

# Prints "median X ms  p90 Y ms" over N runs of the command (stdout/stderr discarded).
timeit() {
  NO_COLOR=1 perl -MTime::HiRes=time -e '
    my ($n, @cmd) = @ARGV; my @t;
    open(my $out, ">&", \*STDOUT) or die;
    open(STDOUT, ">", "/dev/null"); open(STDERR, ">", "/dev/null");
    for (1..$n) { my $s = time; system(@cmd) == 0 or die; push @t, (time - $s) * 1000 }
    @t = sort { $a <=> $b } @t;
    printf $out "median %7.1f ms   p90 %7.1f ms\n", $t[int($n/2)], $t[int($n*0.9)];
  ' "$@"
}

peak_mb() {
  if /usr/bin/time -l true >/dev/null 2>&1; then
    NO_COLOR=1 /usr/bin/time -l "$@" 2>&1 >/dev/null | awk '/maximum resident set size/ { printf "%.1f MB\n", $1 / 1048576 }'
  else
    NO_COLOR=1 /usr/bin/time -v "$@" 2>&1 >/dev/null | awk -F': ' '/Maximum resident set size/ { printf "%.1f MB\n", $2 / 1024 }'
  fi
}

report() {
  echo "== $1 ($("$1" --version 2>/dev/null | head -1))"
  printf '  startup (--version)     '; timeit "$RUNS" "$1" --version
  printf '  100 requests, sequential '; timeit "$REQ_RUNS" "$1" "$WORK/hundred.yaml"
  printf '  100 requests, parallel   '; timeit "$REQ_RUNS" "$1" "$WORK/hundred.yaml" --execution parallel
  printf '  peak memory (sequential) '; peak_mb "$1" "$WORK/hundred.yaml"
}

echo "machine: $(uname -sm)"
report "$BIN"
[ -z "$OTHER" ] || report "$OTHER"
