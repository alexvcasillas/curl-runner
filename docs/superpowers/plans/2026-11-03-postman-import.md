# Postman Collection Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert a Postman Collection (v2.0 or v2.1) into a folder of curl-runner YAML files plus a `.env`. One converter in core, used by a new `curl-runner convert postman` command and by the studio's Postman import tab.

**Architecture:**
- A new `convert::postman` module in `curl-runner-core`, built from four parts:
  - parse the Postman JSON into serde types;
  - map each request into a small ordered model, collecting a report;
  - emit YAML text with disabled fields commented out in place;
  - write everything to disk safely.
- It doesn't go through `CurlRunnerIr` or `serialize_to_yaml`, because that serializer sorts keys and must stay byte-identical to the TS output.
- The CLI and the studio both call `convert_postman` and `write_postman`.

**Tech Stack:** Rust (edition as in the workspace), `serde` and `serde_json` (already in core), GPUI through `gpui_kit` for the studio, MDX docs.

**Spec:** `docs/superpowers/specs/2026-11-03-postman-import-design.md`

## Global Constraints

- Every change gets unit tests, plus `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` (AGENTS.md).
- Never write a secret value to a file: YAML, `.env`, logs or the report. `Debug` on any type that holds a value must hide it, like the existing `Extracted`.
- Never overwrite an existing file or `.env` value.
- Every unsupported input appears in the report (`ImportReport.attention`). Nothing is dropped silently.
- Test inputs are hand-written collections only. No real-world collection, company name or URL in the repo, and none in commits or PRs either. Use `api.example.com`-style hosts.
- Existing convert snapshots and goldens must not change. `crates/curl-runner/tests/golden/convert` is TS-generated, so never regenerate it.
- Keep Postman's order for headers, params and form fields. Don't sort.
- Variable names: only `[A-Za-z0-9_]` (any other character becomes `_`). A name equal to a curl-runner dynamic variable, or starting with one of `DATE`, `TIME`, `UUID`, `RANDOM`, `CURRENT_TIME`, `TIMESTAMP` or `store.`, gets the suffix `_VAR`. Every rename is reported.
- Copy for users (report lines, docs, changeset) follows the repo's writing rules: no em dashes, short sentences.

## Review Focus

1. A raw JSON body with an unquoted variable (`"count": {{n}}`) isn't valid JSON. It must become a string body with `${n}`, not an error. Test in Task 3.
2. Two requests with the same name in one folder, or a name that slugs to nothing (`🚀`, `///`), must still give distinct, non-empty file names. Test in Task 5.
3. A `{{var}}` used but not defined in the collection stays `${var}` and is reported as a variable to set. Test in Task 3.
4. Importing the same collection twice into the same folder (the CI regenerate case) must overwrite nothing, add `-2` files, and keep existing `.env` values. Test in Task 5.
5. Folder or request names containing `/`, `\` or `..` must never write outside the target folder. Test in Task 5.

---

### Task 1: Move the secret-naming rules and safe-write helpers into core

The Postman mapper needs the same "is this header or param a secret, and what's its variable name" rules as the studio's curl import, and the same no-overwrite writing. Today they're private to the studio.

**Files:**
- Create: `crates/curl-runner-core/src/convert/secrets.rs`
- Modify: `crates/curl-runner-core/src/convert/mod.rs` (add `mod secrets;` and `pub use secrets::*;`)
- Modify: `crates/curl-runner-studio/src/model/import.rs` (delete the moved items, import them from core, keep `pick_var`)

**Interfaces:**
- Produces, in `curl_runner_core::convert`:
  - `pub const SECRET_PARAMS: [&str; 9]`, the same list as today.
  - `pub fn secret_header(name: &str) -> bool`
  - `pub fn header_var(name: &str) -> String`
  - `pub fn param_var(name: &str) -> String`
  - `pub fn screaming(s: &str) -> String`
  - `pub fn unique_path(path: &Path, taken: impl Fn(&Path) -> bool) -> PathBuf`
  - `pub fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()>`
- These are moves with no behaviour change. The studio's `model::import` keeps re-exporting `unique_path` and `write_new` (`pub use curl_runner_core::convert::{unique_path, write_new};`) so its callers don't change.

- [ ] **Step 1: Move the unit tests first.** Copy the studio tests that cover these helpers into `secrets.rs`'s `#[cfg(test)] mod tests`. Add these if they're missing:

```rust
#[test]
fn header_and_param_names() {
    assert!(secret_header("X-Api-Key"));
    assert!(secret_header("authorization"));
    assert!(!secret_header("Content-Type"));
    assert_eq!(header_var("X-Auth-Token"), "AUTH_TOKEN");
    assert_eq!(param_var("apikey"), "API_KEY");
    assert_eq!(param_var("access_token"), "ACCESS_TOKEN");
    assert_eq!(screaming("🚀"), "SECRET");
}

#[test]
fn unique_path_adds_a_number() {
    let taken = |p: &Path| p == Path::new("a/x.yaml") || p == Path::new("a/x-2.yaml");
    assert_eq!(unique_path(Path::new("a/x.yaml"), taken), PathBuf::from("a/x-3.yaml"));
}

#[test]
fn write_new_never_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("sub/f.txt");
    write_new(&p, b"one").unwrap();
    assert_eq!(write_new(&p, b"two").unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "one");
}
```

- [ ] **Step 2: Run and see it fail.** Run `cargo test -p curl-runner-core convert::secrets`. Expected: compile error, since the module doesn't exist yet.
- [ ] **Step 3: Move the code.** Move the items into `secrets.rs` unchanged. `secret_header` uses `crate::security::redact::is_sensitive_header`. Add `tempfile` to core's `[dev-dependencies]` if it isn't there.
- [ ] **Step 4: Run both crates' tests.** Run `cargo test -p curl-runner-core convert::secrets && cargo test -p curl-runner-studio model::import`. Expected: PASS, with the studio tests unchanged.
- [ ] **Step 5: Commit.** `git commit -am "refactor(core): share secret naming and safe writes with convert"`

---

### Task 2: Parse Postman collections

**Files:**
- Create: `crates/curl-runner-core/src/convert/postman/mod.rs` (module root, `pub use` of the public API)
- Create: `crates/curl-runner-core/src/convert/postman/model.rs` (serde types)
- Create: `crates/curl-runner-core/tests/fixtures/postman/sample.postman_collection.json`
- Create: `crates/curl-runner-core/tests/fixtures/postman/edge.postman_collection.json`
- Modify: `crates/curl-runner-core/src/convert/mod.rs` (`pub mod postman;`)

**Interfaces:**
- Produces `pub fn parse_collection(json: &str) -> Result<model::Collection, ConvertError>`.
- `model` types use `#[serde(default)]` everywhere and keep unknown fields out of the way:
  - `Collection { info: Info, item: Vec<Item>, auth: Option<Auth>, variable: Vec<Variable>, event: Vec<Event> }`
  - `Info { name: String, schema: String, description: Option<Description> }`
  - `enum Item { Folder(Folder), Request(RequestItem) }`, untagged. It's a folder when it has `item`.
  - `Folder { name, description, item, auth, event }`
  - `RequestItem { name, request: Request, event, response: Vec<serde_json::Value> }`
  - `Request { method, header: Vec<KeyValue>, url: Url, body: Option<Body>, auth: Option<Auth>, description }`. Postman also allows `request` as a bare URL string, so accept that too.
  - `enum Url { Raw(String), Object { raw: Option<String>, host, path, query: Vec<KeyValue>, variable: Vec<KeyValue> } }`
  - `KeyValue { key, value: Option<String>, disabled: bool, description: Option<Description>, kind: Option<String> /* "file" | "text" */, src: Option<serde_json::Value> }`. `kind` is `type` in the JSON.
  - `enum Description { Text(String), Rich { content: String } }` plus `fn text(&self) -> &str`.
  - `Body { mode, raw: Option<String>, urlencoded: Vec<KeyValue>, formdata: Vec<KeyValue>, graphql: Option<serde_json::Value>, options: Option<serde_json::Value> }`
  - `Auth { kind: String /* "type" */, rest: serde_json::Map<String, Value> }` (flattened). `Auth::params(&self) -> Vec<(String, String)>` reads Postman's `[{key, value}]` array under `rest[kind]`.
  - `Variable { key, value: Option<serde_json::Value>, secret: bool }`. `Debug` hides `value`.
  - `Event { listen: String }`
- Errors (exact text):
  - Not JSON: `Not a Postman collection: <serde error>`.
  - No `info` or `item`: `Not a Postman collection: missing "info" or "item"`.
  - Schema not v2.0 or v2.1 (check `info.schema` contains `/v2.0.` or `/v2.1.`): `Unsupported Postman collection schema: <schema>. Export it as Collection v2.1 from Postman.`

- [ ] **Step 1: Write both fixtures by hand.**
  - `sample` has the motivating shape from the spec:
    - Collection `apikey` auth (`in: header`, `key: X-Api-Key`, `value: {{apiKey}}`).
    - Variables: `baseUrl`, plus `apiKey` with `secret: true` and value `test-key-123`.
    - 2 folders, 5 requests.
    - One request with `auth: {type: noauth}`.
    - One `formdata` request: 1 file field without `src`, 1 file field with `src`, 3 text fields of which 2 are `disabled` with descriptions.
    - One raw JSON request with `options.raw.language: json`.
    - One `GET` with an object `url`, a query param and one disabled query param.
    - String descriptions on every request, and one `{content}` description.
  - `edge` has:
    - A nested folder (depth 3).
    - A string `url`.
    - A path variable `:id` with `url.variable`.
    - A `prerequest` event.
    - An `oauth2` auth.
    - A `bearer` auth and a `basic` auth.
    - `urlencoded`, `graphql` and `file` body modes.
    - A raw JSON body with `"count": {{n}}`.
    - `{{$guid}}` and `{{$randomColor}}`.
    - A saved `response`.
    - A request with a literal `Authorization: Bearer abc` header.
    - Two requests named `Create` in one folder.
    - A request named `🚀`, a folder named `../escape`, and a variable named `UUID`.
- [ ] **Step 2: Write the failing tests** in `postman/tests.rs`:

```rust
fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/postman").join(name),
    ).unwrap()
}

#[test]
fn parses_the_sample() {
    let c = parse_collection(&fixture("sample.postman_collection.json")).unwrap();
    assert_eq!(c.item.len(), 2);
    assert!(c.variable.iter().any(|v| v.key == "apiKey" && v.secret));
    assert_eq!(c.auth.as_ref().unwrap().kind, "apikey");
}

#[test]
fn parses_edge_shapes() {
    parse_collection(&fixture("edge.postman_collection.json")).unwrap();
}

#[test]
fn rejects_what_isnt_a_v2_collection() {
    assert!(parse_collection("nope").unwrap_err().0.starts_with("Not a Postman collection: "));
    assert_eq!(parse_collection("{}").unwrap_err().0,
        "Not a Postman collection: missing \"info\" or \"item\"");
    let v1 = r#"{"info":{"name":"x","schema":"https://schema.getpostman.com/json/collection/v1.0.0/collection.json"},"item":[]}"#;
    assert!(parse_collection(v1).unwrap_err().0.starts_with("Unsupported Postman collection schema: "));
}

#[test]
fn variable_debug_hides_the_value() {
    let c = parse_collection(&fixture("sample.postman_collection.json")).unwrap();
    assert!(!format!("{:?}", c.variable).contains("test-key-123"));
}
```

- [ ] **Step 3: Run and see it fail.** Run `cargo test -p curl-runner-core convert::postman`. Expected: compile error.
- [ ] **Step 4: Implement `model.rs` and `parse_collection`.**
- [ ] **Step 5: Run and see it pass.** Same command. Expected: PASS.
- [ ] **Step 6: Commit.** `git add -A crates/curl-runner-core && git commit -m "feat(core): parse Postman collections"`

---

### Task 3: Map requests and build the report

This task turns the parsed collection into an ordered, format-neutral model. The emitter (Task 4) turns that into YAML.

**Files:**
- Create: `crates/curl-runner-core/src/convert/postman/map.rs`
- Create: `crates/curl-runner-core/src/convert/postman/report.rs`
- Test: `crates/curl-runner-core/src/convert/postman/tests.rs`

**Interfaces:**
- Consumes the `model` types from Task 2, and `secret_header`, `header_var`, `param_var` and `SECRET_PARAMS` from Task 1.
- Produces:
  - `pub fn map_collection(c: &model::Collection) -> Mapped`
  - `pub struct Mapped { pub requests: Vec<MappedRequest>, pub variables: Vec<ImportedVar>, pub description: Option<String>, pub report: ImportReport }`
  - `pub struct MappedRequest { pub folders: Vec<String>, pub name: String, pub comments: Vec<String>, pub method: String, pub url: String, pub headers: Vec<Field>, pub params: Vec<Field>, pub body: MappedBody, pub auth: Option<MappedAuth>, pub file_vars: Vec<(String, String)> }`
    - `folders` are the raw names from the collection root down.
    - `comments` are description lines: the request's first, then its folders' from nearest to farthest.
    - `file_vars` go to the file's `global.variables` and come from path variables.
  - `pub struct Field { pub key: String, pub value: FieldValue, pub disabled: bool, pub note: Option<String> }`
  - `pub enum FieldValue { Text(String), File { path: String, placeholder: bool } }`
  - `pub enum MappedBody { None, Json(serde_json::Value), Text(String), Form(Vec<Field>) }`
  - `pub enum MappedAuth { Bearer(String), Basic { username: String, password: String } }`
  - `pub struct ImportedVar { pub name: String, pub value: String, pub secret: bool }`. `Debug` hides `value`.
  - `pub struct ImportReport { pub folders: usize, pub requests: usize, pub attention: Vec<Attention> }`
  - `pub enum Attention { PickFile { request: String, field: String }, SetSecret { var: String }, SetVariable { var: String }, Renamed { from: String, to: String }, Script { at: String }, UnsupportedAuth { at: String, kind: String }, UnsupportedBody { at: String, mode: String }, DynamicVar { at: String, name: String }, Examples { at: String, count: usize }, KeptEnv { var: String } }`
    - `impl Display` gives one short line each, for example `Pick a file for "file" in Convert document`, `Set the secret API_KEY`, `Skipped a pre-request script in Documents/Upload`.
    - `KeptEnv` is filled in by Task 5.
    - `at` is the request's or folder's path joined with `/`.
- Rules, all from the spec's Mapping table:
  - `{{x}}` becomes `${x'}`, where `x'` is the renamed name.
  - Dynamic variables: `$guid` and `$randomUUID` become `UUID`, `$timestamp` becomes `TIMESTAMP`, `$randomInt` becomes `RANDOM:0-1000`. Anything else is left as `{{$x}}` and reported with `DynamicVar`.
  - Auth inheritance: request, then nearest folder, then collection. `noauth` stops it.
  - `apikey` becomes a header or a param (`in: query`). `bearer` and `basic` become `MappedAuth`.
  - Credential values that aren't already `{{var}}` become variables:
    - an apikey value uses `header_var` or `param_var`;
    - a bearer token becomes `API_TOKEN`;
    - basic credentials become `API_USER` and `API_PASSWORD`.
    - Each one becomes a secret `ImportedVar`.
  - Literal secret headers and params (`secret_header`, `SECRET_PARAMS`) are scrubbed the same way.
  - A variable is secret if Postman marks it `secret: true`, or if a credential references it.
  - Variables referenced but not defined get a `SetVariable` entry, once per name. Secret variables with an empty value get a `SetSecret` entry.
  - Content type: set `application/json` for a raw body whose language is `json`. For `urlencoded`, set `application/x-www-form-urlencoded`. Never add one if a header already sets it, in any letter case.
  - A raw body becomes `Json` when, after `{{x}}` is replaced with `${x}` inside strings, it parses as JSON. Otherwise it becomes `Text` with `${x}`.
  - `graphql` becomes `Json({query, variables})`.
  - The `file` mode is reported with `UnsupportedBody` and gives `MappedBody::None`.
  - The URL comes from `raw` when present, otherwise it's built from `host` and `path` plus the query. Query items move from the URL into `params`, with disabled ones kept as disabled params.
  - A path variable `:id` becomes `${id}`, with `(id, value)` added to `file_vars`.

- [ ] **Step 1: Write the failing tests.** They use `map_collection(&parse_collection(&fixture(..)).unwrap())`.

```rust
#[test]
fn sample_maps_like_the_collection() {
    let m = mapped("sample.postman_collection.json");
    assert_eq!((m.report.folders, m.report.requests), (2, 5));
    let api_key = m.variables.iter().find(|v| v.name == "apiKey").unwrap();
    assert!(api_key.secret);
    let r = req(&m, "Create customer");
    assert_eq!(r.url, "${baseUrl}/v1/customers");
    assert!(r.headers.iter().any(|h| h.key == "X-Api-Key"
        && matches!(&h.value, FieldValue::Text(v) if v == "${apiKey}")));
    assert!(!req(&m, "Health").headers.iter().any(|h| h.key == "X-Api-Key")); // noauth
}

#[test]
fn disabled_fields_keep_their_place_and_note() {
    let r = req(&mapped("sample.postman_collection.json"), "Convert document");
    let MappedBody::Form(f) = &r.body else { panic!() };
    let keys: Vec<_> = f.iter().map(|x| (x.key.as_str(), x.disabled)).collect();
    assert_eq!(keys, /* the fixture's order, with its disabled flags */);
    assert!(f.iter().any(|x| x.disabled && x.note.is_some()));
    assert!(m_has(&r, |a| matches!(a, Attention::PickFile { .. })));
}

#[test]
fn unquoted_variable_in_json_becomes_text() { // Review Focus 1
    let r = req(&mapped("edge.postman_collection.json"), "Count");
    assert!(matches!(&r.body, MappedBody::Text(t) if t.contains("\"count\": ${n}")));
}

#[test]
fn undefined_variables_are_reported_once() { // Review Focus 3
    let m = mapped("edge.postman_collection.json");
    let n = m.report.attention.iter()
        .filter(|a| matches!(a, Attention::SetVariable { var } if var == "n"))
        .count();
    assert_eq!(n, 1);
}

#[test]
fn edge_report_lists_every_unsupported_part() {
    let a = mapped("edge.postman_collection.json").report.attention;
    for want in [
        "Script", "UnsupportedAuth", "UnsupportedBody", "DynamicVar", "Examples", "Renamed",
    ] {
        assert!(a.iter().any(|x| format!("{x:?}").starts_with(want)), "{want}");
    }
}

#[test]
fn literal_bearer_is_scrubbed_into_a_secret() {
    let m = mapped("edge.postman_collection.json");
    let r = req(&m, "Literal auth");
    assert!(r.headers.iter().all(|h| !matches!(&h.value, FieldValue::Text(v) if v.contains("abc"))));
    assert!(m.variables.iter().any(|v| v.secret && v.value == "abc"));
}

#[test]
fn renames_reserved_and_invalid_names() {
    let m = mapped("edge.postman_collection.json");
    assert!(m.report.attention.iter()
        .any(|a| matches!(a, Attention::Renamed { from, to } if from == "UUID" && to == "UUID_VAR")));
}

#[test]
fn path_variables_become_file_variables() {
    let r = req(&mapped("edge.postman_collection.json"), "Get item");
    assert!(r.url.contains("${id}"));
    assert_eq!(r.file_vars, vec![("id".to_string(), "42".to_string())]);
}

#[test]
fn imported_var_debug_hides_the_value() {
    let m = mapped("sample.postman_collection.json");
    assert!(!format!("{:?} {:?}", m.variables, m.report).contains("test-key-123"));
}
```

  Write small `mapped`, `req` and `m_has` helpers. Fill the expected arrays from your fixture.
- [ ] **Step 2: Run and see it fail.** Run `cargo test -p curl-runner-core convert::postman`. Expected: compile error.
- [ ] **Step 3: Implement `map.rs` and `report.rs`.**
- [ ] **Step 4: Run and see it pass.** Same command. Expected: PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(core): map Postman requests and report what needs attention"`

---

### Task 4: Emit YAML with commented-out disabled fields

**Files:**
- Create: `crates/curl-runner-core/src/convert/postman/emit.rs`
- Test: `crates/curl-runner-core/src/convert/postman/tests.rs`

**Interfaces:**
- Consumes `MappedRequest` from Task 3.
- Produces `pub fn emit_request(r: &MappedRequest, collection: &str) -> String`.
- Output layout, in this order:
  1. `# Imported from Postman: <collection>`
  2. A `#` line for each entry in `comments`, then a blank line.
  3. `global:` with `variables:`, only if `file_vars` isn't empty.
  4. `request:` with, in order: `name`, `method`, `url`, `headers`, `params`, `auth`, then `body` or `formData`.
- Disabled entries are commented-out lines at the same indent as enabled ones. Removing the leading `# ` gives valid YAML. A field's `note` goes on its own comment line just above it: `  # <note>` before an enabled field, `  # # <note>` before a disabled one.
- A map that would have only disabled entries is still written: its key line comes first, then the commented entries. That way, uncommenting any one entry gives valid YAML.
- Quote scalars with the same rules core already uses for YAML safety. Reuse a quoting helper from `yaml_serializer.rs` if one is `pub(crate)`, otherwise write a small `yaml_scalar(&str) -> String`. `${var}` references stay unquoted where that's valid, like the studio's `unquote_references`.
- A JSON body is written as block YAML, in key order. A form file field becomes `{ file: <path> }`.

- [ ] **Step 1: Write the failing tests.**

```rust
#[test]
fn every_emitted_file_reads_as_yaml() {
    for f in ["sample.postman_collection.json", "edge.postman_collection.json"] {
        for r in mapped(f).requests {
            let y = emit_request(&r, "Sample");
            crate::parser::parse_raw_stream(&y).unwrap_or_else(|e| panic!("{}: {e:?}\n{y}", r.name));
        }
    }
}

#[test]
fn uncommenting_any_disabled_line_still_parses_and_adds_it() {
    let r = req(&mapped("sample.postman_collection.json"), "Convert document");
    let y = emit_request(&r, "Sample");
    let disabled: Vec<_> = y.lines().enumerate()
        .filter(|(_, l)| l.trim_start().starts_with("# ") && !l.trim_start().starts_with("# #") && l.contains(':'))
        .filter(|(_, l)| !l.starts_with('#')) // skip top-of-file comments
        .map(|(i, _)| i).collect();
    assert!(!disabled.is_empty());
    for i in disabled {
        let lines: Vec<String> = y.lines().enumerate()
            .map(|(j, l)| if j == i { l.replacen("# ", "", 1) } else { l.to_string() }).collect();
        crate::parser::parse_raw_stream(&lines.join("\n")).unwrap();
    }
}

#[test]
fn keeps_postman_order_and_descriptions() {
    let r = req(&mapped("sample.postman_collection.json"), "Create customer");
    let y = emit_request(&r, "Sample");
    assert!(y.starts_with("# Imported from Postman: Sample\n# "));
    assert!(y.find("X-Api-Key").unwrap() < y.find("Content-Type").unwrap()); // fixture order
}

#[test]
fn the_runner_reads_what_postman_would_send() {
    // Run through the real parser with the sample's variables applied.
    let m = mapped("sample.postman_collection.json");
    let r = req(&m, "Create customer");
    let y = emit_request(&r, "Sample");
    let env = |k: &str| match k {
        "baseUrl" => Some("https://api.example.com".into()),
        "apiKey" => Some("k".into()),
        _ => None,
    };
    let parsed = /* the core entry point that loads one YAML string with an env lookup */;
    assert_eq!(parsed.url, "https://api.example.com/v1/customers");
    assert_eq!(parsed.headers.get("X-Api-Key").map(String::as_str), Some("k"));
}
```

  For the last test, use whatever core function the CLI uses to load one file with environment values (see `parser.rs`). If there isn't one that takes a string, add the smallest `pub(crate)` helper there.
- [ ] **Step 2: Run and see it fail.** Run `cargo test -p curl-runner-core convert::postman`. Expected: compile error.
- [ ] **Step 3: Implement `emit.rs`.**
- [ ] **Step 4: Run and see it pass.** Same command, then `cargo test -p curl-runner-core` to check existing snapshots are unchanged. Expected: all PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(core): emit Postman requests as YAML with optional fields commented out"`

---

### Task 5: Plan files and write them safely

**Files:**
- Create: `crates/curl-runner-core/src/convert/postman/write.rs`
- Modify: `crates/curl-runner-core/src/convert/postman/mod.rs` (public API below)
- Test: `crates/curl-runner-core/src/convert/postman/tests.rs`

**Interfaces:**
- Consumes `map_collection` and `emit_request` from Tasks 3 and 4, and `unique_path` and `write_new` from Task 1.
- Produces, re-exported from `convert::postman`:
  - `pub fn convert_postman(json: &str) -> Result<PostmanImport, ConvertError>`, which runs parse, map and emit.
  - `pub struct PostmanImport { pub name: String, pub files: Vec<PostmanFile>, pub variables: Vec<ImportedVar>, pub readme: Option<String>, pub report: ImportReport }`
  - `pub struct PostmanFile { pub rel_path: PathBuf, pub name: String, pub yaml: String }`
    - `rel_path` is safe, relative and free of collisions within this import.
    - Folder names slug with the same rule as file names.
    - The file name is `slug(name) + ".yaml"`: lowercase ASCII, any run of other characters becomes `-`, trimmed, with an empty result falling back to `request`.
  - `pub fn write_postman(root: &Path, folder: &Path, import: &PostmanImport, secret_note: &str) -> io::Result<WriteOutcome>`
  - `pub struct WriteOutcome { pub written: Vec<PathBuf> /* relative to root */, pub kept_env: Vec<String>, pub readme: bool }`
- Write rules:
  - Files go to `root/folder/rel_path` through `unique_path` (checking disk and earlier files in this write) and `write_new`.
  - `.env` lives at `root/.env`:
    - Add only keys that aren't there yet, after a `# Imported from Postman: <name>` comment line.
    - A secret becomes two lines, `# <NAME> is a secret: <secret_note>` and `# <NAME>=`.
    - A key that already exists with a different value is kept and named in `kept_env`.
    - Never rewrite existing lines, only append. Create the file if it's missing.
  - The README goes to `root/folder/README.md`, only if `import.readme` is `Some` and the file doesn't exist.
- The CLI passes `secret_note = "set it in .env.local"`. The studio passes `"the studio keeps it in your Keychain"`.

- [ ] **Step 1: Write the failing tests.**

```rust
#[test]
fn writes_the_sample_tree_and_env() {
    let dir = tempfile::tempdir().unwrap();
    let imp = convert_postman(&fixture("sample.postman_collection.json")).unwrap();
    let out = write_postman(dir.path(), Path::new("api"), &imp, "set it in .env.local").unwrap();
    assert_eq!(out.written.len(), 5);
    let env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
    assert!(env.contains("baseUrl="));
    assert!(env.contains("# apiKey="));
    assert!(!env.contains("test-key-123"));
    for f in &out.written {
        assert!(!std::fs::read_to_string(dir.path().join(f)).unwrap().contains("test-key-123"));
    }
}

#[test]
fn same_names_and_empty_slugs_get_distinct_files() { // Review Focus 2
    let imp = convert_postman(&fixture("edge.postman_collection.json")).unwrap();
    let paths: std::collections::HashSet<_> = imp.files.iter().map(|f| f.rel_path.clone()).collect();
    assert_eq!(paths.len(), imp.files.len());
    assert!(imp.files.iter().all(|f| f.rel_path.file_stem().is_some_and(|s| !s.is_empty())));
}

#[test]
fn importing_twice_overwrites_nothing() { // Review Focus 4
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".env"), "baseUrl=https://mine.example.com\n").unwrap();
    let imp = convert_postman(&fixture("sample.postman_collection.json")).unwrap();
    let first = write_postman(dir.path(), Path::new(""), &imp, "x").unwrap();
    let again = write_postman(dir.path(), Path::new(""), &imp, "x").unwrap();
    assert!(again.written.iter().all(|p| !first.written.contains(p)));
    assert!(again.written.iter().any(|p| p.to_string_lossy().ends_with("-2.yaml")));
    assert!(std::fs::read_to_string(dir.path().join(".env")).unwrap()
        .starts_with("baseUrl=https://mine.example.com\n"));
    assert_eq!(first.kept_env, vec!["baseUrl".to_string()]);
}

#[test]
fn names_never_escape_the_target_folder() { // Review Focus 5
    let dir = tempfile::tempdir().unwrap();
    let imp = convert_postman(&fixture("edge.postman_collection.json")).unwrap();
    let out = write_postman(dir.path(), Path::new("in"), &imp, "x").unwrap();
    for p in &out.written {
        assert!(dir.path().join(p).canonicalize().unwrap().starts_with(dir.path().join("in").canonicalize().unwrap()));
        assert!(p.components().all(|c| matches!(c, std::path::Component::Normal(_))));
    }
}
```

- [ ] **Step 2: Run and see it fail.** Run `cargo test -p curl-runner-core convert::postman`. Expected: compile error.
- [ ] **Step 3: Implement `write.rs` and `convert_postman`.**
- [ ] **Step 4: Run and see it pass.** Same command. Expected: PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(core): write converted Postman collections without overwriting"`

---

### Task 6: `curl-runner convert postman`, docs and changeset

**Files:**
- Modify: `crates/curl-runner-core/src/convert/types.rs` (`Subcommand::Postman`)
- Modify: `crates/curl-runner-core/src/convert/mod.rs` (`parse_convert_args` maps `"postman"`)
- Modify: `crates/curl-runner/src/convert_cmd.rs` (a separate `run_postman` path, and `help` mentions `postman`)
- Create: `crates/curl-runner/tests/convert_postman_cli.rs` (no goldens, since TS never had this)
- Modify: `packages/docs/content/docs/features/convert.mdx` (a "Postman to YAML" section and the CLI reference row)
- Create: `.changeset/postman-import.md`

**Interfaces:**
- Consumes `convert_postman`, `write_postman`, `PostmanImport` and `Attention` from Task 5.
- Behaviour:
  - `curl-runner convert postman <file> [-o <dir>]` writes into `<dir>`, or into `./<slug(collection name)>` when `-o` is left out. It always calls `write_postman(dir, "", ..., "set it in .env.local")`.
  - stderr gets:
    - the count line: `Converted <n> requests in <f> folders to <dir>`;
    - then `Needs your attention:` and one `  - <Attention>` line each, including the `KeptEnv` entries from `WriteOutcome`;
    - then `Run them from <dir> so curl-runner loads its .env.`
  - Exit `1` with the error message when the file can't be read or `convert_postman` fails. Otherwise exit `0`, even with attention items.
  - The other subcommands' output must not change. The existing `convert_cli.rs` goldens prove that.

- [ ] **Step 1: Write the failing CLI tests** with `assert_cmd`, in a tempdir with the sample fixture copied in:
  - `writes_files_and_reports`: exit 0. stderr contains `Converted 5 requests in 2 folders` and `Set the secret` or `Pick a file`. `<dir>/.env` exists.
  - `rejects_a_v1_collection`: exit 1. stderr contains `Unsupported Postman collection schema`.
  - `default_output_is_named_after_the_collection`: with no `-o`, the slug folder exists in the working directory.
- [ ] **Step 2: Run and see it fail.** Run `cargo test -p curl-runner --test convert_postman_cli`. Expected: FAIL, since the subcommand is unknown and the help is printed.
- [ ] **Step 3: Implement** `Subcommand::Postman`, argument parsing and `run_postman`.
- [ ] **Step 4: Run and see it pass.** Run `cargo test -p curl-runner --test convert_postman_cli && cargo test -p curl-runner --test convert_cli`. Expected: PASS.
- [ ] **Step 5: Write the docs.** Add a "Postman to YAML" section to `convert.mdx`, after "YAML to Curl". It covers:
  - the command and an example;
  - what's converted;
  - the report, and that secrets go to `.env.local`;
  - running from the output folder;
  - what's skipped.
  
  Add `convert postman <collection.json>` to the CLI reference table, and say there that `-o` is a directory for this subcommand. Then run `cd packages/docs && pnpm test` and `npx biome check`. Expected: PASS.
- [ ] **Step 6: Write the changeset** `.changeset/postman-import.md`, with `"@curl-runner/cli": minor` and one paragraph a user notices: `curl-runner convert postman` turns a Postman collection into YAML files and a `.env`, optional fields come in commented out, and secrets never land in a file. The studio sentence is added in Task 7.
- [ ] **Step 7: Commit.** `git add -A && git commit -m "feat(cli): convert Postman collections to YAML"`

---

### Task 7: The studio's Postman tab

**Files:**
- Modify: `crates/curl-runner-studio/src/ui/import_dialog.rs`
- Modify: `packages/docs/content/docs/features/studio.mdx`
- Modify: `.changeset/postman-import.md`

**Interfaces:**
- Consumes `convert_postman`, `write_postman`, `PostmanImport`, `ImportedVar` and `Attention` from Task 5, plus the existing `Written`, `Extracted` and `Imported` flow and `refuse_write_into`.
- Dialog state: `postman: Option<(PathBuf, Result<PostmanImport, String>)>`.
  - `pub(super) fn set_postman(&mut self, path: PathBuf, cx: &mut Context<Self>)` reads the file, calls `convert_postman` and stores the result. The file picker and a dropped file both call it.
  - In `pick`, the Postman tab picks a file, not a directory.
- `count()` for Postman returns the number of files, or `0` without a pick or on an error.
- `write()` for Postman:
  - Call `write_postman(&root, &self.target_folder(cx), import, "the studio keeps it in your Keychain")`.
  - `Written.secrets` holds one `Extracted { var, value, place: "Postman variable".into() }` per secret `ImportedVar` with a non-empty value. They're saved only when `save_secrets` is on, through the existing path.
  - `first` is the first written file. `error` is any write error.
- Rendering (`render_postman`):
  - the picker row, then `<file name>: <n> requests in <f> folders`;
  - then the attention list, each `Attention` with `Display`, `PickFile` ones first;
  - an error shows with the existing `note` helper;
  - otherwise reuse the dialog's existing layout pieces (`left_right`, `column`, `note`, `empty_state`) and follow the Import mockup's spacing.
- After the import:
  - a dialog note lists the remaining attention items;
  - `PickFile` rows open their file when clicked, through the workspace's existing open-file path;
  - `KeptEnv` items from `WriteOutcome` are added to that list.
- OpenAPI keeps its Coming soon state.

- [ ] **Step 1: Write the failing tests** in the dialog's `mod tests`, reusing `open`, `collection()` and `test_secrets()`:
  - `postman_import_writes_files_and_env`:
    - Write the sample fixture's JSON to a temp file, `set_postman` it, and set the tab to Postman.
    - `count()` is 5.
    - After `import`, the target folder has the files, `.env` has `baseUrl=`, and no file contains `test-key-123`.
  - `postman_secrets_go_to_the_keychain`: with `save_secrets = true`, the Keychain holds `test-key-123` under `apiKey` and `meta.secret_names` contains `apiKey`. With `false`, the Keychain is untouched.
  - `a_bad_postman_file_shows_an_error_and_writes_nothing`: `count()` is 0, `import` sets `error`, and nothing is written.
  - Rename `coming_soon_tabs_import_nothing` to `openapi_tab_imports_nothing` and set the tab to `ImportTab::OpenApi`.

  Load the fixture with `include_str!("../../../curl-runner-core/tests/fixtures/postman/sample.postman_collection.json")` so the studio tests don't depend on the working directory.
- [ ] **Step 2: Run and see it fail.** Run `cargo test -p curl-runner-studio ui::import_dialog`. Expected: compile error or FAIL.
- [ ] **Step 3: Implement** the state, `set_postman`, `count`, `write`, `render_postman` and the picker change.
- [ ] **Step 4: Run and see it pass.** Same command, then the whole workspace: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`. Expected: all PASS.
- [ ] **Step 5: Check it in the running app.** Build and run the studio. Import the sample fixture into a scratch collection through ⌘I, Postman tab. Take a screenshot of the preview and of the summary for the final report, and check that both match the dialog's existing style.
- [ ] **Step 6: Docs and changeset.**
  - In `features/studio.mdx`, describe importing a Postman collection (where it is, what you get, secrets in the Keychain, the attention list).
  - Add a sentence to the changeset saying the studio's Import dialog now takes Postman collections.
  - Run `cd packages/docs && pnpm test`. Expected: PASS.
- [ ] **Step 7: Commit.** `git add -A && git commit -m "feat(studio): import Postman collections"`
