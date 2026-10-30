---
'@curl-runner/cli': minor
---

**curl-runner studio (preview)**

A macOS desktop app for your request files: browse a folder as a collection, edit YAML with live validation and quick fixes, run a request or a whole collection and read the response, checks, retries and timing. Run history is kept on your machine, environments come from your `.env` files and secrets live in the macOS Keychain. Sharing and sync run against a built-in mock server for now. The studio isn't distributed yet; build it from source with `cargo run -p curl-runner-studio`.

The CLI is unchanged: the `validate` rules moved into the core library, with the same output and exit codes.
