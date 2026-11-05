import type { Metadata } from 'next';
import { redirect } from 'next/navigation';
import { AuthShell } from '@/components/AuthShell';
import { Notice } from '@/components/Notice';
import { api } from '@/lib/api';
import { getViewer } from '@/lib/public-data';
import { destination, first, type Provider, parseClient } from '@/lib/sign-in';
import type { Info } from '@/lib/types';
import { SignInForm } from './SignInForm';

export const metadata: Metadata = { title: 'Sign in' };

export default async function SignInPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const sp = await searchParams;
  const client = parseClient(first(sp.client));
  const returnTo = destination(first(sp.return_to), client);

  if (await getViewer()) {
    redirect(returnTo);
  }

  const info = await api<Info>('/v1/info', { auth: 'none' });
  const providers = info.sign_in.filter((p): p is Provider => p === 'github' || p === 'google');

  return (
    <AuthShell footer>
      {client === 'device' ? (
        <Notice tone="info" role="status">
          Signing in for the curl-runner CLI. After you sign in, you get a code to paste in your
          terminal.
        </Notice>
      ) : null}
      {client === 'studio' ? (
        <Notice tone="info" role="status">
          Signing in for the curl-runner studio. After you sign in, your browser asks to open it.
        </Notice>
      ) : null}
      <SignInForm providers={providers} returnTo={returnTo} client={client} />
      <p className="m-0 text-center text-[12px] leading-[18px] text-fg-subtle">
        New here? Signing in creates your account. You only need a team to publish. To open a
        collection someone shared, signing in is enough.
      </p>
    </AuthShell>
  );
}
