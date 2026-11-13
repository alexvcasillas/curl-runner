import { describe, expect, it } from 'vitest';
import {
  accessLabel,
  collectionsExportHref,
  collectionsHref,
  formatBytes,
  parseCollectionsQuery,
} from '@/lib/admin-collections';

describe('parseCollectionsQuery', () => {
  it('accepts the listed filter', () => {
    expect(parseCollectionsQuery({ filter: 'listed' }).filter).toBe('listed');
    expect(collectionsHref({ q: '', filter: 'listed', trail: [] })).toBe(
      '/admin/collections?filter=listed',
    );
  });

  it('keeps a good query', () => {
    expect(
      parseCollectionsQuery({ q: ' spam ', filter: 'disabled', cursor: 'abc_1', trail: '-.abc' }),
    ).toEqual({ q: 'spam', filter: 'disabled', cursor: 'abc_1', trail: ['-', 'abc'] });
  });

  it('falls back on anything unknown', () => {
    expect(parseCollectionsQuery({ filter: 'bogus', cursor: 'a b!', trail: ['x'] })).toEqual({
      q: '',
      filter: 'all',
      cursor: undefined,
      trail: [],
    });
  });

  it('caps the search at the server limit', () => {
    expect(parseCollectionsQuery({ q: 'a'.repeat(300) }).q).toHaveLength(200);
  });
});

describe('hrefs', () => {
  it('carries only what differs from the default', () => {
    expect(collectionsHref({ q: '', filter: 'all', trail: [] })).toBe('/admin/collections');
    expect(collectionsHref({ q: 'x', filter: 'public', cursor: 'c1', trail: ['-'] })).toBe(
      '/admin/collections?q=x&filter=public&cursor=c1&trail=-',
    );
    expect(collectionsExportHref({ q: 'x y', filter: 'disabled' })).toBe(
      '/admin/collections/export?q=x+y&filter=disabled',
    );
  });
});

describe('wording', () => {
  it('formats sizes the way the board does', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(9 * 1024)).toBe('9 KB');
    expect(formatBytes(86_016)).toBe('84 KB');
    expect(formatBytes(1_258_291)).toBe('1.2 MB');
    expect(formatBytes(2_254_857_830)).toBe('2.1 GB');
    expect(formatBytes(671_088_640)).toBe('640 MB');
  });

  it('names access', () => {
    expect(accessLabel('off')).toBe('Off');
    expect(accessLabel('restricted')).toBe('Restricted');
  });
});
