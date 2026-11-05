import type { Metadata } from 'next';
import { Badge } from '@/components/AuthMessage';
import { AuthShell } from '@/components/AuthShell';
import { CopyField } from '@/components/CopyField';
import { api } from '@/lib/api';
import type { Me } from '@/lib/types';

export const metadata: Metadata = { title: 'Paste this code' };

/** Where `client=device` lands after sign-in: the hand-off code for the CLI to paste. */
export default async function DevicePage() {
  const me = await api<Me>('/v1/me');
  const { code } = await api<{ code: string }>('/v1/auth/handoff', {
    method: 'POST',
    json: { session_label: 'CLI' },
  });
  return (
    <AuthShell width="md" signedInAs={me.user.email}>
      <div className="flex flex-col gap-5">
        <Badge tone="ok" />
        <div className="flex flex-col gap-1.5">
          <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">
            Paste this code in your terminal
          </h1>
          <p className="m-0 text-fg-muted">
            It signs in the curl-runner CLI as {me.user.email}. It works once and expires in 2
            minutes.
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
          <div>Opened https://cloud.curl-runner.com/sign-in?client=device</div>
          <div>
            Paste the code from your browser:{' '}
            <span aria-hidden="true" className="inline-block h-3.5 w-[7px] bg-fg align-[-2px]" />
          </div>
        </div>
        <p className="m-0 text-[12px] text-fg-subtle">
          When the terminal says{' '}
          <code className="bg-transparent p-0 text-success">Signed in as {me.user.email}</code>, you
          can close this tab. Code expired? Run the command again.
        </p>
      </div>
    </AuthShell>
  );
}
