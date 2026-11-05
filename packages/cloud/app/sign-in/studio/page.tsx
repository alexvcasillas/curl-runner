import type { Metadata } from 'next';
import { AuthShell } from '@/components/AuthShell';
import { api } from '@/lib/api';
import type { Me } from '@/lib/types';
import { StudioHandoff } from './StudioHandoff';

export const metadata: Metadata = { title: 'Back to curl-runner studio' };

/**
 * Where `client=studio` lands after sign-in. Opening the studio is a click (a POST),
 * so a link on another site can't mint a code or launch the app by loading this page.
 */
export default async function StudioPage() {
  const me = await api<Me>('/v1/me');
  return (
    <AuthShell width="md" signedInAs={me.user.email}>
      <StudioHandoff email={me.user.email} />
    </AuthShell>
  );
}
