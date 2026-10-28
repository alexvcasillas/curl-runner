import { prerenderToNodeStream } from 'react-dom/static.node';
import { describe, expect, it } from 'vitest';
import AlternativesPage from '@/app/(marketing)/alternatives/page';
import { COMPARISON_CARDS, COMPARISON_CHECKED, COMPARISON_ROWS } from '@/lib/alternatives';

describe('alternatives data', () => {
  it('has no unverified placeholders', () => {
    const all = JSON.stringify([COMPARISON_ROWS, COMPARISON_CARDS]);
    expect(all).not.toContain('[VERIFY]');
  });
  it('has an ISO checked date', () => {
    expect(COMPARISON_CHECKED).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });
  it('has 7 rows and 4 cards', () => {
    expect(COMPARISON_ROWS).toHaveLength(7);
    expect(COMPARISON_CARDS).toHaveLength(4);
  });
  it('has no empty cells', () => {
    for (const r of COMPARISON_ROWS) {
      for (const v of Object.values(r)) {
        expect(v.trim()).not.toBe('');
      }
    }
  });

  it('renders an accessible table, issue link and no client script', async () => {
    const { prelude } = await prerenderToNodeStream(<AlternativesPage />);
    const chunks: Buffer[] = [];
    for await (const c of prelude) {
      chunks.push(Buffer.from(c));
    }
    const html = Buffer.concat(chunks).toString('utf8');
    expect(html).toContain('scope="col"');
    expect(html).toContain('scope="row"');
    expect(html).toContain('https://github.com/alexvcasillas/curl-runner/issues/new');
    expect(html).toContain(COMPARISON_CHECKED);
    for (const c of COMPARISON_CARDS) {
      expect(html).toMatch(new RegExp(`curl-runner vs (<!-- -->)?${c.tool}<`));
    }
    expect(html).not.toContain('[VERIFY]');
  });
});
