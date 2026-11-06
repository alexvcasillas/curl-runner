'use client';

import { useState } from 'react';
import type { BannerInput, PlansInput, TestEmailResult } from '@/app/admin/settings/actions';
import type { ActionResult } from '@/lib/action-result';
import { can, type Perm } from '@/lib/admin';
import {
  type BannerAudience,
  type BannerTone,
  type EnvContext,
  envDisplay,
  MAX_BANNER,
  PAUSED_COPY,
  type PlanName,
  type SettingsData,
  SIGNUP_MODES,
  type SignupMode,
} from '@/lib/admin-settings';
import { cx } from '@/lib/cx';
import { Button } from '../Button';
import { ConfirmDialog } from '../ConfirmDialog';
import { Field } from '../Field';
import { LockIcon } from '../icons';
import { Notice } from '../Notice';
import { PageHeader } from '../PageHeader';
import { Pill } from '../Pill';
import { Segmented } from '../Segmented';
import { Select } from '../Select';
import { useToast } from '../Toast';
import { AdminCard, CardHead } from './AdminCard';

export type SettingsActions = {
  signup: (mode: SignupMode, domains: string[]) => Promise<ActionResult>;
  plans: (input: PlansInput) => Promise<ActionResult>;
  banner: (input: BannerInput) => Promise<ActionResult>;
  publishing: (paused: boolean) => Promise<ActionResult>;
  testEmail: () => Promise<TestEmailResult>;
};

const LOGGED = 'Logged as your change.';

/** One section's save: pending while it runs, the API's message if it says no. */
function useSave() {
  const toast = useToast();
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function run(call: () => Promise<ActionResult>, done: string) {
    if (pending) {
      return;
    }
    setPending(true);
    setError(null);
    try {
      const result = await call();
      if (result.ok) {
        toast({ message: `${done} ${LOGGED}` });
      } else {
        setError(result.message);
      }
    } catch {
      setError('That didn’t work. Try again.');
    } finally {
      setPending(false);
    }
  }
  return { pending, error, run };
}

function Switch({
  label,
  on,
  onChange,
  disabled,
}: {
  label: string;
  on: boolean;
  onChange: (next: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!on)}
      className={cx(
        'relative h-5 w-9 shrink-0 cursor-pointer rounded-full border-0 p-0 transition-colors duration-[120ms] disabled:cursor-not-allowed disabled:opacity-50',
        on ? 'bg-success' : 'bg-surface-300',
      )}
    >
      <span
        aria-hidden="true"
        className={cx(
          'absolute top-0.5 size-4 rounded-full bg-fg transition-[left] duration-[120ms]',
          on ? 'left-[18px]' : 'left-0.5',
        )}
      />
    </button>
  );
}

function Foot({
  children,
  onSave,
  pending,
  disabled,
  error,
}: {
  children: React.ReactNode;
  onSave: () => void;
  pending: boolean;
  disabled: boolean;
  error: string | null;
}) {
  return (
    <>
      {error ? (
        <div className="px-4 pb-3">
          <Notice tone="err" role="alert">
            {error}
          </Notice>
        </div>
      ) : null}
      <div className="flex min-h-12 flex-wrap items-center justify-between gap-3 border-t border-hairline bg-raised px-4 py-2 text-[12px] text-fg-muted">
        <span className="min-w-0 flex-1 basis-[240px]">{children}</span>
        <Button size="sm" variant="primary" loading={pending} disabled={disabled} onClick={onSave}>
          Save
        </Button>
      </div>
    </>
  );
}

const SET_IN_ENV = <Pill icon={<LockIcon size={12} />}>Set in the environment</Pill>;

/** Server settings, one card per section, each saved on its own. */
export function SettingsView({
  settings: s,
  env,
  permissions,
  actions,
}: {
  settings: SettingsData;
  env: EnvContext;
  permissions: Perm[];
  actions: SettingsActions;
}) {
  const toast = useToast();
  const canWrite = can(permissions, 'settings');

  // Sign-up
  const [mode, setMode] = useState<SignupMode>(s.signup_mode);
  const [domains, setDomains] = useState<string[]>(s.signup_domains);
  const [draft, setDraft] = useState('');
  const signup = useSave();

  function addDomain(raw: string) {
    const d = raw.trim().replace(/,$/, '').toLowerCase();
    if (d && !domains.includes(d)) {
      setDomains([...domains, d]);
    }
    setDraft('');
    return d ? [...new Set([...domains, d])] : domains;
  }

  // Plans
  const [plan, setPlan] = useState<PlanName>(s.default_plan);
  const [limit, setLimit] = useState(String(s.free_member_limit));
  const [restricted, setRestricted] = useState(s.free_restricted ? 'allowed' : 'not');
  const [grace, setGrace] = useState(String(s.grace_days));
  const plans = useSave();

  // Banner
  const [bannerOn, setBannerOn] = useState(Boolean(s.banner_message));
  const [message, setMessage] = useState(s.banner_message ?? '');
  const [tone, setTone] = useState<BannerTone>(s.banner_tone);
  const [audience, setAudience] = useState<BannerAudience>(s.banner_audience);
  const [link, setLink] = useState(s.banner_link ?? '');
  const banner = useSave();
  const [bannerMissing, setBannerMissing] = useState(false);

  // Pause
  const [paused, setPaused] = useState(s.publishing_paused);
  const [confirmPause, setConfirmPause] = useState(false);
  const pause = useSave();

  const [testing, setTesting] = useState(false);

  async function sendTest() {
    setTesting(true);
    try {
      const r = await actions.testEmail();
      toast(
        r.ok
          ? { message: `Test email sent to ${r.to}. ${LOGGED}` }
          : { tone: 'err', message: r.message },
      );
    } catch {
      toast({ tone: 'err', message: 'That didn’t work. Try again.' });
    } finally {
      setTesting(false);
    }
  }

  return (
    <>
      <PageHeader title="Server settings">
        Changes apply right away and are logged. Secrets and URLs stay in the environment, so
        they’re shown here but can’t be edited.
      </PageHeader>

      {canWrite ? null : (
        <Notice tone="info">Read only: you need the settings permission to change these.</Notice>
      )}

      <AdminCard label="Who can create teams">
        <CardHead
          id="s-signup"
          title="Who can create teams"
          aside={
            <span className="text-[12px] text-fg-subtle">
              Anyone can always sign in to open what’s shared with them.
            </span>
          }
        />
        <div className="flex flex-col gap-3 p-4">
          <div role="radiogroup" aria-labelledby="s-signup" className="grid gap-2.5 sm:grid-cols-3">
            {SIGNUP_MODES.map((m) => {
              const on = mode === m.value;
              return (
                <button
                  key={m.value}
                  type="button"
                  role="radio"
                  aria-checked={on}
                  disabled={!canWrite}
                  onClick={() => setMode(m.value)}
                  className={cx(
                    'flex cursor-pointer items-start gap-2.5 rounded-[8px] border bg-bg p-3 text-left text-fg disabled:cursor-not-allowed',
                    on ? 'border-fg bg-surface-100' : 'border-control hover:bg-surface-100',
                  )}
                >
                  <span
                    aria-hidden="true"
                    className={cx(
                      'mt-0.5 size-3.5 shrink-0 rounded-full border',
                      on ? 'border-[4px] border-fg' : 'border-strong',
                    )}
                  />
                  <span className="flex flex-col gap-0.5">
                    <span className="font-medium">{m.title}</span>
                    <span className="text-[12px] text-fg-muted">{m.desc}</span>
                  </span>
                </button>
              );
            })}
          </div>
          {mode === 'domains' ? (
            <div className="flex flex-col gap-1.5">
              <span className="text-[12px] text-fg-muted">Allowed domains</span>
              <div className="flex min-h-8 flex-wrap items-center gap-1.5 rounded-[6px] border border-control px-1.5 py-1">
                {domains.map((d) => (
                  <span
                    key={d}
                    className="inline-flex h-6 items-center gap-1 rounded-[4px] bg-surface-300 pr-1 pl-2 font-mono text-[12px]"
                  >
                    {d}
                    <button
                      type="button"
                      aria-label={`Remove ${d}`}
                      disabled={!canWrite}
                      onClick={() => setDomains(domains.filter((x) => x !== d))}
                      className="grid size-4 cursor-pointer place-items-center rounded-[3px] border-0 bg-transparent p-0 text-fg-muted hover:text-fg disabled:cursor-not-allowed"
                    >
                      ×
                    </button>
                  </span>
                ))}
                <input
                  aria-label="Add a domain"
                  placeholder="Add a domain"
                  value={draft}
                  disabled={!canWrite}
                  onChange={(e) => setDraft(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ',') {
                      e.preventDefault();
                      addDomain(draft);
                    }
                  }}
                  onBlur={() => draft.trim() && addDomain(draft)}
                  className="h-6 min-w-[120px] flex-1 border-0 bg-transparent px-1 text-[13px] text-fg outline-none placeholder:text-fg-subtle"
                />
              </div>
            </div>
          ) : null}
          <div className="flex flex-col gap-0.5">
            <span className="text-[12px] text-fg-muted">Always allowed: admins</span>
            <span className="text-[12px] text-fg-subtle">
              Everyone on the Admins page can create teams in any mode.
            </span>
          </div>
        </div>
        <Foot
          error={signup.error}
          pending={signup.pending}
          disabled={!canWrite}
          onSave={() => {
            const all = draft.trim() ? addDomain(draft) : domains;
            return signup.run(() => actions.signup(mode, all), 'Sign-up settings saved.');
          }}
        >
          Was <code>SIGNUP</code> and <code>SIGNUP_DOMAINS</code>. The environment values seed this
          the first time the server starts.
        </Foot>
      </AdminCard>

      <AdminCard label="Plans">
        <CardHead id="s-plans" title="Plans" />
        <div className="flex flex-col gap-4 p-4">
          <Select
            label="Plan for new teams"
            value={plan}
            disabled={!canWrite}
            onChange={(e) => setPlan(e.target.value as PlanName)}
            options={[
              { value: 'free', label: 'Free' },
              { value: 'team', label: 'Team' },
            ]}
            className="max-w-[240px]"
          />
          <div className="grid gap-4 sm:grid-cols-3">
            <Field
              label="Free plan: members"
              type="number"
              min={1}
              max={100}
              inputMode="numeric"
              value={limit}
              disabled={!canWrite}
              onChange={(e) => setLimit(e.target.value)}
              hint="Accepted plus pending invites."
            />
            <Select
              label="Free plan: restricted collections"
              value={restricted}
              disabled={!canWrite}
              onChange={(e) => setRestricted(e.target.value)}
              options={[
                { value: 'not', label: 'Not allowed' },
                { value: 'allowed', label: 'Allowed' },
              ]}
            />
            <Field
              label="Grace period after a failed payment"
              type="number"
              min={0}
              max={365}
              inputMode="numeric"
              value={grace}
              disabled={!canWrite}
              onChange={(e) => setGrace(e.target.value)}
              hint="In days."
            />
          </div>
          <div className="flex flex-wrap items-center gap-3 rounded-[8px] border border-hairline px-3 py-2.5">
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="font-medium">Stripe</span>
              <span className="text-[12px] text-fg-muted [overflow-wrap:anywhere]">
                {env.billing ? (
                  <>
                    Billing is on · prices{' '}
                    {s.env
                      .filter((v) => v.key.startsWith('STRIPE_PRICE_') && v.value)
                      .map((v, i) => (
                        <span key={v.key}>
                          {i > 0 ? ', ' : ''}
                          <span className="font-mono">{v.value}</span>
                        </span>
                      ))}
                  </>
                ) : (
                  'Billing is off on this server.'
                )}
              </span>
            </span>
            {SET_IN_ENV}
          </div>
        </div>
        <Foot
          error={plans.error}
          pending={plans.pending}
          disabled={!canWrite}
          onSave={() =>
            plans.run(
              () =>
                actions.plans({
                  defaultPlan: plan,
                  memberLimit: Number(limit),
                  restricted: restricted === 'allowed',
                  graceDays: Number(grace),
                }),
              'Plan settings saved.',
            )
          }
        >
          Raising a limit applies to every Free team at once. Lowering one never removes members or
          collections.
        </Foot>
      </AdminCard>

      <AdminCard label="Announcement banner">
        <CardHead
          id="s-banner"
          title="Announcement banner"
          aside={
            <Switch
              label="Show the banner"
              on={bannerOn}
              disabled={!canWrite}
              onChange={setBannerOn}
            />
          }
        />
        <div className="flex flex-col gap-4 p-4">
          <Field
            label="Message, one sentence"
            value={message}
            maxLength={MAX_BANNER}
            disabled={!canWrite || !bannerOn}
            error={bannerMissing ? 'Write the message to show.' : null}
            onChange={(e) => {
              setMessage(e.target.value);
              setBannerMissing(false);
            }}
          />
          <div className="grid gap-4 sm:grid-cols-3">
            <div className="flex flex-col gap-1.5 text-[12px] text-fg-muted">
              Tone
              <Segmented
                label="Tone"
                value={tone}
                onChange={setTone}
                options={[
                  { value: 'info', label: 'Info' },
                  { value: 'warning', label: 'Warning' },
                ]}
              />
            </div>
            <div className="flex flex-col gap-1.5 text-[12px] text-fg-muted">
              Show to
              <Segmented
                label="Show to"
                value={audience}
                onChange={setAudience}
                options={[
                  { value: 'everyone', label: 'Everyone' },
                  { value: 'signed_in', label: 'Signed-in people' },
                ]}
              />
            </div>
            <Field
              label="Link (optional)"
              type="url"
              mono
              value={link}
              disabled={!canWrite || !bannerOn}
              onChange={(e) => setLink(e.target.value)}
            />
          </div>
        </div>
        <Foot
          error={banner.error}
          pending={banner.pending}
          disabled={!canWrite}
          onSave={() => {
            if (bannerOn && !message.trim()) {
              setBannerMissing(true);
              return;
            }
            return banner.run(
              () =>
                actions.banner({
                  message: bannerOn ? message : '',
                  tone,
                  audience,
                  link: bannerOn ? link : '',
                }),
              bannerOn ? 'Banner saved.' : 'Banner removed.',
            );
          }}
        >
          Shown at the top of every dashboard page. People can dismiss it until the message changes.
        </Foot>
      </AdminCard>

      <AdminCard label="Pause publishing">
        <CardHead
          id="s-pause"
          title="Pause publishing"
          aside={
            <Switch
              label="Pause publishing"
              on={paused}
              disabled={!canWrite || pause.pending}
              onChange={(next) => {
                if (next) {
                  setConfirmPause(true);
                } else {
                  pause.run(async () => {
                    const r = await actions.publishing(false);
                    if (r.ok) {
                      setPaused(false);
                    }
                    return r;
                  }, 'Publishing resumed.');
                }
              }}
            />
          }
        />
        <div className="flex flex-col gap-3 p-4">
          <span className="text-fg-muted">
            For migrations. Publishes and studio syncs get <code>503</code> “{PAUSED_COPY}” Share
            links and pulls keep working.
          </span>
          {paused ? (
            <Notice tone="warn">Publishing is paused right now. Publishes get a 503.</Notice>
          ) : null}
          {pause.error ? (
            <Notice tone="err" role="alert">
              {pause.error}
            </Notice>
          ) : null}
        </div>
      </AdminCard>

      <AdminCard label="From the environment">
        <CardHead id="s-env" title="From the environment" aside={SET_IN_ENV} />
        <div>
          {s.env.map((v) => {
            const shown = envDisplay(v, env);
            return (
              <div
                key={v.key}
                className="flex min-h-11 flex-wrap items-center gap-x-3 gap-y-1 border-b border-hairline px-4 py-1.5"
              >
                <span className="w-[220px] shrink-0 font-mono text-[12px]">{v.key}</span>
                <span
                  className={cx(
                    'min-w-0 flex-1 [overflow-wrap:anywhere]',
                    shown.mono ? 'font-mono text-[12px]' : 'text-fg-muted',
                  )}
                >
                  {shown.text}
                </span>
                {v.key === 'MAIL_FROM' ? (
                  <Button size="sm" loading={testing} disabled={!canWrite} onClick={sendTest}>
                    Send test email to me
                  </Button>
                ) : null}
              </div>
            );
          })}
        </div>
        <div className="flex min-h-10 items-center border-t border-hairline bg-raised px-4 py-1.5 text-[12px] text-fg-muted">
          Change these in Coolify, then redeploy. Secrets are never shown.
        </div>
      </AdminCard>

      <ConfirmDialog
        open={confirmPause}
        onClose={() => setConfirmPause(false)}
        title="Pause publishing?"
        description={`Publishes and studio syncs get 503 “${PAUSED_COPY}” Share links and pulls keep working.`}
        notice={{
          tone: 'warn',
          children: 'CI and studio syncs will fail until you turn this off.',
        }}
        confirmLabel="Pause publishing"
        pendingLabel="Pausing…"
        variant="primary"
        onConfirm={async () => {
          const r = await actions.publishing(true);
          if (!r.ok) {
            throw new Error(r.message);
          }
          setPaused(true);
          setConfirmPause(false);
          toast({ message: `Publishing paused. ${LOGGED}` });
        }}
      />
    </>
  );
}
