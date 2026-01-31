/**
 * Tree-style console output renderer.
 */

import { ANSI_COLORS, colorize } from './formatters';
import type { ColorName } from './types';

export interface TreeNode {
  label: string;
  value?: string;
  children?: TreeNode[];
  color?: string;
}

/**
 * Renders tree-structured output to console.
 */
export class TreeRenderer {
  private color(text: string, colorName: string): string {
    if (!colorName || !(colorName in ANSI_COLORS)) {
      return text;
    }
    return colorize(text, colorName as ColorName);
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
          for (const line of lines) {
            console.log(`${contentPrefix}${line}`);
          }
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
