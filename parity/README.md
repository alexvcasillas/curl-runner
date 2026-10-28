# TS vs Rust parity harness

Runs every case in `parity/cases/` with a TS 1.x reference binary (`TS_CMD`)
and the Rust CLI, for each flag set, against one local `testkit` server, and compares
the exit code and the normalized `--output` JSON (plus the normalized stdout for
`--dry-run`, where the printed curl commands are the contract).

```
cargo build -p curl-runner            # the harness builds it if missing
export TS_CMD=/path/to/curl-runner-1.x   # required, see below
parity/run.sh                         # all cells
parity/run.sh -v -k conditional -f parallel   # filter by case substring / flag set, show diffs
KEEP=1 parity/run.sh                  # keep per-cell ts/rust outputs (path printed at the end)
```

The TS sources were removed in 2.0 (the harness is no longer part of CI), so the reference is a
prebuilt TS 1.x binary, passed by path in `TS_CMD`. Options:

- the 1.25.0 release binary of `@curl-runner/cli` (GitHub release `@curl-runner/cli@1.25.0`, archive for your platform);
- build one from a pre-2.0 checkout (the TS sources no longer exist on 2.0 branches), e.g. `git worktree add ../curl-runner-1x 7670118 && cd ../curl-runner-1x && bun install && cd packages/cli && CURL_RUNNER_VERSION=1.25.0 bun build --compile src/cli.ts --outfile <path>` (the same command, run in `packages/cli`, produced the reference binary for the final parity run).

Env: `TS_CMD` (required), `RUST_BIN`, `RUN_TIMEOUT` (seconds per run, default 30), `ALLOW_FILE`. Exit 1 when any cell fails (below), 2 on setup errors.

A cell fails when: either CLI crashes or hangs (exit 124/126/127/>=128; never allow-listable), either side writes no `--output` file or invalid JSON, the testkit server died, or the differences are not exactly an allow-listed signature.

Flag sets: `default`, `--continue-on-error`, `--strict-exit`, `--execution parallel`, `--dry-run`.

## Files

- `run.sh` harness; starts `cargo run -p testkit --bin testkit-server -- --port 0`
  equivalent (the built binary) and exports `BASE_URL` / `BASE_HOST`.
- `normalize.jq` drops `duration`, `metrics`, `updatedAt`, `capturedAt`; masks UUIDs,
  ISO dates, RFC 1123 dates, epoch-ms timestamps, times of day, multipart boundaries and
  measured times in validation errors. Keys are sorted (`jq -S`): key order is not part
  of the contract (CHANGES-FROM-TS.md).
- `expected-diffs.txt` allow-list, one `case|flagset|signature|reason` per line (exact names, no globs). The signature is the exact set of differences the cell may have (`exit`, `out:<top-level key>`, `stdout`, `noout-ts`, `noout-rust`), so a second unrelated diff in the same cell still fails. Each reason cites the CHANGES-FROM-TS.md entry; unused entries are listed as notes.
- `cases/` copies of `packages/cli/examples/*.yaml` (+ `sample-file.txt`), plus `nested/` (YAMLs in a subdirectory, run from the cases root, e.g. a cwd-relative upload), plus the snapshot cases `snapshot-compare`, `snapshot-ci`, `snapshot-update`, `snapshot-update-failing` and their seeded `__snapshots__/` files (each cell runs in a fresh copy, so stored snapshots are compared and rewritten per cell without leaking).

## How the cases differ from the examples

URLs are rewritten: `https://jsonplaceholder.typicode.com` -> `${BASE_URL}/jp`,
`https://httpbin.org` -> `${BASE_URL}/httpbin`, `http://localhost:3000` -> `${BASE_URL}`
(the testkit server mimics these, see `crates/testkit/src/lib.rs`). Other edits, all
to keep results deterministic:

- `validation-headers.yaml`: `host: "localhost:3000"` -> `"${BASE_HOST}"`.
- `parallel.yaml`: `${API_URL}/...` -> `${BASE_URL}/api/test/...` (nested variables are
  not resolved by either CLI), `delay/2` -> `delay/0`.
- `test-with-retry.yaml`: `delay/1` -> `delay/100` (so the `timeout: 2` unit fix shows
  deterministically).
- `timeout-with-retry.yaml`: endpoint `delay/300`, `timeout: 100`, `retry.delay: 50` (ms), so the request really times out and retries in Rust.
- `response-time-validation.yaml`: `"> 10, < 3000"` -> `"> 0, < 3000"` (a local server
  answers in under 10 ms; TS only passes by counting process spawn time).

The testkit `/api/test/*` mock echoes a `curl/<version>` User-Agent as `curl`, since the
TS CLI uses the system curl and Rust a different libcurl.

## Profile cases

`performance-profiling.yaml` runs with every flag set (normal mode: a `global.profile` block does not switch the mode) and with two extra flag sets, `profile` and `profile-concurrent`, that use `--profile`/`--profile-export out.json`. `normalize.jq` masks the measured latencies in those export files and keeps iteration counts, failures and failure rates; the exit code is compared as usual.

## Excluded cases (add back with their features)

| Example | Reason | Task |
|---|---|---|
| `response-diffing-basic.yaml`, `response-diffing-environments.yaml` | use httpbin.org; replaced by the local `response-diffing-*` cases (seeded `__baselines__`) | 13 |
| `--watch` | not ported, interactive | 14 |

## Results

Final run before the 2.0 merge (2026-10-28, `TS_CMD` = a macOS `bun build --compile` of the 1.25.0 TS sources at the base commit, Rust debug build, macOS arm64):

```
cells: 309  ok: 282  allowed: 27  known-bug: 0  unlisted diffs: 0
```

309 cells = 61 cases (incl. `nested/`) x 5 flag sets, plus 2 profile flag sets for the profile cases. The 27 allow-listed cells are the deliberate changes in `crates/CHANGES-FROM-TS.md` (Precedence, Request timeout units, Transport failures, Dry run with store, `--output` aggregation); one allow-list entry (`snapshot-update.yaml|parallel|snapfiles`) is unused by this run and is listed as a note.
