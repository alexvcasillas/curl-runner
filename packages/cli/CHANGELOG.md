# @curl-runner/cli

## 2.14.0

### Minor Changes

- [#237](https://github.com/alexvcasillas/curl-runner/pull/237) [`5729b97`](https://github.com/alexvcasillas/curl-runner/commit/5729b976f06cf07f8804b503968193bf62dfb2f4) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **One switcher for every collection**
  
  The sidebar header now names the open collection and switches between all of them.
  
  - **New:** click it (or ⌘K › Switch collection…) for **My collections** and **Shared with me** in one menu, plus Open folder, Open from Cloud and Open a shared link.
  - **New:** right-click a row to reveal it in Finder, remove a folder from the list, or remove a shared copy (its folder and the values you filled in, after asking).
  - **Improved:** the list under the tree that changed with what was open is gone.

- [#236](https://github.com/alexvcasillas/curl-runner/pull/236) [`c6f48e2`](https://github.com/alexvcasillas/curl-runner/commit/c6f48e2dc86d5434c59cf3637eb7007376663297) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **List any public collection in the gallery**
  
  A collection that syncs from the studio can now be listed too.
  
  - **New:** Share (studio) and Sharing (dashboard) take the gallery description directly: write it, save it, then switch on **List in the gallery**. Edit it any time; clearing it takes the collection out of the gallery.
  - **Improved:** a publish whose `cloud:` block has a description still sets it, as before.

## 2.13.1

### Patch Changes

- [#234](https://github.com/alexvcasillas/curl-runner/pull/234) [`8d1d77f`](https://github.com/alexvcasillas/curl-runner/commit/8d1d77fb5e6e083aa66bf99e9a9350bf019eeab1) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Studio fixes after 2.13.0**
  
  - **Fixed:** `curl-runner.yaml` is checked as a config file: only YAML syntax errors show, not "Missing request(s)".
  - **Fixed:** adding a shared collection from a link (the website's Open in curl-runner studio) closes the dialog. Before, it could stay open for good with Close doing nothing, and each retry added another copy.
  - **Fixed:** a one-line toast's icon lines up with its text.

## 2.13.0

### Minor Changes

- [#230](https://github.com/alexvcasillas/curl-runner/pull/230) [`642ea19`](https://github.com/alexvcasillas/curl-runner/commit/642ea19719063bf522e095a1137f134d0be5e5e7) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **The studio syncs collections whose source is Cloud**
  
  A collection set to sync from the studio now syncs both ways with your team on curl-runner Cloud and self-hosted servers.
  
  - **New:** saves go up a moment later, and teammates' changes come down on open and at the interval you pick in Settings › Accounts & sync › Check for team changes (1, 5 or 15 minutes, or only on open).
  - **New:** a file changed on both sides opens the conflict screen with your teammate's version, who made it and when.
  - **Improved:** the sidebar foot of a synced collection shows its sync state and opens the collection's Cloud sharing from Share.

- [#233](https://github.com/alexvcasillas/curl-runner/pull/233) [`6b79907`](https://github.com/alexvcasillas/curl-runner/commit/6b79907b4dd9f986ae5908ea935ae0282f7c2584) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Open your team's synced collections on any Mac**
  
  Synced collections follow you and your teammates to every studio.
  
  - **New:** **Open from Cloud…** (palette, Welcome screen, or a team in the account menu) lists a team's collections and opens a synced one into a folder under `~/curl-runner/{team}` that syncs with it.
  - **New:** a collection's settings show where it lives on Cloud, and team owners can switch it between publishing and syncing, with the same warnings as the dashboard. Members see the source but can't change it.

- [#231](https://github.com/alexvcasillas/curl-runner/pull/231) [`6cf7383`](https://github.com/alexvcasillas/curl-runner/commit/6cf73830838383a4a00e7c4be91165f89ade4a33) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Publish or sync, your choice**
  
  Putting a folder on curl-runner Cloud now asks how it stays up to date.
  
  - **New:** pick **Publish versions** (the folder is the source, best in a git repo) or **Sync with my team** (Cloud is the source, every save syncs both ways). Outside a git repo the studio suggests Sync, and the sidebar's button reads **Put on Cloud**.
  - **Improved:** a synced collection's sidebar foot shows its version and who changed it last, such as `v27 · sam@acme.dev edited 1 h ago`.

## 2.12.0

### Minor Changes

- [#226](https://github.com/alexvcasillas/curl-runner/pull/226) [`42e54c5`](https://github.com/alexvcasillas/curl-runner/commit/42e54c5d00a41f5049473766347e1a20c544a9e4) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **List public collections in the gallery from the studio**
  
  A public collection on curl-runner Cloud can be listed on curl-runner.com/collections straight from the Share dialog.
  
  - **New:** "List in the gallery" under Public in Share, with a link to the collection's page once listed.
  - **Improved:** without a description the switch says how to add one (the `cloud:` block in `curl-runner.yaml`); Off or Restricted takes the collection out of the gallery.

## 2.11.0

### Minor Changes

- [#221](https://github.com/alexvcasillas/curl-runner/pull/221) [`7cfa309`](https://github.com/alexvcasillas/curl-runner/commit/7cfa30994fe37beaa1c9aab03ccba9a74671dd87) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Share published collections from the studio**
  
  Share sets who else can open a published collection, with the same settings as the dashboard's Sharing page.
  
  - **New:** Share at the foot of the sidebar, in the palette, and from the title bar's chip ("Shared · 3", "Public").
  - **New:** Off, Public (anyone with the link) or Restricted (people on the list, after they sign in; Team plan).
  - **New:** copy, reset and set the link to expire in 7 or 30 days; add people by email or a whole domain like `*@globex.com`, and see who has opened it.
  - **New:** moving a public collection to Restricted, and resetting the link, ask first. A Free team sees Restricted locked, with Upgrade.

## 2.10.2

### Patch Changes

- [#219](https://github.com/alexvcasillas/curl-runner/pull/219) [`d086efd`](https://github.com/alexvcasillas/curl-runner/commit/d086efdf6d71e6660ff358402bf3bd5b29f36c5d) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Publishing from a git folder records where it came from**
  
  The studio sends the folder's git details with each version, as CI does, so Cloud can say where a collection comes from.
  
  - **Improved:** a publish from a git repository carries its commit, branch and repository. The sidebar foot and the dashboard show them (`main@4f2a9c1`).
  - **Improved:** uncommitted changes leave the commit out, and credentials in the remote URL are never sent.
  - **Improved:** with a commit to show, the sidebar foot reads "2 hours ago · main@4f2a9c1", without who published it.

## 2.10.1

### Patch Changes

- [#217](https://github.com/alexvcasillas/curl-runner/pull/217) [`59f1ec7`](https://github.com/alexvcasillas/curl-runner/commit/59f1ec7df273d9cfb6708f466aaafd34a422ea25) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **The publish foot reads in two lines**
  
  The sidebar foot says the version on one line and when and by whom on the next.
  
  - **Improved:** "Published v14", then "2 hours ago by CI · main@4f2a9c1" below it.

## 2.10.0

### Minor Changes

- [#216](https://github.com/alexvcasillas/curl-runner/pull/216) [`3149488`](https://github.com/alexvcasillas/curl-runner/commit/31494887579a8b476522b32078d7a48df59a1e2e) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Publish to curl-runner Cloud from the studio**
  
  The studio publishes a collection folder the way `curl-runner publish` does, signed in as you, and shows where it stands at the foot of the sidebar.
  
  - **New:** Publish at the foot of the sidebar and in the palette. The first time, pick the team and the collection's name, suggested from the folder.
  - **New:** the same checks as the CLI run on your Mac first: YAML errors and secret values in the files stop the publish, each with a link to the file.
  - **New:** optionally saves the collection's name to `curl-runner.yaml`, so CI publishes to the same collection.
  - **New:** the sidebar foot shows the published version, who published it and when, and how many files changed since.
  - **New:** clear refusals for a collection that syncs from the studio, a plan limit (with Upgrade), and an account without a team.

### Patch Changes

- [#214](https://github.com/alexvcasillas/curl-runner/pull/214) [`ee31d7d`](https://github.com/alexvcasillas/curl-runner/commit/ee31d7d7a68d5bb0b2abba6f3cb9a1ae474ab831) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Teams get their own section in the account menu**
  
  Each team you're in is listed on its own row, as designed.
  
  - **Improved:** the account menu's Teams section shows each team's initial, name, your role and its plan ("Acme · Owner · Team").

## 2.9.0

### Minor Changes

- [#207](https://github.com/alexvcasillas/curl-runner/pull/207) [`481c258`](https://github.com/alexvcasillas/curl-runner/commit/481c258abdd025198181ed583ca75bcde85b6d64) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **The studio knows your Cloud teams**
  
  The account menu shows the teams you're in, the first step towards publishing from the studio.
  
  - **New:** the account menu lists your teams and your role in each ("Owner of Acme").
  - **Improved:** opening Settings › Accounts & sync checks your account with the server, so a team you just joined shows up.

### Patch Changes

- [#211](https://github.com/alexvcasillas/curl-runner/pull/211) [`c3d4bd8`](https://github.com/alexvcasillas/curl-runner/commit/c3d4bd8335a04c56f48fdb82505a742bc9f2a165) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **The status bar names the server, the avatar the account**
  
  The status bar no longer repeats the account the title bar's avatar already shows.
  
  - **Improved:** the status bar shows only the server (`Cloud`, your server's host, or `Offline`); clicking it opens Settings › Accounts & sync.
  - **Improved:** it uses the status bar's muted text and brightens on hover.

- [#210](https://github.com/alexvcasillas/curl-runner/pull/210) [`6c2e2ea`](https://github.com/alexvcasillas/curl-runner/commit/6c2e2ea8768fead9355f57cde973e79ed8fb905e) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Sync now no longer looks like it works when it can't**
  
  Collections don't sync with curl-runner Cloud yet, so the studio says that instead of showing a button that does nothing.
  
  - **Fixed:** on a server that can't sync collections yet, Settings › Accounts & sync reads "Sync is coming soon" and turns Sync now off.

## 2.8.0

### Minor Changes

- [#200](https://github.com/alexvcasillas/curl-runner/pull/200) [`448aaa4`](https://github.com/alexvcasillas/curl-runner/commit/448aaa48ce128e1e7d1312054a2395f5c62a98a2) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **See who you're signed in as in the studio**
  
  The studio shows your Cloud account in the title bar and the status bar, with a menu to switch servers or sign out.
  
  - **New:** an avatar in the title bar opens the account menu: who you are, the server, other servers you're signed in to, Accounts & sync, Change server and Sign out.
  - **New:** the status bar shows the account and server, and opens the same menu.
  - **Improved:** when a server ends your session, the studio says so and offers Sign in again instead of quietly showing you as signed out.

- [#202](https://github.com/alexvcasillas/curl-runner/pull/202) [`e5ab2ba`](https://github.com/alexvcasillas/curl-runner/commit/e5ab2bac20f5c75753a12acdd83488d5bd0d0958) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Sign in to curl-runner Cloud from the studio**
  
  The studio now ships with its Cloud features on: sign in, and open the collections teams share with you.
  
  - **New:** sign in to curl-runner Cloud or a self-hosted server from the studio, through your browser. Sessions stay in the macOS Keychain, one per server.
  - **New:** open a shared collection from its page or by pasting the link, and take its updates as they're published.
  - **Improved:** Share says sharing from the studio is coming soon instead of failing. Until then, publish with `curl-runner publish`.

## 2.7.1

### Patch Changes

- [#188](https://github.com/alexvcasillas/curl-runner/pull/188) [`90bd0d6`](https://github.com/alexvcasillas/curl-runner/commit/90bd0d6c521192b2c9de829ce9e18cde4bf210d2) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Clearer errors for suspended accounts, teams and links**
  
  When a Cloud admin suspends an account or a team, or turns off a share link, the CLI now says so.
  
  - **Improved:** `login` says your account is suspended, so you know to contact support instead of trying again.
  - **Improved:** `publish` to a suspended team says the team is suspended and exits with `2`, like other access problems.
  - **Improved:** `pull` of a link that no longer works says it may have been turned off by an admin, as well as reset or expired.

## 2.7.0

### Minor Changes

- [#185](https://github.com/alexvcasillas/curl-runner/pull/185) [`b6abe56`](https://github.com/alexvcasillas/curl-runner/commit/b6abe56dc4a12c90badf58a84208c0aea6d50bbe) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Sign in through your browser**
  
  Sign the CLI in with the same account you use on the dashboard.
  
  - **New:** `curl-runner login --browser` signs in with GitHub, Google or email in your browser.
  - **Improved:** self-hosted servers advertise their dashboard URL, so `login --browser` finds it without extra setup.

## 2.6.0

### Minor Changes

- [#176](https://github.com/alexvcasillas/curl-runner/pull/176) [`52350b9`](https://github.com/alexvcasillas/curl-runner/commit/52350b9c427cb2a52665b11adaa189c76fb01815) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Cloud publishing from the CLI**
  
  Publish a collection to curl-runner Cloud from CI, and pull a shared one onto your machine. Cloud is in private preview.
  
  - **New:** `curl-runner publish` sends a folder to Cloud as a new version. It checks every file first, and `--dry-run` shows what would change.
  - **New:** `curl-runner pull <link>` downloads a shared collection, and `curl-runner pull <dir>` updates it.
  - **New:** `curl-runner login` and `curl-runner logout` sign in with an emailed code and sign out.
  - **New:** a GitHub Action, `alexvcasillas/curl-runner/actions/publish`, installs the CLI and publishes on every push.

## 2.5.0

### Minor Changes

- [#172](https://github.com/alexvcasillas/curl-runner/pull/172) [`3f1fbaa`](https://github.com/alexvcasillas/curl-runner/commit/3f1fbaa3360767c9f555202c13c5a8f62d4fe3ad) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Import OpenAPI specs in the studio**

  - **New:** The studio's Import dialog imports OpenAPI 3.0, 3.1 and Swagger 2.0 specs on its OpenAPI tab, with a preview and a summary of what needs your attention.
  - **Improved:** Files dropped on the Import dialog are routed to the right tab by content.

- [#170](https://github.com/alexvcasillas/curl-runner/pull/170) [`acdd9ef`](https://github.com/alexvcasillas/curl-runner/commit/acdd9ef24b0115b94e2fcbdacf278c926f07607f) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Convert OpenAPI specs to YAML**

  `curl-runner convert openapi <spec.json|spec.yaml>` turns an OpenAPI 3.0, 3.1 or Swagger 2.0 document into YAML files and a `.env`.

  - **New:** One file per operation, in a folder per tag, with sample values from the spec's examples and schemas and a check on the documented success status.
  - **New:** Optional parameters from the spec come in commented out.
  - **New:** Variables are prefixed per API, so two APIs can share one `.env`.
  - **New:** Credentials from auth schemes and secret-named parameters never land in a file. Their names are listed in `.env` and you set the values in `.env.local`.
  - **New:** `convert openapi` prints what it skipped or needs from you.

## 2.4.0

### Minor Changes

- [#164](https://github.com/alexvcasillas/curl-runner/pull/164) [`86318b1`](https://github.com/alexvcasillas/curl-runner/commit/86318b1854a97e8561085bab90e554362cc50af9) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Convert Postman collections to YAML**

  `curl-runner convert postman <collection.json>` turns a Postman collection into YAML files and a `.env`.

  - **New:** One file per request, in a folder per Postman folder. Optional fields that are off in Postman come in commented out.
  - **New:** Postman secrets never land in a file. Their names are listed in `.env` and you set the values in `.env.local`.
  - **New:** `convert postman` prints what it skipped or needs from you, such as scripts or files to pick.
  - **New:** The studio's Import dialog now takes Postman collections too, and saves their secrets to the Keychain.

## 2.3.0

### Minor Changes

- [#157](https://github.com/alexvcasillas/curl-runner/pull/157) [`7ba1f71`](https://github.com/alexvcasillas/curl-runner/commit/7ba1f71d6d318755f90f8589bbb27e4452f6844a) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Download curl-runner studio for macOS**

  - **New:** The curl-runner studio is now a download: a signed, notarized macOS app for Apple silicon and Intel, attached to each release as `curl-runner-studio-<version>.dmg`.
  - **New:** The studio works with your local YAML files out of the box, and a short walkthrough runs your first request. Sync and sharing are marked as coming soon.
  - **Fixed:** `curl-runner diff a b` now honours `diff.exclude` and `diff.match` from your config file and the file's `global` section.

## 2.2.0

### Minor Changes

- [#144](https://github.com/alexvcasillas/curl-runner/pull/144) [`9b94ce0`](https://github.com/alexvcasillas/curl-runner/commit/9b94ce034ee5ca8d35d5e5f4538d8f6618469146) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **curl-runner studio (preview)**

  - **New:** A macOS desktop app for your request files. Browse a folder as a collection, edit YAML with live validation and quick fixes, run a request or a whole collection and read the response, checks, retries and timing.
  - **New:** Run history is kept on your machine, environments come from your `.env` files and secrets live in the macOS Keychain.

  Sharing and sync run against a built-in mock server for now. The studio isn't distributed yet; build it from source with `cargo run -p curl-runner-studio`.

  The CLI is unchanged: the `validate` rules moved into the core library, with the same output and exit codes.

## 2.1.1

### Patch Changes

- [#142](https://github.com/alexvcasillas/curl-runner/pull/142) [`9305f85`](https://github.com/alexvcasillas/curl-runner/commit/9305f850cce2fbb55e43b8070bf197ccae77a1e8) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Validator, convert and flag fixes**

  - **Fixed:** `validate` accepts every key the config model supports, such as `retry.retryableStatuses`, `profile.histogram`, snapshot and diff fields, `global.env` and `security` (#136).
  - **Fixed:** Numeric request fields (`timeout`, `retry.count`, `retry.delay`, `maxRedirects` and others) accept `${VAR}` and `${VAR:default}` in `validate`. Global settings are not interpolated by the runner, so `validate` still reports `${...}` there (#137).
  - **Improved:** `validate` now warns about keys the runner ignores, such as `profile.percentiles`, snapshot and diff `ignoreFields` and `ignoreHeaders`, and `diff.baselineUrl`.
  - **Fixed:** `convert` writes JSON bodies as plain `body` values, quotes YAML safely and keeps curl's form content type (#138).
  - **Fixed:** `convert` maps `--retry` and `--retry-delay` to a matching `retry` block (#139).
  - **Fixed:** `global.http2` now applies to every request (#140).
  - **Fixed:** `-c` works as the short form of `--continue-on-error` (#141).

## 2.1.0

### Minor Changes

- [#133](https://github.com/alexvcasillas/curl-runner/pull/133) [`b5f26d3`](https://github.com/alexvcasillas/curl-runner/commit/b5f26d392216c7794ab3854b11c6d7c111230e56) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Install with Homebrew**

  - **New:** Install with `brew install alexvcasillas/tap/curl-runner` (prebuilt binaries for macOS and Linux, arm64 and x64).
  - **Improved:** `curl-runner upgrade` now detects Homebrew installs and runs `brew upgrade curl-runner` instead of refusing to upgrade them.

## 2.0.0

### Major Changes

- [#127](https://github.com/alexvcasillas/curl-runner/pull/127) [`bec01c6`](https://github.com/alexvcasillas/curl-runner/commit/bec01c646216e99a5b7618cf75e42da6d072d91a) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Rewritten in Rust, on libcurl**

  curl-runner 2.0 is a rewrite of the CLI in Rust. YAML files, flags, subcommands, environment variables, config discovery and exit codes stay the same, with the bug fixes and behavior changes listed below.

  - **Breaking:** `timeout` and `--timeout` are now honored in milliseconds, as documented. 1.x passed the value to curl as seconds. See the migration notes below.
  - **Breaking:** The `curl-runner` bin is a POSIX `sh` launcher. On Windows the npm/bun/pnpm/yarn shims therefore need `sh` on `PATH` (Git for Windows, MSYS2 or Cygwin). Without `sh`, install with `install.ps1` or run the `.exe` from the platform package directly.
  - **Breaking:** On Debian/Ubuntu images keep `ca-certificates` installed, since HTTPS uses the operating system trust store unless `ssl.ca` is set.
  - **Breaking:** Installing with `--omit=optional` leaves no binary.
  - **Breaking:** Transport failures (connection refused, DNS failure, timeout, max redirects exceeded) now fail the request and are retried. In 1.x they were reported as success with status 0 and never retried. Errors read `Request timeout after {timeout}ms`, `Connection timeout after {ms}ms` (when the pool's connect timeout is the smaller one still running), or `curl: (N) <libcurl message>`. Exhausted retries after a transport failure report that error; exhausted retriable statuses keep `Operation returned retriable result`.
  - **Breaking:** Settings now resolve as defaults < `curl-runner.yaml` < test file `global` < environment variables < CLI flags, as documented. In 1.x a test file's `global` beat CLI flags and environment variables.
  - **Breaking:** URL globbing (`{a,b}`, `[1-3]`) is no longer applied; URLs are sent literally.
  - **New:** `cargo install curl-runner --locked` installs from crates.io (`curl-runner` and its engine `curl-runner-core`). `curl-runner upgrade` detects cargo installs and reruns that command.
  - **Improved:** The CLI is now a native Rust binary with libcurl linked statically (rustls for TLS). No Bun, Node.js or `curl` binary is needed at runtime. HTTPS uses the operating system trust store unless `ssl.ca` is set.
  - **Improved:** `curl-runner upgrade` replaces the binary in-process (checksum verified). npm and bun installs still run `npm|bun install -g @curl-runner/cli@latest`.
  - **Improved:** `@curl-runner/cli` has no JavaScript and no postinstall. It ships the binary through one optional dependency per platform (`@curl-runner/cli-linux-x64`, `-linux-arm64`, `-darwin-x64`, `-darwin-arm64`, `-win32-x64`), downloaded by npm, bun, pnpm and yarn according to `os`/`cpu`.
  - **Improved:** Linux binaries are static musl builds.
  - **Fixed:** `--dry-run` prints `--max-time` in fractional seconds (`5000` is `5`, `1500` is `1.5`).
  - **Fixed:** `method: HEAD` no longer waits for a body that never comes.
  - **Fixed:** Profile mode (`--profile`): a failed iteration (transport failure) is counted in `failures` and excluded from the timings instead of counting as a successful sample. `failureRate` is measured failures divided by measured iterations (1.x divided by successful samples, which could exceed 100%). `--profile-concurrency 0` runs sequentially (1.x looped forever). A `--profile-export` write error prints `Cannot export profile results: ...` and exits 1 instead of crashing.
  - **Fixed:** Per request, settings resolve as base < file `global.defaults` < `collection.defaults` < CLI/env-derived defaults < the request itself. `retry` merges field by field (`--retries` and `--retry-delay` no longer wipe each other or the file's other retry fields), and CLI/env output flags beat the file's `global.output`.
  - **Fixed:** A `curl-runner.yaml` `global` with a wrongly typed field (for example `maxConcurrency: abc`) is skipped with a warning.
  - **Fixed:** With several test files, `--output` now holds the aggregate of all files (`total`, `successful`, `failed`, `skipped` summed, `results` concatenated in run order, `duration` the sum of the per-file durations). In 1.x only the last file's summary was saved. The file is also written on Ctrl-C with the completed results, and atomically (temp file, then rename), so an interrupted run never leaves a truncated file.
  - **Fixed:** Numbers are written like `JSON.stringify` on the wire and in files (`2`, `100000000000000000000`, not `2.0` or `1e20`).
  - **Fixed:** Parallel runs keep a sliding window of `maxConcurrency` live requests instead of waiting for the slowest of each chunk. Without a limit (unset, 0 or at least the request count) every request runs and is reported.
  - **Fixed:** On a failure without `continueOnError`, a parallel run stops starting new requests and reports every request already started (1.x reported every request of the chunks run so far).
  - **Fixed:** With `connectionPool.enabled` in parallel mode, 4xx/5xx statuses are judged only through `expect` (1.x marked them failed before `expect` ran, so `expect.status: 404` or `expect.failure` could never pass), `retry` and `maxConcurrency` apply, results are no longer truncated at the first failure, and `maxStreamsPerHost` (default 10) limits connections per host. Pool options now apply per handle in sequential runs too (1.x ignored them outside parallel mode).
  - **Fixed:** `expect.headers` with non-string values (`x-count: 5`, `x-flag: true`) are stringified and compared; 1.x always failed them.
  - **Fixed:** Body, store and condition path lookups read own keys (plus array `length` and numeric indices), not JavaScript prototype members (`toString`, `hasOwnProperty`, ...). `${constructor}` and `${toString}` in interpolation no longer resolve.
  - **Fixed:** Environment variables with non-numeric, negative (for unsigned fields) or out-of-range numeric values, and CLI numeric flags such as `--timeout abc`, are ignored instead of stored as `NaN` or negative numbers. `--keepalive-time abc` no longer force-enables the pool.
  - **Fixed:** `interpolate` failures (a value that no longer fits a typed field, for example `timeout: "${store.name}"`) fail the request with `` invalid value at `timeout`: ... `` instead of running it with the raw value. An explicit `null` in a request replaces the default, like a JS spread. `RANDOM:string:N` with an unrepresentable `N` is left unresolved instead of aborting.
  - **Fixed:** Duplicate mapping keys: the last value wins (they were a parse error). YAML scalars resolve like `Bun.YAML.parse` 1.3 (`yes/no/on/off` are booleans, `010` is 10, ...), so `insecure: yes` loads. Integers beyond the i64/u64 range are read as floats. Nesting deeper than 128 levels is a clean parse error.
  - **Fixed:** Absolute paths now work for files, directories and globs (`curl-runner /abs/path/test.yaml` used to find nothing).
  - **Fixed:** Dry run also checks that formData files exist.
  - **Fixed:** Redaction: explicit secrets are keyed by value (two tokens sharing their first 8 characters no longer overwrite each other), `api_key=`/`token=`/`secret=` values are replaced by span, and known secrets are also masked in every other printed string (bodies, header values, errors, stored values, names, JSON/raw output), and in snapshot and diff output.
  - **Fixed:** Snapshots: parallel requests no longer race on the snapshot file; a snapshot or baseline file that fails to parse is never overwritten (the comparison fails with an error naming the file) so stored snapshots are not lost; write failures are reported as a failed comparison instead of aborting the request. `--diff-compare <label>` saves the run only when the baseline file does not exist (1.x also replaced the file when no request name matched or nothing succeeded).
  - **Fixed:** `validate --fix` no longer writes YAML that fails to read back or changes meaning (empty `{}`/`[]`, strings such as `*/*`, `@x`, `yes`, `-1`); on a file whose root is not one mapping it leaves the file untouched and prints a warning instead of erasing it.
  - **Fixed:** `edit` updates only the edited request inside the full document; 1.x rewrote the file with the first request only and dropped everything else (other requests, `global`, `collection`, `store`, `when`, `snapshot`, ...).
  - **Fixed:** Wizard: the advanced options can switch `followRedirects`/`insecure`/`http2` back off, status codes must be whole numbers (`200,abc` used to write `NaN`), and `init -o <file>` is the default of quick init's `Save to` prompt. A `number` field in the interactive JSON builder that is not a number is kept as a string with a warning (1.x stored `NaN`). Wizard writes are atomic. Without a terminal the wizard reads one answer per stdin line and treats end of input as a cancel (exit 0) instead of hanging.
  - **Fixed:** Watch mode watches each file's directory, so editors that save by rename-over keep working.
  - **Fixed:** Install detection: `~/.local/bin` is checked before the npm environment variables; Homebrew `Cellar`, `~/.cargo/bin` and development builds are never self-replaced. `upgrade` verifies the SHA-256 before extracting, leaves the binary untouched on any failure, and a GitHub release tag without `x.y.z` is a failed lookup.

  ### Migration: timeouts are now milliseconds

  `timeout` and `--timeout` are documented in milliseconds, but 1.x passed the value to curl as seconds (`timeout: 5000` waited 5000 s). 2.0 honors milliseconds. Multiply any value you wrote for the old behavior by 1000: `--timeout 30` becomes `--timeout 30000`, `timeout: 2` becomes `timeout: 2000`. Values you already wrote in milliseconds, as the docs said, need no change (under 1.x those were read as seconds, so `timeout: 5000` meant about 83 minutes). `connectionPool.keepaliveTime` (seconds), `connectionPool.connectTimeout`, `retry.delay`, `--retry-delay` and `--watch-debounce` (milliseconds) are unchanged.

  ### Behavior changes

  Logged deviations that are not bug fixes (full list with reasons in `crates/CHANGES-FROM-TS.md`):

  - Colors are printed unless `NO_COLOR` is set to a non-empty value.
  - Ctrl-C cancels the run, prints the summary of completed requests and `✗ Interrupted`, writes `--output` (and `--profile-export`) and exits 130; a second Ctrl-C exits 130 at once. Watch mode exits 0.
  - Sequential requests share one connection loop, so connections stay alive between requests; DNS/TCP times print as missing for reused connections and profile latencies exclude process start-up.
  - Without the connection pool the curl tool defaults are reproduced (TCP keepalive 60 s, max 50 redirects, `User-Agent: curl/<libcurl version>`). Retry waits are clamped to 24 h.
  - Dry run with `store`: `${store.x}` stays literal in the printed command and no `Stored:` lines are printed (1.x interpolated an empty string).
  - User regexes run on `fancy-regex` (lookaround and backreferences work) with `\d \w \s \b` and `.` translated to JavaScript meanings; JS-only syntax (`\cX`, `[^]`) counts as an invalid pattern, and a pattern that exceeds the backtracking limit counts as no match.
  - The `--output` JSON keeps the same shape but uses a fixed key order. YAML parse error messages of `validate` and `convert yaml` come from a different parser and can read differently (exit codes are unchanged). Tab-indented YAML is rejected on every command.
  - Update notice: runs on a thread, prints after the command, only when stdout is a terminal, skipped when `CI` is set and for `--help`/`--version`/`upgrade`.
  - The protocol allow-list is intersected with the protocols the static libcurl supports (an unsupported entry stays blocked). `.env` values for `${VAR}` come from the process environment first, then the `.env` files chosen by the final `env.environment`.
  - `convert` quirks of the TypeScript implementation are preserved; `convert yaml` YAML parse errors read `YAML Parse error: <parser message>`. Key sorting in `convert` matches ICU `localeCompare` for ASCII and single-accent Latin letters only.

## 1.25.0

### Minor Changes

- [#124](https://github.com/alexvcasillas/curl-runner/pull/124) [`c1ce517`](https://github.com/alexvcasillas/curl-runner/commit/c1ce517195ee50c4298e55441de8225cdcf1a7d0) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Security hardening**

  This release closes several attack vectors that arise when curl-runner processes untrusted server responses (via `store` → `${store.x}` chaining) or shared YAML configs. Secure defaults are enabled out of the box, each with an explicit opt-out.

  - **Fixed:** Argument injection into curl: a `--` end-of-options separator is now emitted before the request URL, so a URL beginning with `-` can no longer be interpreted as a curl flag. Raw request bodies are sent with `--data-raw` instead of `-d`, preventing `body: "@/etc/passwd"` from reading local files.
  - **New:** URL protocol allow-list (SSRF / local file disclosure): only `http`/`https` are permitted by default. `file://`, `gopher://`, `ftp://`, etc. are rejected before curl runs, and `--proto`/`--proto-redir` prevent redirects from escaping into other protocols. Opt in with `--allow-protocol <proto>` or `CURL_RUNNER_ALLOWED_PROTOCOLS`.
  - **New:** Filesystem path confinement: `output`, `saveToFile`, and `formData` file paths are confined to the working directory by default. Opt out with `--allow-path` or `CURL_RUNNER_ALLOW_PATHS`. SSL `ca`/`cert`/`key` paths are exempt: they are TLS handshake material and are never transmitted to the server.
  - **Improved:** Secret redaction: inline credentials (`auth.token`, `auth.password`, and sensitive request headers) are now auto-registered for redaction, the saved results summary is redacted before being written to disk, and detection patterns were extended (JWT, Google API keys, `api_key=`/`token=`/`secret=` assignments).
  - **Fixed:** ReDoS and prototype pollution: user/server-controlled regex matching now caps input length, and `__proto__`/`prototype`/`constructor` keys are rejected during variable interpolation and response-path traversal.
  - **Breaking:** Configs that rely on non-`http(s)` URLs, or that read/write files outside the working directory, may need to opt in with `--allow-protocol` or `--allow-path` (or `CURL_RUNNER_ALLOWED_PROTOCOLS` / `CURL_RUNNER_ALLOW_PATHS`).

## 1.24.0

### Minor Changes

- [#122](https://github.com/alexvcasillas/curl-runner/pull/122) [`7ea38aa`](https://github.com/alexvcasillas/curl-runner/commit/7ea38aa4e6861f543b23b12cac3a2fa40f2f9cea) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **`$absent` and `$exists` assertions**

  Two new body-assertion keywords for response validation.

  - **New:** `$absent` asserts a key is not present in the response. It fails even when the value is `null`.
  - **New:** `$exists` asserts a key is present with any value, including `null`. It's stricter than `*`.

## 1.23.5

### Patch Changes

- [#119](https://github.com/alexvcasillas/curl-runner/pull/119) [`078487f`](https://github.com/alexvcasillas/curl-runner/commit/078487fcd469be9e1a426a3873d6baee321fc846) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Response headers for `expect.headers`**

  - **Fixed:** The CLI captures response headers via `-D -`, so `expect.headers` validation works.
  - **New:** `headerHistory` is exposed, with multi-value header support.

## 1.23.4

### Patch Changes

- [#115](https://github.com/alexvcasillas/curl-runner/pull/115) [`388cb1c`](https://github.com/alexvcasillas/curl-runner/commit/388cb1cdc55a9fc300fd0ec9107e9caf344ca2e5) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **`--quiet` and validation fixes**

  - **Fixed:** The CLI logger is reconfigured after config resolution, so the `--quiet` flag suppresses output.
  - **Fixed:** URL typo detection no longer gives false positives on valid https:// URLs.
  - **Fixed:** Warnings and info for valid files are shown instead of silently swallowed.
  - **Fixed:** Auto-fixes for warnings are applied on valid files.
  - **Improved:** Unknown key messages include the "did you mean" suggestion inline.
  - **Improved:** Incomplete `when` conditions are detected (missing left with operator/right present).
  - **Fixed:** Glob file discovery works for absolute paths.
  - **Improved:** The JSON body is validated when Content-Type is application/json.

## 1.23.3

### Patch Changes

- [#113](https://github.com/alexvcasillas/curl-runner/pull/113) [`b3c4051`](https://github.com/alexvcasillas/curl-runner/commit/b3c4051376c1fe173089fc6def2306473077305c) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Retries on HTTP 429 and 5xx**

  - **Fixed:** The retry mechanism now triggers on HTTP 429/5xx status codes.
  - **New:** Retry-After header support for retries.
  - **New:** Configurable retryable status codes.
  - **Fixed:** CLI defaults now propagate properly.

## 1.23.2

### Patch Changes

- [#111](https://github.com/alexvcasillas/curl-runner/pull/111) [`1df80e6`](https://github.com/alexvcasillas/curl-runner/commit/1df80e67facf1d8a9daa0f6cfeb917961084f18a) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Fewer false regex matches on plain strings**

  - **Fixed:** `isRegexPattern` no longer uses loose character checks, which prevents false positives on plain strings like "C++", "really?", "\*-", "[]".

## 1.23.1

### Patch Changes

- [#108](https://github.com/alexvcasillas/curl-runner/pull/108) [`fa60a11`](https://github.com/alexvcasillas/curl-runner/commit/fa60a11c2f1b6a9ab0311de89444e4a9d552ebb0) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Array body validation**

  - **Fixed:** Array body validation uses element-wise matching when both expected and actual values are arrays, instead of treating all arrays as "one of" enum validators.

## 1.23.0

### Minor Changes

- [#102](https://github.com/alexvcasillas/curl-runner/pull/102) [`cbd556f`](https://github.com/alexvcasillas/curl-runner/commit/cbd556fb767d227c663650b9d67c077d73e4a2dd) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Convert between curl and YAML**

  A bidirectional curl ⇄ YAML conversion engine.

  - **New:** CLI commands: `convert curl`, `convert file`, `convert yaml`.
  - **New:** A shell tokenizer with quote/escape/continuation handling.
  - **New:** A curl semantic parser supporting 40+ flags.
  - **New:** A normalization layer with method inference, body detection, auth detection and query param extraction.
  - **New:** A YAML serializer with stable key ordering and loss-aware comments.
  - **New:** A curl generator producing canonical, shell-safe commands.
  - **New:** A batch script parser for converting entire shell scripts.
  - **New:** Debug mode exposing the token stream, AST and IR.

## 1.22.0

### Minor Changes

- [#100](https://github.com/alexvcasillas/curl-runner/pull/100) [`9b6f0a7`](https://github.com/alexvcasillas/curl-runner/commit/9b6f0a799b562f67cc59c0fbd49b8de8e8a4c94a) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **.env files and secret redaction**

  - **New:** Variables load from .env, .env.local, .env.{env} and .env.{env}.local.
  - **New:** `--env` / `-e` flag to select the environment.
  - **New:** `SECRET_*` prefixed variables are automatically redacted in output.
  - **New:** Pattern-based detection for common API keys (Stripe, AWS, GitHub, NPM, Slack, Paddle, OpenAI, Anthropic).
  - **New:** `--no-redact` flag to disable redaction.
  - **New:** Config options `env.environment` and `env.redactSecrets`.

## 1.21.0

### Minor Changes

- [#98](https://github.com/alexvcasillas/curl-runner/pull/98) [`c99f88c`](https://github.com/alexvcasillas/curl-runner/commit/c99f88c00ec64d55e9ededd6408959e7311552d4) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **`validate` command**

  New `curl-runner validate` command for YAML config validation.

  - **New:** `validate` accepts file paths or glob patterns.
  - **New:** `validate` checks YAML syntax and structure.
  - **New:** `validate` checks curl-runner API correctness (methods, auth types, operators, etc.).
  - **New:** `validate` checks curl options (SSL, retry, timeout configs).
  - **New:** `validate` reports issues with severity (error/warning).
  - **New:** `validate` proposes fixes for common issues.
  - **New:** `validate --fix` auto-fixes issues (uppercase methods, https:// prefix).

## 1.20.0

### Minor Changes

- [#96](https://github.com/alexvcasillas/curl-runner/pull/96) [`022c831`](https://github.com/alexvcasillas/curl-runner/commit/022c831e2509ee23926090aa9c04b183631f1513) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **YAML wizard**

  A YAML wizard for interactive config creation.

  - **New:** `curl-runner init` for quick YAML file creation.
  - **New:** `curl-runner init --wizard` for the full interactive wizard.
  - **New:** `curl-runner edit <file>` to modify existing YAML files.
  - **New:** Templates: basic-get, basic-post, api-test, file-upload, auth-flow.
  - **New:** Preview YAML before saving, with an option to run it immediately.

  Adds 51 new tests for the wizard module.

## 1.19.1

### Patch Changes

- [#94](https://github.com/alexvcasillas/curl-runner/pull/94) [`e5a4f15`](https://github.com/alexvcasillas/curl-runner/commit/e5a4f1561ddd311d4118b1f77d19d02bbc034772) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Modular core**

  - **Improved:** Core functionality is modularized for better testability and maintainability.

  - Extract config utilities to `core/config` (CLI parser, env loader)
  - Extract curl utilities to `core/curl` (args builder, response parser, body parser)
  - Extract interpolation to `core/interpolator` (variable resolution)
  - Extract retry logic to `core/execution` (backoff, retry strategies, post-processor)
  - Extract validation to `core/validation` (response validation, body diffing)
  - Extract formatting to `core/format` (duration, size, JSON, colors, tree renderer)
  - Split types/config.ts into domain-specific modules (json, request, execution, snapshot, diff, profile, watch, global)
  - Add 400+ new unit tests for core modules

## 1.19.0

### Minor Changes

- [#92](https://github.com/alexvcasillas/curl-runner/pull/92) [`f8b7744`](https://github.com/alexvcasillas/curl-runner/commit/f8b77447206083b2e91bd7d45fd54ff453b0a3ec) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **TCP connection pooling with HTTP/2 multiplexing**

  - **New:** `connectionPool` global configuration for request batching, with the options `enabled`, `maxStreamsPerHost`, `keepaliveTime` and `connectTimeout`.
  - **New:** Requests are grouped by host and executed in a single curl process, using `curl -Z --parallel --http2` for HTTP/2 stream multiplexing.
  - **Improved:** TCP/TLS handshake overhead for parallel requests drops dramatically.

## 1.18.0

### Minor Changes

- [#89](https://github.com/alexvcasillas/curl-runner/pull/89) [`6b862c3`](https://github.com/alexvcasillas/curl-runner/commit/6b862c3a57360a068979d0c5a265bc4787b0a500) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **HTTP/2 support with multiplexing**

  Enables HTTP/2 multiplexing for improved performance.

  - **New:** `--http2` flag to use the HTTP/2 protocol for all requests.
  - **New:** Per-request `http2: true` option in YAML config.
  - **New:** Environment variable support: `CURL_RUNNER_HTTP2=true`.

## 1.17.0

### Minor Changes

- [#87](https://github.com/alexvcasillas/curl-runner/pull/87) [`3c23c8c`](https://github.com/alexvcasillas/curl-runner/commit/3c23c8c6856502b2f02fc312157fe22d415af22d) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Dry-run mode**

  Preview curl commands without executing them. Useful for debugging configurations and validating request setups.

  - **New:** `--dry-run` / `-n` flag to show curl commands without making actual API calls.
  - **New:** Status displays as "DRY-RUN" in cyan for all output formats.
  - **New:** JSON output includes a `dryRun: true` field instead of the status code.
  - **New:** Environment variable support: `CURL_RUNNER_DRY_RUN=true`.

## 1.16.5

### Patch Changes

- [#84](https://github.com/alexvcasillas/curl-runner/pull/84) [`14dc8aa`](https://github.com/alexvcasillas/curl-runner/commit/14dc8aa736074b1f80a9695da897670196deeedc) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Variables fall back to `process.env`**

  - **Fixed:** `resolveVariable()` now falls back to `process.env`.

## 1.16.4

### Patch Changes

- [#82](https://github.com/alexvcasillas/curl-runner/pull/82) [`2f329ca`](https://github.com/alexvcasillas/curl-runner/commit/2f329cadb4cce5183a325d0f5206179cff35ad40) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **`upgrade` reads monorepo tags**

  - **Fixed:** The `upgrade` command extracts the semver from monorepo GitHub tags.

## 1.16.3

### Patch Changes

- [#80](https://github.com/alexvcasillas/curl-runner/pull/80) [`d398c44`](https://github.com/alexvcasillas/curl-runner/commit/d398c449ece2968d4df5365da14d6a25411ca95b) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Spawn curl directly**

  - **Fixed:** The CLI spawns curl directly with an args array instead of through a shell, which removes shell interpretation of ${...} variables in curl commands. Previously, unresolved variables would cause errors on macOS bash 3.2 and Debian dash.
  - **Fixed:** Text form fields use --form-string, to prevent @/< interpretation.

## 1.16.2

### Patch Changes

- [#78](https://github.com/alexvcasillas/curl-runner/pull/78) [`f093b13`](https://github.com/alexvcasillas/curl-runner/commit/f093b13e07830ef2abe98d9d97d281238b472a4f) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Install script fixes**

  - **Fixed:** The install script's download URL and binary name extraction are fixed.

## 1.16.1

### Patch Changes

- [#75](https://github.com/alexvcasillas/curl-runner/pull/75) [`e0dc09f`](https://github.com/alexvcasillas/curl-runner/commit/e0dc09f0a90ea761aa62036f93d9848d9f139a76) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **npm package and install script fixes**

  - **Fixed:** The npm package includes the dist folder.
  - **Fixed:** Install script version parsing is fixed.

## 1.16.0

### Minor Changes

- [#70](https://github.com/alexvcasillas/curl-runner/pull/70) [`c7ed9b3`](https://github.com/alexvcasillas/curl-runner/commit/c7ed9b3a7eda5a3a6943ca0f6f89caa1d1ff01d1) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **`upgrade` command**

  - **New:** `curl-runner upgrade` command with auto-detection of the installation source.

## 1.15.0

### Minor Changes

- [#65](https://github.com/alexvcasillas/curl-runner/pull/65) [`084a3c3`](https://github.com/alexvcasillas/curl-runner/commit/084a3c37030920fc5398cbc178fa8c899a370a05) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Response diffing**

  Response diffing for environment and run comparison.

  - **New:** Compare API responses between environments (staging/prod).
  - **New:** Detect API drift between runs over time.
  - **New:** Exclude dynamic paths (timestamps, IDs) from the comparison.
  - **New:** Multiple output formats: terminal, JSON, markdown.
  - **New:** CI-friendly exit codes.
  - **New:** Diff subcommand for offline baseline comparison.

## 1.14.0

### Minor Changes

- [#59](https://github.com/alexvcasillas/curl-runner/pull/59) [`27e6cc2`](https://github.com/alexvcasillas/curl-runner/commit/27e6cc2ba79bc08ba579f938f63f231ea9e3fbe0) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Performance profiling mode**

  - **New:** Performance profiling mode with p50/p95/p99 latency stats.

## 1.13.0

### Minor Changes

- [#61](https://github.com/alexvcasillas/curl-runner/pull/61) [`66b4e8f`](https://github.com/alexvcasillas/curl-runner/commit/66b4e8fd96a10f6659c6e1be844dc510b31e0f56) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Binary download links**

  - **New:** Binary download links on the landing and installation pages.

## 1.12.0

### Minor Changes

- [#60](https://github.com/alexvcasillas/curl-runner/pull/60) [`3476345`](https://github.com/alexvcasillas/curl-runner/commit/34763453df1f5329dd4dc81414c6f965e000962d) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Conditional request chaining**

  - **New:** Conditional request chaining with the `when` field, for skip/run logic based on previous results.

## 1.11.0

### Minor Changes

- [#57](https://github.com/alexvcasillas/curl-runner/pull/57) [`1ab2e0e`](https://github.com/alexvcasillas/curl-runner/commit/1ab2e0e5af2cb891dde29f0193fefe0e14cdaa24) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Snapshot testing**

  Snapshot testing for API regression testing.

  - **New:** `-s, --snapshot` flag to enable snapshot testing.
  - **New:** `-u, --update-snapshots` flag to update all snapshots.
  - **New:** `--update-failing` flag to update only failing snapshots.
  - **New:** `--snapshot-dir <dir>` option for a custom snapshot directory.
  - **New:** `--ci-snapshot` flag to fail on missing snapshots (CI mode).
  - **New:** Snapshot testing via environment variables: `CURL_RUNNER_SNAPSHOT`, `CURL_RUNNER_SNAPSHOT_UPDATE`, etc.
  - **New:** Snapshot testing via YAML config: `snapshot: true` or detailed config with `include`, `exclude`, `match`.
  - **New:** Snapshot exclusion patterns: exact paths, wildcards (`*.timestamp`), array wildcards (`body[*].id`).
  - **New:** Snapshot match rules: wildcards (`*`) and regex patterns (`regex:^v\d+`).

  Snapshots are stored in `__snapshots__/<filename>.snap.json`.

## 1.10.0

### Minor Changes

- [#55](https://github.com/alexvcasillas/curl-runner/pull/55) [`5f675de`](https://github.com/alexvcasillas/curl-runner/commit/5f675de3a576ec45c522695e8a3542246d9b2438) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Watch mode**

  Watch mode re-runs automatically on file changes.

  - **New:** `-w, --watch` flag to enable watch mode.
  - **New:** `--watch-debounce <ms>` option to customize the debounce delay (default: 300ms).
  - **New:** `--no-watch-clear` option to disable screen clearing between runs.
  - **New:** Watch mode via environment variables: `CURL_RUNNER_WATCH`, `CURL_RUNNER_WATCH_DEBOUNCE`, `CURL_RUNNER_WATCH_CLEAR`.
  - **New:** Watch mode via YAML config under `global.watch`.

## 1.9.0

### Minor Changes

- [#37](https://github.com/alexvcasillas/curl-runner/pull/37) [`9968188`](https://github.com/alexvcasillas/curl-runner/commit/996818855042d12b2dce364b66972bca92add697) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **SSL/TLS certificate configuration**

  - **New:** `ssl` configuration option for requests and global settings.
  - **New:** Custom CA certificates (`ssl.ca`).
  - **New:** Mutual TLS (mTLS) with client certificates (`ssl.cert`, `ssl.key`).
  - **New:** Disabling SSL verification (`ssl.verify: false`).

## 1.8.0

### Minor Changes

- [#36](https://github.com/alexvcasillas/curl-runner/pull/36) [`8722304`](https://github.com/alexvcasillas/curl-runner/commit/8722304aab156442e4e015acc8a9c046a5a029c5) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Exponential backoff for retries**

  - **New:** The retry configuration supports a `backoff` multiplier for exponential backoff between retries. When set, the delay increases exponentially with each retry attempt using the formula: `delay * backoff^(attempt-1)`.

  Example usage:

  ```yaml
  retry:
    count: 3
    delay: 1000 # Initial delay: 1 second
    backoff: 2.0 # Multiplier
  # Delays: 1000ms, 2000ms, 4000ms
  ```

  The backoff defaults to 1 (no backoff) to maintain backward compatibility.

## 1.7.0

### Minor Changes

- [#35](https://github.com/alexvcasillas/curl-runner/pull/35) [`c398f6b`](https://github.com/alexvcasillas/curl-runner/commit/c398f6bed818a43a62c42b23aa41d875bdc2a354) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Limit concurrency in parallel mode**

  This helps avoid rate limiting from APIs and prevents overwhelming target servers when running many requests in parallel.

  - **New:** `maxConcurrency` setting in global config to limit simultaneous requests in parallel mode.
  - **New:** `--max-concurrent <n>` CLI flag.
  - **New:** `CURL_RUNNER_MAX_CONCURRENCY` environment variable support.

## 1.6.0

### Minor Changes

- [#34](https://github.com/alexvcasillas/curl-runner/pull/34) [`c9487a3`](https://github.com/alexvcasillas/curl-runner/commit/c9487a30245ae9cee02affb015ab3f0a8758f6fe) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **String transforms for variables**

  New `:upper` and `:lower` transforms for case manipulation. They work with both static variables and environment variables.

  - **New:** `${VAR:upper}` converts the variable value to uppercase.
  - **New:** `${VAR:lower}` converts the variable value to lowercase.

  Example usage:

  ```yaml
  global:
    variables:
      ENV: "production"
      RESOURCE: "Users"

  collection:
    requests:
      - name: API Request
        url: "https://api.example.com/${RESOURCE:lower}"
        headers:
          X-Environment: "${ENV:upper}"
  ```

  This resolves to `https://api.example.com/users` with header `X-Environment: PRODUCTION`.

## 1.5.0

### Minor Changes

- [#33](https://github.com/alexvcasillas/curl-runner/pull/33) [`fd9eb74`](https://github.com/alexvcasillas/curl-runner/commit/fd9eb74d0006a6da27e9e3b8942eb1f4257bdb7e) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Default values for variables (`${VAR:default}`)**

  Variables can now specify fallback values that are used when the variable is not set.

  - **New:** Basic defaults: `${API_TIMEOUT:5000}` uses "5000" if API_TIMEOUT is not defined.
  - **New:** Nested defaults: `${DATABASE_HOST:${DB_HOST:localhost}}` tries DATABASE_HOST, then DB_HOST, then falls back to "localhost".

  This syntax does not conflict with existing dynamic variable patterns like `${DATE:YYYY-MM-DD}` or `${TIME:HH:mm:ss}`.

## 1.4.0

### Minor Changes

- [#31](https://github.com/alexvcasillas/curl-runner/pull/31) [`3a9f252`](https://github.com/alexvcasillas/curl-runner/commit/3a9f2525788a8193bc6947c3d6b32379b14d480c) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **More dynamic variable generators**

  - **New:** `${UUID:short}` returns the first 8 characters of a UUID.
  - **New:** `${RANDOM:min-max}` generates a random integer in the specified range.
  - **New:** `${RANDOM:string:length}` generates a random alphanumeric string of the specified length.

  These variables are useful for API testing scenarios where shorter IDs or random test data are needed.

## 1.3.0

### Minor Changes

- [#28](https://github.com/alexvcasillas/curl-runner/pull/28) [`e35a89d`](https://github.com/alexvcasillas/curl-runner/commit/e35a89ddcf23a03a4d170057490ab6e783271fb2) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **File uploads with multipart/form-data**

  New `formData` property for uploading files and sending form data.

  - **New:** `formData` sends files using the `file` property with the path to the file.
  - **New:** Customize the filename sent to the server with `filename`.
  - **New:** Specify an explicit MIME type with `contentType`.
  - **New:** Mix file attachments with regular form fields in `formData`.
  - **New:** Automatic file existence validation before request execution.

  Example usage:

  ```yaml
  request:
    name: Upload Document
    url: https://api.example.com/upload
    method: POST
    formData:
      title: "My Document"
      document:
        file: "./report.pdf"
        filename: "quarterly-report.pdf"
        contentType: "application/pdf"
  ```

## 1.2.0

### Minor Changes

- [#25](https://github.com/alexvcasillas/curl-runner/pull/25) [`468fadc`](https://github.com/alexvcasillas/curl-runner/commit/468fadc12e25553c74cabdf782098ee52d994723) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **CI-friendly exit codes**

  New CLI options and environment variables for controlling exit code behavior in CI/CD pipelines.

  - **New:** `--strict-exit`: exit with code 1 if any validation fails, regardless of `--continue-on-error`.
  - **New:** `--fail-on <count>`: exit with code 1 only if failures exceed the specified threshold.
  - **New:** `--fail-on-percentage <pct>`: exit with code 1 only if the failure percentage exceeds the threshold.
  - **New:** `CURL_RUNNER_STRICT_EXIT` enables strict exit mode.
  - **New:** `CURL_RUNNER_FAIL_ON` sets the maximum allowed failures.
  - **New:** `CURL_RUNNER_FAIL_ON_PERCENTAGE` sets the maximum allowed failure percentage.

  This enables cleaner CI/CD integration across platforms (GitHub Actions, GitLab CI, CircleCI, Jenkins) without requiring additional parsing logic to detect validation failures.

## 1.1.0

### Minor Changes

- [#22](https://github.com/alexvcasillas/curl-runner/pull/22) [`102be9d`](https://github.com/alexvcasillas/curl-runner/commit/102be9d86e6b11ef92657134c405954bf923c9f3) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Response storage for request chaining**

  Store response values from one request and use them in subsequent requests using the new `store` property and `${store.variableName}` syntax.

  - **New:** `store` property in the request configuration for extracting response values.
  - **New:** JSON path syntax for extracting nested values (e.g., `body.data.token`, `headers.content-type`).
  - **New:** Reference stored values in subsequent requests using `${store.variableName}`.
  - **New:** Stored values work seamlessly with existing static and dynamic variables.

  **Example:**

  ```yaml
  requests:
    - name: Login
      url: https://api.example.com/auth/login
      method: POST
      body:
        username: "admin"
        password: "secret"
      store:
        authToken: body.accessToken
        userId: body.user.id

    - name: Get Profile
      url: https://api.example.com/users/${store.userId}
      headers:
        Authorization: Bearer ${store.authToken}
  ```

  **Supported paths:**

  - `body.field` - Response body fields
  - `body.nested.field` - Nested fields
  - `body.array.0.field` - Array elements by index
  - `headers.name` - Response headers
  - `status` - HTTP status code
  - `metrics.duration` - Response metrics

  **Note:** Response storage requires sequential execution mode (the default).

## 1.0.3

### Patch Changes

- [#11](https://github.com/alexvcasillas/curl-runner/pull/11) [`3fd1c66`](https://github.com/alexvcasillas/curl-runner/commit/3fd1c6640dda618ead28bee21491db28bf9be519) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Output redesign**

  A major redesign of the output formatting for the CLI tool.

  - **Improved:** A new "minimal" pretty format and a tree-based rendering for request results.
  - **Improved:** A simpler output structure and better readability.
  - **Improved:** The documentation clarifies feature availability.

## 1.0.2

### Patch Changes

- [#8](https://github.com/alexvcasillas/curl-runner/pull/8) [`e5d29ba`](https://github.com/alexvcasillas/curl-runner/commit/e5d29ba10590072ff94bcae031552ad772e1d318) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Release process and version checking**

  - **Improved:** The release process is improved.
  - **Improved:** Version checking is improved.

- [#7](https://github.com/alexvcasillas/curl-runner/pull/7) [`0f1d1b5`](https://github.com/alexvcasillas/curl-runner/commit/0f1d1b5609a4848dd2da9482ef219ceee4812229) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Version checking and platform builds**

  - **Improved:** Version checking and version releases are improved.
  - **New:** Platform builds for releases.

## 1.0.1

### Patch Changes

- [#1](https://github.com/alexvcasillas/curl-runner/pull/1) [`41bfb79`](https://github.com/alexvcasillas/curl-runner/commit/41bfb79d64b3229d811b41ab3e84da7961164310) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Automated releases with Changesets**

  - **New:** Changesets configuration for automated versioning and publishing.
  - **New:** GitHub Actions workflow for the CI/CD pipeline.
  - **New:** NPM publishing with proper access controls.
