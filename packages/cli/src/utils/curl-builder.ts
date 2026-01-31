import {
  buildCurlArgs,
  type CurlExecutionResult,
  extractMetricsFromOutput,
  formatArgsForDisplay,
  getStatusCode,
  parseHeadersFromStderr,
  parseMetrics,
} from '../core/curl';
import type { RequestConfig } from '../types/config';

/**
 * Builds and executes curl commands for single requests.
 * Uses shared argument building logic from core/curl.
 */
export class CurlBuilder {
  /**
   * Builds curl command-line arguments from a request config.
   */
  static buildCommand(config: RequestConfig): string[] {
    const { args, url } = buildCurlArgs(config, {
      includeSilentFlags: true,
      includeHttp2Flag: true,
      includeOutputFlag: true,
    });

    // Add URL as the last argument
    args.push(url);
    return args;
  }

  /**
   * Formats args array as shell-safe command string for display/debugging.
   */
  static formatCommandForDisplay(args: string[]): string {
    return formatArgsForDisplay(args);
  }

  /**
   * Executes a curl command with the given arguments.
   */
  static async executeCurl(args: string[]): Promise<CurlExecutionResult> {
    try {
      const proc = Bun.spawn(['curl', ...args], {
        stdout: 'pipe',
        stderr: 'pipe',
      });

      const stdout = await new Response(proc.stdout).text();
      const stderr = await new Response(proc.stderr).text();

      await proc.exited;

      if (proc.exitCode !== 0 && !stdout) {
        return {
          success: false,
          error: stderr || `Command failed with exit code ${proc.exitCode}`,
        };
      }

      const { body, metrics: rawMetrics } = extractMetricsFromOutput(stdout);
      const statusCode = getStatusCode(rawMetrics);
      const headers = statusCode ? parseHeadersFromStderr(stderr) : {};
      const metrics = parseMetrics(rawMetrics);

      return {
        success: true,
        status: statusCode,
        headers,
        body,
        metrics,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }
}
