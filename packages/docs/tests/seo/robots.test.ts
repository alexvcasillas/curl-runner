import { describe, expect, it } from 'vitest';
import robots from '@/app/robots';

describe('robots', () => {
  const result = robots();
  const rules = Array.isArray(result.rules) ? result.rules : [result.rules];

  it('has one rule for every crawler that allows / and disallows only /api/', () => {
    expect(rules).toHaveLength(1);
    expect(rules[0].userAgent).toBe('*');
    expect(rules[0].allow).toBe('/');
    expect(rules[0].disallow).toBe('/api/');
  });

  it('names the sitemap', () => {
    expect(result.sitemap).toBe('https://www.curl-runner.com/sitemap.xml');
  });
});
