import { afterEach, describe, expect, it, vi } from 'vitest';
import { bannerKey, dismissBanner, isBannerDismissed } from '@/lib/banner';

afterEach(() => {
  localStorage.clear();
  vi.restoreAllMocks();
});

describe('banner dismissal', () => {
  it('keys on the version', () => {
    expect(bannerKey(3)).toBe('cr_banner_dismissed=3');
  });

  it('remembers a dismissal for that version only', () => {
    expect(isBannerDismissed(3)).toBe(false);
    dismissBanner(3);
    expect(isBannerDismissed(3)).toBe(true);
    expect(isBannerDismissed(4)).toBe(false);
  });

  it('never throws when storage is blocked', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    expect(isBannerDismissed(1)).toBe(false);
    expect(() => dismissBanner(1)).not.toThrow();
  });
});
