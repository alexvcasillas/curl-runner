# Search Console Fixes + CLI Bug Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the six CLI bugs (#136–#141) and the Search Console findings on the site, then ship them in a patch release.

**Architecture:** One branch (`fix/gsc-and-cli-issues`) and one PR. The site's feature examples are validated in CI by the CLI built from the same branch, so the validator fixes and the site examples that depend on them must land together. CLI fixes go in `crates/`. Each behaviour change gets an entry in `crates/CHANGES-FROM-TS.md` and a line in a patch changeset. Site changes go in `packages/docs`.

**Tech Stack:** Rust (curl-runner, curl-runner-core), Next.js 16 + Fumadocs (packages/docs), Vitest, pnpm, changesets.

**Spec:** the issues #136–#141 (repro, expected, where) and the Search Console findings below. Site design rules: `docs/superpowers/specs/2026-10-28-website-design.md`.

## Global Constraints

- Rust: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` green before every commit (AGENTS.md).
- Site: `pnpm --filter @curl-runner/docs test`, `pnpm typecheck`, `pnpm --filter @curl-runner/docs build` green; feature examples pass `curl-runner validate --strict` (the `examples` test with `CURL_RUNNER_BIN`).
- Every change ships with tests. One-line Conventional Commits; `Fixes #N` goes in the PR body, not in commits.
- Public repo: no employer or customer details anywhere.
- No invented claims on the site; titles and descriptions describe what the page really covers.
- Behaviour changes are logged in `crates/CHANGES-FROM-TS.md` (bug fix vs 1.x where 1.x had the same bug).

## Review Focus

1. **Validator, reverse direction:** a key the runner ignores must not become "valid" while fixing false positives. Pinned in Task 1 (the model-field test also asserts a made-up key still warns).
2. **Interpolated values of every numeric field, not just `timeout`** (`retry.count`, `delay`, `maxConcurrency`), e.g. `${VAR}` and `${VAR:500}`. Pinned in Task 2.
3. **`convert` round trip:** curl → YAML → run must send the same body bytes, including JSON arrays, nested objects, and non-JSON `-d` with a JSON content type. Pinned in Task 3.
4. **`global.http2` against an explicit request or defaults setting:** request `http2: false` beats `global.http2: true`; the CLI `--http2` beats both. Pinned in Task 5.
5. **Combined short flags:** `-vc` sets verbose and continue-on-error; `-c` with `--continue-on-error=false` is not possible (no `=` syntax), so only presence matters. Pinned in Task 6.

---

### Task 1: validate — no false "Unknown key" for supported keys (#136)

**Files:** `crates/curl-runner/src/validate_cmd/{global,request}.rs`, `crates/curl-runner/src/validate_cmd/tests.rs`

- [ ] **Failing tests:** a YAML using every field of `RetryConfig`, `ProfileConfig`, `SnapshotConfig`, `DiffConfig` and `GlobalConfig.env` / `security` (serde camelCase names from `crates/curl-runner-core/src/model/*.rs`) gives 0 warnings under `--strict`; each config also with one made-up key (`bogusKey`) still gives exactly one Unknown-key warning.
- [ ] **Implement:** bring each known-key list in line with the model (`retry`: `retryableStatuses`; `profile`: `histogram`, `exportFile`, plus any other model field; `snapshot` / `diff`: their model fields; `global`: `env`, `security`, with their nested keys checked). Add a unit test per config section that lists the model's serde field names (a `const` array in the test) and asserts the validator accepts exactly that set, so they can't drift silently.
- [ ] **Verify** with the issue's repro file: 0 warnings. **Commit** `fix(validate): accept every key the config model supports`.

### Task 2: validate — interpolated numeric values (#137)

**Files:** `crates/curl-runner/src/validate_cmd/request.rs` (+ global numeric fields), tests.

- [ ] **Failing tests:** `timeout: ${TIMEOUT}`, `timeout: "${TIMEOUT:5000}"`, `retry.count: ${N}`, `retry.delay: ${D}`, `global.maxConcurrency: ${C}` → no error. `timeout: abc` → still the positive-number error; `timeout: -1` → still an error.
- [ ] **Implement:** a numeric field whose string value contains an interpolation (`${...}`, same pattern the interpolator recognises; reuse it from curl-runner-core) is accepted as "resolved at run time", optionally as an info note under `--verbose`, never a warning or error.
- [ ] **Commit** `fix(validate): accept interpolated numeric values`.

### Task 3: convert — JSON `-d` becomes the body (#138)

**Files:** `crates/curl-runner-core/src/convert/*.rs` (IR → YAML), `crates/curl-runner-core/src/convert/tests.rs`

- [ ] **Failing tests:**
  - `convert curl "curl -X POST URL -H 'Content-Type: application/json' -d '{\"name\":\"Ada\"}'"` → YAML `body: { name: Ada }` (no `json:` key).
  - A JSON array and a nested object convert the same way.
  - A round trip: parse the generated YAML with the real parser and build the request; the body bytes equal the original JSON, compared semantically (`serde_json::Value`).
  - Non-JSON data with a JSON content type stays a string body.
- [ ] **Implement** the fix. Also check the reverse path (YAML → curl) still emits `-d` with the JSON, and update any golden files under `crates/curl-runner/tests/golden/convert/` whose output changes (review each diff).
- [ ] **Commit** `fix(convert): emit json request bodies directly`.

### Task 4: convert — `--retry` and `--retry-delay` (#139)

**Files:** `crates/curl-runner-core/src/convert/curl_parser.rs`, IR, IR → YAML, tests.

- [ ] **Failing tests:**
  - `curl --retry 3 URL` → `retry: { count: 3 }`.
  - `--retry 3 --retry-delay 2` → `retry: { count: 3, delay: 2000 }`. curl's delay is in seconds; the YAML is in milliseconds.
  - `--retry-delay` without `--retry` → a warning, not dropped silently.
  - A non-numeric value → a warning.
- [ ] **Implement:** parse into the IR and emit `retry`; YAML → curl emits `--retry` / `--retry-delay` (rounded to seconds, with a warning when that loses precision), replacing the "retry config simplified" note where the mapping is exact.
- [ ] **Commit** `feat(convert): map curl --retry and --retry-delay`.

### Task 5: `global.http2` applies to requests (#140)

**Files:** `crates/curl-runner-core/src/config.rs` (resolution), tests (config + executor/transport).

- [ ] **Failing tests:**
  - With `global.http2: true`, every request resolves `http2 = true` and `--dry-run` prints `--http2`.
  - A request with `http2: false` overrides it.
  - `global.defaults.http2` behaves as today.
  - The CLI `--http2` still wins.
  - Precedence: CLI > request > `global.defaults.http2` > `global.http2`.
- [ ] **Implement:** treat `global.http2` as a default for `defaults.http2` when that's unset. Update `content/docs/features/http2.mdx` and `connection-pooling.mdx` to document `global.http2` as working (remove the "use defaults" workaround notes).
- [ ] **Commit** `fix(config): apply global.http2 to every request`.

### Task 6: `-c` is `--continue-on-error` (#141)

**Files:** `crates/curl-runner/src/args.rs` (+ its tests), `crates/CHANGES-FROM-TS.md`, docs mentioning `-c`.

- [ ] **Failing tests:** replace `short_c_is_a_noop` with: `-c` → `continue_on_error = Some(true)`; `-vc` → verbose and continue-on-error. Add a CLI test: a file with a failing then a passing request, run with `-c`, runs both (exit per `--strict-exit` rules).
- [ ] **Implement**; log in CHANGES-FROM-TS.md ("1.x ignored -c although `--help` listed it / bug fix").
- [ ] **Commit** `fix(cli): make -c an alias of --continue-on-error`.

### Task 7: Search Console — titles, descriptions and the retry page

**Files:** `packages/docs/lib/features-data.ts`, `packages/docs/app/(marketing)/features/[slug]/page.tsx` (metadata from the data), tests under `packages/docs/tests/features/`.

- [ ] **Failing tests:** each feature can define its own `seoTitle` (falls back to `title`). Asserted values:
  - `convert` → `Convert curl to YAML (and back) — curl-runner`;
  - `retry-mechanism` → `Retry failed curl requests with backoff — curl-runner`;
  - `file-uploads` → `Upload files with multipart form data — curl-runner`.

  Each description is 150 characters or fewer and names the main query: "convert curl", "retry", "upload files".
- [ ] **Implement** the field and the metadata. On the retry feature page, add a short, factual section "From curl --retry": curl's `--retry N` / `--retry-delay S` next to the YAML equivalent (link to curl's man page), and what curl-runner adds (per-request policy, backoff, `retryableStatuses`). Restore `retryableStatuses: [429, 500, 502, 503]` in the retry example now that the validator accepts it (Task 1).
- [ ] **Verify:** the examples test with `CURL_RUNNER_BIN` built from this branch passes. **Commit** `feat(docs): sharpen search titles for convert, retry and uploads`.

### Task 8: Search Console — indexing signals

**Files:** `packages/docs/app/sitemap.ts`, `packages/docs/app/robots.*`, feature/docs page templates, `scripts/check-site-urls.sh`, tests.

- [ ] **Failing tests:**
  - `robots` allows `/`, disallows only `/api/`, and names `https://www.curl-runner.com/sitemap.xml`.
  - Each `/features/<slug>` page links to `/docs/features/<slug>`.
  - Each `/docs/features/<slug>` page links back to `/features/<slug>` (an MDX component or the docs page footer).
  - Every sitemap entry has a `lastModified`.
  - No page's canonical points at another URL.
- [ ] **Implement:**
  - Add the docs → marketing back-link, via a small "Feature overview →" link that the docs template adds automatically for feature pages.
  - Add `BreadcrumbList` JSON-LD to docs and feature pages, with factual fields only.
- [ ] **Redirects:** extend `scripts/check-site-urls.sh` so `http://curl-runner.com/`, `http://www.curl-runner.com/` and `https://curl-runner.com/` each reach `https://www.curl-runner.com/` in one permanent redirect (301/308).
- [ ] **Commit** `feat(docs): strengthen indexing signals`.

### Task 9: changeset and docs sweep

**Files:** `.changeset/*.md`, `content/docs/**` mentioning the fixed bugs, `crates/CHANGES-FROM-TS.md`.

- [ ] **Write** a patch changeset for `@curl-runner/cli`. Its first line is the bold headline `**Validator, convert and flag fixes**`, followed by one line per fix (#136–#141).
- [ ] **Docs sweep:** remove every docs note that works around these bugs, which the Task 11–13 reports listed: the `-c` note, the strict false-positive notes, the `timeout: ${VAR}` caveat, `global.http2` and `convert` body notes. Make sure examples now use the natural form.
- [ ] **Verify:** full Rust + site checks green. **Commit** `chore: add changeset for cli fixes`.

### Release (controller, not a subagent)

1. Push, open the PR ("Fixes #136 … #141"), wait for checks, merge (squash). Run the release smoke run first if `release.yml` or the scripts changed.
2. Changesets opens "Version Packages": confirm it bumps to 2.1.1 with the Cargo sync, then merge it. Its checks don't run on bot PRs, so merge with admin bypass, as with #128 and #134.
3. Watch the release run. Confirm:
   - npm (6 packages, provenance on `@curl-runner/cli` through pnpm);
   - the GitHub Release assets;
   - crates.io (trusted publishing);
   - Vercel production Ready.
4. Run the tap workflow; check `brew` gets the new version.
5. Run `scripts/check-site-urls.sh https://www.curl-runner.com`.
6. Close #136–#141 if the merge didn't.

## Unresolved questions

- Owner-only (Search Console UI): submit sitemap, request indexing, "Validate fix" — can't be automated here.
