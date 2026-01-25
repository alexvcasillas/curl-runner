import { SnapshotFormatter } from '../snapshot/snapshot-formatter';
import type {
  ExecutionResult,
  ExecutionSummary,
  GlobalConfig,
  ProfileResult,
  RequestConfig,
} from '../types/config';
import { generateHistogram } from './stats';

interface TreeNode {
  label: string;
  value?: string;
  children?: TreeNode[];
  color?: string;
}

class TreeRenderer {
  private colors: Record<string, string>;

  constructor(colors: Record<string, string>) {
    this.colors = colors;
  }

  private color(text: string, colorName: string): string {
    if (!colorName || !this.colors[colorName]) {
      return text;
    }
    return `${this.colors[colorName]}${text}${this.colors.reset}`;
  }

  render(nodes: TreeNode[], basePrefix: string = '   '): void {
    nodes.forEach((node, index) => {
      const isLast = index === nodes.length - 1;
      const prefix = isLast ? `${basePrefix}└─` : `${basePrefix}├─`;

      if (node.label && node.value) {
        // Regular labeled node with value
        const displayValue = node.color ? this.color(node.value, node.color) : node.value;

        // Handle multiline values (like Response Body)
        const lines = displayValue.split('\n');
        if (lines.length === 1) {
          console.log(`${prefix} ${node.label}: ${displayValue}`);
        } else {
          console.log(`${prefix} ${node.label}:`);
          const contentPrefix = isLast ? `${basePrefix}   ` : `${basePrefix}│  `;
          lines.forEach((line) => {
            console.log(`${contentPrefix}${line}`);
          });
        }
      } else if (node.label && !node.value) {
        // Section header (like "Headers:" or "Metrics:")
        console.log(`${prefix} ${node.label}:`);
      } else if (!node.label && node.value) {
        // Content line without label (like response body lines)
        const continuationPrefix = isLast ? `${basePrefix}   ` : `${basePrefix}│  `;
        console.log(`${continuationPrefix}${node.value}`);
      }

      if (node.children && node.children.length > 0) {
        const childPrefix = isLast ? `${basePrefix}   ` : `${basePrefix}│  `;
        this.render(node.children, childPrefix);
      }
    });
  }
}

export class Logger {
  private config: GlobalConfig['output'];

  private readonly colors = {
    reset: '\x1b[0m',
    bright: '\x1b[1m',
    dim: '\x1b[2m',
    underscore: '\x1b[4m',

    black: '\x1b[30m',
    red: '\x1b[31m',
    green: '\x1b[32m',
    yellow: '\x1b[33m',
    blue: '\x1b[34m',
    magenta: '\x1b[35m',
    cyan: '\x1b[36m',
    white: '\x1b[37m',

    bgBlack: '\x1b[40m',
    bgRed: '\x1b[41m',
    bgGreen: '\x1b[42m',
    bgYellow: '\x1b[43m',
    bgBlue: '\x1b[44m',
    bgMagenta: '\x1b[45m',
    bgCyan: '\x1b[46m',
    bgWhite: '\x1b[47m',
  };

  constructor(config: GlobalConfig['output'] = {}) {
    this.config = {
      verbose: false,
      showHeaders: false,
      showBody: true,
      showMetrics: false,
      format: 'pretty',
      prettyLevel: 'minimal',
      ...config,
    };
  }

  private color(text: string, color: keyof typeof this.colors): string {
    return `${this.colors[color]}${text}${this.colors.reset}`;
  }

  private getShortFilename(filePath: string): string {
    return filePath.replace(/.*\//, '').replace('.yaml', '');
  }

  private shouldShowOutput(): boolean {
    if (this.config.format === 'raw') {
      return false;
    }
    if (this.config.format === 'pretty') {
      return true; // Pretty format should always show output
    }
    return this.config.verbose !== false; // For other formats, respect verbose flag
  }

  private shouldShowHeaders(): boolean {
    if (this.config.format !== 'pretty') {
      return this.config.showHeaders || false;
    }

    const level = this.config.prettyLevel || 'standard';
    switch (level) {
      case 'minimal':
        return false;
      case 'standard':
        return this.config.showHeaders || false;
      case 'detailed':
        return true;
      default:
        return this.config.showHeaders || false;
    }
  }

  private shouldShowBody(): boolean {
    if (this.config.format !== 'pretty') {
      return this.config.showBody !== false;
    }

    const level = this.config.prettyLevel || 'standard';
    switch (level) {
      case 'minimal':
        return false; // Minimal never shows body
      case 'standard':
        return this.config.showBody !== false;
      case 'detailed':
        return true; // Detailed always shows body
      default:
        return this.config.showBody !== false;
    }
  }

  private shouldShowMetrics(): boolean {
    if (this.config.format !== 'pretty') {
      return this.config.showMetrics || false;
    }

    const level = this.config.prettyLevel || 'standard';
    switch (level) {
      case 'minimal':
        return false; // Minimal never shows metrics
      case 'standard':
        return this.config.showMetrics || false;
      case 'detailed':
        return true; // Detailed always shows metrics
      default:
        return this.config.showMetrics || false;
    }
  }

  private shouldShowRequestDetails(): boolean {
    if (this.config.format !== 'pretty') {
      return this.config.verbose || false;
    }

    const level = this.config.prettyLevel || 'standard';
    switch (level) {
      case 'minimal':
        return false;
      case 'standard':
        return this.config.verbose || false;
      case 'detailed':
        return true;
      default:
        return this.config.verbose || false;
    }
  }

  private shouldShowSeparators(): boolean {
    if (this.config.format !== 'pretty') {
      return true;
    }

    const level = this.config.prettyLevel || 'standard';
    switch (level) {
      case 'minimal':
        return false;
      case 'standard':
        return true;
      case 'detailed':
        return true;
      default:
        return true;
    }
  }

  private colorStatusCode(statusStr: string): string {
    // For expected status codes in validation errors, use yellow to distinguish from red actual values
    return this.color(statusStr, 'yellow');
  }

  private logValidationErrors(errorString: string): void {
    // Check if this is a validation error with multiple parts (separated by ';')
    const errors = errorString.split('; ');

    if (errors.length === 1) {
      // Single error - check if it's a status error for special formatting
      const trimmedError = errors[0].trim();
      const statusMatch = trimmedError.match(/^Expected status (.+?), got (.+)$/);
      if (statusMatch) {
        const [, expected, actual] = statusMatch;
        const expectedStatus = this.colorStatusCode(expected.replace(' or ', '|'));
        const actualStatus = this.color(actual, 'red'); // Always red for incorrect actual values
        console.log(
          `  ${this.color('✗', 'red')} ${this.color('Error:', 'red')} Expected ${this.color('status', 'yellow')} ${expectedStatus}, got ${actualStatus}`,
        );
      } else {
        console.log(`  ${this.color('✗', 'red')} ${this.color('Error:', 'red')} ${trimmedError}`);
      }
    } else {
      // Multiple validation errors - show them nicely formatted
      console.log(`  ${this.color('✗', 'red')} ${this.color('Validation Errors:', 'red')}`);
      for (const error of errors) {
        const trimmedError = error.trim();
        if (trimmedError) {
          // Parse different error formats for better formatting
          if (trimmedError.startsWith('Expected ')) {
            // Format 1: "Expected status 201, got 200"
            const statusMatch = trimmedError.match(/^Expected status (.+?), got (.+)$/);
            if (statusMatch) {
              const [, expected, actual] = statusMatch;
              const expectedStatus = this.colorStatusCode(expected.replace(' or ', '|'));
              const actualStatus = this.color(actual, 'red'); // Always red for incorrect actual values
              console.log(
                `    ${this.color('•', 'red')} ${this.color('status', 'yellow')}: expected ${expectedStatus}, got ${actualStatus}`,
              );
            } else {
              // Format 2: "Expected field to be value, got value"
              const fieldMatch = trimmedError.match(/^Expected (.+?) to be (.+?), got (.+)$/);
              if (fieldMatch) {
                const [, field, expected, actual] = fieldMatch;
                console.log(
                  `    ${this.color('•', 'red')} ${this.color(field, 'yellow')}: expected ${this.color(expected, 'green')}, got ${this.color(actual, 'red')}`,
                );
              } else {
                console.log(`    ${this.color('•', 'red')} ${trimmedError}`);
              }
            }
          } else {
            console.log(`    ${this.color('•', 'red')} ${trimmedError}`);
          }
        }
      }
    }
  }

  private formatJson(data: unknown): string {
    if (this.config.format === 'raw') {
      return typeof data === 'string' ? data : JSON.stringify(data);
    }
    if (this.config.format === 'json') {
      return JSON.stringify(data);
    }
    return JSON.stringify(data, null, 2);
  }

  private formatDuration(ms: number): string {
    if (ms < 1000) {
      return `${ms.toFixed(0)}ms`;
    }
    return `${(ms / 1000).toFixed(2)}s`;
  }

  private formatSize(bytes: number | undefined): string {
    if (!bytes) {
      return '0 B';
    }
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / 1024 ** i).toFixed(2)} ${sizes[i]}`;
  }

  logExecutionStart(count: number, mode: string): void {
    if (!this.shouldShowOutput()) {
      return;
    }

    if (this.shouldShowSeparators()) {
      console.log(); // Add spacing before the execution header
      console.log(this.color(`Executing ${count} request(s) in ${mode} mode`, 'dim'));
      console.log();
    } else {
      // For minimal format, still add spacing after processing info
      console.log();
    }
  }

  logRequestStart(_config: RequestConfig, _index: number): void {
    // In the new format, we show everything in logRequestComplete
    // This method is kept for compatibility but simplified
    return;
  }

  logCommand(command: string): void {
    if (this.shouldShowRequestDetails()) {
      console.log(this.color('  Command:', 'dim'));
      console.log(this.color(`    ${command}`, 'dim'));
    }
  }

  logRetry(attempt: number, maxRetries: number): void {
    console.log(this.color(`  ↻ Retry ${attempt}/${maxRetries}...`, 'yellow'));
  }

  logRequestComplete(result: ExecutionResult): void {
    // Handle raw format output - only show response body
    if (this.config.format === 'raw') {
      if (result.success && this.config.showBody && result.body) {
        const bodyStr = this.formatJson(result.body);
        console.log(bodyStr);
      }
      return;
    }

    // Handle JSON format output - structured JSON only
    if (this.config.format === 'json') {
      const jsonResult = {
        request: {
          name: result.request.name,
          url: result.request.url,
          method: result.request.method || 'GET',
        },
        success: result.success,
        status: result.status,
        ...(this.shouldShowHeaders() && result.headers ? { headers: result.headers } : {}),
        ...(this.shouldShowBody() && result.body ? { body: result.body } : {}),
        ...(result.error ? { error: result.error } : {}),
        ...(this.shouldShowMetrics() && result.metrics ? { metrics: result.metrics } : {}),
      };
      console.log(JSON.stringify(jsonResult, null, 2));
      return;
    }

    // Pretty format output (default behavior)
    if (!this.shouldShowOutput()) {
      return;
    }

    const level = this.config.prettyLevel || 'minimal';
    const statusColor = result.success ? 'green' : 'red';
    const statusIcon = result.success ? '✓' : 'x';
    const name = result.request.name || 'Request';

    if (level === 'minimal') {
      // Minimal format: clean tree structure but compact
      const fileTag = result.request.sourceFile
        ? this.getShortFilename(result.request.sourceFile)
        : 'inline';
      console.log(
        `${this.color(statusIcon, statusColor)} ${this.color(name, 'bright')} [${fileTag}]`,
      );

      const treeNodes: TreeNode[] = [];
      const renderer = new TreeRenderer(this.colors);

      treeNodes.push({
        label: result.request.method || 'GET',
        value: result.request.url,
        color: 'blue',
      });

      const statusText = result.status ? `${result.status}` : 'ERROR';
      treeNodes.push({
        label: `${statusIcon} Status`,
        value: statusText,
        color: statusColor,
      });

      if (result.metrics) {
        const durationSize = `${this.formatDuration(result.metrics.duration)} | ${this.formatSize(result.metrics.size)}`;
        treeNodes.push({
          label: 'Duration',
          value: durationSize,
          color: 'cyan',
        });
      }

      renderer.render(treeNodes);

      if (result.error) {
        console.log();
        this.logValidationErrors(result.error);
      }

      // Show snapshot result
      if (result.snapshotResult) {
        console.log();
        this.logSnapshotResult(result.request.name || 'Request', result.snapshotResult);
      }

      console.log();
      return;
    }

    // Standard and detailed formats: use clean tree structure
    console.log(`${this.color(statusIcon, statusColor)} ${this.color(name, 'bright')}`);

    // Build tree structure
    const treeNodes: TreeNode[] = [];
    const renderer = new TreeRenderer(this.colors);

    // Main info nodes
    treeNodes.push({ label: 'URL', value: result.request.url, color: 'blue' });
    treeNodes.push({ label: 'Method', value: result.request.method || 'GET', color: 'yellow' });
    treeNodes.push({
      label: 'Status',
      value: String(result.status || 'ERROR'),
      color: statusColor,
    });

    if (result.metrics) {
      treeNodes.push({
        label: 'Duration',
        value: this.formatDuration(result.metrics.duration),
        color: 'cyan',
      });
    }

    // Add headers section if needed
    if (this.shouldShowHeaders() && result.headers && Object.keys(result.headers).length > 0) {
      const headerChildren: TreeNode[] = Object.entries(result.headers).map(([key, value]) => ({
        label: this.color(key, 'dim'),
        value: String(value),
      }));

      treeNodes.push({
        label: 'Headers',
        children: headerChildren,
      });
    }

    // Add body section if needed
    if (this.shouldShowBody() && result.body) {
      const bodyStr = this.formatJson(result.body);
      const lines = bodyStr.split('\n');
      const maxLines = this.shouldShowRequestDetails() ? Infinity : 10;
      const bodyLines = lines.slice(0, maxLines);

      if (lines.length > maxLines) {
        bodyLines.push(this.color(`... (${lines.length - maxLines} more lines)`, 'dim'));
      }

      treeNodes.push({
        label: 'Response Body',
        value: bodyLines.join('\n'),
      });
    }

    // Add detailed metrics section if needed
    if (this.shouldShowMetrics() && result.metrics && level === 'detailed') {
      const metrics = result.metrics;
      const metricChildren: TreeNode[] = [];

      metricChildren.push({
        label: 'Request Duration',
        value: this.formatDuration(metrics.duration),
        color: 'cyan',
      });

      if (metrics.size !== undefined) {
        metricChildren.push({
          label: 'Response Size',
          value: this.formatSize(metrics.size),
          color: 'cyan',
        });
      }

      if (metrics.dnsLookup) {
        metricChildren.push({
          label: 'DNS Lookup',
          value: this.formatDuration(metrics.dnsLookup),
          color: 'cyan',
        });
      }

      if (metrics.tcpConnection) {
        metricChildren.push({
          label: 'TCP Connection',
          value: this.formatDuration(metrics.tcpConnection),
          color: 'cyan',
        });
      }

      if (metrics.tlsHandshake) {
        metricChildren.push({
          label: 'TLS Handshake',
          value: this.formatDuration(metrics.tlsHandshake),
          color: 'cyan',
        });
      }

      if (metrics.firstByte) {
        metricChildren.push({
          label: 'Time to First Byte',
          value: this.formatDuration(metrics.firstByte),
          color: 'cyan',
        });
      }

      treeNodes.push({
        label: 'Metrics',
        children: metricChildren,
      });
    }

    // Render the tree
    renderer.render(treeNodes);

    if (result.error) {
      console.log();
      this.logValidationErrors(result.error);
    }

    // Show snapshot result
    if (result.snapshotResult) {
      console.log();
      this.logSnapshotResult(result.request.name || 'Request', result.snapshotResult);
    }

    console.log();
  }

  /**
   * Logs snapshot comparison result.
   */
  private logSnapshotResult(requestName: string, result: ExecutionResult['snapshotResult']): void {
    if (!result) {
      return;
    }

    const formatter = new SnapshotFormatter();
    console.log(formatter.formatResult(requestName, result));
  }

  logSummary(summary: ExecutionSummary, isGlobal: boolean = false): void {
    // For raw format, don't show summary
    if (this.config.format === 'raw') {
      return;
    }

    // For JSON format, output structured summary
    if (this.config.format === 'json') {
      const jsonSummary = {
        summary: {
          total: summary.total,
          successful: summary.successful,
          failed: summary.failed,
          duration: summary.duration,
        },
        results: summary.results.map((result) => ({
          request: {
            name: result.request.name,
            url: result.request.url,
            method: result.request.method || 'GET',
          },
          success: result.success,
          status: result.status,
          ...(this.shouldShowHeaders() && result.headers ? { headers: result.headers } : {}),
          ...(this.shouldShowBody() && result.body ? { body: result.body } : {}),
          ...(result.error ? { error: result.error } : {}),
          ...(this.shouldShowMetrics() && result.metrics ? { metrics: result.metrics } : {}),
        })),
      };
      console.log(JSON.stringify(jsonSummary, null, 2));
      return;
    }

    // Pretty format summary (default behavior)
    if (!this.shouldShowOutput()) {
      return;
    }

    const level = this.config.prettyLevel || 'minimal';

    // Add spacing for global summary
    if (isGlobal) {
      console.log(); // Extra spacing before global summary
    }

    if (level === 'minimal') {
      // Simple one-line summary for minimal, similar to docs example
      const statusColor = summary.failed === 0 ? 'green' : 'red';
      const successText =
        summary.failed === 0
          ? `${summary.total} request${summary.total === 1 ? '' : 's'} completed successfully`
          : `${summary.successful}/${summary.total} request${summary.total === 1 ? '' : 's'} completed, ${summary.failed} failed`;

      const summaryPrefix = isGlobal ? '◆ Global Summary' : 'Summary';
      console.log(`${summaryPrefix}: ${this.color(successText, statusColor)}`);
      return;
    }

    // Compact summary for standard/detailed - much simpler
    const _successRate = ((summary.successful / summary.total) * 100).toFixed(1);
    const statusColor = summary.failed === 0 ? 'green' : 'red';
    const successText =
      summary.failed === 0
        ? `${summary.total} request${summary.total === 1 ? '' : 's'} completed successfully`
        : `${summary.successful}/${summary.total} request${summary.total === 1 ? '' : 's'} completed, ${summary.failed} failed`;

    const summaryPrefix = isGlobal ? '◆ Global Summary' : 'Summary';
    console.log();
    console.log(
      `${summaryPrefix}: ${this.color(successText, statusColor)} (${this.color(this.formatDuration(summary.duration), 'cyan')})`,
    );

    if (summary.failed > 0 && this.shouldShowRequestDetails()) {
      summary.results
        .filter((r) => !r.success)
        .forEach((r) => {
          const name = r.request.name || r.request.url;
          console.log(`  ${this.color('•', 'red')} ${name}: ${r.error}`);
        });
    }
  }

  logError(message: string): void {
    console.error(this.color(`✗ ${message}`, 'red'));
  }

  logWarning(message: string): void {
    console.warn(this.color(`⚠ ${message}`, 'yellow'));
  }

  logInfo(message: string): void {
    console.log(this.color(`ℹ ${message}`, 'blue'));
  }

  logSuccess(message: string): void {
    console.log(this.color(`✓ ${message}`, 'green'));
  }

  logFileHeader(fileName: string, requestCount: number): void {
    if (!this.shouldShowOutput() || this.config.format !== 'pretty') {
      return;
    }

    const shortName = fileName.replace(/.*\//, '').replace('.yaml', '');
    console.log();
    console.log(
      this.color(`▶ ${shortName}.yaml`, 'bright') +
        this.color(` (${requestCount} request${requestCount === 1 ? '' : 's'})`, 'dim'),
    );
  }

  logWatch(files: string[]): void {
    console.log();
    console.log(
      `${this.color('Watching for changes...', 'cyan')} ${this.color('(press Ctrl+C to stop)', 'dim')}`,
    );
    const fileList = files.length <= 3 ? files.join(', ') : `${files.length} files`;
    console.log(this.color(`   Files: ${fileList}`, 'dim'));
    console.log();
  }

  logWatchReady(): void {
    console.log();
    console.log(this.color('Watching for changes...', 'cyan'));
  }

  logFileChanged(filename: string): void {
    const timestamp = new Date().toLocaleTimeString('en-US', { hour12: false });
    console.log(this.color('-'.repeat(50), 'dim'));
    console.log(
      `${this.color(`[${timestamp}]`, 'dim')} File changed: ${this.color(filename, 'yellow')}`,
    );
    console.log();
  }

  logProfileStart(
    requestName: string,
    iterations: number,
    warmup: number,
    concurrency: number,
  ): void {
    if (!this.shouldShowOutput()) {
      return;
    }

    console.log();
    console.log(`${this.color('⚡ PROFILING', 'magenta')} ${this.color(requestName, 'bright')}`);
    console.log(
      this.color(
        `   ${iterations} iterations, ${warmup} warmup, concurrency: ${concurrency}`,
        'dim',
      ),
    );
  }

  logProfileResult(result: ProfileResult, showHistogram: boolean): void {
    const { stats, request } = result;
    const name = request.name || request.url;

    if (this.config.format === 'json') {
      console.log(
        JSON.stringify({
          request: { name, url: request.url, method: request.method || 'GET' },
          stats: {
            iterations: stats.iterations,
            warmup: stats.warmup,
            failures: stats.failures,
            failureRate: stats.failureRate,
            min: stats.min,
            max: stats.max,
            mean: stats.mean,
            median: stats.median,
            p50: stats.p50,
            p95: stats.p95,
            p99: stats.p99,
            stdDev: stats.stdDev,
          },
        }),
      );
      return;
    }

    if (this.config.format === 'raw') {
      // Raw format: just print the key stats
      console.log(`${stats.p50}\t${stats.p95}\t${stats.p99}\t${stats.mean}`);
      return;
    }

    // Pretty format
    console.log();
    const statusIcon = stats.failures === 0 ? this.color('✓', 'green') : this.color('⚠', 'yellow');
    console.log(`${statusIcon} ${this.color(name, 'bright')}`);

    // Latency stats table
    console.log(this.color('   ┌─────────────────────────────────────┐', 'dim'));
    console.log(
      `   │ ${this.color('p50', 'cyan')}    ${this.formatLatency(stats.p50).padStart(10)} │ ${this.color('min', 'dim')}   ${this.formatLatency(stats.min).padStart(10)} │`,
    );
    console.log(
      `   │ ${this.color('p95', 'yellow')}    ${this.formatLatency(stats.p95).padStart(10)} │ ${this.color('max', 'dim')}   ${this.formatLatency(stats.max).padStart(10)} │`,
    );
    console.log(
      `   │ ${this.color('p99', 'red')}    ${this.formatLatency(stats.p99).padStart(10)} │ ${this.color('mean', 'dim')}  ${this.formatLatency(stats.mean).padStart(10)} │`,
    );
    console.log(this.color('   └─────────────────────────────────────┘', 'dim'));

    // Additional stats
    console.log(
      this.color(
        `   σ ${stats.stdDev.toFixed(2)}ms | ${stats.iterations} samples | ${stats.failures} failures (${stats.failureRate}%)`,
        'dim',
      ),
    );

    // Optional histogram
    if (showHistogram && stats.timings.length > 0) {
      console.log();
      console.log(this.color('   Distribution:', 'dim'));
      const histogramLines = generateHistogram(stats.timings, 8, 30);
      for (const line of histogramLines) {
        console.log(`   ${this.color(line, 'dim')}`);
      }
    }
  }

  private formatLatency(ms: number): string {
    if (ms < 1) {
      return `${(ms * 1000).toFixed(0)}µs`;
    }
    if (ms < 1000) {
      return `${ms.toFixed(1)}ms`;
    }
    return `${(ms / 1000).toFixed(2)}s`;
  }

  logProfileSummary(results: ProfileResult[]): void {
    if (!this.shouldShowOutput()) {
      return;
    }

    const totalIterations = results.reduce((sum, r) => sum + r.stats.iterations, 0);
    const totalFailures = results.reduce((sum, r) => sum + r.stats.failures, 0);

    console.log();
    console.log(this.color('─'.repeat(50), 'dim'));
    console.log(
      `${this.color('⚡ Profile Summary:', 'magenta')} ${results.length} request${results.length === 1 ? '' : 's'}, ${totalIterations} total iterations`,
    );

    if (totalFailures > 0) {
      console.log(this.color(`   ${totalFailures} total failures`, 'yellow'));
    }
  }
}
