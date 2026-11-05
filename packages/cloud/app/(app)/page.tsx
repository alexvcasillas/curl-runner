import { redirect } from 'next/navigation';
import { getShared } from '@/lib/account-data';
import { getMe } from '@/lib/app-data';
import { ApiError } from '@/lib/errors';
import { routes } from '@/lib/routes';

/** Sends the person to their first team, else what's shared with them, else the new team page. */
export default async function Home() {
  const me = await getMe();
  if (me.teams.length > 0) {
    redirect(routes.team(me.teams[0].slug));
  }
  // A failed lookup must not break `/`: treat it as nothing shared. Only an
  // `ApiError` is swallowed: the 401 sign-in redirect is a thrown `redirect()`.
  let shared: readonly unknown[] = [];
  try {
    shared = await getShared();
  } catch (e) {
    if (!(e instanceof ApiError)) {
      throw e;
    }
  }
  redirect(shared.length > 0 ? routes.shared : routes.newTeam);
}
