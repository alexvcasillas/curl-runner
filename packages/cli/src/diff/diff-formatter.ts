import type { ColorName } from '../core/format';
import { colorize } from '../core/format';
import type { DiffCompareResult, DiffSummary, ResponseDiff } from '../types/config';

/**
 * Formats diff comparison results for various outputs.
 */
export class DiffFormatter {
  private outputFormat: 'terminal' | 'json' | 'markdown';

  constructor(outputFormat: 'terminal' | 'json' | 'markdown' = 'terminal') {
    this.outputFormat = outputFormat;
  }

  private color(text: string, color: ColorName): string {
    if (this.outputFormat !== 'terminal') {
      return text;
    }
    return colorize(text, color);
  }

  /**
   * Formats the complete diff summary.
   */
  formatSummary(summary: DiffSummary, baselineLabel: string, currentLabel: string): string {
    switch (this.outputFormat) {
      case 'json':
        return this.formatSummaryJson(summary);
      case 'markdown':
        return this.formatSummaryMarkdown(summary, baselineLabel, currentLabel);
      default:
        return this.formatSummaryTerminal(summary, baselineLabel, currentLabel);
    }
  }

  /**
   * Terminal format for summary.
   */
  private formatSummaryTerminal(
    summary: DiffSummary,
    baselineLabel: string,
    currentLabel: string,
  ): string {
    const lines: string[] = [];

    lines.push('');
    lines.push(
      `${this.color('Response Diff:', 'bright')} ${this.color(baselineLabel, 'cyan')} → ${this.color(currentLabel, 'cyan')}`,
    );
    lines.push('');

    for (const result of summary.results) {
      lines.push(this.formatResultTerminal(result));
      lines.push('');
    }

    lines.push(this.formatStatsSummary(summary));

    return lines.join('\n');
  }

  /**
   * JSON format for summary.
   */
  private formatSummaryJson(summary: DiffSummary): string {
    return JSON.stringify(
      {
        summary: {
          total: summary.totalRequests,
          unchanged: summary.unchanged,
          changed: summary.changed,
          newBaselines: summary.newBaselines,
        },
        results: summary.results.map((r) => ({
          request: r.requestName,
          status: r.hasDifferences ? 'changed' : r.isNewBaseline ? 'new' : 'unchanged',
          differences: r.differences,
          timingDiff: r.timingDiff,
        })),
      },
      null,
      2,
    );
  }

  /**
   * Markdown format for summary.
   */
  private formatSummaryMarkdown(
    summary: DiffSummary,
    baselineLabel: string,
    currentLabel: string,
  ): string {
    const lines: string[] = [];

    lines.push(`# Response Diff: ${baselineLabel} → ${currentLabel}`);
    lines.push('');
    lines.push(`| Metric | Count |`);
    lines.push(`|--------|-------|`);
    lines.push(`| Total Requests | ${summary.totalRequests} |`);
    lines.push(`| Unchanged | ${summary.unchanged} |`);
    lines.push(`| Changed | ${summary.changed} |`);
    lines.push(`| New Baselines | ${summary.newBaselines} |`);
    lines.push('');

    if (summary.changed > 0) {
      lines.push('## Changes');
      lines.push('');

      for (const result of summary.results) {
        if (result.hasDifferences) {
          lines.push(this.formatResultMarkdown(result));
        }
      }
    }

    if (summary.newBaselines > 0) {
      lines.push('## New Requests');
      lines.push('');
      for (const result of summary.results) {
        if (result.isNewBaseline) {
          lines.push(`- \`${result.requestName}\``);
        }
      }
      lines.push('');
    }

    return lines.join('\n');
  }

  /**
   * Formats a single result for terminal.
   */
  formatResultTerminal(result: DiffCompareResult): string {
    const lines: string[] = [];

    if (result.isNewBaseline) {
      lines.push(`  ${this.color('NEW', 'cyan')} ${this.color(result.requestName, 'bright')}`);
      return lines.join('\n');
    }

    if (!result.hasDifferences) {
      lines.push(
        `  ${this.color('✓', 'green')} ${this.color(result.requestName, 'bright')} ${this.color('(no changes)', 'dim')}`,
      );
      return lines.join('\n');
    }

    lines.push(`  ${this.color('✗', 'red')} ${this.color(result.requestName, 'bright')}`);

    for (const diff of result.differences) {
      lines.push(this.formatDifferenceTerminal(diff));
    }

    if (result.timingDiff) {
      const { baseline, current, changePercent } = result.timingDiff;
      const sign = changePercent >= 0 ? '+' : '';
      const color = changePercent > 20 ? 'red' : changePercent < -20 ? 'green' : 'yellow';
      lines.push(
        `    ${this.color('timing:', 'cyan')} ${baseline}ms → ${current}ms ${this.color(`(${sign}${changePercent.toFixed(0)}%)`, color)}`,
      );
    }

    return lines.join('\n');
  }

  /**
   * Formats a single difference for terminal.
   */
  private formatDifferenceTerminal(diff: ResponseDiff): string {
    const lines: string[] = [];
    const path = diff.path || '(root)';

    switch (diff.type) {
      case 'added':
        lines.push(`    ${this.color(path, 'cyan')}:`);
        lines.push(`      ${this.color(`+ ${this.stringify(diff.current)}`, 'green')}`);
        break;

      case 'removed':
        lines.push(`    ${this.color(path, 'cyan')}:`);
        lines.push(`      ${this.color(`- ${this.stringify(diff.baseline)}`, 'red')}`);
        break;

      case 'changed':
        lines.push(`    ${this.color(path, 'cyan')}:`);
        lines.push(`      ${this.color(`- ${this.stringify(diff.baseline)}`, 'red')}`);
        lines.push(`      ${this.color(`+ ${this.stringify(diff.current)}`, 'green')}`);
        break;

      case 'type_mismatch':
        lines.push(`    ${this.color(path, 'cyan')} (type mismatch):`);
        lines.push(
          `      ${this.color(`- ${this.stringify(diff.baseline)} (${typeof diff.baseline})`, 'red')}`,
        );
        lines.push(
          `      ${this.color(`+ ${this.stringify(diff.current)} (${typeof diff.current})`, 'green')}`,
        );
        break;
    }

    return lines.join('\n');
  }

  /**
   * Formats a single result for markdown.
   */
  private formatResultMarkdown(result: DiffCompareResult): string {
    const lines: string[] = [];

    lines.push(`### \`${result.requestName}\``);
    lines.push('');
    lines.push('```diff');

    for (const diff of result.differences) {
      lines.push(this.formatDifferenceMarkdown(diff));
    }

    lines.push('```');

    if (result.timingDiff) {
      const { baseline, current, changePercent } = result.timingDiff;
      const sign = changePercent >= 0 ? '+' : '';
      lines.push('');
      lines.push(`**Timing:** ${baseline}ms → ${current}ms (${sign}${changePercent.toFixed(0)}%)`);
    }

    lines.push('');

    return lines.join('\n');
  }

  /**
   * Formats a single difference for markdown.
   */
  private formatDifferenceMarkdown(diff: ResponseDiff): string {
    const lines: string[] = [];
    const path = diff.path || '(root)';

    switch (diff.type) {
      case 'added':
        lines.push(`# ${path}:`);
        lines.push(`+ ${this.stringify(diff.current)}`);
        break;

      case 'removed':
        lines.push(`# ${path}:`);
        lines.push(`- ${this.stringify(diff.baseline)}`);
        break;

      case 'changed':
        lines.push(`# ${path}:`);
        lines.push(`- ${this.stringify(diff.baseline)}`);
        lines.push(`+ ${this.stringify(diff.current)}`);
        break;

      case 'type_mismatch':
        lines.push(`# ${path} (type mismatch):`);
        lines.push(`- ${this.stringify(diff.baseline)} (${typeof diff.baseline})`);
        lines.push(`+ ${this.stringify(diff.current)} (${typeof diff.current})`);
        break;
    }

    return lines.join('\n');
  }

  /**
   * Formats stats summary for terminal.
   */
  private formatStatsSummary(summary: DiffSummary): string {
    const parts: string[] = [];

    if (summary.unchanged > 0) {
      parts.push(this.color(`${summary.unchanged} unchanged`, 'green'));
    }
    if (summary.changed > 0) {
      parts.push(this.color(`${summary.changed} changed`, 'red'));
    }
    if (summary.newBaselines > 0) {
      parts.push(this.color(`${summary.newBaselines} new`, 'cyan'));
    }

    return `Summary: ${parts.join(', ')} (${summary.totalRequests} total)`;
  }

  /**
   * Converts value to display string.
   */
  private stringify(value: unknown): string {
    if (value === undefined) {
      return 'undefined';
    }
    if (value === null) {
      return 'null';
    }
    if (typeof value === 'string') {
      return `"${value}"`;
    }
    if (typeof value === 'object') {
      const str = JSON.stringify(value);
      if (str.length > 80) {
        return `${str.slice(0, 77)}...`;
      }
      return str;
    }
    return String(value);
  }
}
