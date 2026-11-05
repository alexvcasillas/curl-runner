'use client';

import Link from 'next/link';
import { useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import { initials, teamLetter } from '@/lib/initials';
import { routes } from '@/lib/routes';
import type { MeTeam } from '@/lib/types';
import { Button } from './Button';
import { ButtonLink } from './ButtonLink';
import { Card, CardBody, CardFooter, CardTitle } from './Card';
import { ConfirmDialog } from './ConfirmDialog';
import { LeaveTeamDialog } from './LeaveTeamDialog';
import { Pill } from './Pill';
import { GitHubMark, GoogleMark } from './provider-marks';
import { useToast } from './Toast';

export type ProviderRow = {
  provider: 'email' | 'github' | 'google' | string;
  /** The line under the name: the address, or when it was linked. */
  detail: string;
  linked: boolean;
};

export type SessionRow = {
  id: string;
  label: string;
  kind: 'web' | 'device' | string;
  /** "Last used 2 hours ago · expires Feb 3, 2027". */
  meta: string;
  current: boolean;
};

const NAV = [
  { href: '#profile', label: 'Profile' },
  { href: '#signin', label: 'Sign-in methods' },
  { href: '#devices', label: 'Devices' },
  { href: '#teams', label: 'Teams' },
];

const PROVIDER_NAME: Record<string, string> = {
  email: 'Email code',
  github: 'GitHub',
  google: 'Google',
};

function EmailMark() {
  return (
    <svg
      aria-hidden="true"
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
    >
      <rect x="2" y="3.5" width="12" height="9" rx="1.5" />
      <path d="M2.5 4.5L8 8.5l5.5-4" />
    </svg>
  );
}

function DeviceIcon({ kind, label }: { kind: string; label: string }) {
  const d =
    kind === 'web'
      ? 'M2 3.5h12v8H2zM5.5 14h5M8 11.5V14'
      : /studio/i.test(label)
        ? 'M2.5 2.5h11v11h-11zM2.5 5.5h11M6 5.5v8'
        : 'M3 4.5l3.5 3.5L3 11.5M8.5 11.5H13';
  return (
    <svg
      aria-hidden="true"
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d={d} />
    </svg>
  );
}

function Row({
  icon,
  children,
  action,
}: {
  icon: React.ReactNode;
  children: React.ReactNode;
  action?: React.ReactNode;
}) {
  return (
    <div className="box-border flex min-h-14 flex-wrap items-center gap-3 border-t border-hairline px-6 py-2.5 max-sm:px-4">
      {icon}
      <span className="flex min-w-[180px] flex-1 flex-col">{children}</span>
      {action}
    </div>
  );
}

const iconBox =
  'grid size-8 shrink-0 place-items-center rounded-[8px] border border-control text-fg-muted';

/** Profile, sign-in methods, devices and teams, as CloudAccount draws them. */
export function AccountView({
  email,
  name,
  providers,
  sessions,
  teams,
  showPlan,
  onRevoke,
  onLeave,
}: {
  email: string;
  name: string | null;
  providers: ProviderRow[];
  sessions: SessionRow[];
  teams: MeTeam[];
  /** Billing is on, so the plan is worth naming. */
  showPlan: boolean;
  onRevoke: (id: string) => Promise<ActionResult>;
  onLeave: (team: string) => Promise<ActionResult>;
}) {
  const toast = useToast();
  const [gone, setGone] = useState<string[]>([]);
  const [revoking, setRevoking] = useState<SessionRow | null>(null);
  const [leaving, setLeaving] = useState<MeTeam | null>(null);
  const shown = sessions.filter((s) => !gone.includes(s.id));

  return (
    <div className="flex flex-wrap items-start gap-8">
      <nav
        aria-label="Account"
        className="flex max-w-[220px] flex-[1_1_180px] flex-col gap-0.5 max-sm:max-w-none max-sm:flex-row max-sm:flex-wrap"
      >
        {NAV.map((n, i) => (
          <a
            key={n.href}
            href={n.href}
            aria-current={i === 0 ? 'true' : undefined}
            className={`flex h-8 items-center rounded-[6px] px-2.5 text-[13px] no-underline hover:bg-surface-200 hover:text-fg ${i === 0 ? 'bg-surface-300 text-fg' : 'text-fg-muted'}`}
          >
            {n.label}
          </a>
        ))}
      </nav>

      <div className="flex min-w-0 flex-[999_1_560px] flex-col gap-6">
        <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">Account</h1>

        <Card id="profile" aria-labelledby="h-profile">
          <CardBody className="flex-row flex-wrap items-center gap-4">
            <span
              aria-hidden="true"
              className="inline-grid size-12 place-items-center rounded-full bg-surface-300 text-[16px] font-medium text-fg-muted"
            >
              {initials(name || email)}
            </span>
            <div className="flex min-w-[200px] flex-1 flex-col">
              <CardTitle id="h-profile">{name || email}</CardTitle>
              {name ? <span className="text-fg-muted">{email}</span> : null}
            </div>
          </CardBody>
          <CardFooter>
            Your name comes from GitHub or Google the first time you sign in with them. Collections
            shared with {email}, or with *@{email.split('@')[1]}, open for you.
          </CardFooter>
        </Card>

        <Card id="signin" aria-labelledby="h-signin">
          <CardBody className="gap-1.5">
            <CardTitle id="h-signin">Sign-in methods</CardTitle>
            <p className="m-0 text-fg-muted">
              Every method signs in to the same account, as long as the email matches.
            </p>
          </CardBody>
          {providers.map((p) => (
            <Row
              key={p.provider}
              icon={
                <span className={iconBox}>
                  {p.provider === 'github' ? (
                    <GitHubMark />
                  ) : p.provider === 'google' ? (
                    <GoogleMark />
                  ) : (
                    <EmailMark />
                  )}
                </span>
              }
              action={
                p.provider === 'email' ? (
                  <Pill tone="ok">Always on</Pill>
                ) : p.linked ? (
                  <Pill tone="ok">Linked</Pill>
                ) : (
                  <ButtonLink href={routes.signIn} size="sm">
                    Sign in with {PROVIDER_NAME[p.provider] ?? p.provider}
                  </ButtonLink>
                )
              }
            >
              <span className="font-medium">{PROVIDER_NAME[p.provider] ?? p.provider}</span>
              <span className="text-[12px] text-fg-muted">{p.detail}</span>
            </Row>
          ))}
        </Card>

        <Card id="devices" aria-labelledby="h-devices">
          <CardBody className="gap-1.5">
            <CardTitle id="h-devices">Devices</CardTitle>
            <p className="m-0 text-fg-muted">
              Browsers, the CLI and the studio stay signed in for 90 days after their last use.
              Revoke anything you don’t recognise.
            </p>
          </CardBody>
          {shown.map((s) => (
            <Row
              key={s.id}
              icon={
                <span className={iconBox}>
                  <DeviceIcon kind={s.kind} label={s.label} />
                </span>
              }
              action={
                <Button
                  size="sm"
                  aria-label={`${s.current ? 'Sign out' : 'Revoke'} ${s.label}`}
                  onClick={() => setRevoking(s)}
                >
                  {s.current ? 'Sign out' : 'Revoke'}
                </Button>
              }
            >
              <span className="flex flex-wrap items-center gap-2 font-medium [overflow-wrap:anywhere]">
                {s.label}
                {s.current ? (
                  <Pill tone="info" size="sm">
                    This browser
                  </Pill>
                ) : null}
              </span>
              <span className="text-[12px] text-fg-muted">{s.meta}</span>
            </Row>
          ))}
          <CardFooter>
            Revoking signs that device out on its next request. Publish tokens for CI live under
            each team’s Tokens.
          </CardFooter>
        </Card>

        <Card id="teams" aria-labelledby="h-teams">
          <CardBody>
            <CardTitle id="h-teams">Teams</CardTitle>
          </CardBody>
          {teams.length === 0 ? (
            <Row icon={null}>
              <span className="text-fg-muted">You aren’t on a team yet.</span>
            </Row>
          ) : null}
          {teams.map((t) => (
            <Row
              key={t.slug}
              icon={
                <span
                  aria-hidden="true"
                  className="grid size-8 place-items-center rounded-[8px] bg-surface-300 font-semibold"
                >
                  {teamLetter(t.name)}
                </span>
              }
              action={
                t.role === 'member' ? (
                  <Button size="sm" aria-label={`Leave ${t.name}`} onClick={() => setLeaving(t)}>
                    Leave team
                  </Button>
                ) : null
              }
            >
              <Link href={routes.team(t.slug)} className="font-medium text-fg hover:text-fg">
                {t.name}
              </Link>
              <span className="text-[12px] text-fg-muted">
                {t.role === 'owner' ? 'Owner' : 'Member'}
                {showPlan ? ` · ${t.plan === 'team' ? 'Team' : 'Free'} plan` : ''}
              </span>
            </Row>
          ))}
          <CardFooter>
            <span className="min-w-0 flex-1">
              To delete your account, leave or hand over your teams first, then email support.
            </span>
            <ButtonLink href={routes.newTeam} size="sm">
              New team
            </ButtonLink>
          </CardFooter>
        </Card>

        <form action={routes.signOut} method="post" className="flex justify-end">
          <Button type="submit">Sign out</Button>
        </form>
      </div>

      <ConfirmDialog
        open={revoking !== null}
        onClose={() => setRevoking(null)}
        title={
          revoking?.current ? 'Sign out of this browser?' : `Sign out ${revoking?.label ?? ''}?`
        }
        description={
          revoking?.current ? (
            'You’ll land on the sign-in page.'
          ) : revoking?.kind === 'device' && /cli/i.test(revoking.label) ? (
            <>
              Its next request fails. Run <code>curl-runner login</code> on that machine to sign in
              again.
            </>
          ) : (
            'Its next request fails. Sign in there again to use it.'
          )
        }
        confirmLabel={revoking?.current ? 'Sign out' : 'Sign out device'}
        pendingLabel="Signing out…"
        onConfirm={async () => {
          if (!revoking) {
            return;
          }
          const result = await onRevoke(revoking.id);
          if (!result.ok) {
            throw new Error(result.message);
          }
          // Revoking this browser redirects from the action, so this is another device.
          setGone((g) => [...g, revoking.id]);
          toast({ message: `${revoking.label} is signed out.` });
          setRevoking(null);
        }}
      />

      {leaving ? (
        <LeaveTeamDialog
          open
          onClose={() => setLeaving(null)}
          teamName={leaving.name}
          email={email}
          soleOwner={false}
          onLeave={() => onLeave(leaving.slug)}
        />
      ) : null}
    </div>
  );
}
