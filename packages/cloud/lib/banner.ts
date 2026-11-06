// The announcement banner's "hide it for me" memory. Safe to import from client components.

import type { Info } from './types';

export type AnnouncementBanner = NonNullable<Info['banner']>;

/** One key per version, so saving the banner again shows it again. */
export function bannerKey(version: number): string {
  return `cr_banner_dismissed=${version}`;
}

/** Whether this browser hid this version. Blocked storage means "not hidden". */
export function isBannerDismissed(version: number): boolean {
  try {
    return window.localStorage.getItem(bannerKey(version)) === '1';
  } catch {
    return false;
  }
}

/** Hides this version in this browser. Blocked storage just means it comes back next time. */
export function dismissBanner(version: number): void {
  try {
    window.localStorage.setItem(bannerKey(version), '1');
  } catch {
    // Private window or blocked site data: hiding lasts until the page reloads.
  }
}
