import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { KitDemo } from './KitDemo';

export const metadata: Metadata = { title: 'Component kit' };

/** Every kit component in its states, for checking against the boards. Never in production. */
export default function KitPage() {
  if (process.env.NODE_ENV === 'production') {
    notFound();
  }
  return <KitDemo />;
}
