// Regenerates the expected outputs of `cases.json` from the TypeScript CLI.
// NOTE (2.0): the TS sources this script imports/runs were removed from packages/cli/src in the
// Rust cutover. To regenerate, check out the last pre-2.0 commit (7670118) and run it from there.
//
//   bun run crates/curl-runner/tests/golden/convert/generate.ts [cases.json] [outDir]
//
// For every case it runs `bun run packages/cli/src/cli.ts convert <args>` in a
// fresh temp dir holding the case's `files`, and records stdout, stderr (ANSI
// colors included), the exit code and, when the case has `out`, the file the
// command wrote. The Rust test (`tests/convert_cli.rs`) replays the same cases
// against the Rust binary and requires identical bytes.

import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

interface Case {
  name: string;
  args: string[];
  files?: Record<string, string>;
  out?: string;
}

const here = import.meta.dir;
const cli = resolve(here, '../../../../../packages/cli/src/cli.ts');
const casesPath = resolve(process.argv[2] ?? join(here, 'cases.json'));
const outDir = resolve(process.argv[3] ?? here);
const cases: Case[] = JSON.parse(readFileSync(casesPath, 'utf8'));
mkdirSync(outDir, { recursive: true });

for (const c of cases) {
  const work = mkdtempSync(join(tmpdir(), 'convert-golden-'));
  try {
    for (const [name, content] of Object.entries(c.files ?? {})) {
      const p = join(work, name);
      mkdirSync(dirname(p), { recursive: true });
      writeFileSync(p, content);
    }
    const r = Bun.spawnSync(['bun', 'run', cli, 'convert', ...c.args], {
      cwd: work,
      env: { PATH: process.env.PATH ?? '', HOME: work, CI: '1' },
    });
    writeFileSync(join(outDir, `${c.name}.stdout`), r.stdout);
    writeFileSync(join(outDir, `${c.name}.stderr`), r.stderr);
    writeFileSync(join(outDir, `${c.name}.code`), `${r.exitCode}\n`);
    if (c.out) {
      try {
        writeFileSync(join(outDir, `${c.name}.out`), readFileSync(join(work, c.out)));
      } catch {
        // the command did not write the file
      }
    }
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}
console.log(`generated ${cases.length} cases in ${outDir}`);
