import { redirect } from 'next/navigation';
import { api } from '@/lib/api';
import { getMe } from '@/lib/app-data';
import { routes } from '@/lib/routes';

/** Sends the person to their first team, else what's shared with them, else the new team page. */
export default async function Home() {
  const me = await getMe();
  if (me.teams.length > 0) {
    redirect(routes.team(me.teams[0].slug));
  }
  const shared = await api<unknown[]>('/v1/me/shared');
  redirect(shared.length > 0 ? routes.shared : routes.newTeam);
}
