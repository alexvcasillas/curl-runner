#!/usr/bin/env bash
# Differential parity harness: runs every case in parity/cases with the TS CLI
# (a 1.x binary) and the Rust CLI over a set of flag sets, against a local testkit server,
# and compares exit code and normalized --output JSON.
#
# Usage: parity/run.sh [-k <case-substring>] [-f <flagset-name>] [-v]
#   RUST_BIN   Rust binary (default: target/debug/curl-runner, built if missing)
#   TS_CMD     TS 1.x reference binary path (required), e.g. a downloaded
#              @curl-runner/cli 1.25.0 release binary or `bun build --compile` of the TS sources at a pre-2.0 commit
#   KEEP=1     keep the scratch directory
# Every cell must produce an out.json on both sides, valid JSON, and neither CLI may
# crash or hang (exit 124/126/127/>=128). Those failures can never be allow-listed.
# Other differences are tokens (exit, out:<top-level key>, stdout, snapfiles, basefiles, noout-ts, noout-rust,
# badjson-ts, badjson-rust) joined with "+" in sorted order; a cell passes when it has
# none, or when parity/expected-diffs.txt lists that exact case|flagset|signature.
# Exit 0 only when no cell fails; exit 1 otherwise (2 for setup errors).
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CASES="$ROOT/parity/cases"
ALLOW="${ALLOW_FILE:-$ROOT/parity/expected-diffs.txt}"
NORM="$ROOT/parity/normalize.jq"
RUST_BIN="${RUST_BIN:-$ROOT/target/debug/curl-runner}"
TS_CMD="${TS_CMD:-}"
[ -n "$TS_CMD" ] || { echo "set TS_CMD to a TS 1.x curl-runner binary (see parity/README.md)" >&2; exit 2; }
FILTER=""
ONLY_FLAGSET=""
VERBOSE=0
while getopts "k:f:v" o; do
  case $o in
    k) FILTER=$OPTARG ;;
    f) ONLY_FLAGSET=$OPTARG ;;
    v) VERBOSE=1 ;;
    *) echo "usage: $0 [-k substr] [-f flagset] [-v]" >&2; exit 2 ;;
  esac
done

# name|flags (flags are word-split)
FLAGSETS=(
  "default|"
  "continue-on-error|--continue-on-error"
  "strict-exit|--strict-exit"
  "parallel|--execution parallel"
  "dry-run|--dry-run"
)

# Extra flag sets for the profile cases (a `global.profile` block alone does not
# switch to profile mode, in TS or Rust). Their out.json is the --profile-export
# file; parity/normalize.jq masks the measured latencies.
PROFILE_FLAGSETS=(
  "profile|--profile 6 --profile-warmup 2 --profile-export out.json"
  "profile-concurrent|--profile 8 --profile-concurrency 3 --profile-histogram --profile-export out.json"
)

for t in jq; do
  command -v "$t" >/dev/null || { echo "missing tool: $t" >&2; exit 2; }
done
[ -x "$RUST_BIN" ] || (cd "$ROOT" && cargo build -p curl-runner) || exit 2
[ -x "$RUST_BIN" ] || { echo "RUST_BIN not executable: $RUST_BIN" >&2; exit 2; }
(cd "$ROOT" && cargo build -q -p testkit --bin testkit-server) || exit 2
SERVER_BIN="$ROOT/target/debug/testkit-server"

# Per-run time limit (30 s) (a hang shows up as exit 124/142 and fails the cell).
TIMEOUT=(perl -e 'alarm $ENV{RUN_TIMEOUT}||30; exec @ARGV' --)
command -v timeout >/dev/null && TIMEOUT=(timeout "${RUN_TIMEOUT:-30}")
command -v gtimeout >/dev/null && TIMEOUT=(gtimeout "${RUN_TIMEOUT:-30}")

SCRATCH="$(mktemp -d)"
SERVER_PID=""
cleanup() {
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null
  [ "${KEEP:-0}" = 1 ] && echo "scratch kept: $SCRATCH" || rm -rf "$SCRATCH"
}
trap cleanup EXIT

"$SERVER_BIN" --port 0 >"$SCRATCH/server.log" 2>&1 &
SERVER_PID=$!
for _ in $(seq 1 100); do
  grep -q '^listening ' "$SCRATCH/server.log" 2>/dev/null && break
  sleep 0.1
done
BASE_URL="$(sed -n 's/^listening //p' "$SCRATCH/server.log")"
[ -n "$BASE_URL" ] || { echo "testkit server failed to start" >&2; cat "$SCRATCH/server.log" >&2; exit 2; }
export BASE_URL BASE_HOST="${BASE_URL#http://}" NO_COLOR=1

# Allow-list: "case|flagset|signature|reason" (exact names, no globs); '#' comments.
# A reason starting with "BUG" marks a known, not-yet-fixed bug (reported as known-bug).
ALLOW_CASE=(); ALLOW_FLAGS=(); ALLOW_SIG=(); ALLOW_REASON=(); ALLOW_USED=()
while IFS= read -r line; do
  case "$line" in ''|'#'*) continue ;; esac
  IFS='|' read -r c f g r <<<"$line"
  if [ -z "$g" ] || [[ "$c$f$g" == *'*'* ]]; then
    echo "bad allow-list line (need exact case|flagset|signature|reason): $line" >&2; exit 2
  fi
  ALLOW_CASE+=("$c"); ALLOW_FLAGS+=("$f"); ALLOW_SIG+=("$g"); ALLOW_REASON+=("$r"); ALLOW_USED+=(0)
done <"$ALLOW"

ALLOW_NOTE=""
REASON=""
allowed_reason() { # case flagset signature -> sets REASON, returns 0 when allow-listed
  local i
  ALLOW_NOTE=""
  for i in "${!ALLOW_CASE[@]}"; do
    if [[ "$1" == "${ALLOW_CASE[$i]}" && "$2" == "${ALLOW_FLAGS[$i]}" ]]; then
      if [ "$3" = "${ALLOW_SIG[$i]}" ]; then
        ALLOW_USED[$i]=1; REASON="${ALLOW_REASON[$i]}"; return 0
      fi
      ALLOW_NOTE=" (allow-list expects signature ${ALLOW_SIG[$i]})"
    fi
  done
  return 1
}

# run_one <impl> <case> <flags...>: fresh dir with the cases, results in $dir
run_one() {
  local impl=$1 case=$2; shift 2
  local dir="$SCRATCH/$impl"
  rm -rf "$dir"; mkdir -p "$dir"; cp -R "$CASES"/. "$dir"/
  local cmd
  if [ "$impl" = ts ]; then cmd=($TS_CMD); else cmd=("$RUST_BIN"); fi
  (cd "$dir" && "${TIMEOUT[@]}" "${cmd[@]}" "$case" "$@" --output out.json >stdout.txt 2>stderr.txt </dev/null)
  echo $? >"$dir/exit.txt"
  # snapshot files written next to the cases: byte-compared after masking the
  # volatile fields (updatedAt, hash, RFC 1123 dates)
  : >"$dir/snap.norm"
  local sf
  for sf in "$dir"/__snapshots__/*; do
    [ -f "$sf" ] || continue
    { echo "== $(basename "$sf")"
      sed -E 's/("updatedAt": ")[^"]*"/\1<masked>"/; s/("hash": ")[^"]*"/\1<masked>"/; s/(Mon|Tue|Wed|Thu|Fri|Sat|Sun), [0-9]{2} [A-Z][a-z]{2} [0-9]{4} [0-9:]{8} GMT/<date>/g' "$sf"
      echo; } >>"$dir/snap.norm"
  done
  # baseline files (response diffing, any depth under __baselines__ or .bl): byte-compared
  # after masking capturedAt, hash, timing and RFC 1123 dates
  : >"$dir/base.norm"
  local bf
  while IFS= read -r bf; do
    { echo "== ${bf#"$dir"/}"
      sed -E 's/("capturedAt": ")[^"]*"/\1<masked>"/; s/("hash": ")[^"]*"/\1<masked>"/; s/("timing": )[0-9.e+-]+/\1<masked>/; s/(Mon|Tue|Wed|Thu|Fri|Sat|Sun), [0-9]{2} [A-Z][a-z]{2} [0-9]{4} [0-9:]{8} GMT/<date>/g' "$bf"
      echo; } >>"$dir/base.norm"
  done < <(find "$dir/__baselines__" "$dir/.bl" -type f 2>/dev/null | sort)
  rm -f "$dir/state"
  if [ ! -f "$dir/out.json" ]; then
    echo noout >"$dir/state"; echo "NO OUTPUT FILE" >"$dir/norm.json"
  elif jq -S -f "$NORM" "$dir/out.json" >"$dir/norm.json" 2>/dev/null; then
    echo ok >"$dir/state"
  else
    echo badjson >"$dir/state"; echo "INVALID JSON" >"$dir/norm.json"
  fi
  # stdout is only compared for dry-run (command lines); strip ANSI + volatile bits
  sed -E $'s/\x1b\\[[0-9;]*m//g; s/Duration: .*$/Duration: X/; s/\\([0-9.]+m?s\\)$/(T)/; s/Results saved to .*$//; s/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/<uuid>/g; s/(^|[^0-9])1[0-9]{12}([^0-9]|$)/\\1<epoch-ms>\\2/g; s/(^|[^0-9:])[0-9]{2}:[0-9]{2}:[0-9]{2}([^0-9:]|$)/\\1<time>\\2/g' "$dir/stdout.txt" >"$dir/stdout.norm"
}

is_fatal() { # exit code of a crash/hang/missing binary
  [ "$1" -ge 124 ] 2>/dev/null && [ "$1" != 130 ]
}

declare -a ROWS
PASS=0; ALLOWED=0; KNOWN=0; FAIL=0; TOTAL=0
FAILS=""
mkdir -p "$SCRATCH/diffs"

# nested/: cases whose YAML sits in a subdirectory (run from the cases root)
for f in "$CASES"/*.yaml "$CASES"/nested/*.yaml; do
  case_name="${f#"$CASES"/}"
  [[ -n "$FILTER" && "$case_name" != *"$FILTER"* ]] && continue
  row="$case_name"
  sets=("${FLAGSETS[@]}")
  [[ "$case_name" == performance-profiling* ]] && sets+=("${PROFILE_FLAGSETS[@]}")
  # profile-* cases only make sense in profile mode
  [[ "$case_name" == profile-* ]] && sets=("${PROFILE_FLAGSETS[@]}")
  for fs in "${sets[@]}"; do
    fs_name="${fs%%|*}"; fs_flags="${fs#*|}"
    [[ -n "$ONLY_FLAGSET" && "$fs_name" != "$ONLY_FLAGSET" ]] && continue
    TOTAL=$((TOTAL+1))
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
      echo "FATAL: testkit server died before $case_name [$fs_name]" >&2
      cat "$SCRATCH/server.log" >&2
      exit 1
    fi
    # shellcheck disable=SC2086
    run_one ts "$case_name" $fs_flags
    # shellcheck disable=SC2086
    run_one rust "$case_name" $fs_flags
    ts_rc=$(cat "$SCRATCH/ts/exit.txt"); rs_rc=$(cat "$SCRATCH/rust/exit.txt")
    ts_state=$(cat "$SCRATCH/ts/state"); rs_state=$(cat "$SCRATCH/rust/state")
    fatal=""
    is_fatal "$ts_rc" && fatal="${fatal}ts-exit-$ts_rc "
    is_fatal "$rs_rc" && fatal="${fatal}rust-exit-$rs_rc "
    tokens=()
    [ "$ts_rc" = "$rs_rc" ] || tokens+=(exit)
    [ "$ts_state" = ok ] || tokens+=("$ts_state-ts")
    [ "$rs_state" = ok ] || tokens+=("$rs_state-rust")
    if [ "$ts_state" = ok ] && [ "$rs_state" = ok ]; then
      while IFS= read -r k; do tokens+=("out:$k"); done < <(
        jq -rn --slurpfile a "$SCRATCH/ts/norm.json" --slurpfile b "$SCRATCH/rust/norm.json" \
          '($a[0]) as $x | ($b[0]) as $y | ([$x,$y] | map(if type=="object" then keys else [] end) | add | unique)[] | select($x[.] != $y[.])')
      # a non-object top level that differs
      cmp -s "$SCRATCH/ts/norm.json" "$SCRATCH/rust/norm.json" || [ "${#tokens[@]}" -gt 0 ] || tokens+=("out:<root>")
    fi
    cmp -s "$SCRATCH/ts/snap.norm" "$SCRATCH/rust/snap.norm" || tokens+=(snapfiles)
    cmp -s "$SCRATCH/ts/base.norm" "$SCRATCH/rust/base.norm" || tokens+=(basefiles)
    # stdout is also compared for the diff cases (their output is the diff report), except in
    # parallel runs where results print in completion order (nondeterministic in both CLIs)
    if [ "$fs_name" = dry-run ] || [[ "$case_name" == response-diffing-* && "$fs_name" != parallel && "$case_name" != response-diffing-compare-timings.yaml ]]; then
      cmp -s "$SCRATCH/ts/stdout.norm" "$SCRATCH/rust/stdout.norm" || tokens+=(stdout)
    fi
    sig=""
    [ "${#tokens[@]}" -eq 0 ] || sig="$(printf '%s\n' "${tokens[@]}" | sort -u | paste -sd+ -)"
    if [ -z "$fatal" ] && [ -z "$sig" ]; then
      PASS=$((PASS+1)); cell="ok"
    else
      d="$SCRATCH/diffs/${case_name}__${fs_name}"; mkdir -p "$d"
      for x in snap.norm base.norm norm.json stdout.norm stdout.txt stderr.txt exit.txt; do cp "$SCRATCH/ts/$x" "$d/ts-$x"; cp "$SCRATCH/rust/$x" "$d/rust-$x"; done
      if [ -z "$fatal" ] && allowed_reason "$case_name" "$fs_name" "$sig"; then
        if [[ "$REASON" == BUG* ]]; then KNOWN=$((KNOWN+1)); cell="known-bug"
        else ALLOWED=$((ALLOWED+1)); cell="allowed"; fi
      else
        FAIL=$((FAIL+1)); cell="DIFF"
        FAILS="${FAILS}${case_name} [${fs_name}]: ${fatal:+FATAL $fatal}${sig:+signature $sig}${ALLOW_NOTE}\n"
        if [ "$VERBOSE" = 1 ]; then
          echo "--- $case_name [$fs_name]: ${fatal}${sig}${ALLOW_NOTE}"
          diff "$d/ts-norm.json" "$d/rust-norm.json" | head -30
          diff "$d/ts-stdout.norm" "$d/rust-stdout.norm" | head -20
          [ -n "$fatal" ] && tail -5 "$d/rust-stderr.txt"
        fi
      fi
    fi
    row="$row|$fs_name=$cell"
  done
  ROWS+=("$row")
done

echo
echo "== parity matrix (cell: ok | allowed | known-bug | DIFF) =="
printf '%s\n' "${ROWS[@]}" | sed 's/|/  /g'
echo
for i in "${!ALLOW_CASE[@]}"; do
  [ "${ALLOW_USED[$i]}" = 1 ] || echo "note: allow-list entry unused: ${ALLOW_CASE[$i]}|${ALLOW_FLAGS[$i]}|${ALLOW_SIG[$i]}"
done
echo "cells: $TOTAL  ok: $PASS  allowed: $ALLOWED  known-bug: $KNOWN  unlisted diffs: $FAIL"
if [ "$FAIL" -gt 0 ]; then
  printf "\nUnlisted diffs:\n%b" "$FAILS"
  [ "${KEEP:-0}" = 1 ] || echo "(rerun with KEEP=1 to keep per-cell ts/rust outputs under the scratch dir)"
  exit 1
fi
