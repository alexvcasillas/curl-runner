import type { Metadata } from 'next';
import { Notice } from '@/components/Notice';
import { getInfo, getMe } from '@/lib/app-data';
import { env } from '@/lib/env';
import { createTeam } from './actions';
import { NewTeamForm } from './NewTeamForm';

export const metadata: Metadata = { title: 'New team' };

export default async function NewTeamPage() {
  const [me, info] = await Promise.all([getMe(), getInfo()]);
  const first = me.teams.length === 0;
  return (
    <div className="mx-auto flex w-full max-w-[520px] flex-col gap-6 pt-12 max-sm:pt-0">
      <div className="flex flex-col gap-1.5">
        <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">
          {first ? 'Create your first team' : 'Create a team'}
        </h1>
        <p className="m-0 text-fg-muted">
          A team owns collections, publish tokens and the people who can change them. Customers you
          share with never join it.
        </p>
      </div>
      {me.admin ? (
        <Notice tone="info" role="status" className="px-3.5 py-3 text-[13px] leading-5">
          <span className="font-medium">
            You’re the first account on this server, so you’re its admin.
          </span>{' '}
          <span className="text-fg-muted">
            You can open the admin panel from the account menu. To let colleagues create teams too,
            add your domain in Admin, Settings.
          </span>
        </Notice>
      ) : null}
      <NewTeamForm
        email={me.user.email}
        billing={info.billing}
        slugHost={new URL(env().PUBLIC_APP_URL).host}
        first={first}
        action={createTeam}
      />
    </div>
  );
}
