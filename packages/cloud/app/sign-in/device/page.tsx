import type { Metadata } from 'next';
import { AuthShell } from '@/components/AuthShell';
import { api } from '@/lib/api';
import { env } from '@/lib/env';
import type { Me } from '@/lib/types';
import { DeviceHandoff } from './DeviceHandoff';

export const metadata: Metadata = { title: 'Sign in the CLI' };

/**
 * Where `client=device` lands after sign-in. Showing the code is a click (a POST),
 * so a link on another site can't mint one by loading this page.
 */
export default async function DevicePage() {
  const me = await api<Me>('/v1/me');
  return (
    <AuthShell width="md" signedInAs={me.user.email}>
      <DeviceHandoff email={me.user.email} appUrl={env().PUBLIC_APP_URL} />
    </AuthShell>
  );
}
