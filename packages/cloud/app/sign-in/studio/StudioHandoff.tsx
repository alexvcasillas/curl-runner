'use client';

import { useState, useTransition } from 'react';
import { Badge } from '@/components/AuthMessage';
import { Button } from '@/components/Button';
import { ButtonLink } from '@/components/ButtonLink';
import { CopyField } from '@/components/CopyField';
import { Notice } from '@/components/Notice';
import { external } from '@/lib/routes';
import { mintStudioLink } from '../handoff';

/** The studio hand-off: a button first. The click mints the code and opens the app. */
export function StudioHandoff({ email }: { email: string }) {
  const [opened, setOpened] = useState<{ code: string; url: string } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, run] = useTransition();

  function open() {
    setError(null);
    run(async () => {
      const r = await mintStudioLink();
      if (r.ok) {
        setOpened({ code: r.code, url: r.url });
        window.location.href = r.url;
      } else {
        setError(r.message);
      }
    });
  }

  return (
    <div className="flex flex-col gap-5">
      <Badge tone="ok" />
      <div className="flex flex-col gap-1.5">
        <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">
          Back to curl-runner studio
        </h1>
        <p className="m-0 text-fg-muted">
          {opened
            ? `You’re signed in as ${email}. Your browser asks to open the studio. Allow it, and the studio finishes signing in.`
            : `You’re signed in as ${email}. Open the studio to finish signing in there.`}
        </p>
      </div>
      {error ? (
        <Notice tone="err" role="alert">
          {error}
        </Notice>
      ) : null}
      {opened ? (
        <ButtonLink href={opened.url} variant="primary" className="h-10! w-full text-[14px]!">
          Open curl-runner studio
        </ButtonLink>
      ) : (
        <Button
          variant="primary"
          loading={pending}
          onClick={open}
          className="h-10! w-full text-[14px]!"
        >
          {pending ? 'Opening…' : 'Open curl-runner studio'}
        </Button>
      )}
      {opened ? (
        <>
          <Notice tone="info">
            Studio didn’t open? The studio may not be installed on this Mac.{' '}
            <a href={external.download}>Download it</a>, then sign in from its welcome screen. Or
            paste this code in the studio. It works once and expires in 2 minutes.
          </Notice>
          <CopyField variant="secret" label="code" value={opened.code} />
        </>
      ) : null}
    </div>
  );
}
