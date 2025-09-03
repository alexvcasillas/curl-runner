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
      verbose: true,
      showHeaders: true,
      showBody: true,
      showMetrics: true,
      format: 'pretty',
      ...config,
    };
  }

  private color(text: string, color: keyof typeof this.colors): string {
    return `${this.colors[color]}${text}${this.colors.reset}`;
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
    this.printSeparator('═');
    console.log(this.color('🚀 CURL RUNNER', 'bright'));
    console.log(this.color(`Executing ${count} request(s) in ${mode} mode`, 'cyan'));
    this.printSeparator('═');
    console.log();
  }

  logRequestStart(config: RequestConfig, index: number): void {
    const name = config.name || `Request #${index}`;
    console.log(this.color(`▶ ${name}`, 'bright'));
    console.log(
      `  ${this.color(config.method || 'GET', 'yellow')} ${this.color(config.url, 'blue')}`,
    );

    if (this.config.verbose && config.headers && Object.keys(config.headers).length > 0) {
      console.log(this.color('  Headers:', 'dim'));
      for (const [key, value] of Object.entries(config.headers)) {
        console.log(`    ${key}: ${value}`);
      }
    }

    if (this.config.verbose && config.body) {
      console.log(this.color('  Body:', 'dim'));
      const bodyStr = this.formatJson(config.body);
      for (const line of bodyStr.split('\n')) {
        console.log(`    ${line}`);
      }
    }
  }

  logCommand(command: string): void {
    if (this.config.verbose) {
      console.log(this.color('  Command:', 'dim'));
      console.log(this.color(`    ${command}`, 'dim'));
    }
  }

  logRetry(attempt: number, maxRetries: number): void {
    console.log(this.color(`  ↻ Retry ${attempt}/${maxRetries}...`, 'yellow'));
  }

  logRequestComplete(result: ExecutionResult): void {
    const statusColor = result.success ? 'green' : 'red';
    const statusIcon = result.success ? '✓' : '✗';

    console.log(
      `  ${this.color(statusIcon, statusColor)} ` +
        `Status: ${this.color(String(result.status || 'ERROR'), statusColor)}`,
    );

    if (result.error) {
      console.log(`  ${this.color('Error:', 'red')} ${result.error}`);
    }

    if (this.config.showMetrics && result.metrics) {
      const metrics = result.metrics;
      const parts = [`Duration: ${this.color(this.formatDuration(metrics.duration), 'cyan')}`];

      if (metrics.size !== undefined) {
        parts.push(`Size: ${this.color(this.formatSize(metrics.size), 'cyan')}`);
      }

      if (this.config.verbose) {
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

    if (this.config.showHeaders && result.headers && Object.keys(result.headers).length > 0) {
      console.log(this.color('  Response Headers:', 'dim'));
      for (const [key, value] of Object.entries(result.headers)) {
        console.log(`    ${key}: ${value}`);
      }
    }

    if (this.config.showBody && result.body) {
      console.log(this.color('  Response Body:', 'dim'));
      const bodyStr = this.formatJson(result.body);
      const lines = bodyStr.split('\n');
      const maxLines = this.config.verbose ? Infinity : 10;
      for (const line of lines.slice(0, maxLines)) {
        console.log(`    ${line}`);
      }
      if (lines.length > maxLines) {
        console.log(this.color(`    ... (${lines.length - maxLines} more lines)`, 'dim'));
      }
    }

    console.log();
  }

  logSummary(summary: ExecutionSummary): void {
    this.printSeparator('═');
    console.log(this.color('📊 EXECUTION SUMMARY', 'bright'));
    this.printSeparator();

    const successRate = ((summary.successful / summary.total) * 100).toFixed(1);
    const statusColor =
      summary.failed === 0 ? 'green' : summary.successful === 0 ? 'red' : 'yellow';

    console.log(`  Total Requests: ${this.color(String(summary.total), 'cyan')}`);
    console.log(`  Successful: ${this.color(String(summary.successful), 'green')}`);
    console.log(`  Failed: ${this.color(String(summary.failed), 'red')}`);
    console.log(`  Success Rate: ${this.color(`${successRate}%`, statusColor)}`);
    console.log(`  Total Duration: ${this.color(this.formatDuration(summary.duration), 'cyan')}`);

    if (summary.failed > 0 && this.config.verbose) {
      console.log();
      console.log(this.color('  Failed Requests:', 'red'));
      summary.results
        .filter((r) => !r.success)
        .forEach((r) => {
          const name = r.request.name || r.request.url;
          console.log(`    • ${name}: ${r.error}`);
        });
    }

    this.printSeparator('═');
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
}
