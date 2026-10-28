// Re-records scalars.json: for each `yaml` input, what Bun.YAML.parse returns
// as JSON.stringify prints it (NaN and ±Infinity print as null), or `error`.
// Inputs: Bun's keyword/number edge cases plus an exhaustive set of short
// tokens over "019.eE+-xo_" (recorded with Bun 1.3.4).
// Run: bun crates/curl-runner-core/tests/fixtures/bun-yaml/generate.ts
const path = `${import.meta.dir}/scalars.json`;
const inputs: { yaml: string }[] = JSON.parse(await Bun.file(path).text());
const rows = inputs.map(({ yaml }) => {
  try {
    return { yaml, json: JSON.stringify(Bun.YAML.parse(yaml)) };
  } catch {
    return { yaml, error: true };
  }
});
await Bun.write(path, `${rows.map((r) => JSON.stringify(r)).join(",\n").replace(/^/, "[\n")}\n]\n`);
console.log(rows.length, Bun.version);
