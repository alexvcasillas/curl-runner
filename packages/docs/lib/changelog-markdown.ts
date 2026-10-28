import { Marked, type Token, type Tokens } from 'marked';
import { bundledLanguages, codeToHtml } from 'shiki';
import { YAML_THEME } from '@/components/site/Yaml';
import type { ChangeEntry } from '@/lib/changelog';

/** Entries whose body is longer than this collapse behind a native `<details>`. */
export const COLLAPSE_THRESHOLD = 1200;
/** Roughly how much of a collapsed entry stays visible. */
const PREVIEW_LENGTH = 600;
/** A collapsed tail shorter than this is shown instead of hidden. */
const MIN_HIDDEN_LENGTH = 300;

export interface RenderedEntry {
  headlineHtml: string | null;
  previewHtml: string;
  /** The rest of a long entry, or null when the whole entry is shown. */
  restHtml: string | null;
}

type HighlightedCode = Tokens.Code & { highlighted?: string };

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

const markdown = new Marked({
  gfm: true,
  renderer: {
    // The changelog is trusted repo content, but raw HTML in it is shown as text, never rendered.
    html({ text }) {
      return escapeHtml(text);
    },
    code(token) {
      const { highlighted, text } = token as HighlightedCode;
      return `<div class="release-code">${highlighted ?? `<pre><code>${escapeHtml(text)}</code></pre>`}</div>`;
    },
    // Releases are h2 and entry headlines h3, so headings inside a note start at h4.
    heading({ tokens, depth }) {
      const level = Math.min(depth + 1, 6);
      return `<h${level}>${this.parser.parseInline(tokens)}</h${level}>\n`;
    },
  },
});

async function highlightCode(tokens: Token[]): Promise<void> {
  const jobs: Promise<void>[] = [];
  markdown.walkTokens(tokens, (token) => {
    if (token.type !== 'code') {
      return;
    }
    const code = token as HighlightedCode;
    const lang = code.lang && code.lang in bundledLanguages ? code.lang : 'text';
    jobs.push(
      codeToHtml(code.text, { lang, theme: YAML_THEME }).then((html) => {
        code.highlighted = html;
      }),
    );
  });
  await Promise.all(jobs);
}

interface Group {
  tokens: Token[];
  migration: boolean;
}

function size(group: Group): number {
  return group.tokens.reduce((n, t) => n + t.raw.length, 0);
}

/**
 * Splits a note into groups: each heading starts a section; content before the
 * first heading (or a note without headings) is one group per block. Migration
 * sections move to the front, after the introduction.
 */
function groupTokens(tokens: Token[]): Group[] {
  const intro: Group[] = [];
  const migrations: Group[] = [];
  const sections: Group[] = [];
  let current: Group | null = null;

  for (const token of tokens) {
    if (token.type === 'space') {
      continue;
    }
    if (token.type === 'heading') {
      current = { tokens: [token], migration: /^migration\b/i.test(token.text) };
      (current.migration ? migrations : sections).push(current);
    } else if (current) {
      current.tokens.push(token);
    } else {
      intro.push({ tokens: [token], migration: false });
    }
  }
  return [...intro, ...migrations, ...sections];
}

function renderGroups(groups: Group[]): string {
  return groups
    .map((group) => {
      const html = markdown.parser(group.tokens);
      return group.migration ? `<div class="release-migration">${html}</div>` : html;
    })
    .join('');
}

/** Renders one entry's Markdown, splitting long entries into a preview and the rest. */
export async function renderEntry(entry: ChangeEntry): Promise<RenderedEntry> {
  const tokens = markdown.lexer(entry.body);
  await highlightCode(tokens);
  const groups = groupTokens(tokens);
  const headlineHtml = entry.headline
    ? markdown.parseInline(entry.headline, { async: false })
    : null;

  let shown = groups.length;
  if (entry.body.length > COLLAPSE_THRESHOLD) {
    // Never hide a migration box; then fill the preview up to about PREVIEW_LENGTH.
    let visible = 0;
    shown = 0;
    while (
      shown < groups.length &&
      (shown === 0 || groups[shown].migration || visible < PREVIEW_LENGTH)
    ) {
      visible += size(groups[shown]);
      shown += 1;
    }
    const hidden = groups.slice(shown).reduce((n, g) => n + size(g), 0);
    if (hidden < MIN_HIDDEN_LENGTH) {
      shown = groups.length;
    }
  }

  return {
    headlineHtml,
    previewHtml: renderGroups(groups.slice(0, shown)),
    restHtml: shown < groups.length ? renderGroups(groups.slice(shown)) : null,
  };
}

/** Plain-text summary of an entry for feeds: the headline, else the first paragraph. */
export function entrySummary(entry: ChangeEntry, max = 280): string {
  const source = entry.headline ?? entry.body.split(/\n\s*\n/)[0] ?? '';
  const text = source
    .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/`|\*\*/g, '')
    .replace(/\s+/g, ' ')
    .trim();
  return text.length > max ? `${text.slice(0, max - 1).trimEnd()}…` : text;
}
