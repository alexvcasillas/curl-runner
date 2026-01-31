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
}

interface ValidationIssue {
  file: string;
  path: string;
  severity: 'error' | 'warning';
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
const VALID_AUTH_TYPES = ['basic', 'bearer'] as const;

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

export class ValidateCommand {
  private issues: ValidationIssue[] = [];
  private currentFile = '';

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

      if (!result.valid) {
        invalidFiles++;
        totalIssues += result.issues.length;

        console.log(`${color('✗', 'red')} ${file}`);
        for (const issue of result.issues) {
          const icon = issue.severity === 'error' ? color('●', 'red') : color('●', 'yellow');
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

    let content: YamlFile;

    try {
      content = await YamlParser.parseFile(file);
    } catch (error) {
      const msg = error instanceof Error ? error.message : String(error);
      this.addIssue('', 'error', `Failed to parse YAML: ${msg}`);
      return { file, valid: false, issues: this.issues };
    }

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

    return {
      file,
      valid: this.issues.length === 0,
      issues: this.issues,
      fixedContent,
    };
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

    // Check for unknown top-level keys
    const validKeys = ['version', 'global', 'collection', 'requests', 'request'];
    for (const key of Object.keys(content)) {
      if (!validKeys.includes(key)) {
        this.addIssue(key, 'warning', `Unknown top-level key "${key}"`);
      }
    }
  }

  private validateGlobalConfig(config: GlobalConfig, path: string): void {
    // Execution mode
    if (config.execution && !VALID_EXECUTION_MODES.includes(config.execution)) {
      this.addIssue(
        `${path}.execution`,
        'error',
        `Invalid execution mode "${config.execution}". Must be: ${VALID_EXECUTION_MODES.join(', ')}`,
        {
          description: 'Use "sequential" or "parallel"',
          apply: () => 'sequential',
        },
      );
    }

    // Max concurrency
    if (config.maxConcurrency !== undefined) {
      if (typeof config.maxConcurrency !== 'number' || config.maxConcurrency < 1) {
        this.addIssue(`${path}.maxConcurrency`, 'error', 'Must be a positive number');
      }
    }

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
      if (config.watch.debounce !== undefined && typeof config.watch.debounce !== 'number') {
        this.addIssue(`${path}.watch.debounce`, 'error', 'Must be a number');
      }
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
    if (config.variables && typeof config.variables !== 'object') {
      this.addIssue(`${path}.variables`, 'error', 'Must be an object');
    }

    // Defaults
    if (config.defaults) {
      this.validateRequestConfig(config.defaults, `${path}.defaults`, true);
    }
  }

  private validateOutputConfig(output: NonNullable<GlobalConfig['output']>, path: string): void {
    if (output.format && !VALID_OUTPUT_FORMATS.includes(output.format)) {
      this.addIssue(
        `${path}.format`,
        'error',
        `Invalid format "${output.format}". Must be: ${VALID_OUTPUT_FORMATS.join(', ')}`,
      );
    }

    if (output.prettyLevel && !VALID_PRETTY_LEVELS.includes(output.prettyLevel)) {
      this.addIssue(
        `${path}.prettyLevel`,
        'error',
        `Invalid prettyLevel "${output.prettyLevel}". Must be: ${VALID_PRETTY_LEVELS.join(', ')}`,
      );
    }
  }

  private validateConnectionPool(
    pool: NonNullable<GlobalConfig['connectionPool']>,
    path: string,
  ): void {
    if (pool.maxStreamsPerHost !== undefined) {
      if (typeof pool.maxStreamsPerHost !== 'number' || pool.maxStreamsPerHost < 1) {
        this.addIssue(`${path}.maxStreamsPerHost`, 'error', 'Must be a positive number');
      }
    }
    if (pool.keepaliveTime !== undefined && typeof pool.keepaliveTime !== 'number') {
      this.addIssue(`${path}.keepaliveTime`, 'error', 'Must be a number');
    }
    if (pool.connectTimeout !== undefined && typeof pool.connectTimeout !== 'number') {
      this.addIssue(`${path}.connectTimeout`, 'error', 'Must be a number');
    }
  }

  private validateCIConfig(ci: NonNullable<GlobalConfig['ci']>, path: string): void {
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
  }

  private validateSSLConfig(ssl: SSLConfig, path: string): void {
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
  }

  private validateProfileConfig(profile: NonNullable<GlobalConfig['profile']>, path: string): void {
    if (profile.iterations !== undefined) {
      if (typeof profile.iterations !== 'number' || profile.iterations < 1) {
        this.addIssue(`${path}.iterations`, 'error', 'Must be a positive number');
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
    if (!collection.name) {
      this.addIssue(`${path}.name`, 'warning', 'Collection name is recommended');
    }

    if (!collection.requests || !Array.isArray(collection.requests)) {
      this.addIssue(`${path}.requests`, 'error', 'Collection must have a requests array');
    } else {
      collection.requests.forEach((req, i) => {
        this.validateRequest(req, `${path}.requests[${i}]`);
      });
    }

    if (collection.variables && typeof collection.variables !== 'object') {
      this.addIssue(`${path}.variables`, 'error', 'Must be an object');
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
    // URL is required for non-defaults
    if (!isDefaults && !request.url) {
      this.addIssue(`${path}.url`, 'error', 'URL is required');
    }

    // URL format check
    if (request.url) {
      if (typeof request.url !== 'string') {
        this.addIssue(`${path}.url`, 'error', 'URL must be a string');
      } else if (!request.url.startsWith('http') && !request.url.includes('${')) {
        this.addIssue(`${path}.url`, 'warning', 'URL should start with http:// or https://', {
          description: 'Prepend https://',
          apply: () => `https://${request.url}`,
        });
      }
    }

    // Method validation
    if (request.method) {
      const upperMethod = request.method.toUpperCase();
      if (!VALID_METHODS.includes(upperMethod as (typeof VALID_METHODS)[number])) {
        this.addIssue(
          `${path}.method`,
          'error',
          `Invalid method "${request.method}". Must be: ${VALID_METHODS.join(', ')}`,
        );
      } else if (request.method !== upperMethod) {
        this.addIssue(`${path}.method`, 'warning', `Method should be uppercase: "${upperMethod}"`, {
          description: `Change to "${upperMethod}"`,
          apply: () => upperMethod,
        });
      }
    }

    // Headers validation
    if (request.headers) {
      if (typeof request.headers !== 'object' || Array.isArray(request.headers)) {
        this.addIssue(`${path}.headers`, 'error', 'Headers must be an object');
      }
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

    // FormData validation
    if (request.formData) {
      this.validateFormData(request.formData, `${path}.formData`);
    }

    // Timeout validation
    if (request.timeout !== undefined) {
      if (typeof request.timeout !== 'number' || request.timeout <= 0) {
        this.addIssue(`${path}.timeout`, 'error', 'Timeout must be a positive number');
      }
    }

    // Auth validation
    if (request.auth) {
      this.validateAuth(request.auth, `${path}.auth`);
    }

    // SSL validation
    if (request.ssl) {
      this.validateSSLConfig(request.ssl, `${path}.ssl`);
    }

    // Retry validation
    if (request.retry) {
      this.validateRetry(request.retry, `${path}.retry`);
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
      if (typeof request.store !== 'object' || Array.isArray(request.store)) {
        this.addIssue(`${path}.store`, 'error', 'Store must be an object');
      }
    }

    // Max redirects validation
    if (request.maxRedirects !== undefined) {
      if (typeof request.maxRedirects !== 'number' || request.maxRedirects < 0) {
        this.addIssue(`${path}.maxRedirects`, 'error', 'Must be a non-negative number');
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
        }
      }
    }
  }

  private validateAuth(auth: NonNullable<RequestConfig['auth']>, path: string): void {
    if (!auth.type) {
      this.addIssue(`${path}.type`, 'error', 'Auth type is required');
      return;
    }

    if (!VALID_AUTH_TYPES.includes(auth.type)) {
      this.addIssue(
        `${path}.type`,
        'error',
        `Invalid auth type "${auth.type}". Must be: ${VALID_AUTH_TYPES.join(', ')}`,
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
    }

    if (auth.type === 'bearer') {
      if (!auth.token) {
        this.addIssue(`${path}.token`, 'error', 'Token required for bearer auth');
      }
    }
  }

  private validateRetry(retry: NonNullable<RequestConfig['retry']>, path: string): void {
    if (retry.count === undefined) {
      this.addIssue(`${path}.count`, 'error', 'Retry count is required');
    } else if (typeof retry.count !== 'number' || retry.count < 0) {
      this.addIssue(`${path}.count`, 'error', 'Retry count must be a non-negative number');
    }

    if (retry.delay !== undefined && (typeof retry.delay !== 'number' || retry.delay < 0)) {
      this.addIssue(`${path}.delay`, 'error', 'Retry delay must be a non-negative number');
    }

    if (retry.backoff !== undefined && (typeof retry.backoff !== 'number' || retry.backoff < 1)) {
      this.addIssue(`${path}.backoff`, 'error', 'Retry backoff must be >= 1');
    }
  }

  private validateWhenCondition(when: WhenCondition | string, path: string): void {
    if (typeof when === 'string') {
      // String shorthand - basic validation
      if (!when.includes('store.')) {
        this.addIssue(
          path,
          'warning',
          'When condition should reference store values (e.g., "store.status == 200")',
        );
      }
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

    if (hasAll) {
      if (!Array.isArray(when.all)) {
        this.addIssue(`${path}.all`, 'error', 'Must be an array');
      } else {
        when.all.forEach((cond, i) => {
          this.validateConditionExpression(cond, `${path}.all[${i}]`);
        });
      }
    }

    if (hasAny) {
      if (!Array.isArray(when.any)) {
        this.addIssue(`${path}.any`, 'error', 'Must be an array');
      } else {
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

  private validateConditionExpression(
    cond: { left?: string; operator?: string; right?: unknown },
    path: string,
  ): void {
    if (!cond.left) {
      this.addIssue(`${path}.left`, 'error', 'Left operand is required');
    }

    if (!cond.operator) {
      this.addIssue(`${path}.operator`, 'error', 'Operator is required');
    } else if (!VALID_OPERATORS.includes(cond.operator as (typeof VALID_OPERATORS)[number])) {
      this.addIssue(
        `${path}.operator`,
        'error',
        `Invalid operator "${cond.operator}". Must be: ${VALID_OPERATORS.join(', ')}`,
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
    // Status validation
    if (expect.status !== undefined) {
      if (Array.isArray(expect.status)) {
        for (const s of expect.status) {
          if (typeof s !== 'number' || s < 100 || s > 599) {
            this.addIssue(
              `${path}.status`,
              'error',
              'Status codes must be numbers between 100-599',
            );
            break;
          }
        }
      } else if (typeof expect.status !== 'number' || expect.status < 100 || expect.status > 599) {
        this.addIssue(`${path}.status`, 'error', 'Status code must be a number between 100-599');
      }
    }

    // Headers validation
    if (expect.headers && (typeof expect.headers !== 'object' || Array.isArray(expect.headers))) {
      this.addIssue(`${path}.headers`, 'error', 'Expected headers must be an object');
    }

    // Response time validation
    if (expect.responseTime !== undefined) {
      if (typeof expect.responseTime !== 'string') {
        this.addIssue(
          `${path}.responseTime`,
          'error',
          'Response time must be a string (e.g., "< 1000")',
        );
      } else if (
        !expect.responseTime.match(
          /^[<>]=?\s*(\d+|\$\{[^}]+\})(\s*,\s*[<>]=?\s*(\d+|\$\{[^}]+\}))?$/,
        )
      ) {
        this.addIssue(
          `${path}.responseTime`,
          'warning',
          'Response time should be like "< 1000", "> 500, < 2000"',
        );
      }
    }
  }

  private addIssue(
    path: string,
    severity: 'error' | 'warning',
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
      (current as Record<string, unknown>)[lastKey as string] = value;
    }
  }

  private toYaml(data: YamlFile): string {
    // Use Bun's yaml module if available, otherwise simple serialization
    const yaml = this.serializeYaml(data);
    return yaml;
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
        obj === ''
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
  -h, --help      Show this help message

${color('ARGUMENTS:', 'yellow')}
  files           File paths or glob patterns (default: *.yaml, *.yml)

${color('EXAMPLES:', 'yellow')}
  curl-runner validate                    # Validate all YAML in current dir
  curl-runner validate api.yaml           # Validate single file
  curl-runner validate "tests/**/*.yaml"  # Validate with glob pattern
  curl-runner validate --fix              # Validate and auto-fix issues
  curl-runner validate -fq                # Fix quietly (only show errors)

${color('WHAT IT VALIDATES:', 'yellow')}
  • YAML syntax and structure
  • Required fields (url, auth.type, etc.)
  • Valid HTTP methods (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
  • Valid auth types (basic, bearer)
  • SSL/TLS config consistency
  • Retry/timeout numeric values
  • When conditions and operators
  • Expect assertions format
  • Output format and pretty level values

${color('AUTO-FIXABLE ISSUES:', 'yellow')}
  • Lowercase HTTP methods → UPPERCASE
  • Missing https:// prefix on URLs
`);
}
