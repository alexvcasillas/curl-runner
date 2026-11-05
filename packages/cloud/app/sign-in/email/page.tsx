import type { Metadata } from 'next';
import { AuthMessage } from '@/components/AuthMessage';
import { AuthShell } from '@/components/AuthShell';
import { first } from '@/lib/sign-in';
import { AutoRun } from '../AutoRun';
import { verifyLink } from '../actions';

export const metadata: Metadata = { title: 'Signing you in' };

/** The emailed link: `/sign-in/email?token=crl_…`. */
export default async function EmailLinkPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const sp = await searchParams;
  const token = first(sp.token)?.trim();
  if (!token) {
    return (
      <AuthShell width="md">
        <AuthMessage
          title="This link has expired"
          body="Sign-in links work for 10 minutes, and only the newest one works. Ask for a new code and use that email."
          actionLabel="Send a new code"
          actionHref="/sign-in"
        />
      </AuthShell>
    );
  }
  return (
    <AuthShell width="md">
      <AutoRun title="Signing you in" run={verifyLink.bind(null, token, first(sp.return_to))} />
    </AuthShell>
  );
}
