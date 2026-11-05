'use server';

import { cookies } from 'next/headers';
import { redirect } from 'next/navigation';
import { api } from '@/lib/api';
import { ApiError, messageFor } from '@/lib/errors';
import { safeReturnTo } from '@/lib/return-to';
import { cookieOptions, setSession } from '@/lib/session';
import {
  type AuthProblem,
  destination,
  FLOW_MAX_AGE,
  FLOW_PATH,
  OAUTH_COOKIE,
  PROVIDER_NAMES,
  type Provider,
  parseClient,
  RETURN_COOKIE,
  type SignInClient,
} from '@/lib/sign-in';

type Session = { session: string; user: { id: string; email: string; name: string | null } };

export type EmailStartResult =
  | { ok: true; email: string }
  | { ok: false; kind: 'throttled'; retryAfter?: number; message: string }
  | { ok: false; kind: 'error'; message: string };

export type CodeResult = {
  ok: false;
  kind: 'wrong' | 'expired' | 'throttled' | 'error';
  message: string;
};

export type OAuthStartResult = { ok: false; message: string };

function text(formData: FormData, name: string): string {
  const v = formData.get(name);
  return typeof v === 'string' ? v.trim() : '';
}

async function forgetFlowCookie(name: string) {
  (await cookies()).set(name, '', cookieOptions(0, FLOW_PATH));
}

/** `POST /v1/auth/email/start`. The sign-in stays `web`: the CLI and studio hand-offs come after. */
export async function startEmail(formData: FormData): Promise<EmailStartResult> {
  const email = text(formData, 'email').toLowerCase();
  if (!email) {
    return { ok: false, kind: 'error', message: 'Enter your email address.' };
  }
  const to = destination(formData.get('return_to'), parseClient(formData.get('client')));
  try {
    await api('/v1/auth/email/start', {
      method: 'POST',
      json: { email, client: 'web', label: 'Browser' },
      auth: 'none',
    });
  } catch (e) {
    if (!(e instanceof ApiError)) {
      throw e;
    }
    if (e.code === 'rate_limited') {
      return { ok: false, kind: 'throttled', retryAfter: e.retryAfter, message: messageFor(e) };
    }
    return { ok: false, kind: 'error', message: messageFor(e) };
  }
  // The emailed link carries no destination, so keep it for the link page.
  const jar = await cookies();
  if (to === '/') {
    jar.set(RETURN_COOKIE, '', cookieOptions(0, FLOW_PATH));
  } else {
    jar.set(RETURN_COOKIE, to, cookieOptions(FLOW_MAX_AGE, FLOW_PATH));
  }
  return { ok: true, email };
}

/** `POST /v1/auth/email/verify {email, code}`, then the session cookie and the destination. */
export async function verifyCode(formData: FormData): Promise<CodeResult> {
  const email = text(formData, 'email').toLowerCase();
  const code = text(formData, 'code').replace(/\s+/g, '');
  const to = destination(formData.get('return_to'), parseClient(formData.get('client')));
  if (!/^\d{6}$/.test(code)) {
    return { ok: false, kind: 'wrong', message: 'Enter the 6 digits from the email.' };
  }
  let res: Session;
  try {
    res = await api<Session>('/v1/auth/email/verify', {
      method: 'POST',
      json: { email, code },
      auth: 'none',
    });
  } catch (e) {
    if (!(e instanceof ApiError)) {
      throw e;
    }
    if (e.code === 'code_invalid') {
      return { ok: false, kind: 'wrong', message: messageFor(e) };
    }
    if (e.code === 'code_expired') {
      return { ok: false, kind: 'expired', message: messageFor(e) };
    }
    return {
      ok: false,
      kind: e.code === 'rate_limited' ? 'throttled' : 'error',
      message: messageFor(e),
    };
  }
  await setSession(res.session);
  await forgetFlowCookie(RETURN_COOKIE);
  redirect(to);
}

function failure(e: ApiError, fallbackTitle: string): AuthProblem {
  return {
    title: e.code === 'rate_limited' ? 'Too many tries' : fallbackTitle,
    body: messageFor(e),
    actionLabel: 'Back to sign in',
    actionHref: '/sign-in',
  };
}

/** `POST /v1/auth/email/verify {token}`: the emailed link. Returns what to show if it fails. */
export async function verifyLink(
  token: string,
  returnTo: string | undefined,
): Promise<AuthProblem | undefined> {
  const remembered = (await cookies()).get(RETURN_COOKIE)?.value;
  // A `return_to` on the link wins over the destination kept at start. Both are checked.
  const to = returnTo ? safeReturnTo(returnTo) : safeReturnTo(remembered);
  let res: Session;
  try {
    res = await api<Session>('/v1/auth/email/verify', {
      method: 'POST',
      json: { token },
      auth: 'none',
    });
  } catch (e) {
    if (!(e instanceof ApiError)) {
      throw e;
    }
    if (e.code === 'code_expired' || e.code === 'code_invalid') {
      return {
        title: 'This link has expired',
        body: 'Sign-in links work for 10 minutes, and only the newest one works. Ask for a new code and use that email.',
        actionLabel: 'Send a new code',
        actionHref: '/sign-in',
      };
    }
    if (e.code === 'invalid_request') {
      return {
        title: 'Enter this code in your terminal',
        body: 'This email was sent to sign in the curl-runner CLI, so its link doesn’t work in a browser.',
        actionLabel: 'Sign in on the web instead',
        actionHref: '/sign-in',
        hint: 'The CLI asked for this code. Type the 6 digits from the same email into your terminal instead.',
      };
    }
    return failure(e, 'We couldn’t sign you in');
  }
  await setSession(res.session);
  await forgetFlowCookie(RETURN_COOKIE);
  redirect(to);
}

/** `POST /v1/auth/oauth/{provider}/start`: keeps the binding in `cr_oauth`, then goes to the provider. */
export async function startOAuth(
  provider: Provider,
  returnTo: string,
  client: SignInClient,
): Promise<OAuthStartResult> {
  const to = destination(returnTo, client);
  let started: { url: string; binding: string };
  try {
    started = await api(`/v1/auth/oauth/${provider}/start`, {
      method: 'POST',
      json: { client: 'web', return_to: to },
      auth: 'none',
    });
  } catch (e) {
    if (!(e instanceof ApiError)) {
      throw e;
    }
    return { ok: false, message: messageFor(e) };
  }
  if (!/^https?:\/\//.test(started.url)) {
    return { ok: false, message: messageFor(new ApiError(502, 'unavailable', '')) };
  }
  (await cookies()).set(OAUTH_COOKIE, started.binding, cookieOptions(FLOW_MAX_AGE, FLOW_PATH));
  redirect(started.url);
}

const EXPIRED: AuthProblem = {
  title: 'That sign-in has expired',
  body: 'You took longer than 10 minutes, or opened the provider’s page twice. Start again and it’ll go straight through.',
  actionLabel: 'Start again',
  actionHref: '/sign-in',
};

/** `POST /v1/auth/oauth/{provider}/callback`. Returns what to show if it fails. */
export async function finishOAuth(
  provider: Provider,
  code: string | undefined,
  state: string | undefined,
): Promise<AuthProblem | undefined> {
  const binding = (await cookies()).get(OAUTH_COOKIE)?.value;
  if (!binding || !code || !state) {
    await forgetFlowCookie(OAUTH_COOKIE);
    return EXPIRED;
  }
  let res: Session & { return_to?: string | null };
  try {
    res = await api(`/v1/auth/oauth/${provider}/callback`, {
      method: 'POST',
      json: { code, state, binding },
      auth: 'none',
    });
  } catch (e) {
    await forgetFlowCookie(OAUTH_COOKIE);
    if (!(e instanceof ApiError)) {
      throw e;
    }
    if (e.code === 'invalid_request') {
      return EXPIRED;
    }
    if (e.code === 'unauthorized') {
      const name = PROVIDER_NAMES[provider];
      return {
        title: `We couldn’t verify your ${name} email`,
        body: `Your ${name} account needs a verified primary email. Verify it in ${name} settings, or sign in with an email code.`,
        actionLabel: 'Back to sign in',
        actionHref: '/sign-in',
      };
    }
    return failure(e, 'We couldn’t sign you in');
  }
  await forgetFlowCookie(OAUTH_COOKIE);
  await setSession(res.session);
  redirect(safeReturnTo(res.return_to));
}
