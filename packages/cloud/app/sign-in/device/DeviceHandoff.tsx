'use client';

import { useState, useTransition } from 'react';
import { Badge } from '@/components/AuthMessage';
import { Button } from '@/components/Button';
import { CopyField } from '@/components/CopyField';
import { Notice } from '@/components/Notice';
import { mintDeviceCode } from '../handoff';

/** The CLI hand-off: a button first, then the code. The code is minted on the click. */
export function DeviceHandoff({ email, appUrl }: { email: string; appUrl: string }) {
  const [code, setCode] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, run] = useTransition();

  function show() {
    setError(null);
    run(async () => {
      const r = await mintDeviceCode();
      if (r.ok) {
        setCode(r.code);
      } else {
        setError(r.message);
      }
    });
  }

  if (!code) {
    return (
      <div className="flex flex-col gap-5">
        <Badge tone="ok" />
        <div className="flex flex-col gap-1.5">
          <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">
            Sign in the curl-runner CLI
          </h1>
          <p className="m-0 text-fg-muted">
            You’re signed in as {email}. Show a code, then paste it in your terminal.
          </p>
        </div>
        {error ? (
          <Notice tone="err" role="alert">
            {error}
          </Notice>
        ) : null}
        <Button
          variant="primary"
          loading={pending}
          onClick={show}
          className="h-10! w-full text-[14px]!"
        >
          {pending ? 'Getting your code…' : 'Show code'}
        </Button>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-5">
      <Badge tone="ok" />
      <div className="flex flex-col gap-1.5">
        <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">
          Paste this code in your terminal
        </h1>
        <p className="m-0 text-fg-muted">
          It signs in the curl-runner CLI as {email}. It works once and expires in 2 minutes.
        </p>
      </div>
      <CopyField variant="secret" label="code" value={code} />
      <div
        role="img"
        aria-label="Your terminal"
        className="rounded-[8px] border border-hairline bg-canvas px-4 py-3.5 font-mono text-[12px] leading-5 text-fg-muted"
      >
        <div>
          <span className="text-fg-subtle">$</span>{' '}
          <span className="text-fg">curl-runner login --browser</span>
        </div>
        <div>Opened {appUrl}/sign-in?client=device</div>
        <div>
          Paste the code from your browser:{' '}
          <span aria-hidden="true" className="inline-block h-3.5 w-[7px] bg-fg align-[-2px]" />
        </div>
      </div>
      <p className="m-0 text-[12px] text-fg-subtle">
        When the terminal says{' '}
        <code className="bg-transparent p-0 text-success">Signed in as {email}</code>, you can close
        this tab. Code expired? Run the command again.
      </p>
    </div>
  );
}
