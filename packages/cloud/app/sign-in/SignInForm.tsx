'use client';

import { useState, useTransition } from 'react';
import { Button } from '@/components/Button';
import { Notice } from '@/components/Notice';
import { GitHubMark, GoogleMark } from '@/components/provider-marks';
import { waitFor } from '@/lib/errors';
import { PROVIDER_NAMES, type Provider, type SignInClient } from '@/lib/sign-in';
import { startEmail, startOAuth, verifyCode } from './actions';
import { CodeInput } from './CodeInput';

export type SignInFormProps = {
  /** From `/v1/info.sign_in`, without `email`. */
  providers: Provider[];
  /** A local path, or `/`. Already safe. */
  returnTo: string;
  client: SignInClient;
};

type Step = 'email' | 'code';
type CodeProblem = { kind: 'wrong' | 'expired' | 'throttled' | 'error'; message: string };

const LARGE = 'h-10! w-full text-[14px]!';
const INPUT =
  'box-border h-10 w-full min-w-0 rounded-[6px] border bg-bg px-3 text-[14px] text-fg outline-none placeholder:text-fg-subtle focus-visible:border-blue';

function throttledCopy(retryAfter: number | undefined, providers: Provider[]): string {
  const wait = retryAfter === undefined ? 'a few minutes' : waitFor(retryAfter);
  const other = providers.length
    ? `, or use ${providers.map((p) => PROVIDER_NAMES[p]).join(' or ')}`
    : '';
  return `Too many sign-in emails for this address. Try again in ${wait}${other}.`;
}

/** Email code, then the code step, with GitHub and Google above when the server has them. */
export function SignInForm({ providers, returnTo, client }: SignInFormProps) {
  const [step, setStep] = useState<Step>('email');
  const [email, setEmail] = useState('');
  const [code, setCode] = useState('');
  const [emailProblem, setEmailProblem] = useState<{
    kind: 'throttled' | 'error';
    message: string;
  } | null>(null);
  const [codeProblem, setCodeProblem] = useState<CodeProblem | null>(null);
  const [oauthProblem, setOauthProblem] = useState<string | null>(null);
  const [resent, setResent] = useState(false);
  const [pending, run] = useTransition();
  const [busy, setBusy] = useState<'send' | 'verify' | 'oauth' | null>(null);

  function emailData(to: string) {
    const f = new FormData();
    f.set('email', to);
    f.set('return_to', returnTo);
    f.set('client', client);
    return f;
  }

  function send(to: string, again = false) {
    setBusy('send');
    run(async () => {
      const r = await startEmail(emailData(to));
      if (r.ok) {
        setEmail(r.email);
        setEmailProblem(null);
        setCodeProblem(null);
        setCode('');
        setResent(again);
        setStep('code');
      } else if (step === 'code') {
        setCodeProblem({
          kind: r.kind === 'throttled' ? 'throttled' : 'error',
          message: r.message,
        });
      } else {
        setEmailProblem({
          kind: r.kind,
          message: r.kind === 'throttled' ? throttledCopy(r.retryAfter, providers) : r.message,
        });
      }
    });
  }

  function verify() {
    setBusy('verify');
    const f = emailData(email);
    f.set('code', code);
    run(async () => {
      const r = await verifyCode(f);
      // A success redirects, so only a problem comes back.
      if (r && !r.ok) {
        setCodeProblem({ kind: r.kind, message: r.message });
      }
    });
  }

  function oauth(provider: Provider) {
    setBusy('oauth');
    setOauthProblem(null);
    run(async () => {
      const r = await startOAuth(provider, returnTo, client);
      if (r && !r.ok) {
        setOauthProblem(r.message);
      }
    });
  }

  const working = pending;

  if (step === 'code') {
    const expired = codeProblem?.kind === 'expired';
    return (
      <div className="flex flex-col gap-6">
        <div className="flex flex-col gap-1.5">
          <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">
            Check your email
          </h1>
          <p className="m-0 text-fg-muted">
            We sent a 6-digit code to <span className="text-fg">{email}</span>. It works for 10
            minutes. You can also click the link in the email.
          </p>
        </div>
        {codeProblem?.kind === 'wrong' ? (
          <Notice tone="err" role="alert">
            That code isn’t right. Check the latest email from us. After 5 wrong tries the code
            stops working.
          </Notice>
        ) : null}
        {expired ? (
          <Notice tone="err" role="alert">
            This code has expired.{' '}
            <button
              type="button"
              onClick={() => send(email, true)}
              className="cursor-pointer border-0 bg-transparent p-0 text-[12px] text-blue hover:text-blue-hover"
            >
              Send a new code
            </button>
          </Notice>
        ) : null}
        {codeProblem?.kind === 'throttled' || codeProblem?.kind === 'error' ? (
          <Notice tone={codeProblem.kind === 'throttled' ? 'warn' : 'err'} role="alert">
            {codeProblem.message}
          </Notice>
        ) : null}
        {resent && !codeProblem ? (
          <Notice tone="ok" role="status">
            We sent a new code. Use the latest email.
          </Notice>
        ) : null}
        <form
          className="flex flex-col gap-6"
          onSubmit={(e) => {
            e.preventDefault();
            verify();
          }}
        >
          <CodeInput
            value={code}
            onChange={(v) => {
              setCode(v);
              if (codeProblem?.kind === 'wrong') {
                setCodeProblem(null);
              }
            }}
            invalid={codeProblem?.kind === 'wrong'}
            disabled={working}
          />
          <Button
            type="submit"
            variant="primary"
            className={LARGE}
            loading={working && busy === 'verify'}
            disabled={expired || working}
          >
            {working && busy === 'verify' ? 'Signing in…' : 'Sign in'}
          </Button>
        </form>
        <div className="flex justify-between text-[12px]">
          <button
            type="button"
            onClick={() => {
              setStep('email');
              setCode('');
              setCodeProblem(null);
              setResent(false);
            }}
            className="cursor-pointer border-0 bg-transparent p-0 text-fg-muted hover:text-fg"
          >
            Use a different email
          </button>
          <button
            type="button"
            disabled={working}
            onClick={() => send(email, true)}
            className="cursor-pointer border-0 bg-transparent p-0 text-blue hover:text-blue-hover disabled:cursor-not-allowed disabled:text-fg-disabled"
          >
            Resend code
          </button>
        </div>
      </div>
    );
  }

  const throttled = emailProblem?.kind === 'throttled';
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1.5">
        <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">
          Sign in to curl-runner Cloud
        </h1>
        <p className="m-0 text-fg-muted">
          {providers.length
            ? 'No password. Use GitHub, Google or a code we email you.'
            : 'No password. We email you a code.'}
        </p>
      </div>
      {providers.length ? (
        <>
          <div className="flex flex-col gap-2">
            {providers.map((p) => (
              <Button key={p} className={LARGE} disabled={working} onClick={() => oauth(p)}>
                {p === 'github' ? <GitHubMark /> : <GoogleMark />}
                Continue with {PROVIDER_NAMES[p]}
              </Button>
            ))}
            {oauthProblem ? (
              <Notice tone="err" role="alert">
                {oauthProblem}
              </Notice>
            ) : null}
          </div>
          <div className="flex items-center gap-3 text-[12px] text-fg-subtle">
            <span className="h-px flex-1 bg-hairline" />
            or
            <span className="h-px flex-1 bg-hairline" />
          </div>
        </>
      ) : null}
      {emailProblem ? (
        <Notice tone={throttled ? 'warn' : 'err'} role="alert">
          {emailProblem.message}
        </Notice>
      ) : null}
      <form
        className="flex flex-col gap-3"
        onSubmit={(e) => {
          e.preventDefault();
          send(email);
        }}
      >
        <div className="flex flex-col gap-1.5">
          <label htmlFor="email" className="text-[12px] leading-4 text-fg-muted">
            Email
          </label>
          <input
            id="email"
            name="email"
            type="email"
            required
            autoComplete="email"
            placeholder="you@company.com"
            value={email}
            onChange={(e) => {
              setEmail(e.target.value);
              if (emailProblem) {
                setEmailProblem(null);
              }
            }}
            className={`${INPUT} ${emailProblem?.kind === 'error' ? 'border-danger' : 'border-control'}`}
          />
        </div>
        <Button
          type="submit"
          variant="primary"
          className={LARGE}
          loading={working && busy === 'send'}
          disabled={throttled || working}
        >
          {working && busy === 'send' ? 'Sending code…' : 'Email me a code'}
        </Button>
      </form>
    </div>
  );
}
