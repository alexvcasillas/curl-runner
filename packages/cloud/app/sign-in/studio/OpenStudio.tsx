'use client';

import { useEffect, useRef } from 'react';

/** Hands the browser the `curl-runner://` link once. The page stays as the fallback. */
export function OpenStudio({ url }: { url: string }) {
  const opened = useRef(false);
  useEffect(() => {
    if (opened.current) {
      return;
    }
    opened.current = true;
    window.location.href = url;
  }, [url]);
  return null;
}
