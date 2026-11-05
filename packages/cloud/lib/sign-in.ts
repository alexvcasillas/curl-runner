import { safeReturnTo } from './return-to';

/** Who is signing in. `device` is the CLI, `studio` is the desktop app. The API call is always `web`. */
export type SignInClient = 'web' | 'device' | 'studio';
export type Provider = 'github' | 'google';

export const DEVICE_PATH = '/sign-in/device';
export const STUDIO_PATH = '/sign-in/studio';

/** The short-lived OAuth binding, readable only on the sign-in pages. */
export const OAUTH_COOKIE = 'cr_oauth';
/** Where an emailed link should land: the link itself carries no destination. */
export const RETURN_COOKIE = 'cr_return_to';
/** Ten minutes, as long as the code and the OAuth state live. */
export const FLOW_MAX_AGE = 600;
export const FLOW_PATH = '/sign-in';

export const PROVIDER_NAMES: Record<Provider, string> = { github: 'GitHub', google: 'Google' };

export function isProvider(value: string): value is Provider {
  return value === 'github' || value === 'google';
}

export function parseClient(value: unknown): SignInClient {
  return value === 'device' || value === 'studio' ? value : 'web';
}

/** First value of a query parameter. */
export function first(value: string | string[] | undefined): string | undefined {
  return Array.isArray(value) ? value[0] : value;
}

/** Where to go once signed in: the hand-off page for the CLI and the studio, else a safe `return_to`. */
export function destination(returnTo: unknown, client: SignInClient): string {
  if (client === 'device') {
    return DEVICE_PATH;
  }
  if (client === 'studio') {
    return STUDIO_PATH;
  }
  return safeReturnTo(typeof returnTo === 'string' ? returnTo : null);
}

/** A sign-in step that went wrong, as the page that shows it needs it. */
export type AuthProblem = {
  title: string;
  body: string;
  actionLabel: string;
  actionHref: string;
  /** An extra info notice under the action. */
  hint?: string;
};
