import { notFound } from 'next/navigation';
import { getMe } from '@/lib/app-data';

// A placeholder: the collections list replaces it.
export default async function TeamPage({ params }: { params: Promise<{ team: string }> }) {
  const { team } = await params;
  const me = await getMe();
  const current = me.teams.find((t) => t.slug === team);
  if (!current) {
    notFound();
  }
  return <h1 className="m-0 text-[24px] leading-8 font-semibold">{current.name}</h1>;
}
