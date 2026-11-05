import type { Metadata } from 'next';
import { InviteDead } from '@/components/InviteDead';

export const metadata: Metadata = { title: 'Invite not found' };

/** An expired or used invite, or a token that can't be one. Next sends it with a 404. */
export default function InviteNotFound() {
  return <InviteDead />;
}
