# OpenAPI import

Status: draft, 2026-11-04

The owner handed the design calls for this feature to Claude. Every call below is Claude's, made to match the Postman import (`2026-11-03-postman-import-design.md`) wherever the two overlap.

## Why

Most HTTP APIs publish an OpenAPI document. It's usually the first thing someone has when they want to try an API, and they often have it before any Postman collection exists. The studio's Import dialog still shows its OpenAPI tab as "Coming soon", and the Postman spec named OpenAPI as the next round.

With an importer, someone points curl-runner at `openapi.yaml` and gets a folder of runnable requests, one per operation. Each request already checks the documented success status. Most of the work they'd do by hand is filling in example values, and the converter does that from the document's examples and schemas.

Two shapes guide the design:

- **A small, hand-written spec.** OpenAPI 3.0 in YAML. About 15 operations grouped by tags, one server, an API key or bearer token, JSON bodies with `example`s on some schemas and none on others, a few `$ref`s into `components`.
- **A large generated spec.** OpenAPI 3.1 in JSON, several megabytes, hundreds of operations, deep `$ref` chains, `allOf`/`oneOf` everywhere, recursive schemas, several security schemes per operation, form-encoded bodies. Payment and cloud provider specs look like this.

Swagger 2.0 still shows up in older APIs and many generated SDKs, so it's accepted too.

## Goals

1. One converter in `curl-runner-core`, shared by the CLI and the studio. It reuses the Postman importer's emitter, file writer and report.
2. `curl-runner convert openapi <spec> [-o <dir>]` writes a folder of YAML files and a `.env`, the same way `convert postman` does.
3. The studio's OpenAPI tab works: pick or drop a spec, see what will be written, import, get a summary.
4. A converted request runs against the API once the user sets the base URL (if the document lacks an absolute one) and the secrets. Nothing else is required, though the user will usually want to change sample values.
5. Each request checks its documented success status with `expect.status`, so a converted folder is a smoke test from the start.
6. Converting the large shape is fast: under a second in a release build for a 10 MB spec.

## Non-goals

- Fetching a spec from a URL. The CLI and studio take a local file. `curl -o spec.json <url>` covers this. A follow-up candidate.
- External `$ref`s (to another file or a URL). They're reported, and the schema behind one is treated as empty.
- Checking response bodies against schemas. Only the status is asserted.
- Webhooks, callbacks and links. They're counted in the report.
- Running OAuth 2 or OpenID Connect flows. Those schemes become a bearer token the user supplies.
- Keeping a spec and its YAML in sync, or exporting back to OpenAPI.
- Validating the OpenAPI document itself. The converter is lenient: it converts what it can read.

## Anti-goals

- A secret value written to any file. OpenAPI documents rarely hold secrets, but an `example` on an `Authorization` header or an API key parameter can. The same scrubbing rules as Postman apply.
- Overwriting an existing file or `.env` value.
- Dropping something that changes what a request sends without reporting it.
- Hanging, crashing or blowing the stack on a recursive or very deep schema.
- A report so long that the important items get lost. A large spec must not produce a line per operation for the same issue.
- Real-world specs in the repo. Tests use small hand-written specs only.

## Input

- OpenAPI 3.0.x, 3.1.x (`openapi` field) and Swagger 2.0 (`swagger: "2.0"`). Reject anything else with a message naming what was found, for example "Not an OpenAPI document: no \"openapi\" or \"swagger\" field".
- JSON or YAML, detected from the content rather than the extension. The core already depends on `serde_json` and `serde_yaml_ng`.
- Parse into `serde_json::Value` and walk it. Don't use strongly typed structs that fail the whole document because one field doesn't match the schema. Real specs often break small rules, and one bad operation shouldn't block the rest.
- Internal `$ref`s (`#/...`) are resolved wherever OpenAPI allows them: path items, parameters, request bodies, responses, schemas, security schemes, examples. JSON Pointer escaping (`~0`, `~1`) is handled. A `$ref` that can't be resolved is reported once per target.
- Swagger 2.0 is normalized into the 3.x shape internally before mapping, so the mapper handles one shape: `host` + `basePath` + `schemes` become a server, `in: body` becomes a request body, `in: formData` becomes a form body, `consumes` sets the media type, and `securityDefinitions` become security schemes.

## Mapping

| OpenAPI | curl-runner |
| --- | --- |
| `info.title` | Collection name. The CLI's default output folder is its slug. |
| `info.description` | `README.md` in the output folder, created only if missing. |
| First tag of an operation | Subfolder (slugged). An operation is written once, under its first tag. An untagged operation goes in a folder named after its first path segment that isn't a version (`v1`), `api` or a `{template}`, and in the root only when there's no such segment. (Amended after a real 600-operation spec without tags came out as one flat folder.) |
| Operation | One YAML file, named from `summary`, else `operationId`, else `method path`. The name is slugged with the Postman importer's `slug` and collisions get the same numeric suffixes. |
| `summary`, else `operationId`, else `GET /pets/{id}` | `name:` |
| `description`, `operationId`, `deprecated`, `externalDocs.url` | `#` comment lines at the top of the file, then the tag's description. HTML tags are stripped and basic entities decoded. |
| `servers[0].url` (operation, then path item, then document) | `${API_BASE_URL}` at the start of every URL, with its value in `.env` (see Variable names) |
| Server variables (`{region}`) | Replaced with their `default`. Report a variable with no default. |
| More than one server | The first is used. The others are listed as comment lines in `.env` next to the base URL variable. |
| A relative server URL (`/v1`) or no servers | The base URL variable is set to the relative part (or empty) and the report asks the user to set it. |
| An operation or path item with its own `servers` | The URL uses that server's literal URL rather than the base URL variable. Report it once per server. |
| Path template `{id}` | `${id}` in the URL, with a sample value in the file's `global.variables`, as Postman path variables are handled |
| Required query or header parameter | Enabled `params:` or `headers:` entry with a sample value |
| Optional query or header parameter | A disabled (commented-out) entry with a sample value, and the parameter's description as a note. This reuses the Postman importer's disabled fields. |
| Array query parameter | Its items joined with `,`. `explode` and the other `style` variants aren't modeled. A required parameter that needs them is reported. Optional ones are commented out, so they aren't. |
| Cookie parameter | One `Cookie` header joining the required ones. Optional ones are left out and reported together, once per request. |
| Path-item `parameters` | Merged with the operation's. The operation's parameters win on the same `name` + `in`. |
| `requestBody` | See Bodies below |
| `responses` | `expect.status:` set to the lowest documented `2xx` (or `2XX` → `200`). When there's no 2xx, only `default`, or no responses, no `expect` is written. |
| Security | See Auth below |
| `deprecated: true` | A `# Deprecated` comment |
| `x-*` extensions | Ignored |
| `webhooks`, `callbacks`, `links` | Counted in the report, not converted |

The file layout, `.env` handling, the README and the "write new files only" rule come from the Postman writer unchanged.

### Sample values

One generator gives a parameter or body its sample value. In order of preference:

1. The parameter's or media type's own `example`, then the first of its `examples` (resolving `$ref` and using `value`).
2. The schema's `example`, then `examples[0]` (3.1), `default`, `const` (3.1), then `enum[0]`.
3. A value generated from the schema:
   - `object`: every property in document order. Required and optional properties are both included, since the body is an example. Leaving optional ones out would hide fields the user may want. `additionalProperties` alone gives `{}`.
   - `array`: one item.
   - `string`: by `format`: `date-time` → an RFC 3339 timestamp, `date`, `email` → `user@example.com`, `uuid`, `uri` → `https://example.com`, `binary` → a file (multipart only). Otherwise `"string"`. `minLength` is ignored.
   - `integer`/`number`: `minimum` if set, else `1`. One rather than zero, because most integer path parameters are ids and ids rarely start at 0. `boolean`: `true`. `null` (3.1 `type: [x, "null"]`): the non-null type.
   - `allOf`: the parts merged as objects. `oneOf`/`anyOf`: the first option. When a request's body schema has more than one option at its top level, the report notes it, once per request.
   - A schema with no type: inferred from `properties`/`items`, else `null`.
4. Recursion stops when a `$ref` is already on the current path, and that position gets `{}` (object) or `[]` (array). Nesting stops at depth 10 the same way. Neither case is reported, since the value is only a sample.

5. Every sample has a size budget, about 200 values (the builder tunes it). Once it's spent, the generator stops adding optional properties. Required properties still get values until a hard cap of twice the budget, after which they get `{}`, `[]` or `null`. Nothing is reported, since the value is only a sample. This keeps a body built from a large spec's wide, cross-referencing schemas to a readable size, and keeps the conversion fast.

Generated values are typed as the schema says: an integer parameter gets `0`, not `"0"`. A query or header value is written as text.

### Bodies

Media type preference when a request body lists several: `application/json` or any `+json`, then `multipart/form-data`, then `application/x-www-form-urlencoded`, then `text/*`, `application/xml`. Anything else (`application/octet-stream`, images) is reported and left out.

- **JSON:** `body:` as structured YAML from the sample value, plus a `Content-Type` header with the chosen media type.
- **multipart:** each top-level property becomes a form field. `format: binary` (3.0), `contentMediaType` (3.1) or Swagger's `type: file` becomes a file field with the Postman importer's placeholder path, and is listed as "pick a file". Required fields are enabled. Optional ones are disabled, as in Postman. No `Content-Type` header, because curl sets the boundary.
- **form-urlencoded:** a urlencoded string body with the required properties, plus a `Content-Type` header. Optional properties are listed by name in a comment above the body. A string can't hold disabled lines, so this is the closest equivalent.
- **text and XML:** the `example` as a string when there is one, else an empty string, plus a `Content-Type`. XML isn't generated from schemas.
- **No request body:** no `body:`. A form schema that is explicitly empty (no properties, no `allOf`, `additionalProperties` false or absent) also means no body, and isn't reported.

### Auth

An operation's `security` overrides the document's. `security: []` means no auth.

`security` lists alternatives. Use the first requirement object. Every API key scheme in it applies. Only one `auth:` block exists, so a second bearer, basic or OAuth scheme in the same requirement is reported as skipped. When an operation offers more than one alternative, the report says which was used, once per distinct set of choices in the document, never once per operation.

| Scheme | curl-runner |
| --- | --- |
| `apiKey`, `in: header` | A header with the scheme's `name`, value `${VAR}` |
| `apiKey`, `in: query` | A query param |
| `apiKey`, `in: cookie` | Added to the `Cookie` header |
| `http`, `scheme: bearer` | `auth: {type: bearer, token: ${VAR}}` |
| `http`, `scheme: basic` | `auth: {type: basic, username: ${VAR_USERNAME}, password: ${VAR_PASSWORD}}` |
| `oauth2`, `openIdConnect` | `auth: {type: bearer, token: ${VAR}}`. The report says to put an access token there, once per scheme. |
| Any other `http` scheme (digest, ...) | Request without auth, reported once per scheme |

`VAR` is the API prefix (see Variable names) plus the security scheme's key from `components.securitySchemes`, made safe with the Postman importer's `var_name` and uppercased, for example `PETSTORE_API_KEY`. The builder picks the suffixes. Every auth variable except a basic username is a secret, handled exactly as in the Postman spec's Secrets section: the CLI writes `# NAME=` with the `.env.local` note, and the studio lists it as "set this secret" through the Keychain path. A literal credential found in a parameter `example` (the curl import's scrub list) also becomes a secret variable. Path parameters are identifiers, so one is scrubbed only when the document gives it a value and its name is exactly one of the secret parameter names. A generated sample is never treated as a secret.

### Variable names

Every import writes into the one `.env` at the output root, and that file keeps existing keys. Common names would collide: a second API's `baseUrl` or `bearerAuth` would quietly reuse the first API's value and send requests to the wrong host. Postman kept `baseUrl` because that was the collection's own name. Here the converter makes the names up, so it prefixes them per API:

- The prefix comes from `info.title`: uppercased, `[A-Z0-9_]` only, a run of other characters becomes one `_`, and it's cut at a word boundary to at most 24 characters. An empty title gives `API`. "Swagger Petstore - OpenAPI 3.0" becomes `SWAGGER_PETSTORE`.
- The base URL is `<PREFIX>_BASE_URL`. Auth variables are `<PREFIX>_<SCHEME>` plus their suffix.
- Names still go through `var_name`, and renames are reported as in Postman.
- Path parameters are per file (`global.variables`), so they keep their own names, made safe the same way.

## Report

The Postman `ImportReport` and its `Attention` list, with new variants where the existing ones don't fit (unresolved `$ref`, skipped media type, multiple servers, auth alternative chosen, webhooks/callbacks count). Their messages say "the API" rather than "the collection" when the item is document-wide. The builder decides whether the wording comes from a field on the report or from new variants.

Aggregation is the important rule. An issue that repeats across operations (an unresolved `$ref` target, an unsupported scheme, a skipped media type) is one entry with a count and up to three example operations, never one entry per operation. "Pick a file" stays one per field, because each one is something to do.

## Shared code

The Postman module's emitter, writer, slug and report are format-neutral already (`MappedRequest`, `Field`, `MappedBody`, `MappedAuth`, `ImportedVar`). Move them into a shared module under `convert/` (for example `convert/import/`). `postman/` and the new `openapi/` both map into it. Then:

- `MappedRequest` gains an optional expected status. The emitter writes `expect.status` when it's set. Postman never sets it, so Postman output doesn't change.
- `PostmanImport`/`write_postman` become a shared import type and writer. Keep the existing `postman::` names working as re-exports or aliases if that keeps the Postman diff small. Either way, every existing Postman test and snapshot passes unchanged.
- The curl converter's serializer and snapshots are untouched, as before.

## CLI

`curl-runner convert openapi <spec.json|spec.yaml> [-o <dir>]`

- Same behavior as `convert postman`: the default folder is the slug of `info.title`, only missing `.env` keys are added, the report goes to stderr, and the exit status is non-zero only when the input can't be read or isn't OpenAPI or Swagger.
- `convert --help` lists the new subcommand. `convert openapi --help` gets its own help, as Postman does.
- The shared `convert` help is pinned to TypeScript goldens. Adding a line there needs the golden updated in the same change, which is fine, since the TS CLI is gone. The builder checks how the Postman line was added and does the same.

## Studio

The OpenAPI tab replaces its "Coming soon" state with the same flow as the Postman tab: choose, preview, import, summary. No board exists for it. The Postman tab is the reference, and the OpenAPI tab should match it pixel for pixel apart from its copy.

- The picker accepts `.json`, `.yaml` and `.yml`. The copy says "Pick an OpenAPI or Swagger file, or drop it here."
- Share the Postman tab's rendering (preview pane, attention list, summary) rather than copying it. The two tabs differ only in their source and copy.
- **Drops.** Only the Postman tab accepts drops today. Both the Postman and OpenAPI tabs accept them, and a drop on either is routed by content. A file whose top level has `openapi` or `swagger` (parsed as JSON or YAML) goes to the OpenAPI tab, and the dialog switches to it. Anything else goes to Postman, as today, and shows Postman's parse error if it isn't a collection. The curl and Folder tabs don't accept drops.
- The dev fill aid (`CURL_RUNNER_STUDIO_FILL`) routes a spec file to the OpenAPI tab, so screenshots can be scripted.
- The `openapi_tab_imports_nothing` test is replaced with tests for the working tab. The Folder tab keeps its own behavior.
- The empty-state copy that says OpenAPI comes "in a later build" goes away.

## Constraints

- Rust, in `curl-runner-core/src/convert/openapi/`. No new dependency unless the builder shows it's needed. `serde_json` and `serde_yaml_ng` cover parsing.
- Performance is a requirement (AGENTS.md):
  - resolve `$ref`s lazily by pointer lookup on the parsed `Value`, not by building a fully expanded copy;
  - memoize nothing that could make recursion unbounded;
  - add a test that generates a large synthetic spec (thousands of operations, deep `$ref` chains) and converts it within a generous time budget, so a quadratic path shows up.
- Every change gets unit tests, plus `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.
- Test inputs are small hand-written specs, inline or under `crates/curl-runner-core/tests/fixtures/openapi/`. Together they cover both shapes in Why, Swagger 2.0, YAML and JSON input, a recursive schema, an unresolved `$ref` and each auth row.
- Docs, in the same PRs (owner rule): a new `features/openapi-import.mdx` page modeled on `postman-import.mdx`, an "OpenAPI to YAML" section and CLI reference row in `features/convert.mdx`, `cli-commands.mdx`, `studio.mdx` no longer listing OpenAPI as Coming soon, and a studio screenshot of the OpenAPI tab (preview and summary), taken in the background as the Postman screenshots were.
- A changeset (minor) describing what a user notices.
- Public repo: no employer or customer details anywhere (AGENTS.md).

## Delivery

Two PRs, so each one stays reviewable:

1. **Core and CLI.** The shared module move, the OpenAPI converter, `convert openapi`, CLI docs and the convert docs. Changeset included.
2. **Studio.** The OpenAPI tab, drop routing, studio docs and screenshots.

Then the changesets version PR and the release, checked the same way as 2.4.0. The owner authorized Claude to merge both PRs once CI is green, merge the version PR and run the release, without a separate check-in.

## Done when

- Converting the fixture specs gives one YAML file per operation, in a folder per tag, with `baseUrl` and the auth secrets in `.env`, optional parameters commented out in place, and `expect.status` on every operation with a documented 2xx.
- Every unsupported input in the fixtures appears in the report, aggregated as described.
- End to end, outside the repo: the public Swagger Petstore v3 spec is converted with the CLI. The user then sets `SWAGGER_PETSTORE_BASE_URL` (the spec's server is relative) and the API key secret, and may change path parameter values to records that exist. Nothing else is edited, and every `GET` runs green with `curl-runner` against the live demo server. Writes (POST/PUT/DELETE) are run too, but a failure there caused only by demo data doesn't block release. A large public spec (Stripe's or GitHub's) converts in under a second in a release build, with a readable report. That timing is a hard gate for the release.
- The studio imports the same Petstore spec through the OpenAPI tab, and the screenshots match the Postman tab's layout.

## Left to the builder

- Module names and how the Postman types are moved or aliased.
- Suffixes for auth variable names (`_TOKEN`, `_USERNAME`, ...).
- The exact report wording and new `Attention` variants, within the aggregation rule.
- The `date-time`/`date`/`uuid` sample values. They can be fixed literals, since output must be deterministic for tests.
- Whether tag descriptions repeat in every file of the folder (as Postman folder descriptions do) or only in a folder README. Pick the one that reads better on a large spec.
