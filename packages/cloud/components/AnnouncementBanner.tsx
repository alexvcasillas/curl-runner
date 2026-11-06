'use client';

import { useEffect, useState } from 'react';
import {
  type AnnouncementBanner as Announcement,
  dismissBanner,
  isBannerDismissed,
} from '@/lib/banner';
import { Banner } from './Banner';

/**
 * The admins' announcement, across the top of a page. It hides per browser until
 * an admin saves it again (its `version` changes). Whether it was hidden is only
 * known in the browser, so it shows once the page has loaded rather than flashing
 * for someone who already dismissed it.
 */
export function AnnouncementBanner({ banner }: { banner: Announcement | null | undefined }) {
  const [shown, setShown] = useState(false);
  const version = banner?.version;

  useEffect(() => {
    setShown(version !== undefined && !isBannerDismissed(version));
  }, [version]);

  if (!banner || !shown) {
    return null;
  }
  return (
    <Banner
      tone={banner.tone === 'warning' ? 'warn' : 'info'}
      title={banner.message}
      className="rounded-none border-x-0 border-t-0"
      action={
        <span className="flex items-center gap-1.5">
          {banner.link ? (
            <a
              href={banner.link}
              target="_blank"
              rel="noreferrer"
              className="inline-flex h-7 items-center text-[12px]"
            >
              Learn more
            </a>
          ) : null}
          <button
            type="button"
            aria-label="Dismiss announcement"
            onClick={() => {
              dismissBanner(banner.version);
              setShown(false);
            }}
            className="grid size-7 cursor-pointer place-items-center rounded-[6px] border-0 bg-transparent text-fg-muted hover:bg-surface-200 hover:text-fg"
          >
            <svg
              aria-hidden="true"
              width="14"
              height="14"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            >
              <path d="M4 4l8 8M12 4l-8 8" />
            </svg>
          </button>
        </span>
      }
    />
  );
}
