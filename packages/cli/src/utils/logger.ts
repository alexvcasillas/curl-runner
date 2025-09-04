import type {
  ExecutionResult,
  ExecutionSummary,
  GlobalConfig,
  RequestConfig,
} from '../types/config';

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
      prettyLevel: 'standard',
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
    if (this.config.format === 'raw') return false;
    if (this.config.format === 'pretty') return true; // Pretty format should always show output
    return this.config.verbose !== false; // For other formats, respect verbose flag
  }

  private shouldShowHeaders(): boolean {
    if (this.config.format !== 'pretty') return this.config.showHeaders || false;
    
    const level = this.config.prettyLevel || 'standard';
    switch (level) {
      case 'minimal': return false;
      case 'standard': return this.config.showHeaders || false;
      case 'detailed': return true;
      default: return this.config.showHeaders || false;
    }
  }

  private shouldShowBody(): boolean {
    if (this.config.format !== 'pretty') return this.config.showBody !== false;
    
    const level = this.config.prettyLevel || 'standard';
    switch (level) {
      case 'minimal': return false;  // Minimal never shows body
      case 'standard': return this.config.showBody !== false;
      case 'detailed': return true;  // Detailed always shows body
      default: return this.config.showBody !== false;
    }
  }

  private shouldShowMetrics(): boolean {
    if (this.config.format !== 'pretty') return this.config.showMetrics || false;
    
    const level = this.config.prettyLevel || 'standard';
    switch (level) {
      case 'minimal': return false;  // Minimal never shows metrics
      case 'standard': return this.config.showMetrics || false;
      case 'detailed': return true;  // Detailed always shows metrics
      default: return this.config.showMetrics || false;
    }
  }

  private shouldShowRequestDetails(): boolean {
    if (this.config.format !== 'pretty') return this.config.verbose || false;
    
    const level = this.config.prettyLevel || 'standard';
    switch (level) {
      case 'minimal': return false;
      case 'standard': return this.config.verbose || false;
      case 'detailed': return true;
      default: return this.config.verbose || false;
    }
  }

  private shouldShowSeparators(): boolean {
    if (this.config.format !== 'pretty') return true;
    
    const level = this.config.prettyLevel || 'standard';
    switch (level) {
      case 'minimal': return false;
      case 'standard': return true;
      case 'detailed': return true;
      default: return true;
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
        console.log(`  ${this.color('✗', 'red')} ${this.color('Error:', 'red')} Expected ${this.color('status', 'yellow')} ${expectedStatus}, got ${actualStatus}`);
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
              console.log(`    ${this.color('•', 'red')} ${this.color('status', 'yellow')}: expected ${expectedStatus}, got ${actualStatus}`);
            } else {
              // Format 2: "Expected field to be value, got value" 
              const fieldMatch = trimmedError.match(/^Expected (.+?) to be (.+?), got (.+)$/);
              if (fieldMatch) {
                const [, field, expected, actual] = fieldMatch;
                console.log(`    ${this.color('•', 'red')} ${this.color(field, 'yellow')}: expected ${this.color(expected, 'green')}, got ${this.color(actual, 'red')}`);
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

  private printSeparator(char: string = '─', length: number = 60): void {
    console.log(this.color(char.repeat(length), 'dim'));
  }

  logExecutionStart(count: number, mode: string): void {
    if (!this.shouldShowOutput()) return;
    
    if (this.shouldShowSeparators()) {
      this.printSeparator('═');
      console.log(this.color('🚀 CURL RUNNER', 'bright'));
      console.log(this.color(`Executing ${count} request(s) in ${mode} mode`, 'cyan'));
      this.printSeparator('═');
      console.log();
    }
  }

  logRequestStart(config: RequestConfig, index: number): void {
    if (!this.shouldShowOutput()) return;

    const name = config.name || `Request #${index}`;
    const sourceFile = config.sourceFile ? ` ${this.color(`[${this.getShortFilename(config.sourceFile)}]`, 'cyan')}` : '';
    console.log(this.color(`▶ ${name}`, 'bright') + sourceFile);
    console.log(
      `  ${this.color(config.method || 'GET', 'yellow')} ${this.color(config.url, 'blue')}`,
    );

    if (this.shouldShowRequestDetails() && config.headers && Object.keys(config.headers).length > 0) {
      console.log(this.color('  Headers:', 'dim'));
      for (const [key, value] of Object.entries(config.headers)) {
        console.log(`    ${key}: ${value}`);
      }
    }

    if (this.shouldShowRequestDetails() && config.body) {
      console.log(this.color('  Body:', 'dim'));
      const bodyStr = this.formatJson(config.body);
      for (const line of bodyStr.split('\n')) {
        console.log(`    ${line}`);
      }
    }
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
    if (!this.shouldShowOutput()) return;

    const statusColor = result.success ? 'green' : 'red';
    const statusIcon = result.success ? '✓' : '✗';

    console.log(
      `  ${this.color(statusIcon, statusColor)} ` +
        `Status: ${this.color(String(result.status || 'ERROR'), statusColor)}`,
    );

    if (result.error) {
      this.logValidationErrors(result.error);
    }

    if (this.shouldShowMetrics() && result.metrics) {
      const metrics = result.metrics;
      const parts = [`Duration: ${this.color(this.formatDuration(metrics.duration), 'cyan')}`];

      if (metrics.size !== undefined) {
        parts.push(`Size: ${this.color(this.formatSize(metrics.size), 'cyan')}`);
      }

      if (this.shouldShowRequestDetails()) {
        if (metrics.dnsLookup) {
          parts.push(`DNS: ${this.formatDuration(metrics.dnsLookup)}`);
        }
        if (metrics.tcpConnection) {
          parts.push(`TCP: ${this.formatDuration(metrics.tcpConnection)}`);
        }
        if (metrics.tlsHandshake) {
          parts.push(`TLS: ${this.formatDuration(metrics.tlsHandshake)}`);
        }
        if (metrics.firstByte) {
          parts.push(`TTFB: ${this.formatDuration(metrics.firstByte)}`);
        }
      }

      console.log(`  ${parts.join(' | ')}`);
    }

    if (this.shouldShowHeaders() && result.headers && Object.keys(result.headers).length > 0) {
      console.log(this.color('  Response Headers:', 'dim'));
      for (const [key, value] of Object.entries(result.headers)) {
        console.log(`    ${key}: ${value}`);
      }
    }

    if (this.shouldShowBody() && result.body) {
      console.log(this.color('  Response Body:', 'dim'));
      const bodyStr = this.formatJson(result.body);
      const lines = bodyStr.split('\n');
      const maxLines = this.shouldShowRequestDetails() ? Infinity : 10;
      for (const line of lines.slice(0, maxLines)) {
        console.log(`    ${line}`);
      }
      if (lines.length > maxLines) {
        console.log(this.color(`    ... (${lines.length - maxLines} more lines)`, 'dim'));
      }
    }

    console.log();
  }

  logSummary(summary: ExecutionSummary, isGlobal: boolean = false): void {
    // For raw format, don't show summary
    if (this.config.format === 'raw') return;

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
    if (!this.shouldShowOutput()) return;

    if (this.shouldShowSeparators()) {
      this.printSeparator('═');
      const title = isGlobal ? '🎯 OVERALL SUMMARY' : '📊 EXECUTION SUMMARY';
      console.log(this.color(title, 'bright'));
      this.printSeparator();
    }

    const successRate = ((summary.successful / summary.total) * 100).toFixed(1);
    const statusColor =
      summary.failed === 0 ? 'green' : summary.successful === 0 ? 'red' : 'yellow';

    console.log(`  Total Requests: ${this.color(String(summary.total), 'cyan')}`);
    console.log(`  Successful: ${this.color(String(summary.successful), 'green')}`);
    console.log(`  Failed: ${this.color(String(summary.failed), 'red')}`);
    console.log(`  Success Rate: ${this.color(`${successRate}%`, statusColor)}`);
    console.log(`  Total Duration: ${this.color(this.formatDuration(summary.duration), 'cyan')}`);

    if (summary.failed > 0 && this.shouldShowRequestDetails()) {
      console.log();
      console.log(this.color('  Failed Requests:', 'red'));
      summary.results
        .filter((r) => !r.success)
        .forEach((r) => {
          const name = r.request.name || r.request.url;
          console.log(`    • ${name}: ${r.error}`);
        });
    }

    if (this.shouldShowSeparators()) {
      this.printSeparator('═');
    }
  }

  logError(message: string): void {
    console.error(this.color(`❌ ${message}`, 'red'));
  }

  logWarning(message: string): void {
    console.warn(this.color(`⚠️  ${message}`, 'yellow'));
  }

  logInfo(message: string): void {
    console.log(this.color(`ℹ️  ${message}`, 'blue'));
  }

  logSuccess(message: string): void {
    console.log(this.color(`✅ ${message}`, 'green'));
  }

  logFileHeader(fileName: string, requestCount: number): void {
    if (!this.shouldShowOutput() || this.config.format !== 'pretty') return;
    
    const shortName = fileName.replace(/.*\//, '').replace('.yaml', '');
    console.log();
    this.printSeparator('─');
    console.log(this.color(`📄 ${shortName}.yaml`, 'bright') + this.color(` (${requestCount} request${requestCount === 1 ? '' : 's'})`, 'dim'));
    this.printSeparator('─');
  }
}
