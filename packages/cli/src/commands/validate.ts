// Validate command for curl-runner
// Validates YAML files against curl-runner schema and proposes/applies fixes

import { Glob } from 'bun';
import { YamlParser } from '../parser/yaml';
import type {
  GlobalConfig,
  RequestConfig,
  SSLConfig,
  WhenCondition,
  YamlFile,
} from '../types/config';
import { color } from '../utils/colors';

interface ValidateOptions {
  fix?: boolean;
  quiet?: boolean;
  strict?: boolean;
}

interface ValidationIssue {
  file: string;
  path: string;
  severity: 'error' | 'warning' | 'info';
  message: string;
  fix?: {
    description: string;
    apply: () => unknown;
  };
}

interface ValidationResult {
  file: string;
  valid: boolean;
  issues: ValidationIssue[];
  fixedContent?: string;
}

// Valid HTTP methods
const VALID_METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'] as const;

// Valid auth types
const VALID_AUTH_TYPES = ['basic', 'bearer', 'digest', 'ntlm', 'oauth2'] as const;

// Valid execution modes
const VALID_EXECUTION_MODES = ['sequential', 'parallel'] as const;

// Valid output formats
const VALID_OUTPUT_FORMATS = ['json', 'pretty', 'raw'] as const;

// Valid pretty levels
const VALID_PRETTY_LEVELS = ['minimal', 'standard', 'detailed'] as const;

// Valid condition operators
const VALID_OPERATORS = [
  '==',
  '!=',
  '>',
  '<',
  '>=',
  '<=',
  'contains',
  'matches',
  'exists',
  'not-exists',
] as const;

// Valid snapshot update modes
const VALID_SNAPSHOT_UPDATE_MODES = ['none', 'all', 'failing'] as const;

// Valid diff output formats
const VALID_DIFF_OUTPUT_FORMATS = ['terminal', 'json', 'markdown'] as const;

// Common URL typos
const URL_TYPOS: Record<string, string> = {
  htpp: 'http',
  htps: 'https',
  htp: 'http',
  'http:/': 'http://',
  'https:/': 'https://',
  locahost: 'localhost',
  localhsot: 'localhost',
  loaclhost: 'localhost',
  localost: 'localhost',
  '127.0.0.0': '127.0.0.1',
};

// Standard HTTP headers with proper casing
const STANDARD_HEADERS: Record<string, string> = {
  'content-type': 'Content-Type',
  'content-length': 'Content-Length',
  'content-encoding': 'Content-Encoding',
  'content-language': 'Content-Language',
  'content-disposition': 'Content-Disposition',
  accept: 'Accept',
  'accept-encoding': 'Accept-Encoding',
  'accept-language': 'Accept-Language',
  'accept-charset': 'Accept-Charset',
  authorization: 'Authorization',
  'cache-control': 'Cache-Control',
  connection: 'Connection',
  cookie: 'Cookie',
  'set-cookie': 'Set-Cookie',
  host: 'Host',
  origin: 'Origin',
  referer: 'Referer',
  'user-agent': 'User-Agent',
  'x-requested-with': 'X-Requested-With',
  'x-forwarded-for': 'X-Forwarded-For',
  'x-forwarded-proto': 'X-Forwarded-Proto',
  'x-api-key': 'X-API-Key',
  'x-auth-token': 'X-Auth-Token',
  'x-csrf-token': 'X-CSRF-Token',
  'x-request-id': 'X-Request-ID',
  'x-correlation-id': 'X-Correlation-ID',
  etag: 'ETag',
  'if-match': 'If-Match',
  'if-none-match': 'If-None-Match',
  'if-modified-since': 'If-Modified-Since',
  'if-unmodified-since': 'If-Unmodified-Since',
  'last-modified': 'Last-Modified',
  location: 'Location',
  pragma: 'Pragma',
  'proxy-authorization': 'Proxy-Authorization',
  range: 'Range',
  'content-range': 'Content-Range',
  te: 'TE',
  trailer: 'Trailer',
  'transfer-encoding': 'Transfer-Encoding',
  upgrade: 'Upgrade',
  vary: 'Vary',
  via: 'Via',
  warning: 'Warning',
  'www-authenticate': 'WWW-Authenticate',
  'access-control-allow-origin': 'Access-Control-Allow-Origin',
  'access-control-allow-methods': 'Access-Control-Allow-Methods',
  'access-control-allow-headers': 'Access-Control-Allow-Headers',
  'access-control-allow-credentials': 'Access-Control-Allow-Credentials',
  'access-control-expose-headers': 'Access-Control-Expose-Headers',
  'access-control-max-age': 'Access-Control-Max-Age',
  'strict-transport-security': 'Strict-Transport-Security',
  'x-content-type-options': 'X-Content-Type-Options',
  'x-frame-options': 'X-Frame-Options',
  'x-xss-protection': 'X-XSS-Protection',
};

// Known valid request config keys
const VALID_REQUEST_KEYS = [
  'name',
  'url',
  'method',
  'headers',
  'params',
  'body',
  'formData',
  'timeout',
  'followRedirects',
  'maxRedirects',
  'auth',
  'proxy',
  'insecure',
  'ssl',
  'output',
  'http2',
  'retry',
  'variables',
  'store',
  'when',
  'expect',
  'snapshot',
  'diff',
  'sourceOutputConfig',
  'sourceFile',
];

// Known valid global config keys
const VALID_GLOBAL_KEYS = [
  'execution',
  'maxConcurrency',
  'continueOnError',
  'dryRun',
  'http2',
  'connectionPool',
  'ci',
  'ssl',
  'watch',
  'profile',
  'snapshot',
  'diff',
  'variables',
  'output',
  'defaults',
];

// Known valid collection keys
const VALID_COLLECTION_KEYS = ['name', 'description', 'variables', 'defaults', 'requests'];

// Sensitive patterns that shouldn't be hardcoded
const SENSITIVE_PATTERNS = [
  /^(password|passwd|pwd|secret|api[_-]?key|apikey|token|auth|credential|private[_-]?key)$/i,
  /Bearer\s+[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+/i, // JWT
  /^[A-Fa-f0-9]{32,}$/, // Hex keys
  /^sk[-_]live[-_]/i, // Stripe keys
  /^AKIA[0-9A-Z]{16}$/i, // AWS keys
];

export class ValidateCommand {
  private issues: ValidationIssue[] = [];
  private currentFile = '';
  private definedVariables: Set<string> = new Set();
  private storedVariables: Set<string> = new Set();
  private requestNames: Set<string> = new Set();

  async run(args: string[]): Promise<void> {
    const options = this.parseArgs(args);
    const patterns = args.filter((a) => !a.startsWith('-'));

    if (patterns.length === 0) {
      patterns.push('*.yaml', '*.yml');
    }

    console.log();
    console.log(color('curl-runner validate', 'bright'));
    console.log();

    const files = await this.findYamlFiles(patterns);

    if (files.length === 0) {
      console.log(color('No YAML files found', 'yellow'));
      return;
    }

    console.log(`${color('Files:', 'cyan')} ${files.length} found`);
    console.log();

    let totalIssues = 0;
    let totalFixed = 0;
    let invalidFiles = 0;

    for (const file of files) {
      const result = await this.validateFile(file, options);
      const errors = result.issues.filter((i) => i.severity === 'error');
      const warnings = result.issues.filter((i) => i.severity === 'warning');
      const infos = result.issues.filter((i) => i.severity === 'info');

      if (!result.valid || (options.strict && warnings.length > 0)) {
        invalidFiles++;
        totalIssues += result.issues.length;

        console.log(`${color('✗', 'red')} ${file}`);
        for (const issue of result.issues) {
          const icon =
            issue.severity === 'error'
              ? color('●', 'red')
              : issue.severity === 'warning'
                ? color('●', 'yellow')
                : color('●', 'blue');
          console.log(`  ${icon} ${issue.path}: ${issue.message}`);
          if (issue.fix && !options.fix) {
            console.log(`    ${color('fix:', 'cyan')} ${issue.fix.description}`);
          }
        }

        if (options.fix && result.fixedContent) {
          await Bun.write(file, result.fixedContent);
          const fixedCount = result.issues.filter((i) => i.fix).length;
          totalFixed += fixedCount;
          console.log(`  ${color('✓', 'green')} Fixed ${fixedCount} issue(s)`);
        }

        // Show summary per file
        const parts = [];
        if (errors.length > 0) {
          parts.push(`${errors.length} error(s)`);
        }
        if (warnings.length > 0) {
          parts.push(`${warnings.length} warning(s)`);
        }
        if (infos.length > 0) {
          parts.push(`${infos.length} info`);
        }
        if (parts.length > 0) {
          console.log(`  ${color('→', 'dim')} ${parts.join(', ')}`);
        }
        console.log();
      } else if (!options.quiet) {
        console.log(`${color('✓', 'green')} ${file}`);
      }
    }

    console.log();
    if (invalidFiles === 0) {
      console.log(color('All files valid!', 'green'));
    } else {
      console.log(
        `${color('Summary:', 'cyan')} ${invalidFiles} file(s) with ${totalIssues} issue(s)`,
      );
      if (options.fix) {
        console.log(`${color('Fixed:', 'green')} ${totalFixed} issue(s)`);
      }
      process.exit(1);
    }
  }

  private parseArgs(args: string[]): ValidateOptions {
    const options: ValidateOptions = {};

    for (const arg of args) {
      if (arg === '--fix' || arg === '-f') {
        options.fix = true;
      } else if (arg === '--quiet' || arg === '-q') {
        options.quiet = true;
      } else if (arg === '--strict' || arg === '-s') {
        options.strict = true;
      }
    }

    return options;
  }

  private async findYamlFiles(patterns: string[]): Promise<string[]> {
    const files: Set<string> = new Set();
    const fs = await import('node:fs/promises');

    for (const pattern of patterns) {
      try {
        const stat = await fs.stat(pattern);
        if (stat.isDirectory()) {
          const globber = new Glob(`${pattern}/**/*.{yaml,yml}`);
          for await (const file of globber.scan('.')) {
            files.add(file);
          }
        } else if (stat.isFile() && (pattern.endsWith('.yaml') || pattern.endsWith('.yml'))) {
          files.add(pattern);
        }
      } catch {
        // Treat as glob pattern
        const globber = new Glob(pattern);
        for await (const file of globber.scan('.')) {
          if (file.endsWith('.yaml') || file.endsWith('.yml')) {
            files.add(file);
          }
        }
      }
    }

    return Array.from(files).sort();
  }

  private async validateFile(file: string, options: ValidateOptions): Promise<ValidationResult> {
    this.issues = [];
    this.currentFile = file;
    this.definedVariables = new Set();
    this.storedVariables = new Set();
    this.requestNames = new Set();

    let content: YamlFile;

    try {
      content = await YamlParser.parseFile(file);
    } catch (error) {
      const msg = error instanceof Error ? error.message : String(error);
      this.addIssue('', 'error', `Failed to parse YAML: ${msg}`);
      return { file, valid: false, issues: this.issues };
    }

    // Collect defined variables first
    this.collectDefinedVariables(content);

    // Validate structure
    this.validateYamlStructure(content);

    // Validate global config
    if (content.global) {
      this.validateGlobalConfig(content.global, 'global');
    }

    // Validate collection
    if (content.collection) {
      this.validateCollection(content.collection, 'collection');
    }

    // Validate single request
    if (content.request) {
      this.validateRequest(content.request, 'request');
    }

    // Validate requests array
    if (content.requests) {
      if (!Array.isArray(content.requests)) {
        this.addIssue('requests', 'error', 'Must be an array');
      } else {
        content.requests.forEach((req, i) => {
          this.validateRequest(req, `requests[${i}]`);
        });
      }
    }

    // Check for duplicate request names
    this.checkDuplicateRequestNames();

    // Build fixed content if needed
    let fixedContent: string | undefined;
    if (options.fix && this.issues.some((i) => i.fix)) {
      const fixedData = JSON.parse(JSON.stringify(content));
      for (const issue of this.issues) {
        if (issue.fix) {
          const result = issue.fix.apply();
          this.applyFix(fixedData, issue.path, result);
        }
      }
      fixedContent = this.toYaml(fixedData);
    }

    const errors = this.issues.filter((i) => i.severity === 'error');

    return {
      file,
      valid: errors.length === 0,
      issues: this.issues,
      fixedContent,
    };
  }

  private collectDefinedVariables(content: YamlFile): void {
    // Global variables
    if (content.global?.variables) {
      for (const key of Object.keys(content.global.variables)) {
        this.definedVariables.add(key);
      }
    }

    // Collection variables
    if (content.collection?.variables) {
      for (const key of Object.keys(content.collection.variables)) {
        this.definedVariables.add(key);
      }
    }

    // Environment variables are always available
    this.definedVariables.add('env');

    // Request variables (individual requests)
    const allRequests = [
      ...(content.requests || []),
      ...(content.collection?.requests || []),
      content.request,
    ].filter(Boolean) as RequestConfig[];

    for (const req of allRequests) {
      if (req.variables) {
        for (const key of Object.keys(req.variables)) {
          this.definedVariables.add(key);
        }
      }
      // Store variables become available after the request
      if (req.store) {
        for (const key of Object.keys(req.store)) {
          this.storedVariables.add(key);
        }
      }
    }
  }

  private validateYamlStructure(content: YamlFile): void {
    const hasRequest = 'request' in content;
    const hasRequests = 'requests' in content;
    const hasCollection = 'collection' in content;

    if (!hasRequest && !hasRequests && !hasCollection) {
      this.addIssue(
        '',
        'error',
        'Missing request(s). Must have "request", "requests", or "collection"',
      );
    }

    if (hasRequest && hasRequests) {
      this.addIssue('', 'warning', 'Both "request" and "requests" defined; both will be executed');
    }

    if (hasCollection && (hasRequest || hasRequests)) {
      this.addIssue('', 'warning', 'Collection mixed with request/requests; may cause confusion');
    }

    // Check for unknown top-level keys
    const validKeys = ['version', 'global', 'collection', 'requests', 'request'];
    for (const key of Object.keys(content)) {
      if (!validKeys.includes(key)) {
        this.addIssue(key, 'warning', `Unknown top-level key "${key}"`, {
          description: `Remove unknown key "${key}"`,
          apply: () => undefined,
        });
      }
    }

    // Validate version if present
    if (content.version !== undefined) {
      if (typeof content.version !== 'string') {
        this.addIssue('version', 'warning', 'Version should be a string');
      }
    }
  }

  private validateGlobalConfig(config: GlobalConfig, path: string): void {
    // Check for unknown keys
    this.checkUnknownKeys(config, VALID_GLOBAL_KEYS, path);

    // Execution mode
    if (config.execution && !VALID_EXECUTION_MODES.includes(config.execution)) {
      const suggestion = this.findClosestMatch(config.execution, [...VALID_EXECUTION_MODES]);
      this.addIssue(
        `${path}.execution`,
        'error',
        `Invalid execution mode "${config.execution}". Must be: ${VALID_EXECUTION_MODES.join(', ')}`,
        suggestion
          ? {
              description: `Change to "${suggestion}"`,
              apply: () => suggestion,
            }
          : undefined,
      );
    }

    // Max concurrency
    if (config.maxConcurrency !== undefined) {
      if (typeof config.maxConcurrency !== 'number' || config.maxConcurrency < 1) {
        this.addIssue(`${path}.maxConcurrency`, 'error', 'Must be a positive number');
      }
      if (config.execution === 'sequential') {
        this.addIssue(
          `${path}.maxConcurrency`,
          'warning',
          'maxConcurrency has no effect with sequential execution',
        );
      }
    }

    // Boolean fields
    this.validateBooleanField(config, 'continueOnError', path);
    this.validateBooleanField(config, 'dryRun', path);
    this.validateBooleanField(config, 'http2', path);

    // Output config
    if (config.output) {
      this.validateOutputConfig(config.output, `${path}.output`);
    }

    // Connection pool
    if (config.connectionPool) {
      this.validateConnectionPool(config.connectionPool, `${path}.connectionPool`);
    }

    // CI config
    if (config.ci) {
      this.validateCIConfig(config.ci, `${path}.ci`);
    }

    // SSL config
    if (config.ssl) {
      this.validateSSLConfig(config.ssl, `${path}.ssl`);
    }

    // Watch config
    if (config.watch) {
      this.validateWatchConfig(config.watch, `${path}.watch`);
    }

    // Profile config
    if (config.profile) {
      this.validateProfileConfig(config.profile, `${path}.profile`);
    }

    // Snapshot config
    if (config.snapshot) {
      this.validateSnapshotConfig(config.snapshot, `${path}.snapshot`);
    }

    // Diff config
    if (config.diff) {
      this.validateDiffConfig(config.diff, `${path}.diff`);
    }

    // Variables
    if (config.variables) {
      this.validateVariables(config.variables, `${path}.variables`);
    }

    // Defaults
    if (config.defaults) {
      this.validateRequestConfig(config.defaults, `${path}.defaults`, true);
    }
  }

  private validateBooleanField(obj: Record<string, unknown>, field: string, path: string): void {
    if (obj[field] !== undefined && typeof obj[field] !== 'boolean') {
      const strVal = String(obj[field]).toLowerCase();
      if (strVal === 'true' || strVal === 'false') {
        this.addIssue(
          `${path}.${field}`,
          'warning',
          `Should be boolean, not string "${obj[field]}"`,
          {
            description: `Convert to ${strVal}`,
            apply: () => strVal === 'true',
          },
        );
      } else {
        this.addIssue(`${path}.${field}`, 'error', 'Must be a boolean');
      }
    }
  }

  private validateVariables(variables: Record<string, string>, path: string): void {
    if (typeof variables !== 'object' || Array.isArray(variables)) {
      this.addIssue(path, 'error', 'Must be an object');
      return;
    }

    for (const [key, value] of Object.entries(variables)) {
      // Check variable naming
      if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(key)) {
        this.addIssue(
          `${path}.${key}`,
          'warning',
          'Variable names should use alphanumeric characters and underscores',
        );
      }

      // Check for sensitive values
      if (typeof value === 'string' && this.isSensitiveValue(key, value)) {
        this.addIssue(
          `${path}.${key}`,
          'warning',
          'Potentially sensitive value hardcoded. Consider using environment variables like "$' +
            '{env.VAR}"',
        );
      }
    }
  }

  private validateOutputConfig(output: NonNullable<GlobalConfig['output']>, path: string): void {
    const validKeys = [
      'verbose',
      'showHeaders',
      'showBody',
      'showMetrics',
      'format',
      'prettyLevel',
      'saveToFile',
      'dryRun',
    ];
    this.checkUnknownKeys(output, validKeys, path);

    if (output.format && !VALID_OUTPUT_FORMATS.includes(output.format)) {
      const suggestion = this.findClosestMatch(output.format, [...VALID_OUTPUT_FORMATS]);
      this.addIssue(
        `${path}.format`,
        'error',
        `Invalid format "${output.format}". Must be: ${VALID_OUTPUT_FORMATS.join(', ')}`,
        suggestion
          ? {
              description: `Change to "${suggestion}"`,
              apply: () => suggestion,
            }
          : undefined,
      );
    }

    if (output.prettyLevel && !VALID_PRETTY_LEVELS.includes(output.prettyLevel)) {
      const suggestion = this.findClosestMatch(output.prettyLevel, [...VALID_PRETTY_LEVELS]);
      this.addIssue(
        `${path}.prettyLevel`,
        'error',
        `Invalid prettyLevel "${output.prettyLevel}". Must be: ${VALID_PRETTY_LEVELS.join(', ')}`,
        suggestion
          ? {
              description: `Change to "${suggestion}"`,
              apply: () => suggestion,
            }
          : undefined,
      );
    }

    // Boolean fields
    this.validateBooleanField(output as Record<string, unknown>, 'verbose', path);
    this.validateBooleanField(output as Record<string, unknown>, 'showHeaders', path);
    this.validateBooleanField(output as Record<string, unknown>, 'showBody', path);
    this.validateBooleanField(output as Record<string, unknown>, 'showMetrics', path);
  }

  private validateConnectionPool(
    pool: NonNullable<GlobalConfig['connectionPool']>,
    path: string,
  ): void {
    const validKeys = ['enabled', 'maxStreamsPerHost', 'keepaliveTime', 'connectTimeout'];
    this.checkUnknownKeys(pool, validKeys, path);

    this.validateBooleanField(pool as Record<string, unknown>, 'enabled', path);

    if (pool.maxStreamsPerHost !== undefined) {
      if (typeof pool.maxStreamsPerHost !== 'number' || pool.maxStreamsPerHost < 1) {
        this.addIssue(`${path}.maxStreamsPerHost`, 'error', 'Must be a positive number');
      } else if (pool.maxStreamsPerHost > 100) {
        this.addIssue(
          `${path}.maxStreamsPerHost`,
          'warning',
          'Very high stream count may cause issues with some servers',
        );
      }
    }
    if (pool.keepaliveTime !== undefined) {
      if (typeof pool.keepaliveTime !== 'number' || pool.keepaliveTime < 0) {
        this.addIssue(`${path}.keepaliveTime`, 'error', 'Must be a non-negative number');
      }
    }
    if (pool.connectTimeout !== undefined) {
      if (typeof pool.connectTimeout !== 'number' || pool.connectTimeout < 0) {
        this.addIssue(`${path}.connectTimeout`, 'error', 'Must be a non-negative number');
      }
    }
  }

  private validateCIConfig(ci: NonNullable<GlobalConfig['ci']>, path: string): void {
    const validKeys = ['strictExit', 'failOn', 'failOnPercentage'];
    this.checkUnknownKeys(ci, validKeys, path);

    this.validateBooleanField(ci as Record<string, unknown>, 'strictExit', path);

    if (ci.failOn !== undefined && (typeof ci.failOn !== 'number' || ci.failOn < 0)) {
      this.addIssue(`${path}.failOn`, 'error', 'Must be a non-negative number');
    }
    if (ci.failOnPercentage !== undefined) {
      if (
        typeof ci.failOnPercentage !== 'number' ||
        ci.failOnPercentage < 0 ||
        ci.failOnPercentage > 100
      ) {
        this.addIssue(`${path}.failOnPercentage`, 'error', 'Must be a number between 0 and 100');
      }
    }

    // Warn if both failOn and failOnPercentage are set
    if (ci.failOn !== undefined && ci.failOnPercentage !== undefined) {
      this.addIssue(
        path,
        'info',
        'Both failOn and failOnPercentage are set; failOn takes precedence',
      );
    }
  }

  private validateSSLConfig(ssl: SSLConfig, path: string): void {
    const validKeys = ['verify', 'ca', 'cert', 'key'];
    this.checkUnknownKeys(ssl as Record<string, unknown>, validKeys, path);

    this.validateBooleanField(ssl as Record<string, unknown>, 'verify', path);

    if (ssl.ca && typeof ssl.ca !== 'string') {
      this.addIssue(`${path}.ca`, 'error', 'Must be a string path');
    }
    if (ssl.cert && typeof ssl.cert !== 'string') {
      this.addIssue(`${path}.cert`, 'error', 'Must be a string path');
    }
    if (ssl.key && typeof ssl.key !== 'string') {
      this.addIssue(`${path}.key`, 'error', 'Must be a string path');
    }
    if (ssl.cert && !ssl.key) {
      this.addIssue(`${path}`, 'warning', 'cert provided without key; mTLS may fail');
    }
    if (ssl.key && !ssl.cert) {
      this.addIssue(`${path}`, 'warning', 'key provided without cert; mTLS may fail');
    }

    // Verify path patterns
    if (ssl.ca && typeof ssl.ca === 'string') {
      this.validateFilePath(ssl.ca, `${path}.ca`);
    }
    if (ssl.cert && typeof ssl.cert === 'string') {
      this.validateFilePath(ssl.cert, `${path}.cert`);
    }
    if (ssl.key && typeof ssl.key === 'string') {
      this.validateFilePath(ssl.key, `${path}.key`);
    }
  }

  private validateFilePath(filePath: string, path: string): void {
    // Skip if it's a variable reference
    if (filePath.includes('${')) {
      return;
    }

    // Check for suspicious patterns
    if (filePath.includes('..')) {
      this.addIssue(path, 'info', 'Path contains ".."; ensure this is intentional');
    }
  }

  private validateWatchConfig(watch: NonNullable<GlobalConfig['watch']>, path: string): void {
    const validKeys = ['paths', 'debounce', 'ignore'];
    this.checkUnknownKeys(watch as Record<string, unknown>, validKeys, path);

    if (watch.debounce !== undefined) {
      if (typeof watch.debounce !== 'number' || watch.debounce < 0) {
        this.addIssue(`${path}.debounce`, 'error', 'Must be a non-negative number');
      }
    }

    if (watch.paths !== undefined && !Array.isArray(watch.paths)) {
      this.addIssue(`${path}.paths`, 'error', 'Must be an array of paths');
    }

    if (watch.ignore !== undefined && !Array.isArray(watch.ignore)) {
      this.addIssue(`${path}.ignore`, 'error', 'Must be an array of patterns');
    }
  }

  private validateProfileConfig(profile: NonNullable<GlobalConfig['profile']>, path: string): void {
    const validKeys = ['iterations', 'warmup', 'concurrency', 'percentiles'];
    this.checkUnknownKeys(profile as Record<string, unknown>, validKeys, path);

    if (profile.iterations !== undefined) {
      if (typeof profile.iterations !== 'number' || profile.iterations < 1) {
        this.addIssue(`${path}.iterations`, 'error', 'Must be a positive number');
      } else if (profile.iterations > 10000) {
        this.addIssue(`${path}.iterations`, 'warning', 'Very high iteration count may take long');
      }
    }
    if (profile.warmup !== undefined) {
      if (typeof profile.warmup !== 'number' || profile.warmup < 0) {
        this.addIssue(`${path}.warmup`, 'error', 'Must be a non-negative number');
      }
    }
    if (profile.concurrency !== undefined) {
      if (typeof profile.concurrency !== 'number' || profile.concurrency < 1) {
        this.addIssue(`${path}.concurrency`, 'error', 'Must be a positive number');
      }
    }
  }

  private validateSnapshotConfig(
    snapshot: NonNullable<GlobalConfig['snapshot']>,
    path: string,
  ): void {
    const validKeys = ['updateMode', 'dir', 'ignoreFields', 'ignoreHeaders'];
    this.checkUnknownKeys(snapshot as Record<string, unknown>, validKeys, path);

    if (snapshot.updateMode && !VALID_SNAPSHOT_UPDATE_MODES.includes(snapshot.updateMode)) {
      this.addIssue(
        `${path}.updateMode`,
        'error',
        `Invalid updateMode "${snapshot.updateMode}". Must be: ${VALID_SNAPSHOT_UPDATE_MODES.join(', ')}`,
      );
    }
    if (snapshot.dir && typeof snapshot.dir !== 'string') {
      this.addIssue(`${path}.dir`, 'error', 'Must be a string');
    }
  }

  private validateDiffConfig(diff: NonNullable<GlobalConfig['diff']>, path: string): void {
    const validKeys = ['outputFormat', 'dir', 'ignoreFields', 'ignoreHeaders', 'baselineUrl'];
    this.checkUnknownKeys(diff as Record<string, unknown>, validKeys, path);

    if (diff.outputFormat && !VALID_DIFF_OUTPUT_FORMATS.includes(diff.outputFormat)) {
      this.addIssue(
        `${path}.outputFormat`,
        'error',
        `Invalid outputFormat "${diff.outputFormat}". Must be: ${VALID_DIFF_OUTPUT_FORMATS.join(', ')}`,
      );
    }
    if (diff.dir && typeof diff.dir !== 'string') {
      this.addIssue(`${path}.dir`, 'error', 'Must be a string');
    }
  }

  private validateCollection(collection: NonNullable<YamlFile['collection']>, path: string): void {
    this.checkUnknownKeys(collection as Record<string, unknown>, VALID_COLLECTION_KEYS, path);

    if (!collection.name) {
      this.addIssue(`${path}.name`, 'warning', 'Collection name is recommended');
    }

    if (!collection.requests || !Array.isArray(collection.requests)) {
      this.addIssue(`${path}.requests`, 'error', 'Collection must have a requests array');
    } else {
      if (collection.requests.length === 0) {
        this.addIssue(`${path}.requests`, 'warning', 'Collection has no requests');
      }
      collection.requests.forEach((req, i) => {
        this.validateRequest(req, `${path}.requests[${i}]`);
      });
    }

    if (collection.variables) {
      this.validateVariables(collection.variables, `${path}.variables`);
    }

    if (collection.defaults) {
      this.validateRequestConfig(collection.defaults, `${path}.defaults`, true);
    }
  }

  private validateRequest(request: RequestConfig, path: string): void {
    this.validateRequestConfig(request, path, false);
  }

  private validateRequestConfig(
    request: Partial<RequestConfig>,
    path: string,
    isDefaults: boolean,
  ): void {
    // Check for unknown keys
    this.checkUnknownKeys(request as Record<string, unknown>, VALID_REQUEST_KEYS, path);

    // Track request name
    if (request.name) {
      this.requestNames.add(`${path}:${request.name}`);
    }

    // URL is required for non-defaults
    if (!isDefaults && !request.url) {
      this.addIssue(`${path}.url`, 'error', 'URL is required');
    }

    // URL validation
    if (request.url) {
      this.validateURL(request.url, `${path}.url`);
    }

    // Method validation
    if (request.method) {
      this.validateMethod(request.method, `${path}.method`);
    }

    // Headers validation
    if (request.headers) {
      this.validateHeaders(request.headers, `${path}.headers`, request);
    }

    // Params validation
    if (request.params) {
      if (typeof request.params !== 'object' || Array.isArray(request.params)) {
        this.addIssue(`${path}.params`, 'error', 'Params must be an object');
      }
    }

    // Body and formData mutual exclusion
    if (request.body !== undefined && request.formData !== undefined) {
      this.addIssue(`${path}`, 'error', 'Cannot use both "body" and "formData"');
    }

    // Body validation
    if (request.body !== undefined) {
      this.validateBody(request.body, `${path}.body`, request);
    }

    // FormData validation
    if (request.formData) {
      this.validateFormData(request.formData, `${path}.formData`);
    }

    // Timeout validation
    if (request.timeout !== undefined) {
      this.validateTimeout(request.timeout, `${path}.timeout`);
    }

    // Auth validation
    if (request.auth) {
      this.validateAuth(request.auth, `${path}.auth`);
    }

    // SSL validation
    if (request.ssl) {
      this.validateSSLConfig(request.ssl, `${path}.ssl`);
    }

    // Insecure validation
    if (request.insecure !== undefined) {
      this.validateBooleanField(request as Record<string, unknown>, 'insecure', path);
      if (request.insecure === true) {
        this.addIssue(`${path}.insecure`, 'warning', 'Insecure mode disables SSL verification');
      }
    }

    // Proxy validation
    if (request.proxy) {
      this.validateProxyURL(request.proxy, `${path}.proxy`);
    }

    // Retry validation
    if (request.retry) {
      this.validateRetry(request.retry, `${path}.retry`, request.method);
    }

    // When condition validation
    if (request.when) {
      this.validateWhenCondition(request.when, `${path}.when`);
    }

    // Expect validation
    if (request.expect) {
      this.validateExpect(request.expect, `${path}.expect`);
    }

    // Store validation
    if (request.store) {
      this.validateStore(request.store, `${path}.store`);
    }

    // Max redirects validation
    if (request.maxRedirects !== undefined) {
      if (typeof request.maxRedirects !== 'number' || request.maxRedirects < 0) {
        this.addIssue(`${path}.maxRedirects`, 'error', 'Must be a non-negative number');
      } else if (request.maxRedirects > 50) {
        this.addIssue(`${path}.maxRedirects`, 'warning', 'Very high redirect limit');
      }
    }

    // followRedirects validation
    this.validateBooleanField(request as Record<string, unknown>, 'followRedirects', path);
    this.validateBooleanField(request as Record<string, unknown>, 'http2', path);

    // Snapshot/diff validation
    if (request.snapshot !== undefined && typeof request.snapshot === 'object') {
      this.validateSnapshotConfig(
        request.snapshot as NonNullable<GlobalConfig['snapshot']>,
        `${path}.snapshot`,
      );
    }

    if (request.diff !== undefined && typeof request.diff === 'object') {
      this.validateDiffConfig(request.diff as NonNullable<GlobalConfig['diff']>, `${path}.diff`);
    }
  }

  private validateURL(url: string, path: string): void {
    if (typeof url !== 'string') {
      this.addIssue(path, 'error', 'URL must be a string');
      return;
    }

    // Skip extensive validation if URL contains variables
    const hasVariables = url.includes('${');

    // Check for common typos
    let fixedUrl = url;
    let hasTypo = false;
    for (const [typo, fix] of Object.entries(URL_TYPOS)) {
      if (url.toLowerCase().includes(typo)) {
        fixedUrl = url.replace(new RegExp(typo, 'gi'), fix);
        hasTypo = true;
        this.addIssue(path, 'error', `URL contains typo "${typo}"`, {
          description: `Fix to "${fix}"`,
          apply: () => fixedUrl,
        });
        break;
      }
    }

    // Check for missing protocol
    if (!hasTypo && !url.startsWith('http') && !hasVariables) {
      this.addIssue(path, 'warning', 'URL should start with http:// or https://', {
        description: 'Prepend https://',
        apply: () => `https://${url}`,
      });
    }

    // Check for double slashes in path (not protocol)
    if (!hasVariables && url.match(/https?:\/\/[^/]+\/\/+/)) {
      const fixed = url.replace(/(https?:\/\/[^/]+)(\/+)/g, '$1/');
      this.addIssue(path, 'warning', 'URL contains double slashes in path', {
        description: 'Remove extra slashes',
        apply: () => fixed,
      });
    }

    // Check for spaces in URL
    if (url.includes(' ')) {
      const fixed = url.replace(/ /g, '%20');
      this.addIssue(path, 'error', 'URL contains spaces', {
        description: 'Encode spaces as %20',
        apply: () => fixed,
      });
    }

    // Security warning for http in production
    if (url.startsWith('http://') && !url.includes('localhost') && !url.includes('127.0.0.1')) {
      this.addIssue(path, 'info', 'Using HTTP instead of HTTPS; consider using HTTPS for security');
    }

    // Validate port number if present
    if (!hasVariables) {
      const portMatch = url.match(/:(\d+)/);
      if (portMatch) {
        const port = parseInt(portMatch[1], 10);
        if (port < 1 || port > 65535) {
          this.addIssue(path, 'error', `Invalid port number ${port}. Must be 1-65535`);
        }
      }
    }
  }

  private validateProxyURL(proxy: string, path: string): void {
    if (typeof proxy !== 'string') {
      this.addIssue(path, 'error', 'Proxy must be a string URL');
      return;
    }

    if (!proxy.includes('${') && !proxy.match(/^(https?|socks[45]?):\/\//i)) {
      this.addIssue(
        path,
        'error',
        'Proxy URL should start with http://, https://, socks4://, or socks5://',
      );
    }
  }

  private validateMethod(method: string, path: string): void {
    const upperMethod = method.toUpperCase();
    if (!VALID_METHODS.includes(upperMethod as (typeof VALID_METHODS)[number])) {
      const suggestion = this.findClosestMatch(upperMethod, [...VALID_METHODS]);
      this.addIssue(
        path,
        'error',
        `Invalid method "${method}". Must be: ${VALID_METHODS.join(', ')}`,
        suggestion
          ? {
              description: `Change to "${suggestion}"`,
              apply: () => suggestion,
            }
          : undefined,
      );
    } else if (method !== upperMethod) {
      this.addIssue(path, 'warning', `Method should be uppercase: "${upperMethod}"`, {
        description: `Change to "${upperMethod}"`,
        apply: () => upperMethod,
      });
    }
  }

  private validateHeaders(
    headers: Record<string, string>,
    path: string,
    request: Partial<RequestConfig>,
  ): void {
    if (typeof headers !== 'object' || Array.isArray(headers)) {
      this.addIssue(path, 'error', 'Headers must be an object');
      return;
    }

    const seenHeaders = new Map<string, string>();

    for (const [key, value] of Object.entries(headers)) {
      // Check for proper header casing
      const lowerKey = key.toLowerCase();
      const standardCasing = STANDARD_HEADERS[lowerKey];

      if (standardCasing && key !== standardCasing) {
        this.addIssue(
          `${path}.${key}`,
          'info',
          `Header "${key}" should be "${standardCasing}" for consistency`,
          {
            description: `Rename to "${standardCasing}"`,
            apply: () => ({ rename: standardCasing, value }),
          },
        );
      }

      // Check for duplicate headers (case-insensitive)
      if (seenHeaders.has(lowerKey)) {
        this.addIssue(
          `${path}.${key}`,
          'warning',
          `Duplicate header "${key}" (case-insensitive match with "${seenHeaders.get(lowerKey)}")`,
        );
      }
      seenHeaders.set(lowerKey, key);

      // Check header value
      if (typeof value !== 'string' && typeof value !== 'number') {
        this.addIssue(`${path}.${key}`, 'error', 'Header value must be a string or number');
      }

      // Check for sensitive values in headers
      if (typeof value === 'string' && this.isSensitiveValue(key, value)) {
        this.addIssue(
          `${path}.${key}`,
          'warning',
          'Potentially sensitive header value. Consider using variables like "$' + '{env.TOKEN}"',
        );
      }
    }

    // Check Content-Type consistency with body
    const contentType = this.getHeaderValue(headers, 'content-type');
    if (request.body !== undefined && !contentType) {
      this.addIssue(path, 'info', 'Body present but no Content-Type header; consider adding one');
    }

    // Warn if both auth block and Authorization header exist
    const hasAuthHeader = this.getHeaderValue(headers, 'authorization');
    if (request.auth && hasAuthHeader) {
      this.addIssue(
        path,
        'warning',
        'Both auth block and Authorization header present; auth block may be ignored',
      );
    }
  }

  private getHeaderValue(headers: Record<string, string>, name: string): string | undefined {
    const lowerName = name.toLowerCase();
    for (const [key, value] of Object.entries(headers)) {
      if (key.toLowerCase() === lowerName) {
        return value;
      }
    }
    return undefined;
  }

  private validateBody(body: unknown, path: string, request: Partial<RequestConfig>): void {
    // Check if method supports body
    const method = (request.method || 'GET').toUpperCase();
    if (['GET', 'HEAD', 'OPTIONS'].includes(method)) {
      this.addIssue(path, 'warning', `Body with ${method} method may be ignored by some servers`);
    }

    // If body is a string, check if it looks like JSON
    if (typeof body === 'string') {
      const trimmed = body.trim();
      if (
        (trimmed.startsWith('{') && trimmed.endsWith('}')) ||
        (trimmed.startsWith('[') && trimmed.endsWith(']'))
      ) {
        try {
          JSON.parse(trimmed);
        } catch {
          this.addIssue(path, 'warning', 'Body looks like JSON but is invalid; check syntax');
        }
      }
    }
  }

  private validateFormData(formData: NonNullable<RequestConfig['formData']>, path: string): void {
    if (typeof formData !== 'object' || Array.isArray(formData)) {
      this.addIssue(path, 'error', 'FormData must be an object');
      return;
    }

    for (const [key, value] of Object.entries(formData)) {
      if (typeof value === 'object' && value !== null && 'file' in value) {
        if (typeof value.file !== 'string') {
          this.addIssue(`${path}.${key}.file`, 'error', 'File path must be a string');
        } else {
          this.validateFilePath(value.file, `${path}.${key}.file`);
        }
        if (value.contentType && typeof value.contentType !== 'string') {
          this.addIssue(`${path}.${key}.contentType`, 'error', 'contentType must be a string');
        }
        if (value.filename && typeof value.filename !== 'string') {
          this.addIssue(`${path}.${key}.filename`, 'error', 'filename must be a string');
        }
      }
    }
  }

  private validateTimeout(timeout: unknown, path: string): void {
    if (typeof timeout !== 'number') {
      // Try to parse string
      if (typeof timeout === 'string') {
        const parsed = parseInt(timeout, 10);
        if (!Number.isNaN(parsed) && parsed > 0) {
          this.addIssue(path, 'warning', 'Timeout should be a number, not string', {
            description: `Convert to ${parsed}`,
            apply: () => parsed,
          });
          return;
        }
      }
      this.addIssue(path, 'error', 'Timeout must be a positive number');
      return;
    }

    if (timeout <= 0) {
      this.addIssue(path, 'error', 'Timeout must be a positive number');
    } else if (timeout < 100) {
      this.addIssue(path, 'warning', 'Very short timeout (<100ms) may cause failures');
    } else if (timeout > 300000) {
      this.addIssue(path, 'warning', 'Very long timeout (>5min) - is this intentional?');
    }
  }

  private validateAuth(auth: NonNullable<RequestConfig['auth']>, path: string): void {
    const validKeys = ['type', 'username', 'password', 'token'];
    this.checkUnknownKeys(auth as Record<string, unknown>, validKeys, path);

    if (!auth.type) {
      this.addIssue(`${path}.type`, 'error', 'Auth type is required');
      return;
    }

    if (!VALID_AUTH_TYPES.includes(auth.type as (typeof VALID_AUTH_TYPES)[number])) {
      const suggestion = this.findClosestMatch(auth.type, [...VALID_AUTH_TYPES]);
      this.addIssue(
        `${path}.type`,
        'error',
        `Invalid auth type "${auth.type}". Must be: ${VALID_AUTH_TYPES.join(', ')}`,
        suggestion
          ? {
              description: `Change to "${suggestion}"`,
              apply: () => suggestion,
            }
          : undefined,
      );
      return;
    }

    if (auth.type === 'basic') {
      if (!auth.username) {
        this.addIssue(`${path}.username`, 'error', 'Username required for basic auth');
      }
      if (!auth.password) {
        this.addIssue(`${path}.password`, 'warning', 'Password missing for basic auth');
      }
      // Check for hardcoded credentials
      if (auth.password && typeof auth.password === 'string' && !auth.password.includes('${')) {
        this.addIssue(
          `${path}.password`,
          'warning',
          'Hardcoded password. Consider using "$' + '{env.PASSWORD}"',
        );
      }
    }

    if (auth.type === 'bearer') {
      if (!auth.token) {
        this.addIssue(`${path}.token`, 'error', 'Token required for bearer auth');
      }
      // Check for hardcoded token
      if (auth.token && typeof auth.token === 'string' && !auth.token.includes('${')) {
        this.addIssue(
          `${path}.token`,
          'warning',
          'Hardcoded token. Consider using "$' + '{env.TOKEN}"',
        );
      }
    }
  }

  private validateRetry(
    retry: NonNullable<RequestConfig['retry']>,
    path: string,
    method?: string,
  ): void {
    const validKeys = ['count', 'delay', 'backoff'];
    this.checkUnknownKeys(retry as Record<string, unknown>, validKeys, path);

    if (retry.count === undefined) {
      this.addIssue(`${path}.count`, 'error', 'Retry count is required');
    } else if (typeof retry.count !== 'number' || retry.count < 0) {
      this.addIssue(`${path}.count`, 'error', 'Retry count must be a non-negative number');
    } else if (retry.count > 10) {
      this.addIssue(`${path}.count`, 'warning', 'High retry count (>10) may cause long delays');
    }

    if (retry.delay !== undefined) {
      if (typeof retry.delay !== 'number' || retry.delay < 0) {
        this.addIssue(`${path}.delay`, 'error', 'Retry delay must be a non-negative number');
      }
    }

    if (retry.backoff !== undefined) {
      if (typeof retry.backoff !== 'number' || retry.backoff < 1) {
        this.addIssue(`${path}.backoff`, 'error', 'Retry backoff must be >= 1');
      } else if (retry.backoff > 5) {
        this.addIssue(
          `${path}.backoff`,
          'warning',
          'High backoff multiplier may cause very long delays',
        );
      }
    }

    // Warn about retrying non-idempotent methods
    if (
      method &&
      ['POST', 'PATCH'].includes(method.toUpperCase()) &&
      retry.count &&
      retry.count > 0
    ) {
      this.addIssue(path, 'info', `Retrying ${method} requests may cause duplicate operations`);
    }
  }

  private validateWhenCondition(when: WhenCondition | string, path: string): void {
    if (typeof when === 'string') {
      // String shorthand - validate syntax
      this.validateWhenString(when, path);
      return;
    }

    // Object form
    const hasAll = 'all' in when && when.all;
    const hasAny = 'any' in when && when.any;
    const hasSingle = 'left' in when;

    if (!hasAll && !hasAny && !hasSingle) {
      this.addIssue(path, 'error', 'When condition must have "all", "any", or "left"');
      return;
    }

    if ((hasAll && hasAny) || (hasAll && hasSingle) || (hasAny && hasSingle)) {
      this.addIssue(
        path,
        'warning',
        'When condition has multiple forms; behavior may be unexpected',
      );
    }

    if (hasAll) {
      if (!Array.isArray(when.all)) {
        this.addIssue(`${path}.all`, 'error', 'Must be an array');
      } else {
        if (when.all.length === 0) {
          this.addIssue(`${path}.all`, 'warning', 'Empty "all" array - condition always true');
        }
        when.all.forEach((cond, i) => {
          this.validateConditionExpression(cond, `${path}.all[${i}]`);
        });
      }
    }

    if (hasAny) {
      if (!Array.isArray(when.any)) {
        this.addIssue(`${path}.any`, 'error', 'Must be an array');
      } else {
        if (when.any.length === 0) {
          this.addIssue(`${path}.any`, 'warning', 'Empty "any" array - condition always false');
        }
        when.any.forEach((cond, i) => {
          this.validateConditionExpression(cond, `${path}.any[${i}]`);
        });
      }
    }

    if (hasSingle) {
      this.validateConditionExpression(
        when as { left?: string; operator?: string; right?: unknown },
        path,
      );
    }
  }

  private validateWhenString(when: string, path: string): void {
    // Check for common patterns
    if (!when.includes('store.') && !when.includes('env.')) {
      this.addIssue(path, 'warning', 'When string should reference store.* or env.* values');
    }

    // Check for valid operator
    const operatorPattern = /\s*(==|!=|>=|<=|>|<|contains|matches|exists|not-exists)\s*/;
    if (!operatorPattern.test(when)) {
      this.addIssue(
        path,
        'warning',
        'When string should contain a valid operator (==, !=, >, <, >=, <=, contains, matches, exists)',
      );
    }
  }

  private validateConditionExpression(
    cond: { left?: string; operator?: string; right?: unknown },
    path: string,
  ): void {
    if (!cond.left) {
      this.addIssue(`${path}.left`, 'error', 'Left operand is required');
    } else {
      // Validate left references store or env
      if (!cond.left.startsWith('store.') && !cond.left.startsWith('env.')) {
        this.addIssue(
          `${path}.left`,
          'warning',
          'Left operand should start with "store." or "env."',
        );
      }
    }

    if (!cond.operator) {
      this.addIssue(`${path}.operator`, 'error', 'Operator is required');
    } else if (!VALID_OPERATORS.includes(cond.operator as (typeof VALID_OPERATORS)[number])) {
      const suggestion = this.findClosestMatch(cond.operator, [...VALID_OPERATORS]);
      this.addIssue(
        `${path}.operator`,
        'error',
        `Invalid operator "${cond.operator}". Must be: ${VALID_OPERATORS.join(', ')}`,
        suggestion
          ? {
              description: `Change to "${suggestion}"`,
              apply: () => suggestion,
            }
          : undefined,
      );
    }

    // exists/not-exists don't need right operand
    if (cond.operator !== 'exists' && cond.operator !== 'not-exists') {
      if (cond.right === undefined) {
        this.addIssue(`${path}.right`, 'error', 'Right operand is required');
      }
    }
  }

  private validateExpect(expect: NonNullable<RequestConfig['expect']>, path: string): void {
    const validKeys = ['failure', 'status', 'headers', 'body', 'responseTime'];
    this.checkUnknownKeys(expect as Record<string, unknown>, validKeys, path);

    this.validateBooleanField(expect as Record<string, unknown>, 'failure', path);

    // Status validation
    if (expect.status !== undefined) {
      this.validateExpectStatus(expect.status, `${path}.status`);
    }

    // Headers validation
    if (expect.headers) {
      if (typeof expect.headers !== 'object' || Array.isArray(expect.headers)) {
        this.addIssue(`${path}.headers`, 'error', 'Expected headers must be an object');
      }
    }

    // Response time validation
    if (expect.responseTime !== undefined) {
      this.validateResponseTime(expect.responseTime, `${path}.responseTime`);
    }
  }

  private validateExpectStatus(status: number | number[], path: string): void {
    if (Array.isArray(status)) {
      if (status.length === 0) {
        this.addIssue(path, 'warning', 'Empty status array - no status will match');
      }
      for (const s of status) {
        if (typeof s !== 'number' || s < 100 || s > 599) {
          this.addIssue(path, 'error', 'Status codes must be numbers between 100-599');
          break;
        }
      }
    } else if (typeof status === 'string') {
      // Handle string status
      const parsed = parseInt(status, 10);
      if (!Number.isNaN(parsed) && parsed >= 100 && parsed <= 599) {
        this.addIssue(path, 'warning', 'Status should be a number, not string', {
          description: `Convert to ${parsed}`,
          apply: () => parsed,
        });
      } else {
        this.addIssue(path, 'error', 'Status code must be a number between 100-599');
      }
    } else if (typeof status !== 'number' || status < 100 || status > 599) {
      this.addIssue(path, 'error', 'Status code must be a number between 100-599');
    }
  }

  private validateResponseTime(responseTime: unknown, path: string): void {
    if (typeof responseTime !== 'string') {
      this.addIssue(path, 'error', 'Response time must be a string (e.g., "< 1000")');
      return;
    }

    const pattern = /^[<>]=?\s*(\d+|\$\{[^}]+\})(\s*,\s*[<>]=?\s*(\d+|\$\{[^}]+\}))?$/;
    if (!responseTime.match(pattern)) {
      this.addIssue(path, 'warning', 'Response time should be like "< 1000", "> 500, < 2000"');
    }
  }

  private validateStore(store: Record<string, string>, path: string): void {
    if (typeof store !== 'object' || Array.isArray(store)) {
      this.addIssue(path, 'error', 'Store must be an object');
      return;
    }

    for (const [key, value] of Object.entries(store)) {
      // Validate variable name
      if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(key)) {
        this.addIssue(
          `${path}.${key}`,
          'warning',
          'Store variable names should use alphanumeric characters and underscores',
        );
      }

      // Validate JSON path
      if (typeof value !== 'string') {
        this.addIssue(`${path}.${key}`, 'error', 'Store path must be a string');
      } else {
        // Check for valid response paths
        const validPaths = ['body', 'status', 'headers', 'responseTime'];
        const firstPart = value.split('.')[0];
        if (!validPaths.includes(firstPart)) {
          this.addIssue(
            `${path}.${key}`,
            'warning',
            `Store path should start with: ${validPaths.join(', ')}`,
          );
        }
      }
    }
  }

  private checkDuplicateRequestNames(): void {
    const names = new Map<string, string[]>();

    for (const entry of this.requestNames) {
      const [path, name] = entry.split(':');
      if (!names.has(name)) {
        names.set(name, []);
      }
      names.get(name)!.push(path);
    }

    for (const [name, paths] of names) {
      if (paths.length > 1) {
        this.addIssue(
          paths[1],
          'warning',
          `Duplicate request name "${name}" (also at ${paths[0]})`,
        );
      }
    }
  }

  private checkUnknownKeys(obj: Record<string, unknown>, validKeys: string[], path: string): void {
    for (const key of Object.keys(obj)) {
      if (!validKeys.includes(key)) {
        const suggestion = this.findClosestMatch(key, validKeys);
        this.addIssue(
          `${path}.${key}`,
          'warning',
          `Unknown key "${key}"`,
          suggestion
            ? {
                description: `Did you mean "${suggestion}"?`,
                apply: () => ({ rename: suggestion }),
              }
            : undefined,
        );
      }
    }
  }

  private findClosestMatch(input: string, options: string[]): string | null {
    const lowerInput = input.toLowerCase();
    let bestMatch: string | null = null;
    let bestScore = 3; // Maximum edit distance to consider

    for (const option of options) {
      const lowerOption = option.toLowerCase();

      // Exact match (case-insensitive)
      if (lowerInput === lowerOption) {
        return option;
      }

      // Check if starts with same prefix
      if (lowerOption.startsWith(lowerInput) || lowerInput.startsWith(lowerOption)) {
        return option;
      }

      // Simple edit distance
      const distance = this.editDistance(lowerInput, lowerOption);
      if (distance < bestScore) {
        bestScore = distance;
        bestMatch = option;
      }
    }

    return bestMatch;
  }

  private editDistance(a: string, b: string): number {
    if (a.length === 0) {
      return b.length;
    }
    if (b.length === 0) {
      return a.length;
    }

    const matrix: number[][] = [];

    for (let i = 0; i <= b.length; i++) {
      matrix[i] = [i];
    }
    for (let j = 0; j <= a.length; j++) {
      matrix[0][j] = j;
    }

    for (let i = 1; i <= b.length; i++) {
      for (let j = 1; j <= a.length; j++) {
        if (b.charAt(i - 1) === a.charAt(j - 1)) {
          matrix[i][j] = matrix[i - 1][j - 1];
        } else {
          matrix[i][j] = Math.min(
            matrix[i - 1][j - 1] + 1,
            matrix[i][j - 1] + 1,
            matrix[i - 1][j] + 1,
          );
        }
      }
    }

    return matrix[b.length][a.length];
  }

  private isSensitiveValue(key: string, value: string): boolean {
    // Skip if it's a variable reference
    if (value.includes('${')) {
      return false;
    }

    // Check key name
    for (const pattern of SENSITIVE_PATTERNS) {
      if (pattern.test(key)) {
        return true;
      }
    }

    // Check value patterns
    for (const pattern of SENSITIVE_PATTERNS) {
      if (pattern.test(value)) {
        return true;
      }
    }

    return false;
  }

  private addIssue(
    path: string,
    severity: 'error' | 'warning' | 'info',
    message: string,
    fix?: ValidationIssue['fix'],
  ): void {
    this.issues.push({
      file: this.currentFile,
      path: path || '(root)',
      severity,
      message,
      fix,
    });
  }

  private applyFix(data: Record<string, unknown>, path: string, value: unknown): void {
    if (!path || path === '(root)') {
      return;
    }

    const parts = path.split('.').flatMap((p) => {
      const match = p.match(/^(\w+)\[(\d+)\]$/);
      return match ? [match[1], Number(match[2])] : p;
    });

    let current: unknown = data;
    for (let i = 0; i < parts.length - 1; i++) {
      const key = parts[i];
      if (typeof current === 'object' && current !== null) {
        current = (current as Record<string, unknown>)[key as string];
      }
    }

    if (typeof current === 'object' && current !== null) {
      const lastKey = parts[parts.length - 1];

      // Handle rename operations
      if (typeof value === 'object' && value !== null && 'rename' in value) {
        const renameValue = value as { rename: string; value?: unknown };
        const oldValue =
          renameValue.value ?? (current as Record<string, unknown>)[lastKey as string];
        delete (current as Record<string, unknown>)[lastKey as string];
        (current as Record<string, unknown>)[renameValue.rename] = oldValue;
      } else if (value === undefined) {
        // Delete the key
        delete (current as Record<string, unknown>)[lastKey as string];
      } else {
        (current as Record<string, unknown>)[lastKey as string] = value;
      }
    }
  }

  private toYaml(data: YamlFile): string {
    return this.serializeYaml(data);
  }

  private serializeYaml(obj: unknown, indent = 0): string {
    const spaces = '  '.repeat(indent);

    if (obj === null || obj === undefined) {
      return 'null';
    }

    if (typeof obj === 'string') {
      // Check if string needs quoting
      if (
        obj.includes('\n') ||
        obj.includes(':') ||
        obj.includes('#') ||
        obj.startsWith('$') ||
        obj.match(/^[0-9]/) ||
        obj === '' ||
        obj === 'true' ||
        obj === 'false' ||
        obj === 'null'
      ) {
        return JSON.stringify(obj);
      }
      return obj;
    }

    if (typeof obj === 'number' || typeof obj === 'boolean') {
      return String(obj);
    }

    if (Array.isArray(obj)) {
      if (obj.length === 0) {
        return '[]';
      }
      return obj
        .map((item) => {
          if (typeof item === 'object' && item !== null) {
            const serialized = this.serializeYaml(item, indent + 1);
            const lines = serialized.split('\n');
            return `${spaces}- ${lines[0]}\n${lines
              .slice(1)
              .map((l) => `${spaces}  ${l}`)
              .join('\n')}`.trimEnd();
          }
          return `${spaces}- ${this.serializeYaml(item, indent)}`;
        })
        .join('\n');
    }

    if (typeof obj === 'object') {
      const entries = Object.entries(obj).filter(([, v]) => v !== undefined);
      if (entries.length === 0) {
        return '{}';
      }
      return entries
        .map(([key, value]) => {
          if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
            const serialized = this.serializeYaml(value, indent + 1);
            return `${spaces}${key}:\n${serialized
              .split('\n')
              .map((l) => `  ${l}`)
              .join('\n')}`;
          }
          if (Array.isArray(value)) {
            return `${spaces}${key}:\n${this.serializeYaml(value, indent + 1)}`;
          }
          return `${spaces}${key}: ${this.serializeYaml(value, indent)}`;
        })
        .join('\n');
    }

    return String(obj);
  }
}

export function showValidateHelp(): void {
  console.log(`
${color('curl-runner validate', 'bright')}

Validate YAML files against curl-runner schema and curl options.
Discovers issues and proposes fixes, or auto-fixes with --fix.

${color('USAGE:', 'yellow')}
  curl-runner validate [files...] [options]

${color('OPTIONS:', 'yellow')}
  -f, --fix       Auto-fix issues where possible
  -q, --quiet     Only show files with issues
  -s, --strict    Treat warnings as errors
  -h, --help      Show this help message

${color('ARGUMENTS:', 'yellow')}
  files           File paths or glob patterns (default: *.yaml, *.yml)

${color('EXAMPLES:', 'yellow')}
  curl-runner validate                    # Validate all YAML in current dir
  curl-runner validate api.yaml           # Validate single file
  curl-runner validate "tests/**/*.yaml"  # Validate with glob pattern
  curl-runner validate --fix              # Validate and auto-fix issues
  curl-runner validate -fq                # Fix quietly (only show errors)
  curl-runner validate --strict           # Fail on warnings too

${color('VALIDATION CATEGORIES:', 'yellow')}

  ${color('Structure', 'cyan')}
    • YAML syntax and parsing
    • Required fields (url, auth.type, etc.)
    • Unknown/misspelled keys with suggestions
    • Duplicate request names

  ${color('URLs', 'cyan')}
    • Valid URL syntax
    • Common typos (htpp, localhsot, etc.)
    • Double slashes in paths
    • Missing protocol (http/https)
    • Valid port numbers (1-65535)
    • Spaces in URLs

  ${color('HTTP', 'cyan')}
    • Valid methods (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
    • Method casing (lowercase → UPPERCASE)
    • Headers casing (content-type → Content-Type)
    • Content-Type consistency with body
    • Duplicate headers

  ${color('Auth & Security', 'cyan')}
    • Valid auth types (basic, bearer)
    • Required auth fields (username, password, token)
    • Hardcoded credentials warning
    • Insecure mode warnings
    • HTTP vs HTTPS warnings

  ${color('Configuration', 'cyan')}
    • Boolean string values ("true" → true)
    • Numeric string values
    • Valid execution modes
    • Valid output formats
    • SSL certificate consistency

  ${color('Timeouts & Retries', 'cyan')}
    • Positive timeout values
    • Reasonable timeout ranges
    • Retry count limits
    • Non-idempotent retry warnings

  ${color('Conditions & Expectations', 'cyan')}
    • When condition operators
    • Store path references
    • Status code ranges (100-599)
    • Response time format

${color('AUTO-FIXABLE ISSUES:', 'yellow')}
  • Lowercase HTTP methods → UPPERCASE
  • Missing https:// prefix on URLs
  • URL typos (htpp → http, localhsot → localhost)
  • Double slashes in URL paths
  • Spaces in URLs (→ %20)
  • Header casing (content-type → Content-Type)
  • Boolean strings ("true" → true)
  • Numeric strings (timeout: "5000" → 5000)
  • Status code strings ("200" → 200)
  • Execution mode typos
  • Unknown keys (with suggestions)
`);
}
