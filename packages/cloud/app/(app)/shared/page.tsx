import type { Metadata } from 'next';
import { SharedView } from '@/components/SharedView';
import { getShared } from '@/lib/account-data';
import { getMe } from '@/lib/app-data';
import { shareToken, studioLink } from '@/lib/deep-link';
import { env } from '@/lib/env';
import { relativeTimeInline } from '@/lib/time';

export const metadata: Metadata = { title: 'Shared with me' };

export default async function SharedPage() {
  const [me, shared] = await Promise.all([getMe(), getShared()]);
  const now = new Date();
  const api = env().PUBLIC_API_URL;

  return (
    <SharedView
      email={me.user.email}
      items={shared.map((c) => ({
        key: `${c.team_slug}/${c.collection}`,
        name: c.name,
        team: c.team,
        access: c.access,
        version: c.version,
        updated: relativeTimeInline(c.updated_at, now),
        command: `curl-runner pull ${c.link}`,
        studioHref: studioLink(api, c.team_slug, c.collection, shareToken(c.link)),
      }))}
    />
  );
}
