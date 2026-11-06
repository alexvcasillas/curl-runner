# curl-runner studio

A macOS desktop app for curl-runner YAML request suites. Not published.

```sh
cargo run -p curl-runner-studio                    # local only, as released
cargo run -p curl-runner-studio --features server  # sign-in and shared links, on curl-runner Cloud
CURL_RUNNER_STUDIO_MOCK=1 cargo run -p curl-runner-studio --features server  # the same, on the mock
```

With `--features server` the studio talks to `https://api.curl-runner.com`
by default. Settings › Accounts & sync points it at a self-hosted server
instead. Sessions live in the macOS Keychain (service `curl-runner Cloud`,
one per server).

`CURL_RUNNER_STUDIO_MOCK=1` swaps in the in-process mock, with fixture data
and, in debug builds, the App menu › Developer toggles (offline, conflicts,
expired links, a failing sign-in). Team sync and the share settings answer
"unavailable" on the real server until they ship; use the mock for those.

## Share links (`curl-runner://`)

macOS hands `curl-runner://` links only to an app bundle that registers the
scheme. Build one locally:

```sh
scripts/studio-bundle.sh --server   # debug build; --release for a release one
open "target/studio/curl-runner studio.app"
```

The script writes `curl-runner studio.app` under `target/studio/` (or
`$CARGO_TARGET_DIR/studio/`). Its `Info.plist` declares `CFBundleURLTypes`
with the `curl-runner` scheme. The script installs nothing, but opening the
bundle registers it with LaunchServices, which from then on sends it links
such as `curl-runner://open/acme/shipping-api?share=7Kx2Qm`.

To remove the registration, unregister the bundle, then delete it:

```sh
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister \
  -u "target/studio/curl-runner studio.app"
```

Without the bundle, paste a link instead: Welcome › Open a shared link, or ⌘L.
