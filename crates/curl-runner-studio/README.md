# curl-runner studio

A macOS desktop app for curl-runner YAML request suites. Not published.

```sh
cargo run -p curl-runner-studio
```

## Share links (`curl-runner://`)

macOS hands `curl-runner://` links only to an app bundle that registers the
scheme. Build one locally:

```sh
scripts/studio-bundle.sh            # debug build; --release for a release one
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
