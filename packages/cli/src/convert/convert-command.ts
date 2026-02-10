/**
 * Convert command handler.
 * Routes: convert curl "...", convert file script.sh, convert yaml test.yaml
 */

import type { RequestConfig, YamlFile } from '../types/config';
import { extractCurlCommands } from './batch-parser';
import { generateCurlFromIR } from './curl-generator';
import { parseCurl } from './curl-parser';
import { normalize } from './normalizer';
import { tokenize } from './tokenizer';
import type { ConvertOptions, CurlRunnerIR } from './types';
import { serializeBatchToYaml, serializeToYaml } from './yaml-serializer';
import { yamlToIR } from './yaml-to-ir';

export interface ConvertResult {
  output: string;
  warnings: string[];
  debug?: {
    tokens?: string[];
    ast?: unknown;
    ir?: CurlRunnerIR | CurlRunnerIR[];
  };
}

/**
 * Convert a curl command string → YAML.
 */
export function convertCurlToYaml(curlCmd: string, options?: ConvertOptions): ConvertResult {
  const ast = parseCurl(curlCmd);
  const ir = normalize(ast);

  const output = serializeToYaml(ir, {
    pretty: options?.pretty ?? true,
    lossReport: options?.lossReport ?? true,
  });

  const result: ConvertResult = { output, warnings: ir.metadata.warnings };

  if (options?.debug) {
    result.debug = {
      tokens: tokenize(curlCmd.trim()),
      ast,
      ir,
    };
  }

  return result;
}

/**
 * Convert a shell script file → YAML (batch).
 */
export async function convertFileToYaml(
  filePath: string,
  options?: ConvertOptions,
): Promise<ConvertResult> {
  const content = await Bun.file(filePath).text();
  const curlCommands = extractCurlCommands(content);

  if (curlCommands.length === 0) {
    return { output: '', warnings: ['No curl commands found in file'] };
  }

  const irs: CurlRunnerIR[] = [];
  const allWarnings: string[] = [];

  for (let idx = 0; idx < curlCommands.length; idx++) {
    const ast = parseCurl(curlCommands[idx]);
    const ir = normalize(ast);
    ir.name = `request_${idx + 1}`;
    irs.push(ir);
    allWarnings.push(...ir.metadata.warnings);
  }

  if (irs.length === 1) {
    const output = serializeToYaml(irs[0], {
      pretty: options?.pretty ?? true,
      lossReport: options?.lossReport ?? true,
    });
    const result: ConvertResult = { output, warnings: allWarnings };
    if (options?.debug) {
      result.debug = { ir: irs[0] };
    }
    return result;
  }

  const output = serializeBatchToYaml(irs, {
    pretty: options?.pretty ?? true,
    lossReport: options?.lossReport ?? true,
  });

  const result: ConvertResult = { output, warnings: allWarnings };
  if (options?.debug) {
    result.debug = { ir: irs };
  }
  return result;
}

/**
 * Convert a YAML file → curl command(s).
 */
export async function convertYamlToCurl(
  filePath: string,
  _options?: ConvertOptions,
): Promise<ConvertResult> {
  const content = await Bun.file(filePath).text();
  const yaml = Bun.YAML.parse(content) as YamlFile;
  const requests: RequestConfig[] = [];

  if (yaml.request) {
    requests.push(yaml.request);
  }
  if (yaml.requests) {
    requests.push(...yaml.requests);
  }
  if (yaml.collection?.requests) {
    requests.push(...yaml.collection.requests);
  }

  if (requests.length === 0) {
    return { output: '', warnings: ['No requests found in YAML file'] };
  }

  const allWarnings: string[] = [];
  const curlCommands: string[] = [];

  for (const req of requests) {
    const ir = yamlToIR(req);
    allWarnings.push(...ir.metadata.warnings);
    curlCommands.push(generateCurlFromIR(ir));
  }

  const output = curlCommands.join('\n\n');

  return { output, warnings: allWarnings };
}

/**
 * Parse convert subcommand args.
 */
export interface ConvertArgs {
  subcommand: 'curl' | 'file' | 'yaml';
  input: string;
  options: ConvertOptions;
}

export function parseConvertArgs(args: string[]): ConvertArgs | null {
  // args[0] is 'convert', args[1] is subcommand
  if (args.length < 3) {
    return null;
  }

  const sub = args[1];
  if (sub !== 'curl' && sub !== 'file' && sub !== 'yaml') {
    return null;
  }

  const input = args[2];
  const options: ConvertOptions = {};

  for (let i = 3; i < args.length; i++) {
    const arg = args[i];
    switch (arg) {
      case '--output':
      case '-o':
        options.output = args[++i];
        break;
      case '--pretty':
        options.pretty = true;
        break;
      case '--normalize':
        options.normalize = true;
        break;
      case '--loss-report':
        options.lossReport = true;
        break;
      case '--batch':
        options.batch = true;
        break;
      case '--env-detect':
        options.envDetect = true;
        break;
      case '--debug':
        options.debug = true;
        break;
    }
  }

  return { subcommand: sub, input, options };
}
