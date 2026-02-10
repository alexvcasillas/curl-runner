/**
 * Curl semantic parser: maps shell tokens to a structured CurlAST.
 */

import { tokenize } from './tokenizer';
import type { CurlAST } from './types';

/** Flags that take a value argument */
const VALUE_FLAGS = new Set([
  '-X', '--request',
  '-H', '--header',
  '-d', '--data', '--data-raw', '--data-binary', '--data-ascii',
  '--data-urlencode',
  '-F', '--form',
  '--form-string',
  '-u', '--user',
  '-o', '--output',
  '-x', '--proxy',
  '--max-time', '-m',
  '--max-redirs',
  '--cacert', '--cert', '--key',
  '-b', '--cookie',
  '-c', '--cookie-jar',
  '-A', '--user-agent',
  '-e', '--referer',
  '--connect-timeout',
  '--retry',
  '--retry-delay',
  '-w', '--write-out',
  '--resolve',
  '--interface',
  '--local-port',
  '--limit-rate',
  '--ciphers',
  '--proto',
  '--tlsv1.2', '--tlsv1.3',
]);

/** Flags that are boolean (no value) */
const BOOLEAN_FLAGS = new Set([
  '-G', '--get',
  '-I', '--head',
  '-L', '--location',
  '-k', '--insecure',
  '-s', '--silent',
  '-S', '--show-error',
  '-v', '--verbose',
  '--compressed',
  '--http2',
  '--http2-prior-knowledge',
  '--http1.1',
  '-N', '--no-buffer',
  '--raw',
  '-f', '--fail',
  '--fail-early',
  '--globoff',
  '-#', '--progress-bar',
  '--tr-encoding',
  '--tcp-nodelay',
  '--tcp-fastopen',
]);

/**
 * Parse a raw curl command string into a CurlAST.
 * Handles "curl ..." prefix and all common flags.
 */
export function parseCurl(input: string): CurlAST {
  const tokens = tokenize(input.trim());
  return parseTokens(tokens);
}

/**
 * Parse pre-tokenized tokens into CurlAST.
 */
export function parseTokens(tokens: string[]): CurlAST {
  const ast: CurlAST = {
    url: '',
    headers: [],
    unsupportedFlags: [],
  };

  const dataEntries: string[] = [];
  const dataRawEntries: string[] = [];
  const dataBinaryEntries: string[] = [];
  const dataUrlencodeEntries: string[] = [];
  const formEntries: string[] = [];
  const formStringEntries: string[] = [];

  let i = 0;

  // Skip "curl" command if present
  if (tokens.length > 0 && tokens[0] === 'curl') {
    i = 1;
  }

  while (i < tokens.length) {
    const token = tokens[i];

    // URL (non-flag argument)
    if (!token.startsWith('-')) {
      if (!ast.url) {
        ast.url = token;
      }
      i++;
      continue;
    }

    // Handle combined short flags like -sSL
    if (token.startsWith('-') && !token.startsWith('--') && token.length > 2) {
      // Check if this is a known value flag (e.g., -XPOST, -d'data')
      const firstFlag = `-${token[1]}`;
      if (VALUE_FLAGS.has(firstFlag)) {
        // e.g., -XPOST → -X POST
        const value = token.slice(2);
        i = processFlag(firstFlag, value, tokens, i, ast, dataEntries, dataRawEntries, dataBinaryEntries, dataUrlencodeEntries, formEntries, formStringEntries);
        continue;
      }

      // Combined boolean short flags like -sSL
      let allBoolean = true;
      for (let j = 1; j < token.length; j++) {
        const f = `-${token[j]}`;
        if (!BOOLEAN_FLAGS.has(f)) {
          allBoolean = false;
          break;
        }
      }

      if (allBoolean) {
        for (let j = 1; j < token.length; j++) {
          i = processFlag(`-${token[j]}`, undefined, tokens, i, ast, dataEntries, dataRawEntries, dataBinaryEntries, dataUrlencodeEntries, formEntries, formStringEntries);
        }
        i++;
        continue;
      }
    }

    // Value flags
    if (VALUE_FLAGS.has(token)) {
      const value = tokens[i + 1];
      i = processFlag(token, value, tokens, i, ast, dataEntries, dataRawEntries, dataBinaryEntries, dataUrlencodeEntries, formEntries, formStringEntries);
      continue;
    }

    // Boolean flags
    if (BOOLEAN_FLAGS.has(token)) {
      i = processFlag(token, undefined, tokens, i, ast, dataEntries, dataRawEntries, dataBinaryEntries, dataUrlencodeEntries, formEntries, formStringEntries);
      continue;
    }

    // Unknown flag
    if (token.startsWith('-')) {
      // Check if next token looks like a value (doesn't start with -)
      const next = tokens[i + 1];
      if (next && !next.startsWith('-')) {
        ast.unsupportedFlags.push({ flag: token, value: next });
        i += 2;
      } else {
        ast.unsupportedFlags.push({ flag: token });
        i++;
      }
      continue;
    }

    i++;
  }

  if (dataEntries.length > 0) ast.data = dataEntries;
  if (dataRawEntries.length > 0) ast.dataRaw = dataRawEntries;
  if (dataBinaryEntries.length > 0) ast.dataBinary = dataBinaryEntries;
  if (dataUrlencodeEntries.length > 0) ast.dataUrlencode = dataUrlencodeEntries;
  if (formEntries.length > 0) ast.form = formEntries;
  if (formStringEntries.length > 0) ast.formString = formStringEntries;

  return ast;
}

function processFlag(
  flag: string,
  value: string | undefined,
  tokens: string[],
  i: number,
  ast: CurlAST,
  data: string[],
  dataRaw: string[],
  dataBinary: string[],
  dataUrlencode: string[],
  form: string[],
  formString: string[],
): number {
  switch (flag) {
    case '-X':
    case '--request':
      ast.method = value?.toUpperCase();
      return i + 2;

    case '-H':
    case '--header': {
      if (value) {
        const colonIdx = value.indexOf(':');
        if (colonIdx > 0) {
          ast.headers.push({
            key: value.slice(0, colonIdx).trim(),
            value: value.slice(colonIdx + 1).trim(),
          });
        }
      }
      return i + 2;
    }

    case '-d':
    case '--data':
    case '--data-ascii':
      if (value !== undefined) data.push(value);
      return i + 2;

    case '--data-raw':
      if (value !== undefined) dataRaw.push(value);
      return i + 2;

    case '--data-binary':
      if (value !== undefined) dataBinary.push(value);
      return i + 2;

    case '--data-urlencode':
      if (value !== undefined) dataUrlencode.push(value);
      return i + 2;

    case '-F':
    case '--form':
      if (value !== undefined) form.push(value);
      return i + 2;

    case '--form-string':
      if (value !== undefined) formString.push(value);
      return i + 2;

    case '-u':
    case '--user':
      ast.user = value;
      return i + 2;

    case '-o':
    case '--output':
      ast.output = value;
      return i + 2;

    case '-x':
    case '--proxy':
      ast.proxy = value;
      return i + 2;

    case '--max-time':
    case '-m':
      if (value) ast.maxTime = Number(value);
      return i + 2;

    case '--max-redirs':
      if (value) ast.maxRedirs = Number(value);
      return i + 2;

    case '--cacert':
      ast.cacert = value;
      return i + 2;

    case '--cert':
      ast.cert = value;
      return i + 2;

    case '--key':
      ast.key = value;
      return i + 2;

    case '-b':
    case '--cookie':
      ast.cookie = value;
      return i + 2;

    case '-c':
    case '--cookie-jar':
      ast.cookieJar = value;
      return i + 2;

    case '-A':
    case '--user-agent':
      ast.userAgent = value;
      return i + 2;

    case '-e':
    case '--referer':
      ast.referer = value;
      return i + 2;

    case '-G':
    case '--get':
      ast.get = true;
      return i + 1;

    case '-I':
    case '--head':
      ast.head = true;
      return i + 1;

    case '-L':
    case '--location':
      ast.location = true;
      return i + 1;

    case '-k':
    case '--insecure':
      ast.insecure = true;
      return i + 1;

    case '--compressed':
      ast.compressed = true;
      return i + 1;

    case '--http2':
      ast.http2 = true;
      return i + 1;

    case '-s':
    case '--silent':
      ast.silent = true;
      return i + 1;

    case '-v':
    case '--verbose':
      ast.verbose = true;
      return i + 1;

    default:
      // Skip other known value flags
      if (VALUE_FLAGS.has(flag)) return i + 2;
      return i + 1;
  }
}
