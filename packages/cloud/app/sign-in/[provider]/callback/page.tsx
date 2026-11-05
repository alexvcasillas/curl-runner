import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { AuthMessage } from '@/components/AuthMessage';
import { AuthShell } from '@/components/AuthShell';
import { first, isProvider, PROVIDER_NAMES } from '@/lib/sign-in';
import { AutoRun } from '../../AutoRun';
import { finishOAuth } from '../../actions';

export const metadata: Metadata = { title: 'Signing you in' };

/** Where GitHub and Google send people back: `/sign-in/{provider}/callback?code&state`. */
export default async function OAuthCallbackPage({
  params,
  searchParams,
}: {
  params: Promise<{ provider: string }>;
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const { provider } = await params;
  if (!isProvider(provider)) {
    notFound();
  }
  const sp = await searchParams;
  const name = PROVIDER_NAMES[provider];
  // The person said no at the provider, so there is nothing to finish.
  if (first(sp.error)) {
    return (
      <AuthShell width="md">
        <AuthMessage
          title="Sign-in was cancelled"
          body={`You didn’t allow access on ${name}. Start again, or sign in with an email code.`}
          actionLabel="Back to sign in"
          actionHref="/sign-in"
        />
      </AuthShell>
    );
  }
  return (
    <AuthShell width="md">
      <AutoRun
        title={`Finishing sign-in with ${name}`}
        run={finishOAuth.bind(null, provider, first(sp.code), first(sp.state))}
      />
    </AuthShell>
  );
}
