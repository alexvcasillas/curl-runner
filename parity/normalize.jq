# Normalizes a curl-runner --output JSON file for TS-vs-Rust comparison.
# Use with `jq -S -f parity/normalize.jq` (-S sorts keys: key order is not part
# of the contract, see crates/CHANGES-FROM-TS.md).
#
# Dropped (summary and result objects only): `duration`, `metrics` (whole object:
# timings differ per run), `updatedAt`, `capturedAt`.
# Masked: UUIDs, ISO-8601 timestamps, RFC 1123 dates (response `date` header),
# epoch-millisecond timestamps (`${TIMESTAMP}`), times of day (`${TIME}`), multipart boundaries (random per
# request) and measured times in validation errors ("got 8.37ms").
def mask:
  if type == "string" then
    gsub("[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}"; "<uuid>")
    | gsub("[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\\.[0-9]+)?(Z|[+-][0-9]{2}:?[0-9]{2})?"; "<date>")
    | gsub("(?<![0-9])1[0-9]{12}(?![0-9])"; "<epoch-ms>")
    | gsub("(?<![0-9:])[0-9]{2}:[0-9]{2}:[0-9]{2}(?![0-9:])"; "<time>")
    | gsub("-{10,}[A-Za-z0-9]{16,}"; "<boundary>")
    | gsub("got [0-9.]+ms"; "got <n>ms")
    | gsub("(Mon|Tue|Wed|Thu|Fri|Sat|Sun), [0-9]{2} [A-Z][a-z]{2} [0-9]{4} [0-9:]{8} GMT"; "<date>")
  else . end;
# `duration`/`metrics`/... are removed only on the summary object and on result
# objects, never inside response bodies or headers.
def clean: del(.duration, .metrics, .updatedAt, .capturedAt);
# --profile-export files: an array of {request, summary, timings}. Latencies vary per
# run, so they are masked; iteration counts, failures and the failure rate are kept.
def profile_norm:
  if type == "array" and length > 0 and all(type == "object" and has("summary"))
  then map({request,
            summary: (.summary | with_entries(
              if (.key | IN("iterations", "warmup", "failures", "failureRate")) then .
              else .value |= (if type == "number" then "<ms>" else . end) end)),
            timings: {count: (.timings | length), numeric: (.timings | all(type == "number"))}})
  else . end;
profile_norm
| (if type == "object" then clean else . end)
| (if type == "object" and (.results | type) == "array"
   then .results |= map(if type == "object" then clean else . end)
   else . end)
| walk(if type == "string" then mask else . end)
