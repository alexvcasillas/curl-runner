// Regenerates the goldens of the wizard's YAML builder from the TypeScript
// implementation (`packages/cli/src/wizard/{templates,yaml-builder}.ts`).
//
// NOTE (2.0): the TS sources this script imports/runs were removed from packages/cli/src in the
// Rust cutover. To regenerate, check out the last pre-2.0 commit (7670118) and run it from there.
//
//   bun run crates/curl-runner/tests/golden/wizard/generate.ts
//
// For every case below it merges the template defaults (if any) with the
// answers, runs `buildYamlConfig` + `toYamlString` and writes
//   <name>.yaml  the exact bytes TS produces (no trailing newline),
//   <name>.json  the intended config (`JSON.stringify` of `buildYamlConfig`),
// plus `cases.json` (inputs and whether the TS text parses back to the
// intended config under `Bun.YAML.parse`). The Rust test
// (`src/wizard/yaml_builder.rs`) requires identical bytes where TS output
// round-trips, and a text that parses back to the intended config where it
// does not (the documented deviations).

import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { TEMPLATES } from '../../../../../packages/cli/src/wizard/templates';
import type { WizardAnswers } from '../../../../../packages/cli/src/wizard/types';
import {
  buildYamlConfig,
  toYamlString,
} from '../../../../../packages/cli/src/wizard/yaml-builder';

interface Case {
  name: string;
  template?: keyof typeof TEMPLATES;
  answers: Partial<WizardAnswers>;
}

const U = 'https://api.example.com/users';
const base = { url: U, method: 'GET' as const };

const cases: Case[] = [
  { name: 'minimal_get', answers: base },
  { name: 'method_post', answers: { ...base, method: 'POST' } },
  { name: 'method_options', answers: { ...base, method: 'OPTIONS' } },
  { name: 'name', answers: { ...base, name: 'My Request' } },
  { name: 'name_quoted', answers: { ...base, name: 'Users: list #1' } },
  {
    name: 'headers',
    answers: {
      ...base,
      headers: {
        'Content-Type': 'application/json',
        Authorization: 'Bearer token',
        Accept: 'application/json',
        'User-Agent': 'curl-runner',
        'X-Custom-Header': 'value',
      },
    },
  },
  {
    name: 'params',
    answers: { ...base, params: { page: '1', q: 'hello world', sort: 'name' } },
  },
  {
    name: 'body_json_flat',
    answers: { ...base, method: 'POST', bodyType: 'json', body: { name: 'test', value: 123 } },
  },
  {
    name: 'body_json_nested',
    answers: {
      ...base,
      method: 'POST',
      bodyType: 'json',
      body: {
        user: { name: 'ann', age: 31, admin: false, nick: null, tags: ['a', 'b', 'c'] },
        nums: [1, 2.5, -3, 1e21, 0.1],
        ok: true,
        deep: { a: { b: { c: 'x' } } },
      },
    },
  },
  {
    name: 'body_json_array_of_objects',
    answers: {
      ...base,
      method: 'POST',
      bodyType: 'json',
      body: {
        items: [
          { id: 1, name: 'a' },
          { id: 2, name: 'b', extra: { k: 'v' } },
        ],
      },
    },
  },
  {
    name: 'body_json_array_single_object_item',
    answers: {
      ...base,
      method: 'POST',
      bodyType: 'json',
      body: { items: [{ id: 1 }] },
    },
  },
  {
    name: 'body_json_array_nested_arrays',
    answers: {
      ...base,
      method: 'POST',
      bodyType: 'json',
      body: { grid: [[1, 2], [3, 4]], mixed: [{ a: [1, 2] }, [{ b: 1 }, { c: 2 }]] },
    },
  },
  {
    name: 'body_json_top_level_array',
    answers: { ...base, method: 'POST', bodyType: 'json', body: [{ a: 1, b: 2 }, { a: 3 }] },
  },
  {
    name: 'body_json_empty_containers',
    answers: { ...base, method: 'POST', bodyType: 'json', body: { a: {}, b: [], c: 'x' } },
  },
  {
    name: 'body_json_integer_keys',
    answers: {
      ...base,
      method: 'POST',
      bodyType: 'json',
      body: { b: 1, '2': 'two', a: 2, '1': 'one' },
    },
  },
  {
    name: 'body_json_number_body',
    answers: { ...base, method: 'POST', bodyType: 'json', body: 42 },
  },
  {
    name: 'body_json_zero_is_dropped',
    answers: { ...base, method: 'POST', bodyType: 'json', body: 0 },
  },
  {
    name: 'body_json_string_with_colon',
    answers: { ...base, method: 'POST', bodyType: 'json', body: { message: 'hello: world' } },
  },
  {
    name: 'form_data',
    answers: {
      ...base,
      method: 'POST',
      bodyType: 'form',
      formData: { field1: 'value1', file: { file: './upload.txt' } },
    },
  },
  {
    name: 'form_data_empty',
    answers: { ...base, method: 'POST', bodyType: 'form', formData: {} },
  },
  {
    name: 'body_raw',
    answers: { ...base, method: 'POST', bodyType: 'raw', body: 'plain text content' },
  },
  {
    name: 'body_raw_multiline',
    answers: { ...base, method: 'POST', bodyType: 'raw', body: 'line one\nline two' },
  },
  {
    name: 'body_raw_dash_item',
    answers: { ...base, method: 'POST', bodyType: 'raw', body: '- item' },
  },
  {
    name: 'body_raw_empty_dropped',
    answers: { ...base, method: 'POST', bodyType: 'raw', body: '' },
  },
  {
    name: 'body_none_ignores_body',
    answers: { ...base, method: 'POST', bodyType: 'none', body: { a: 1 } },
  },
  { name: 'auth_bearer', answers: { ...base, authType: 'bearer', token: 'my-token' } },
  {
    name: 'auth_basic',
    answers: { ...base, authType: 'basic', username: 'user', password: 'pass' },
  },
  {
    name: 'auth_basic_no_password',
    answers: { ...base, authType: 'basic', username: 'user' },
  },
  { name: 'auth_bearer_without_token', answers: { ...base, authType: 'bearer' } },
  {
    name: 'advanced',
    answers: {
      ...base,
      timeout: 5000,
      followRedirects: false,
      maxRedirects: 3,
      insecure: true,
      http2: true,
    },
  },
  { name: 'follow_redirects_true_omitted', answers: { ...base, followRedirects: true } },
  { name: 'timeout_zero_omitted', answers: { ...base, timeout: 0 } },
  {
    name: 'retry_full',
    answers: {
      ...base,
      retryEnabled: true,
      retryCount: 3,
      retryDelay: 1000,
      retryBackoff: 2,
    },
  },
  { name: 'retry_count_only', answers: { ...base, retryEnabled: true, retryCount: 5 } },
  { name: 'retry_disabled', answers: { ...base, retryEnabled: false, retryCount: 5 } },
  {
    name: 'expect_both',
    answers: { ...base, expectStatus: 200, expectResponseTime: '< 2000' },
  },
  { name: 'expect_status_array', answers: { ...base, expectStatus: [200, 201] } },
  { name: 'expect_time_only', answers: { ...base, expectResponseTime: '< 500' } },
  { name: 'output_file', answers: { ...base, outputFile: 'out/result.json' } },
  {
    name: 'everything',
    answers: {
      name: 'Full request',
      url: 'https://api.example.com/v1/items?x=1',
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', 'X-Trace': 'abc' },
      params: { a: '1' },
      bodyType: 'json',
      body: { title: 'hello', n: 2, list: [1, 2, 3], nested: { ok: true } },
      authType: 'bearer',
      token: 'tok_123',
      timeout: 10000,
      followRedirects: false,
      maxRedirects: 2,
      insecure: true,
      http2: true,
      retryEnabled: true,
      retryCount: 3,
      retryDelay: 500,
      retryBackoff: 2,
      expectStatus: [200, 204],
      expectResponseTime: '< 1500',
      outputFile: 'result.json',
    },
  },
  // Every template, as the wizard starts from it.
  { name: 'template_basic_get', template: 'basic-get', answers: { url: U } },
  {
    name: 'template_basic_post',
    template: 'basic-post',
    answers: { url: U, body: { name: 'test' } },
  },
  { name: 'template_api_test', template: 'api-test', answers: { url: U, name: 'Health' } },
  {
    name: 'template_file_upload',
    template: 'file-upload',
    answers: { url: U, formData: { avatar: { file: './me.png' }, kind: 'avatar' } },
  },
  {
    name: 'template_auth_flow',
    template: 'auth-flow',
    answers: { url: U, token: 'secret', body: { user: 'a', pass: 'b' } },
  },
  // The plain-string rule: ':' '#' newline, leading/trailing space, a leading
  // digit, true/false/null/yes/no in any case are quoted, nothing else.
  ...(
    [
      ['colon', 'a: b'],
      ['hash', 'x#y'],
      ['leading_space', ' x'],
      ['trailing_space', 'x '],
      ['digit_start', '1abc'],
      ['version', '2.0.1'],
      ['true', 'true'],
      ['false_upper', 'FALSE'],
      ['null_mixed', 'Null'],
      ['yes', 'yes'],
      ['no_cap', 'No'],
      ['quote_and_backslash', 'a"b\\c: d'],
      ['tab_colon', 'a\tb: c'],
      ['unicode', 'café: üñî'],
      ['url_value', 'https://example.com/path?x=1&y=2'],
      ['plain', 'hello world'],
      ['on_word', 'on'],
      ['tilde', '~'],
      ['accept_any', '*/*'],
      ['at_sign', '@handle'],
      ['negative_number', '-1'],
      ['flow_chars', 'a [b] {c}'],
      ['leading_quote', "'q'"],
      ['empty', ''],
      ['dollar_var', '${BASE_URL}/x'],
      ['percent', '50%'],
      ['pipe_start', '| x'],
      ['bang', '!x'],
      ['question', '? x'],
    ] as const
  ).map(
    ([n, v]): Case => ({
      name: `str_${n}`,
      answers: { ...base, method: 'POST', bodyType: 'json', body: { k: v } },
    }),
  ),
  {
    name: 'str_in_inline_array',
    answers: {
      ...base,
      method: 'POST',
      bodyType: 'json',
      body: { list: ['a', 'b c', 'x: y', '1', 'true'] },
    },
  },
  {
    name: 'str_in_inline_array_comma',
    answers: { ...base, method: 'POST', bodyType: 'json', body: { list: ['a,b', 'c'] } },
  },
  {
    name: 'keys_special',
    answers: {
      ...base,
      method: 'POST',
      bodyType: 'json',
      body: { 'my key': 1, 'Content-Type': 2, 'a.b': 3, '$ref': 4, 'k:v': 5 },
    },
  },
  { name: 'key_empty', answers: { ...base, method: 'POST', bodyType: 'json', body: { '': 1 } } },
  {
    name: 'key_yes',
    answers: { ...base, method: 'POST', bodyType: 'json', body: { yes: 1, '010': 2 } },
  },
];

const here = import.meta.dir;
mkdirSync(here, { recursive: true });
const manifest: (Case & { tsRoundTrips: boolean })[] = [];

for (const c of cases) {
  const answers = { ...(c.template ? TEMPLATES[c.template].defaults : {}), ...c.answers };
  const config = buildYamlConfig(answers as WizardAnswers);
  const yaml = toYamlString(config);
  const intended = JSON.parse(JSON.stringify(config));
  let ok = false;
  try {
    ok = Bun.deepEquals(Bun.YAML.parse(yaml), intended, true);
  } catch {
    ok = false;
  }
  writeFileSync(join(here, `${c.name}.yaml`), yaml);
  writeFileSync(join(here, `${c.name}.json`), JSON.stringify(intended));
  manifest.push({ ...c, tsRoundTrips: ok });
}

writeFileSync(join(here, 'cases.json'), `${JSON.stringify(manifest, null, 2)}\n`);
const broken = manifest.filter((m) => !m.tsRoundTrips).map((m) => m.name);
console.log(`${manifest.length} cases, ${broken.length} where the TS text does not round-trip:`);
console.log(broken.join('\n'));
