import { describe, expect, it } from 'vitest';
import { clientIp } from '@/lib/api';

describe('clientIp', () => {
  it('takes the right-most X-Forwarded-For entry', () => {
    const h = new Headers({ 'x-forwarded-for': '203.0.113.9, 10.0.0.1,  198.51.100.7 ' });
    expect(clientIp(h)).toBe('198.51.100.7');
  });

  it('skips empty trailing entries', () => {
    expect(clientIp(new Headers({ 'x-forwarded-for': '198.51.100.7, ' }))).toBe('198.51.100.7');
  });

  it('falls back to X-Real-IP', () => {
    expect(clientIp(new Headers({ 'x-real-ip': ' 198.51.100.8 ' }))).toBe('198.51.100.8');
  });

  it('gives null with neither', () => {
    expect(clientIp(new Headers())).toBeNull();
    expect(clientIp(new Headers({ 'x-forwarded-for': ' , ' }))).toBeNull();
  });
});
