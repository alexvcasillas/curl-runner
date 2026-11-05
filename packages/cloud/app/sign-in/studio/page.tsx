import type { Metadata } from 'next';
import { Badge } from '@/components/AuthMessage';
import { AuthShell } from '@/components/AuthShell';
import { ButtonLink } from '@/components/ButtonLink';
import { CopyField } from '@/components/CopyField';
import { Notice } from '@/components/Notice';
import { api } from '@/lib/api';
import { env } from '@/lib/env';
import { external } from '@/lib/routes';
import type { Me } from '@/lib/types';
import { OpenStudio } from './OpenStudio';

export const metadata: Metadata = { title: 'Back to curl-runner studio' };

/** Where `client=studio` lands after sign-in: opens the desktop app with a hand-off code. */
export default async function StudioPage() {
  const me = await api<Me>('/v1/me');
  const { code } = await api<{ code: string }>('/v1/auth/handoff', {
    method: 'POST',
    json: { session_label: 'Studio' },
  });
  const url = `curl-runner://auth/callback?code=${encodeURIComponent(code)}&server=${encodeURIComponent(env().PUBLIC_API_URL)}`;
  return (
    <AuthShell width="md" signedInAs={me.user.email}>
      <OpenStudio url={url} />
      <div className="flex flex-col gap-5">
        <Badge tone="ok" />
        <div className="flex flex-col gap-1.5">
          <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">
            Back to curl-runner studio
          </h1>
          <p className="m-0 text-fg-muted">
            You’re signed in as {me.user.email}. Your browser asks to open the studio. Allow it, and
            the studio finishes signing in.
          </p>
        </div>
        <ButtonLink href={url} variant="primary" className="h-10! w-full text-[14px]!">
          Open curl-runner studio
        </ButtonLink>
        <Notice tone="info">
          Studio didn’t open? The studio may not be installed on this Mac.{' '}
          <a href={external.download}>Download it</a>, then sign in from its welcome screen. Or
          paste this code in the studio. It works once and expires in 2 minutes.
        </Notice>
        <CopyField variant="secret" label="code" value={code} />
      </div>
    </AuthShell>
  );
}
