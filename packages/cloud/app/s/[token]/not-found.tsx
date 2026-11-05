import type { Metadata } from 'next';
import { DeadLink } from '@/components/DeadLink';

export const metadata: Metadata = { title: 'Link not found' };

/** A reset, expired or off share link, or a token that can't be one. Next sends it with a 404. */
export default function ShareNotFound() {
  return <DeadLink />;
}
