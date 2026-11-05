'use client';

import { useState, useTransition } from 'react';
import { Button } from '@/components/Button';
import { ButtonLink } from '@/components/ButtonLink';
import { Card, CardBody, CardFooter } from '@/components/Card';
import { Field } from '@/components/Field';
import { Notice } from '@/components/Notice';
import { routes } from '@/lib/routes';
import { isValidSlug, slugFromName } from '@/lib/slug';
import type { CreateTeamResult } from './actions';

export type NewTeamFormProps = {
  email: string;
  /** Plans exist only on a server with billing. */
  billing: boolean;
  /** No teams yet: the form is followed by the three steps. */
  first?: boolean;
  action: (formData: FormData) => Promise<CreateTeamResult>;
};

const STEPS = [
  ['Create the team', 'You become its owner.'],
  ['Make a publish token', 'Add it to your CI secrets.'],
  ['Push to main', 'CI publishes version 1.'],
] as const;

const TAKEN = 'That team slug is already taken. Try acme-api or acme-eng.';
const BAD_SLUG = 'Use lower case letters, numbers and dashes, and start with a letter or number.';

/** Name and slug. The slug follows the name until someone edits it. */
export function NewTeamForm({ email, billing, first = false, action }: NewTeamFormProps) {
  const [name, setName] = useState('');
  const [slug, setSlug] = useState('');
  const [edited, setEdited] = useState(false);
  const [slugError, setSlugError] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [closed, setClosed] = useState(false);
  const [pending, run] = useTransition();

  function submit(e: React.FormEvent) {
    e.preventDefault();
    setProblem(null);
    if (!name.trim()) {
      setProblem('Enter a team name.');
      return;
    }
    if (!isValidSlug(slug)) {
      setSlugError(BAD_SLUG);
      return;
    }
    const f = new FormData();
    f.set('name', name);
    f.set('slug', slug);
    run(async () => {
      // A success redirects, so only a problem comes back.
      const r = await action(f);
      if (!r) {
        return;
      }
      if (r.kind === 'closed') {
        setClosed(true);
      } else if (r.kind === 'taken') {
        setSlugError(TAKEN);
      } else if (r.kind === 'invalid') {
        setSlugError(r.message);
      } else {
        setProblem(r.message);
      }
    });
  }

  if (closed) {
    return (
      <Card aria-label="Sign-up closed">
        <CardBody className="gap-3">
          <Notice tone="warn" role="alert">
            Creating teams is closed on this server. You can still open collections shared with you,
            and join a team when someone invites you.
          </Notice>
        </CardBody>
        <CardFooter>
          <span className="min-w-0 flex-1 break-words">Signed in as {email}</span>
          <ButtonLink href={routes.shared}>Go to shared with me</ButtonLink>
        </CardFooter>
      </Card>
    );
  }

  return (
    <>
      <Card>
        <form onSubmit={submit} noValidate>
          <CardBody className="gap-5 py-6">
            {problem ? (
              <Notice tone="err" role="alert">
                {problem}
              </Notice>
            ) : null}
            <Field
              label="Team name"
              name="name"
              placeholder="Acme"
              autoComplete="organization"
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                if (!edited) {
                  setSlug(slugFromName(e.target.value));
                  setSlugError(null);
                }
              }}
            />
            <Field
              label="Slug"
              name="slug"
              prefix="cloud.curl-runner.com/t/"
              mono
              placeholder="acme"
              autoComplete="off"
              spellCheck={false}
              value={slug}
              error={slugError}
              onChange={(e) => {
                setSlug(e.target.value);
                setEdited(true);
                setSlugError(null);
              }}
              hint={
                <>
                  Lower case letters, numbers and dashes. CI publishes with it, as{' '}
                  <span className="font-mono text-fg-muted">team: {slug || 'acme'}</span> in the
                  GitHub Action, so pick one you’ll keep.
                </>
              }
            />
          </CardBody>
          <CardFooter>
            <span className="min-w-0 flex-1">
              {billing
                ? 'Starts on the Free plan: 3 members, unlimited public collections.'
                : 'Self-hosted: every feature is on, no plans or limits.'}
            </span>
            <Button type="submit" variant="primary" loading={pending}>
              {pending ? 'Creating team…' : 'Create team'}
            </Button>
          </CardFooter>
        </form>
      </Card>
      {first ? (
        <ol className="m-0 grid list-none grid-cols-[repeat(auto-fit,minmax(min(100%,150px),1fr))] gap-3 p-0">
          {STEPS.map(([title, body], i) => (
            <li
              key={title}
              className="flex flex-col gap-1 rounded-[8px] border border-hairline p-3.5"
            >
              <span className="font-mono text-[11px] text-fg-subtle">{i + 1}</span>
              <span className="font-medium">{title}</span>
              <span className="text-[12px] leading-[18px] text-fg-muted">{body}</span>
            </li>
          ))}
        </ol>
      ) : null}
    </>
  );
}
