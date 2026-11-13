'use client';

import { useState } from 'react';
import { Avatar } from '@/components/Avatar';
import { AccessBadge, PlanBadge, SourceBadge } from '@/components/Badge';
import { Banner } from '@/components/Banner';
import { Button } from '@/components/Button';
import { ButtonLink } from '@/components/ButtonLink';
import { Card, CardBody, CardFooter, CardTitle } from '@/components/Card';
import { Checkbox } from '@/components/Checkbox';
import { ConfirmDialog } from '@/components/ConfirmDialog';
import { CopyField } from '@/components/CopyField';
import { EmptyState } from '@/components/EmptyState';
import { Field } from '@/components/Field';
import { ClockIcon, LockIcon, PeopleIcon } from '@/components/icons';
import { Menu, MenuItem, MenuLabel, MenuSeparator } from '@/components/Menu';
import { Notice } from '@/components/Notice';
import { Pill } from '@/components/Pill';
import { RadioCard } from '@/components/RadioCard';
import { Segmented } from '@/components/Segmented';
import { Select } from '@/components/Select';
import { Shell } from '@/components/Shell';
import { Skeleton } from '@/components/Skeleton';
import { Spinner } from '@/components/Spinner';
import {
  Table,
  TableCell,
  TableHead,
  TableHeader,
  TableLoading,
  TableRow,
  TableRowLink,
} from '@/components/Table';
import { Tabs } from '@/components/Tabs';
import { useToast } from '@/components/Toast';
import type { TeamTab } from '@/lib/routes';
import type { Me } from '@/lib/types';

const ME: Me = {
  user: { id: 'u1', email: 'ada@acme.example', name: 'Ada Lovelace' },
  teams: [
    { slug: 'acme', name: 'Acme', role: 'owner', plan: 'team', suspended: false },
    { slug: 'globex', name: 'Globex Partners', role: 'member', plan: 'free', suspended: false },
  ],
  admin: true,
};

const ROWS = [
  {
    name: 'Payments API',
    slug: 'payments-api',
    source: 'git',
    git: {
      commit: '4f2a9c1aa0',
      ref: 'refs/heads/main',
      repo_url: 'https://github.com/acme/payments',
    },
    access: 'public',
    version: 'v14',
    when: '2 hours ago',
    by: 'Token github-actions',
  },
  {
    name: 'Webhooks',
    slug: 'webhooks',
    source: 'git',
    git: null,
    access: 'restricted',
    version: 'v3',
    when: 'Yesterday',
    by: 'grace@acme.example',
  },
  {
    name: 'Sandbox tour',
    slug: 'sandbox-tour',
    source: 'cloud',
    git: null,
    access: 'public',
    version: 'v27',
    when: '5 minutes ago',
    by: 'ada@acme.example from the studio',
  },
  {
    name: 'Internal admin',
    slug: 'internal-admin',
    source: 'git',
    git: {
      commit: '4f2a9c1aa0',
      ref: 'refs/heads/main',
      repo_url: 'https://github.com/acme/payments',
    },
    access: 'off',
    version: 'v1',
    when: 'Oct 30',
    by: 'Token github-actions',
  },
] as const;

type DialogId = 'delete' | 'revoke' | 'access' | 'member' | 'invite' | 'leave' | 'device' | null;

function Section({
  id,
  title,
  caption,
  children,
}: {
  id: string;
  title: string;
  caption?: string;
  children: React.ReactNode;
}) {
  return (
    <section
      aria-labelledby={id}
      className="flex min-w-0 flex-col gap-3.5 rounded-[12px] border border-hairline bg-bg p-6 max-sm:p-4"
    >
      <h2 id={id} className="m-0 text-[14px] leading-5 font-semibold">
        {title}
      </h2>
      {children}
      {caption ? <p className="m-0 text-[12px] leading-[18px] text-fg-subtle">{caption}</p> : null}
    </section>
  );
}

function Row({ children }: { children: React.ReactNode }) {
  return <div className="flex flex-wrap items-center gap-2.5">{children}</div>;
}

function Who({
  initials,
  title,
  subtitle,
  pending,
  mono,
}: {
  initials: string;
  title: string;
  subtitle: string;
  pending?: boolean;
  mono?: boolean;
}) {
  return (
    <div className="flex items-center gap-2.5 rounded-[6px] border border-hairline bg-bg px-3 py-2.5">
      <Avatar kind={pending ? 'pending' : 'person'} name={initials} size={28} />
      <span className="flex flex-1 flex-col">
        <span className={mono ? 'font-mono text-[12px]' : 'font-medium'}>{title}</span>
        <span className="text-[12px] text-fg-muted">{subtitle}</span>
      </span>
    </div>
  );
}

export function KitDemo() {
  const [billing, setBilling] = useState<'on' | 'off'>('on');
  const [tab, setTab] = useState<TeamTab>('collections');
  const [period, setPeriod] = useState<'annual' | 'monthly'>('annual');
  const [access, setAccess] = useState('public');
  const [dialog, setDialog] = useState<DialogId>(null);
  const toast = useToast();
  const close = () => setDialog(null);
  const wait = () => new Promise<void>((r) => setTimeout(r, 900)).then(close);

  return (
    <Shell
      me={ME}
      info={{ billing: billing === 'on' }}
      team="acme"
      tab={tab}
      banner={
        <Banner
          tone="warn"
          title="Acme is on the Free plan now. Nothing was deleted."
          action={
            <ButtonLink href="#" size="sm">
              Upgrade to Team
            </ButtonLink>
          }
        >
          Webhooks is restricted, so it keeps serving the people already on its list. To publish to
          it or add people, upgrade, or make it public or off.
        </Banner>
      }
    >
      <div className="flex flex-wrap items-center gap-3">
        <div className="mr-auto">
          <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">
            Component kit
          </h1>
          <p className="mt-1 mb-0 text-fg-muted">Every kit component in its states. Dev only.</p>
        </div>
        <Segmented
          label="Billing"
          value={billing}
          onChange={setBilling}
          options={[
            { value: 'on', label: 'Billing on' },
            { value: 'off', label: 'Billing off' },
          ]}
        />
        <Select
          aria-label="Active tab"
          value={tab}
          onChange={(e) => setTab(e.target.value as TeamTab)}
          options={['collections', 'tokens', 'members', 'billing', 'settings'].map((t) => ({
            value: t,
            label: t,
          }))}
        />
      </div>

      <div className="grid grid-cols-[repeat(auto-fill,minmax(min(100%,420px),1fr))] items-start gap-6">
        <Section
          id="k-btn"
          title="Buttons"
          caption="One primary per view. Destructive actions are red outline, and red fill only inside the confirm dialog."
        >
          <Row>
            <Button variant="primary">Create token</Button>
            <Button>Copy link</Button>
            <Button variant="ghost">Docs</Button>
            <Button size="sm">Revoke</Button>
          </Row>
          <Row>
            <Button variant="danger">Reset link</Button>
            <Button variant="danger-fill">Delete collection</Button>
            <Button disabled>Copy link</Button>
            <Button variant="primary" loading>
              Creating team…
            </Button>
            <ButtonLink href="#">A link</ButtonLink>
          </Row>
          <Row>
            <Button size="lg" variant="primary">
              Open in curl-runner studio
            </Button>
          </Row>
        </Section>

        <Section id="k-inp" title="Fields">
          <div className="grid grid-cols-2 gap-3 max-sm:grid-cols-1">
            <Field label="Name" defaultValue="github-actions" />
            <Field
              label="Email"
              type="email"
              placeholder="ada@acme.example"
              hint="We send them an invite."
            />
            <Field label="Error" defaultValue="acme" error="That team slug is already taken." />
            <Select
              label="Role"
              defaultValue="member"
              options={[
                { value: 'member', label: 'Member' },
                { value: 'owner', label: 'Owner' },
              ]}
            />
          </div>
          <Field label="Slug" prefix="cloud.curl-runner.com/t/" mono defaultValue="acme" />
          <Row>
            <Checkbox label="Email them the link" defaultChecked />
            <Checkbox label="Unchecked" />
            <Segmented
              label="Billing period"
              value={period}
              onChange={setPeriod}
              options={[
                { value: 'annual', label: 'Annual' },
                { value: 'monthly', label: 'Monthly' },
              ]}
            />
          </Row>
          <fieldset className="m-0 flex flex-col gap-2 border-0 p-0">
            <legend className="sr-only">Access</legend>
            {[
              ['public', 'Public', 'Anyone with the link. No account needed.'],
              ['restricted', 'Restricted', 'Only people on the list.'],
              ['off', 'Off', 'Nobody outside Acme.'],
            ].map(([v, t, d]) => (
              <RadioCard
                key={v}
                name="access"
                value={v}
                title={t}
                description={d}
                checked={access === v}
                onChange={() => setAccess(v)}
              />
            ))}
          </fieldset>
        </Section>

        <Section
          id="k-pill"
          title="Pills"
          caption="Source, access and plan are neutral facts. Only states that need attention get color."
        >
          <Row>
            <SourceBadge
              source="git"
              git={{ commit: '4f2a9c1aa0', ref: 'refs/heads/main', repo_url: null }}
            />
            <SourceBadge source="git" />
            <SourceBadge source="cloud" />
            <AccessBadge access="public" />
            <AccessBadge access="restricted" />
            <AccessBadge access="off" />
          </Row>
          <Row>
            <PlanBadge plan="free" size="md" />
            <PlanBadge plan="team" size="md" />
            <Pill>Team plan</Pill>
            <Pill tone="info">This browser</Pill>
            <PlanBadge plan="team" />
          </Row>
          <Row>
            <Pill tone="ok">Current</Pill>
            <Pill tone="ok">Active</Pill>
            <Pill tone="ok">Opened Nov 3</Pill>
            <Pill>Invited · not opened</Pill>
            <Pill tone="warn">Expires in 5 days</Pill>
            <Pill tone="err">Past due</Pill>
          </Row>
        </Section>

        <Section id="k-co" title="Callouts">
          <Notice tone="info">Signing in for the curl-runner CLI.</Notice>
          <Notice tone="ok">Invite sent to linus@acme.example.</Notice>
          <Notice tone="warn">The Free plan allows 3 members, counting pending invites.</Notice>
          <Notice tone="err">The old link stops working right away, for everyone.</Notice>
        </Section>

        <Section id="k-card" title="Card with footer">
          <Card aria-labelledby="k-card-t">
            <CardBody className="gap-2">
              <CardTitle as="h3" id="k-card-t">
                Pending invites
              </CardTitle>
              <p className="m-0 text-fg-muted">Invites expire after 7 days.</p>
            </CardBody>
            <CardFooter>
              <span className="flex-1">Footers hold the fine print and one secondary action.</span>
              <Button size="sm">Show older</Button>
            </CardFooter>
          </Card>
        </Section>

        <Section id="k-tabs" title="Tabs">
          <div className="overflow-hidden rounded-[8px] border border-hairline">
            <Tabs
              label="Collection"
              className="border-b border-hairline"
              items={[
                { href: '#versions', label: 'Versions', current: true },
                { href: '#sharing', label: 'Sharing' },
                { href: '#inputs', label: 'Inputs' },
                { href: '#settings', label: 'Settings' },
              ]}
            />
          </div>
        </Section>

        <Section
          id="k-code"
          title="Secrets and commands"
          caption="A secret shown once gets a bright Copy button. Commands get a quiet one."
        >
          <CopyField
            variant="secret"
            label="token"
            value="crt_Hx3nQ8vLp2YtR6wKc9ZbM1aJf5sD7uEg4oNiT0qBVyW"
            onCopy={() => toast({ message: 'Token copied' })}
          />
          <CopyField
            label="command"
            value="curl-runner pull https://cloud.curl-runner.com/s/crsl_V8kq…vEw"
            onCopy={() => toast({ message: 'Link copied' })}
          />
        </Section>

        <Section
          id="k-people"
          title="People and teams"
          caption="Initials, no photos or gradients. People are round, teams are square."
        >
          <Row>
            <Avatar name="Ada Lovelace" />
            <Avatar kind="pending" />
            <Avatar kind="team" name="Acme" />
            <Avatar name="jane@globex.example" size={28} />
            <Avatar kind="team" name="Globex" size={22} active />
            <span className="text-[12px] text-fg-muted">Person · pending invite · team</span>
          </Row>
        </Section>

        <Section id="k-menu" title="Menu">
          <Row>
            <Menu
              label="Row actions"
              trigger="Actions"
              triggerClassName="h-8 cursor-pointer rounded-[6px] border border-control bg-bg px-3 text-[13px] text-fg"
            >
              <MenuLabel>Version</MenuLabel>
              <MenuItem onSelect={() => toast({ message: 'Changes opened' })}>Changes</MenuItem>
              <MenuItem checked>Current</MenuItem>
              <MenuItem checked={false}>Pinned</MenuItem>
              <MenuSeparator />
              <MenuItem href="#" external>
                Docs
              </MenuItem>
            </Menu>
          </Row>
        </Section>

        <Section
          id="k-load"
          title="Loading and toast"
          caption="Skeletons only for lists. Toasts confirm quiet actions, bottom right, for 4 seconds."
        >
          <div className="flex flex-col gap-2">
            <Skeleton width="60%" />
            <Skeleton width="40%" height={8} />
          </div>
          <Row>
            <Spinner />
            <Spinner size={20} />
            <Button onClick={() => toast({ message: 'Link copied' })}>Show a toast</Button>
            <Button
              onClick={() =>
                toast({
                  tone: 'err',
                  message: 'Couldn’t save the access mode. Try again.',
                  action: { label: 'Retry', onClick: () => {} },
                })
              }
            >
              Show an error toast
            </Button>
          </Row>
        </Section>

        <Section
          id="k-dialogs"
          title="Dialogs"
          caption="Every dialog names the thing it changes. The safe choice comes first."
        >
          <Row>
            <Button variant="danger" onClick={() => setDialog('delete')}>
              Delete collection
            </Button>
            <Button onClick={() => setDialog('revoke')}>Revoke token</Button>
            <Button onClick={() => setDialog('access')}>Remove access</Button>
            <Button onClick={() => setDialog('member')}>Remove member</Button>
            <Button onClick={() => setDialog('invite')}>Cancel invite</Button>
            <Button onClick={() => setDialog('leave')}>Leave team</Button>
            <Button onClick={() => setDialog('device')}>Sign out device</Button>
          </Row>
        </Section>
      </div>

      <h2 className="m-0 mt-4 text-[16px] leading-6 font-semibold">Banners</h2>
      <Banner
        tone="err"
        title="We couldn’t take Acme’s last payment."
        action={<Button size="sm">Update card</Button>}
      >
        Update the card by Nov 19 to keep the Team plan. After that, Acme moves to Free and nothing
        is deleted.
      </Banner>
      <Banner
        tone="info"
        title="curl-runner Cloud is in private preview."
        action={
          <ButtonLink href="#" size="sm">
            Read the announcement
          </ButtonLink>
        }
      >
        You can open anything shared with you. Creating teams opens at launch.
      </Banner>
      <Banner tone="warn" title="Too many tries. Try again in 12 minutes." />

      <h2 className="m-0 mt-4 text-[16px] leading-6 font-semibold">Table</h2>
      <Table
        label="Collections"
        minWidth={820}
        footer={
          <>
            <span className="flex-1">4 collections</span>
            <span>Team plan</span>
          </>
        }
      >
        <TableHeader>
          <TableHead>Collection</TableHead>
          <TableHead width={112}>Source</TableHead>
          <TableHead width={120}>Access</TableHead>
          <TableHead width={72}>Version</TableHead>
          <TableHead width={220}>Last publish</TableHead>
        </TableHeader>
        {ROWS.map((r) => (
          <TableRow key={r.slug}>
            <TableCell className="flex flex-col">
              <TableRowLink href={`/t/acme/c/${r.slug}`}>{r.name}</TableRowLink>
              <span className="font-mono text-[12px] text-fg-subtle">{r.slug}</span>
            </TableCell>
            <TableCell width={112}>
              <SourceBadge source={r.source} git={r.git} />
            </TableCell>
            <TableCell width={120}>
              <AccessBadge access={r.access} />
            </TableCell>
            <TableCell width={72} className="font-mono text-[12px]">
              {r.version}
            </TableCell>
            <TableCell width={220} className="flex flex-col text-[12px] leading-4">
              <span>{r.when}</span>
              <span className="text-fg-subtle">{r.by}</span>
            </TableCell>
          </TableRow>
        ))}
      </Table>
      <Table label="Loading collections" minWidth={480}>
        <TableLoading rows={3} />
      </Table>

      <h2 className="m-0 mt-4 text-[16px] leading-6 font-semibold">Empty states and errors</h2>
      <div className="grid grid-cols-[repeat(auto-fill,minmax(min(100%,400px),1fr))] items-start gap-6">
        <EmptyState icon={<LockIcon size={18} />} title="Nobody on the list yet" as="h3">
          Only Acme’s members can open Payments API right now. Add an email or a domain above.
        </EmptyState>
        <EmptyState icon={<PeopleIcon size={18} />} title="It’s just you in Acme" as="h3">
          Invite the people who publish or share with you. Free covers 3 members.
        </EmptyState>
        <EmptyState icon={<ClockIcon size={18} />} title="No versions yet" as="h3">
          The first sync from the studio, or the first CI publish, creates version 1.
        </EmptyState>
        <EmptyState
          code="404 Not Found"
          title="Nothing here"
          as="h3"
          actions={<ButtonLink href="#">Go to your collections</ButtonLink>}
        >
          It doesn’t exist, or it belongs to a team you’re not in.
        </EmptyState>
      </div>

      <ConfirmDialog
        open={dialog === 'delete'}
        onClose={close}
        onConfirm={wait}
        title="Delete Payments API"
        description="This deletes all 14 versions, the share link and the access list. You can’t undo it."
        notice={{
          tone: 'err',
          children:
            '4 people on the access list stop getting updates. Their local copies stay on their machines. The next CI publish creates it again, as a new collection with a new link.',
        }}
        confirmLabel="Delete collection"
        pendingLabel="Deleting…"
        confirmText="payments-api"
      />
      <ConfirmDialog
        open={dialog === 'revoke'}
        onClose={close}
        onConfirm={wait}
        title="Revoke github-actions?"
        description="The next publish that uses it fails with “Sign in or use a valid token.”"
        notice={{
          tone: 'warn',
          children: (
            <>
              Last used 2 hours ago. Create a new token and update <code>CURL_RUNNER_TOKEN</code>{' '}
              first, so CI keeps publishing.
            </>
          ),
        }}
        confirmLabel="Revoke token"
        pendingLabel="Revoking…"
      />
      <ConfirmDialog
        open={dialog === 'access'}
        onClose={close}
        onConfirm={wait}
        title="Remove jane@globex.example?"
        description="They can’t open Payments API or get new versions after this."
        confirmLabel="Remove access"
      >
        <Who
          initials="jane@globex.example"
          title="jane@globex.example"
          subtitle="Opened Nov 3"
          mono
        />
        <p className="m-0 text-[12px] text-fg-subtle">
          Their local copy stays on their machine. If <code>*@globex.example</code> is on the list
          too, they still have access through it.
        </p>
      </ConfirmDialog>
      <ConfirmDialog
        open={dialog === 'member'}
        onClose={close}
        onConfirm={wait}
        title="Remove Linus Torvalds from Acme?"
        description="They lose access to Acme’s collections and tokens straight away."
        confirmLabel="Remove member"
      >
        <Who
          initials="Linus Torvalds"
          title="Linus Torvalds"
          subtitle="linus@acme.example · Member"
        />
        <p className="m-0 text-[12px] text-fg-subtle">
          Tokens they created keep working. Revoke them under Tokens if you need to. On the Team
          plan, the next invoice drops one seat.
        </p>
      </ConfirmDialog>
      <ConfirmDialog
        open={dialog === 'invite'}
        onClose={close}
        onConfirm={wait}
        title="Cancel the invite to dana@acme.example?"
        description="The link in their email stops working. You can invite them again later."
        confirmLabel="Cancel invite"
        cancelLabel="Keep invite"
      >
        <Who
          initials=""
          title="dana@acme.example"
          subtitle="Member · expires in 5 days"
          pending
          mono
        />
      </ConfirmDialog>
      <ConfirmDialog
        open={dialog === 'leave'}
        onClose={close}
        onConfirm={wait}
        title="Leave Globex Partners?"
        description="You lose access to its collections. An owner can invite you again."
        confirmLabel="Leave team"
      >
        <p className="m-0 text-[12px] text-fg-subtle">
          Collections shared with ada@acme.example still open for you, from Shared with me.
        </p>
      </ConfirmDialog>
      <ConfirmDialog
        open={dialog === 'device'}
        onClose={close}
        onConfirm={wait}
        title="Sign out CLI on ada-mbp?"
        description={
          <>
            Its next request fails. Run <code>curl-runner login</code> on that machine to sign in
            again.
          </>
        }
        confirmLabel="Sign out device"
      />
    </Shell>
  );
}
