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

The script writes `target/studio/curl-runner studio.app` (its `Info.plist`
declares `CFBundleURLTypes` with the `curl-runner` scheme) and installs
nothing. Opening the bundle once is enough for LaunchServices to send it
links such as `curl-runner://open/acme/shipping-api?share=7Kx2Qm`. To forget
it, delete the bundle.

Without the bundle, paste a link instead: Welcome › Open a shared link, or ⌘L.
