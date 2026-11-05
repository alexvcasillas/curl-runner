'use client';

import { useState } from 'react';
import { buttonClasses } from './button-styles';
import { InfoIcon } from './icons';

/**
 * Open in studio and its fallback. A custom-scheme link gives the browser no way
 * to say the app is missing, so clicking it opens "Studio didn’t open?", which
 * says what to install. It is always there for someone who looks for it.
 */
export function OpenInStudio({ href, downloadHref }: { href: string; downloadHref: string }) {
  const [help, setHelp] = useState(false);
  return (
    <>
      <a
        href={href}
        onClick={() => setHelp(true)}
        className={buttonClasses('primary', 'lg', 'h-10! w-full text-[14px]! max-sm:h-11!')}
      >
        Open in curl-runner studio
      </a>
      <details
        open={help}
        onToggle={(e) => setHelp(e.currentTarget.open)}
        className="group text-[12px] leading-[18px]"
      >
        <summary className="flex cursor-pointer list-none items-center gap-2 text-fg-muted marker:hidden hover:text-fg [&::-webkit-details-marker]:hidden">
          <InfoIcon size={13} className="shrink-0" />
          Studio didn’t open?
        </summary>
        <div
          role="status"
          className="mt-2 rounded-[6px] border border-info-border bg-info-bg px-3 py-2.5"
        >
          The studio needs to be installed first. Download it below, open it once, then click Open
          again. Version 2.7 or newer.
        </div>
      </details>
      <a
        href={downloadHref}
        className={buttonClasses('secondary', 'lg', 'h-10! w-full text-[14px]! max-sm:h-11!')}
      >
        Download the studio for macOS
      </a>
    </>
  );
}
