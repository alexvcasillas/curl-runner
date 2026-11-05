'use client';

import { useEffect, useRef, useState } from 'react';
import { Button } from './Button';
import { LinkIcon } from './icons';

/** Copies the share link. The page never shows the link itself. */
export function CopyLinkButton({ link, disabled }: { link: string; disabled?: boolean }) {
  const [state, setState] = useState<'idle' | 'copied' | 'failed'>('idle');
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);

  async function copy() {
    clearTimeout(timer.current);
    try {
      await navigator.clipboard.writeText(link);
      setState('copied');
    } catch {
      setState('failed');
    }
    timer.current = setTimeout(() => setState('idle'), 2000);
  }

  return (
    <>
      <Button onClick={copy} disabled={disabled} title={disabled ? 'Sharing is off' : undefined}>
        <LinkIcon size={14} />
        {state === 'copied' ? 'Link copied' : state === 'failed' ? 'Couldn’t copy' : 'Copy link'}
      </Button>
      <span role="status" className="sr-only">
        {state === 'copied' ? 'Link copied' : state === 'failed' ? 'Couldn’t copy the link.' : ''}
      </span>
    </>
  );
}
