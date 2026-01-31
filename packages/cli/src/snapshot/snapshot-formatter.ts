import type { ColorName } from '../core/format';
import { colorize } from '../core/format';
import type { SnapshotCompareResult, SnapshotDiff } from '../types/config';

/**
 * Formats snapshot comparison results for terminal output.
 */
export class SnapshotFormatter {
  private color(text: string, color: ColorName): string {
    return colorize(text, color);
  }

  /**
   * Formats the snapshot result for display.
   */
  formatResult(requestName: string, result: SnapshotCompareResult): string {
    const lines: string[] = [];

    if (result.isNew && result.updated) {
      lines.push(
        `  ${this.color('NEW', 'cyan')} Snapshot created for "${this.color(requestName, 'bright')}"`,
      );
      return lines.join('\n');
    }

    if (result.updated) {
      lines.push(
        `  ${this.color('UPDATED', 'yellow')} Snapshot updated for "${this.color(requestName, 'bright')}"`,
      );
      return lines.join('\n');
    }

    if (result.match) {
      lines.push(
        `  ${this.color('PASS', 'green')} Snapshot matches for "${this.color(requestName, 'bright')}"`,
      );
      return lines.join('\n');
    }

    // Mismatch
    lines.push(
      `  ${this.color('FAIL', 'red')} Snapshot mismatch for "${this.color(requestName, 'bright')}"`,
    );
    lines.push('');
    lines.push(this.formatDiff(result.differences));
    lines.push('');
    lines.push(this.color('  Run with --update-snapshots (-u) to update', 'dim'));

    return lines.join('\n');
  }

  /**
   * Formats differences for display.
   */
  formatDiff(differences: SnapshotDiff[]): string {
    const lines: string[] = [];
    lines.push(`  ${this.color('- Expected', 'red')}`);
    lines.push(`  ${this.color('+ Received', 'green')}`);
    lines.push('');

    for (const diff of differences) {
      lines.push(this.formatDifference(diff));
    }

    return lines.join('\n');
  }

  /**
   * Formats a single difference.
   */
  private formatDifference(diff: SnapshotDiff): string {
    const lines: string[] = [];
    const path = diff.path || '(root)';

    switch (diff.type) {
      case 'added':
        lines.push(`  ${this.color(path, 'cyan')}:`);
        lines.push(`    ${this.color(`+ ${this.stringify(diff.received)}`, 'green')}`);
        break;

      case 'removed':
        lines.push(`  ${this.color(path, 'cyan')}:`);
        lines.push(`    ${this.color(`- ${this.stringify(diff.expected)}`, 'red')}`);
        break;

      case 'changed':
        lines.push(`  ${this.color(path, 'cyan')}:`);
        lines.push(`    ${this.color(`- ${this.stringify(diff.expected)}`, 'red')}`);
        lines.push(`    ${this.color(`+ ${this.stringify(diff.received)}`, 'green')}`);
        break;

      case 'type_mismatch':
        lines.push(`  ${this.color(path, 'cyan')} (type mismatch):`);
        lines.push(
          `    ${this.color(`- ${this.stringify(diff.expected)} (${typeof diff.expected})`, 'red')}`,
        );
        lines.push(
          `    ${this.color(`+ ${this.stringify(diff.received)} (${typeof diff.received})`, 'green')}`,
        );
        break;
    }

    return lines.join('\n');
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
      // Truncate long values
      if (str.length > 80) {
        return `${str.slice(0, 77)}...`;
      }
      return str;
    }
    return String(value);
  }

  /**
   * Formats summary statistics.
   */
  formatSummary(stats: SnapshotStats): string {
    const parts: string[] = [];

    if (stats.passed > 0) {
      parts.push(this.color(`${stats.passed} passed`, 'green'));
    }
    if (stats.failed > 0) {
      parts.push(this.color(`${stats.failed} failed`, 'red'));
    }
    if (stats.updated > 0) {
      parts.push(this.color(`${stats.updated} updated`, 'yellow'));
    }
    if (stats.created > 0) {
      parts.push(this.color(`${stats.created} created`, 'cyan'));
    }

    if (parts.length === 0) {
      return '';
    }

    return `Snapshots: ${parts.join(', ')}`;
  }
}

export interface SnapshotStats {
  passed: number;
  failed: number;
  updated: number;
  created: number;
}
