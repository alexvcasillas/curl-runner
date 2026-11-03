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
  it('has 8 rows and 4 cards', () => {
    expect(COMPARISON_ROWS).toHaveLength(8);
    expect(COMPARISON_CARDS).toHaveLength(4);
  });
  it('shows the studio as a preview next to the CLI', () => {
    const row = COMPARISON_ROWS.find((r) => r.label === 'Primary interface');
    expect(row?.curlRunner).toBe('CLI, plus a macOS app (preview)');
  });
  it('compares importing from Postman and curl', () => {
    const row = COMPARISON_ROWS.find((r) => r.label === 'Import from Postman / curl');
    expect(row).toEqual({
      label: 'Import from Postman / curl',
      curlRunner: 'Postman collections and curl (CLI and app)',
      postman: 'curl, OpenAPI, Insomnia and more',
      insomnia: 'Postman collections, curl, OpenAPI and more',
      curl: 'Not applicable',
      httpie: 'App: Postman, Insomnia, curl',
    });
  });
  it('covers the CLI and the app in the account and license rows', () => {
    const account = COMPARISON_ROWS.find((r) => r.label === 'Account needed');
    const license = COMPARISON_ROWS.find((r) => r.label === 'Open source');
    expect(account?.curlRunner).toBe('No (CLI or app)');
    expect(license?.curlRunner).toBe('Yes, MIT (CLI and app)');
  });
  it('mentions convert postman and the app on the Postman card, fairly', () => {
    const postman = COMPARISON_CARDS.find((c) => c.tool === 'Postman');
    expect(postman?.pickCurlRunner).toContain('convert postman');
    expect(postman?.pickCurlRunner).toContain('(preview)');
    expect(postman?.pickTool).toContain('shared workspaces');
    expect(postman?.pickTool).toContain('mock servers');
    const insomnia = COMPARISON_CARDS.find((c) => c.tool === 'Insomnia');
    expect(insomnia?.pickTool).toContain('GraphQL and gRPC');
  });
  it('has no em dashes in rows or cards', () => {
    expect(JSON.stringify([COMPARISON_ROWS, COMPARISON_CARDS])).not.toContain('—');
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
    expect(html).toContain('A macOS app,');
    expect(html).toContain('Import from Postman / curl');
    expect(html).not.toContain('[VERIFY]');
  });
});
