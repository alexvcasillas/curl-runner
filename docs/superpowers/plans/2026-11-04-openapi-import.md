# OpenAPI Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert an OpenAPI 3.0/3.1 or Swagger 2.0 document (JSON or YAML) into a folder of curl-runner YAML files plus a `.env`, one file per operation. A new `curl-runner convert openapi` command uses the converter, and so does the studio's OpenAPI import tab.

**Architecture:**
- First, move the Postman importer's format-neutral parts (mapped model, emitter, writer, report, slug, `var_name`) into a shared `convert::import` module. Postman keeps its public names as re-exports.
- A new `convert::openapi` module has four parts:
  - parse JSON or YAML into a `serde_json::Value`, normalizing Swagger 2.0 to the 3.x shape;
  - resolve `$ref`s lazily by JSON Pointer;
  - generate sample values from schemas, within a budget;
  - map each operation into the shared `MappedRequest`, collecting an aggregated report.
- The shared writer puts the files on disk. The CLI and the studio both call `convert_openapi` and `write_import`.

**Tech Stack:** Rust (workspace edition), `serde_json` + `serde_yaml_ng` (already in core), GPUI through `gpui_kit` for the studio, MDX docs, changesets.

**Spec:** `docs/superpowers/specs/2026-11-04-openapi-import-design.md`

## Global Constraints

- Every change gets unit tests, plus `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` (AGENTS.md).
- Never write a secret value to a file: YAML, `.env`, logs or the report. `Debug` on anything that holds a value hides it, like `ImportedVar`.
- Never overwrite an existing file or `.env` value.
- Anything that changes what a request sends and isn't converted goes in the report. Issues that repeat across operations are one aggregated entry (count plus up to 3 example request paths). "Pick a file" stays one entry per field.
- Every existing Postman test, convert snapshot and golden passes unchanged, except the `convert` help goldens, which gain one line in Task 6. Never regenerate `crates/curl-runner/tests/golden/convert` from a tool. Edit the line in by hand.
- Test inputs are small hand-written specs with `api.example.com`-style hosts. No real-world spec, company name or URL in the repo, commits or PRs.
- Output is deterministic. No clock, random values or hash-map iteration order in emitted files. Keep document order (`serde_json` `preserve_order` is on).
- Performance: resolve `$ref`s by pointer lookup on the parsed `Value`, never by expanding the document. A 10 MB spec converts in under a second in a release build.
- User-facing copy (report lines, help, docs, changeset) has no em dashes and uses short sentences.
- Commits: Conventional Commits, one line.

## Review Focus

1. A `$ref` cycle through `allOf` or `items` (a `Node` whose `children` are `Node`s) must end in `{}`/`[]`, not a stack overflow. Test in Task 3.
2. A wide schema graph (each of 40 schemas referencing 10 others) must give a small body and finish fast because of the budget. Test in Task 3.
3. Two APIs imported into one root must not share `.env` keys. Their prefixes differ, and a second import of the same API keeps the first's values. Test in Task 5.
4. A parameter `example` on `Authorization` or `api_key` that holds a literal credential must come out as a `${…}` secret variable, never the literal. Test in Task 4.
5. A document where `paths` holds a non-object, an operation with no `responses`, or a `$ref` to a missing target still converts its other operations and reports the bad parts. Test in Task 4.

---

## PR 1: core and CLI

### Task 1: Shared import module

Move the format-neutral parts of `convert/postman/` into `convert/import/` so OpenAPI can use them. Add an expected status to the model.

**Files:**
- Create: `crates/curl-runner-core/src/convert/import/{mod.rs,model.rs,emit.rs,write.rs,report.rs,names.rs}`
- Modify: `crates/curl-runner-core/src/convert/mod.rs` (add `pub mod import;`)
- Modify: `crates/curl-runner-core/src/convert/postman/{mod.rs,map.rs,emit.rs,write.rs,report.rs}`. Delete `emit.rs`/`report.rs` once they're moved, and keep re-exports in `postman/mod.rs`.
- Modify: `crates/curl-runner/src/convert_cmd.rs`, `crates/curl-runner-studio/src/ui/import_dialog.rs` (only if a renamed path breaks them)
- Test: `crates/curl-runner-core/src/convert/import/tests.rs`

**Interfaces:**
- Produces, in `curl_runner_core::convert::import`:
  - `MappedRequest`, `Field`, `FieldValue`, `MappedBody`, `MappedAuth`, `ImportedVar`, `Mapped`, moved unchanged except that `MappedRequest` gains `pub expect_status: Option<u16>`. Postman sets it to `None`.
  - `pub fn emit_request(r: &MappedRequest, source: &str, collection: &str) -> String`. Its first line is `# Imported from {source}: {collection}`. Postman passes `"Postman"`, so its output doesn't change. When `expect_status` is `Some(s)`, `  expect:` / `    status: {s}` follows the body (or the last field if there's no body).
  - `pub struct Import { name, files: Vec<ImportFile>, variables, readme, report }` and `pub struct ImportFile`, which are today's `PostmanImport`/`PostmanFile` renamed.
  - `pub fn plan_import(name: String, mapped: Mapped, source: &str) -> Import`: the file planning and README part of today's `convert_postman`.
  - `pub fn write_import(root: &Path, folder: &Path, import: &Import, secret_note: &str) -> io::Result<WriteOutcome>`, which is today's `write_postman`.
  - `pub fn slug`, `pub fn var_name`, `pub const PLACEHOLDER_FILE: &str = "./pick-a-file"`.
  - `ImportReport` and `Attention`, moved. Task 4 adds variants.
- `postman/mod.rs` keeps `pub use` aliases: `PostmanImport = Import`, `PostmanFile = ImportFile`, `write_postman = write_import`, plus the moved types. `convert_postman` stays in `postman/` and calls `plan_import(…, "Postman")`.

- [ ] **Step 1: Write the failing test** `emit_writes_expect_status`. Build a `MappedRequest` with `method: "GET"`, `url: "${API_BASE_URL}/pets"`, `expect_status: Some(200)`, and no body. Assert that `emit_request(&r, "OpenAPI", "Pets")` starts with `# Imported from OpenAPI: Pets\n` and ends with `  expect:\n    status: 200\n`.
- [ ] **Step 2: Run** `cargo test -p curl-runner-core convert::import`. Expected: FAIL (module missing).
- [ ] **Step 3: Move the code** with `git mv` where whole files move, so history follows. Add `expect_status`, the `source` parameter and `plan_import`.
- [ ] **Step 4: Run** `cargo test --workspace`. Expected: PASS, with every existing Postman test and snapshot untouched (`git diff --stat` shows no fixture or snapshot change).
- [ ] **Step 5: Commit** `refactor(convert): share the import model, emitter and writer`

### Task 2: Parse and normalize OpenAPI documents

**Files:**
- Create: `crates/curl-runner-core/src/convert/openapi/{mod.rs,doc.rs,swagger2.rs,tests.rs}`
- Modify: `crates/curl-runner-core/src/convert/mod.rs` (`pub mod openapi;`)

**Interfaces:**
- Produces:
  - `pub struct Doc { pub root: serde_json::Value, pub version: SpecVersion }` with `pub enum SpecVersion { Swagger2, OpenApi30, OpenApi31 }`. `root` is always the 3.x shape.
  - `pub fn parse_document(text: &str) -> Result<Doc, ConvertError>`. It parses JSON when the first non-whitespace character is `{`, and YAML otherwise.
  - `impl Doc { pub fn resolve<'a>(&'a self, v: &'a Value) -> Resolved<'a> }` follows `$ref` chains (`#/…` only, `~1` → `/`, `~0` → `~`, percent-decoding) with a hop limit of 32. It returns `pub enum Resolved<'a> { Value(&'a Value), Missing(String), External(String) }`. The `String` is the `$ref` text.
- Error copy, exactly:
  - no `openapi`/`swagger` key: `Not an OpenAPI document: no "openapi" or "swagger" field`
  - an unsupported version: `Unsupported OpenAPI version: {v}. Use OpenAPI 3.0, 3.1 or Swagger 2.0.`
  - a parse failure: `Not an OpenAPI document: {serde error}`
- Swagger 2.0 normalization (`swagger2.rs`, `fn to_v3(v: Value) -> Value`):
  - `host` + `basePath` + first of `schemes` (default `https`) → `servers[0].url`. No `host` gives a relative URL.
  - `in: body` → `requestBody.content[<first consumes or application/json>].schema`.
  - `in: formData` → one `requestBody` whose media type is `multipart/form-data` if `consumes` lists it or any field is `type: file`, else `application/x-www-form-urlencoded`. Field `type: file` → `{type: string, format: binary}`.
  - `definitions`, `parameters`, `responses`, `securityDefinitions` move to `components.{schemas,parameters,responses,securitySchemes}`, with every `$ref` string rewritten. `basic` → `{type: http, scheme: basic}`.
  - Response `schema` → `content.application/json.schema`. Only status keys matter here.

- [ ] **Step 1: Write the failing tests:**
  - `parses_yaml_and_json`: the same minimal 3.0 doc both ways gives an equal `root`.
  - `rejects_non_openapi`: `{"info":{},"item":[]}` → the exact error above.
  - `rejects_openapi_2_5`: → `Unsupported OpenAPI version: 2.5. …`.
  - `resolves_escaped_pointer`: a `$ref` to `#/components/schemas/a~1b` resolves.
  - `ref_cycle_is_missing_not_hang`: A → B → A gives `Missing`.
  - `external_ref`: `other.yaml#/X` → `External("other.yaml#/X")`.
  - `swagger2_normalizes`: host `api.example.com`, basePath `/v1`, schemes `[http]` → `servers[0].url == "http://api.example.com/v1"`. A `body` param becomes `requestBody.content["application/json"]`. A `formData` file field → `multipart/form-data` with `format: binary`. `#/definitions/Pet` refs are rewritten to `#/components/schemas/Pet`.
- [ ] **Step 2: Run** `cargo test -p curl-runner-core convert::openapi`. Expected: FAIL.
- [ ] **Step 3: Implement `parse_document`, `Doc::resolve` and `to_v3`.**
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(convert): parse OpenAPI and Swagger 2.0 documents`

### Task 3: Sample values from schemas

**Files:**
- Create: `crates/curl-runner-core/src/convert/openapi/sample.rs`
- Test: in `openapi/tests.rs`

**Interfaces:**
- Consumes: `Doc`, `Doc::resolve`.
- Produces:
  - `pub struct Sampler<'d>`, created with `Sampler::new(doc: &'d Doc)`;
  - `pub fn value(&mut self, schema: &Value) -> Json` for a schema;
  - `pub fn example(&mut self, holder: &Value) -> Option<Json>`: the holder's `example`, else the first of `examples` (resolving `$ref`, taking `.value`). It's used on parameters and media-type objects before falling back to `value(schema)`;
  - `pub fn alternatives(&mut self, schema: &Value) -> usize`: how many top-level `oneOf`/`anyOf` options, 0 if none.
  - `Json` is `crate::model::Json`.
- Rules (spec, Sample values): the order is `example`, `examples[0]`, `default`, `const`, `enum[0]`, then generated. Fixed literals keep output deterministic:
  - `date-time` → `"2024-01-01T00:00:00Z"`
  - `date` → `"2024-01-01"`
  - `email` → `"user@example.com"`
  - `uuid` → `"00000000-0000-4000-8000-000000000000"`
  - `uri`/`url` → `"https://example.com"`
  - `binary` → `"./pick-a-file"`
  - any other string → `"string"`
  - integer/number → `minimum`, else `1`
  - boolean → `true`
  - `type: [x, "null"]` → x
- `allOf` merges the parts' objects in order. `oneOf`/`anyOf` take the first option.
- Recursion: keep a stack of `$ref` strings. A ref already on the stack gives `{}`, or `[]` when the schema is an array. Depth > 10 does the same.
- Budget: `BUDGET = 200` values per `value()` call, with `HARD_CAP = 400`. Past the budget, optional properties are skipped. Past the hard cap, every value is `{}`, `[]` or `null` by type.

- [ ] **Step 1: Write the failing tests:**
  - `uses_example_before_schema`: the media type's `example` wins over the schema's.
  - `formats_are_fixed`: an object with `id: integer`, `at: date-time`, `mail: email` → `{"id":1,"at":"2024-01-01T00:00:00Z","mail":"user@example.com"}`.
  - `minimum_wins`: `{type: integer, minimum: 5}` → `5`.
  - `nullable_31`: `{type: [string, "null"]}` → `"string"`.
  - `all_of_merges`: two parts → both props in order.
  - `one_of_first`: → first option, and `alternatives` = 2.
  - `recursive_ref_stops`: Node `{name, children: [Node]}` → `{"name":"string","children":[{}]}`.
  - `wide_graph_is_budgeted` (Review Focus 2): 40 schemas each with 10 `$ref` props to others. The value has at most 400 leaves and finishes in under 50 ms in a debug test.
  - `missing_ref_is_empty`: `$ref` to a missing target → `{}`.
- [ ] **Step 2: Run** `cargo test -p curl-runner-core convert::openapi::tests::sample`. Expected: FAIL.
- [ ] **Step 3: Implement `Sampler`.**
- [ ] **Step 4: Run** it again. Expected: PASS.
- [ ] **Step 5: Commit** `feat(convert): generate OpenAPI sample values within a budget`

### Task 4: Map operations, auth and the aggregated report

**Files:**
- Create: `crates/curl-runner-core/src/convert/openapi/{map.rs,auth.rs}`
- Modify: `crates/curl-runner-core/src/convert/import/report.rs` (new `Attention` variants)
- Test: `openapi/tests.rs`

**Interfaces:**
- Consumes: `Doc`, `Sampler`, the `import::` model, `var_name`, `crate::convert::{secret_header, header_var, param_var, SECRET_PARAMS}`.
- Produces:
  - `pub fn map_document(doc: &Doc) -> Mapped`. The requests are in `paths` order, then method order `get, put, post, delete, options, head, patch, trace`.
  - `pub fn api_prefix(title: &str) -> String`. Uppercase, `[A-Z0-9_]` only, runs of other characters become `_`, trimmed, cut at the last `_` at or before 24 characters, `API` if empty. `"Swagger Petstore - OpenAPI 3.0"` → `"SWAGGER_PETSTORE"`. `"Pets"` → `"PETS"`.
  - New `Attention` variants. `examples: Vec<String>` holds at most 3 request paths, and `count` counts every hit:
    - `UnresolvedRef { target, count, examples }` → `Couldn't resolve {target} in {count} request(s), like {examples}. Used an empty value.` (`External` refs too, worded `Can't follow the external reference {target} …`)
    - `SkippedBody { media, count, examples }` → `Skipped {media} bodies in {count} request(s), like … Add them by hand.`
    - `AuthChoice { used, others, count }` → `Used {used} auth in {count} request(s) that also accept {others}`
    - `AccessToken { var, kind }` → `Put an {OAuth 2|OpenID Connect} access token in {var}`
    - `UnsupportedScheme { scheme, kind, count }` → `Skipped {kind} auth ({scheme}) in {count} request(s). Add it by hand.`
    - `OwnServer { url, count }` → `{count} request(s) use their own server {url}`
    - `ServerVariable { name }` → `The server variable {name} has no default. Edit the base URL.`
    - `ParamStyle { count, examples }` → `{count} parameter(s) use a style curl-runner doesn't model, like … Check their values.`
    - `CookieParams { count, examples }` → `Left out optional cookies in {count} request(s), like …`
    - `BodyShapes { count, examples }` → `Used the first of several body shapes in {count} request(s), like …`
    - `NotConverted { what, count }` → `Skipped {count} {webhook|callback|link}(s)`
    - `BadOperation { at, reason }` (per item) → `Skipped {at}: {reason}`
  - All of them are aggregated in a `BTreeMap` keyed by the variant's identity while mapping, then emitted in first-seen order.
- Mapping rules come from the spec's Mapping, Bodies and Auth sections. Exact names the tests pin:
  - The base URL variable is `{PREFIX}_BASE_URL`. Its value is the first server's URL with server variables replaced by their defaults and a trailing `/` trimmed. Every URL is `${PREFIX_BASE_URL}{path}`, with `{p}` → `${p}`.
  - A relative or missing server adds `SetVariable { var: "{PREFIX}_BASE_URL" }`.
  - Other servers go in `ImportedVar`-adjacent `.env` comments. Add `pub env_comments: Vec<(String, String)>` to `Mapped` (var name → comment lines). `plan_import` copies it to `Import`, and `write_import` writes the comment lines above that key when it adds the key.
  - Auth variables:
    - apiKey, oauth2 or openIdConnect: `{PREFIX}_{SCHEME}`, where `SCHEME` is `var_name(key).to_ascii_uppercase()`;
    - bearer: `{PREFIX}_{SCHEME}_TOKEN`;
    - basic: `{PREFIX}_{SCHEME}_USERNAME` / `_PASSWORD`. Only the password is secret.
  - All other auth variables are `secret: true` with an empty value.
  - Request `name` is `summary`, else `operationId`, else `"{METHOD} {path}"`. Folder is `tags[0]` if present.
  - Comments are `description` lines, `operationId: {id}`, `Deprecated` if set, `externalDocs.url`, then the tag's description.
  - `expect_status` is the lowest numeric `2xx` key, or `200` for `2XX`, else `None`.
  - Path params go in `file_vars` as text. Required query/header params are enabled `Field`s. Optional ones are `disabled: true`, with the description as `note`. Array values are joined with `,`. `style`/`explode` other than the defaults add `ParamStyle`.
  - A literal credential in a param example (`secret_header`/`SECRET_PARAMS`) becomes `${PREFIX}_{header_var|param_var}`, a secret `ImportedVar`. The value is never kept.

- [ ] **Step 1: Write the failing tests** against inline fixtures. Put `small_v30.yaml` (the spec's small shape) in `crates/curl-runner-core/tests/fixtures/openapi/` and load it with `include_str!`.
  - `maps_small_spec`: 3 tags → `folders == [["pets"],["pets"],["store"],…]`. The first request has `name == "List pets"`, `url == "${PETS_BASE_URL}/pets"`, `expect_status == Some(200)`, and an optional `limit` param that is disabled, with value `"1"` and the description as its note.
  - `path_params_in_file_vars`: `/pets/{petId}` → url `${PETS_BASE_URL}/pets/${petId}`, `file_vars == [("petId","1")]`.
  - `json_body_and_content_type`: POST with `application/json` → `MappedBody::Json` and an enabled `Content-Type: application/json` header.
  - `multipart_file_field`: binary prop → `FieldValue::File { path: "./pick-a-file", placeholder: true }` and `PickFile` in the report. An optional text prop is disabled. No Content-Type header.
  - `urlencoded_required_only`: body text holds only the required fields. The optional names appear in `comments`.
  - `octet_stream_skipped`: 2 operations → one `SkippedBody { media: "application/octet-stream", count: 2, .. }`.
  - `auth_rows`: apiKey header/query/cookie, bearer, basic, oauth2 (→ bearer + `AccessToken`), digest (→ `UnsupportedScheme`), `security: []` → `auth: None`. Check every variable name listed above.
  - `auth_choice_aggregated`: 5 operations with `[{apiKey}, {oauth2}]` → exactly one `AuthChoice { count: 5, .. }`.
  - `literal_credential_scrubbed` (Review Focus 4): header param `Authorization` with example `Bearer sk_live_x` → value `${PETS_AUTHORIZATION}` (the `header_var` naming), a secret var, and `sk_live_x` appears in no request, variable value or report line.
  - `relative_server_needs_setting`: `servers: [{url: /v1}]` → var value `"/v1"` and `SetVariable`.
  - `server_variables_default`: `https://{region}.example.com` with default `eu` → `https://eu.example.com`. No default → `ServerVariable`.
  - `bad_parts_dont_stop_others` (Review Focus 5): one path item is a string, one op has no responses, one has a missing `$ref` body. The other ops map, and there's one `BadOperation` and one `UnresolvedRef`.
  - `api_prefix_cases`: the two examples above, `""` → `API`, and a 40-char title is cut at a word boundary.
  - `swagger2_end_to_end`: a small Swagger 2.0 fixture `petstore_v2.json` maps with basic auth and a formData file.
- [ ] **Step 2: Run** `cargo test -p curl-runner-core convert::openapi`. Expected: FAIL.
- [ ] **Step 3: Implement `map_document`, `auth.rs` and the report variants**, plus their `Display` copy as listed.
- [ ] **Step 4: Run** it again. Expected: PASS.
- [ ] **Step 5: Commit** `feat(convert): map OpenAPI operations to curl-runner requests`

### Task 5: `convert_openapi`, emitted fixtures and the speed test

**Files:**
- Modify: `crates/curl-runner-core/src/convert/openapi/mod.rs`
- Create: `crates/curl-runner-core/tests/fixtures/openapi/large_v31.json`, a hand-written 3.1 doc in the large shape. Keep it small (~15 ops) but cover `allOf`/`oneOf`, recursion, multiple security, form-urlencoded and webhooks.
- Test: `openapi/tests.rs`, and the snapshot style the Postman tests already use for emitted YAML.

**Interfaces:**
- Produces: `pub fn convert_openapi(text: &str) -> Result<Import, ConvertError>`. It runs parse → map → `plan_import(title, mapped, "OpenAPI")`. `name` is `info.title`, or `"OpenAPI import"` if missing. `readme` comes from `info.description`.

- [ ] **Step 1: Write the failing tests:**
  - `emits_small_spec`: snapshot every file of `small_v30.yaml`. Uncommenting the disabled `limit` line (remove the first `# `) still parses as a `RequestConfig`.
  - `emits_large_spec`: snapshot of `large_v31.json` plus its report lines in order.
  - `two_apis_one_root` (Review Focus 3): write `small_v30.yaml` and `large_v31.json` into one temp root, then write `small_v30.yaml` again. `.env` has both `PETS_BASE_URL` and the other prefix's keys. The second run adds `-2` files and reports `KeptEnv` for no key, since its values are the same.
  - `secrets_never_written`: the `.env` holds `# PETS_API_KEY=` lines and no value.
  - `synthetic_speed`: generate 3,000 operations over 300 schemas with 5-deep `$ref` chains in-test. `convert_openapi` finishes under 2 s in debug (`#[cfg_attr(debug_assertions, ignore)]` if CI's debug run is too slow, and run it with `--release` in step 4).
- [ ] **Step 2: Run** `cargo test -p curl-runner-core convert::openapi`. Expected: FAIL.
- [ ] **Step 3: Implement `convert_openapi`. Review the snapshots by hand:** every file has `expect`, every secret is a `${…}`, and the YAML looks right.
- [ ] **Step 4: Run** `cargo test -p curl-runner-core convert::openapi` and `cargo test --release -p curl-runner-core synthetic_speed -- --include-ignored`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(convert): convert OpenAPI documents to request files`

### Task 6: `curl-runner convert openapi`, docs, changeset, PR 1

**Files:**
- Modify: `crates/curl-runner-core/src/convert/mod.rs` (`Subcommand::OpenApi` for `"openapi"`)
- Modify: `crates/curl-runner/src/convert_cmd.rs`. Generalize `run_postman` into `run_import(parsed, kind, …)`, where `kind` gives the converter, the fallback folder (`"postman-import"`/`"openapi-import"`) and the help. Add `openapi_help`.
- Modify: the 7 `crates/curl-runner/tests/golden/convert/help_*.stdout` files. Add `  curl-runner convert openapi <spec>          Convert OpenAPI/Swagger spec → YAML files` after the Postman line, aligned to the existing column.
- Create: `crates/curl-runner/tests/convert_openapi_cli.rs`, modeled on `convert_postman_cli.rs`
- Create: `packages/docs/content/docs/features/openapi-import.mdx`, modeled on `postman-import.mdx`. Cover the mapping table, sample values, auth and variable prefixes, `.env`/`.env.local`, the report, and the studio (a screenshot placeholder is filled in Task 7).
- Modify: `packages/docs/content/docs/features/convert.mdx` (an "OpenAPI to YAML" section plus a CLI reference row), `packages/docs/content/docs/cli-commands.mdx`, `packages/docs/content/docs/features/meta.json`
- Create: `.changeset/openapi-import.md` (`curl-runner: minor`). Model the front matter on the Postman changeset in git history (`git log -p --diff-filter=A -- .changeset`).

- [ ] **Step 1: Write the failing CLI tests:**
  - `converts_into_named_folder`: no `-o` → `./pets/` with the files, and stderr starts `Converted 6 requests in 3 folders to pets`.
  - `lists_secrets_first`: stderr lists "Pick a file" items, then `Set the secret PETS_API_KEY`.
  - `bad_input_exits_1`: a Postman file → exit 1 with `Not an OpenAPI document: no "openapi" or "swagger" field`.
  - `help_lists_openapi`: `convert openapi --help` shows `curl-runner convert openapi <spec.json|spec.yaml> [-o <dir>]`.
- [ ] **Step 2: Run** `cargo test -p curl-runner --test convert_openapi_cli`. Expected: FAIL.
- [ ] **Step 3: Implement the command and update the help goldens by hand. Write the docs and the changeset.**
- [ ] **Step 4: Run** `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`, then `pnpm --filter @curl-runner/docs build`. Expected: all green.
- [ ] **Step 5: Commit** `feat(cli): convert OpenAPI specs with convert openapi`. Push, then open PR 1 with `gh pr create`. Write the body with the `mattpocock-skills:pr` skill. Wait for CI green, then squash-merge.

## PR 2: studio

### Task 7: The studio's OpenAPI tab, docs and screenshots

**Files:**
- Modify: `crates/curl-runner-studio/src/ui/import_dialog.rs`
- Modify: `packages/docs/content/docs/features/studio.mdx`, `packages/docs/content/docs/features/openapi-import.mdx`
- Create: `packages/docs/public/studio/openapi-import.png`, `packages/docs/public/studio/openapi-import-summary.png`
- Create: `.changeset/openapi-import-studio.md` (`curl-runner: minor`, one line about the studio tab)

**Interfaces:**
- Consumes: `convert_openapi`, `write_import`, `Import`, `Attention`.
- Generalize the Postman tab's state and rendering over a `FileImport` kind: `postman`/`postman_done` become one `file_import: Option<(ImportTab, PathBuf, Result<Import, String>)>` plus `file_done: Option<FileImportDone>` (today's `PostmanDone`), or a per-tab pair. The implementer picks whichever keeps the diff smaller. The copy differs per tab:
  - OpenAPI empty state: `Pick an OpenAPI or Swagger file, or drop it here.`
  - Picker label: `OpenAPI spec`. Accepted extensions: `json`, `yaml`, `yml`.
  - Status before a pick: `Choose an OpenAPI spec to import.`
  - Hint under the preview: `Each operation gets its own file, in a folder per tag. The base URL and secrets go to .env.`
- `pub(super) fn route_dropped(path: &Path) -> ImportTab`: returns `OpenApi` when the file's top level (JSON or YAML) has `openapi` or `swagger`, else `Postman`. Both tabs' drop targets call it and switch tab.
- `dev_fill`: a `.json`/`.yaml`/`.yml` path goes through `route_dropped`.
- Remove the Coming soon handling and the "later build" copy for OpenAPI. Replace `openapi_tab_imports_nothing` with the tests below.
- Attention icons: the new variants `UnresolvedRef`, `SkippedBody`, `UnsupportedScheme`, `ServerVariable`, `AccessToken` and `BadOperation` get the warning icon. The rest get info.

- [ ] **Step 1: Write the failing tests** (the `TestAppContext` style already in the file):
  - `openapi_tab_previews_spec`: `set_file_import(OpenApi, small_v30.yaml)` → `count() == 6` and the preview lists `pets/list-pets.yaml`.
  - `openapi_import_writes_and_summarizes`: import into a temp collection → files exist, `.env` has `PETS_BASE_URL`, the summary shows `Set the secret PETS_API_KEY`, and no secret value is on disk.
  - `drop_routes_by_content`: `route_dropped(openapi.json) == OpenApi`, `route_dropped(collection.json) == Postman`, and `route_dropped(spec.yaml with swagger: "2.0") == OpenApi`.
  - The existing Postman tab tests pass unchanged.
- [ ] **Step 2: Run** `cargo test -p curl-runner-studio import_dialog`. Expected: FAIL.
- [ ] **Step 3: Implement.** Then follow the studio fidelity workflow:
  - render the Postman tab and the OpenAPI tab with `CURL_RUNNER_STUDIO_BACKGROUND=1`, `CURL_RUNNER_STUDIO_FILL=<fixture path>` and a temp state dir;
  - take background screenshots with `screencapture -l`;
  - compare them side by side, and fix layout or spacing drift until they match apart from copy;
  - save the preview and summary screenshots to the two PNG paths above, and reference them from `studio.mdx` and `openapi-import.mdx`.
- [ ] **Step 4: Run** `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`, then the docs build. Expected: green.
- [ ] **Step 5: Commit** `feat(studio): import OpenAPI specs`. Push, open PR 2, wait for green CI, squash-merge.

## Release

### Task 8: End to end and release

No repo files change in this task, apart from the version PR. The specs live in the scratchpad.

- [ ] **Step 1: Petstore, end to end.**
  - Download `https://petstore3.swagger.io/api/v3/openapi.json` into the scratchpad.
  - Run `cargo run --release -p curl-runner -- convert openapi openapi.json -o petstore`.
  - Set `SWAGGER_PETSTORE_BASE_URL=https://petstore3.swagger.io/api/v3` and the API key in `.env.local`, and change path ids to existing records where needed.
  - Run every GET with `curl-runner`. Expected: all green. Run the writes and note any failures caused by demo data.
- [ ] **Step 2: Large spec timing.** Download Stripe's `spec3.json` and GitHub's `api.github.com.json` into the scratchpad. `time ./target/release/curl-runner convert openapi …` must take under 1 s each. Read the report and confirm it's short and aggregated. If it misses either check, fix it in a follow-up PR before the release.
- [ ] **Step 3: Studio by hand.** Import the Petstore spec through the OpenAPI tab in a release build, in the background, and screenshot it.
- [ ] **Step 4: Release.** Wait for the changesets "Version Packages" PR, check that its changelog reads well, and merge it once CI is green. Watch the release workflow: GitHub release, npm, Homebrew and the studio artifacts, as for 2.4.0. Confirm `curl-runner --version` from npm shows the new version.
- [ ] **Step 5: Tell the owner.** Send a push notification with the version and links.

## Unresolved questions

- Blog post for the release? The Postman release had one (#167). Default: yes, a short post in PR 2, modeled on `curl-runner-2-4-postman-import.mdx`.
- Should `/alternatives` mention OpenAPI import? Default: add one line in PR 2.
