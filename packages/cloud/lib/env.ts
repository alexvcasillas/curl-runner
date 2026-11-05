import 'server-only';
import { z } from 'zod';

// An unset or empty variable takes the local default; a set one must be an http(s) URL.
const url = (fallback: string) =>
  z
    .string()
    .trim()
    .optional()
    .transform((v) => (v ? v : fallback))
    .pipe(z.url({ protocol: /^https?$/ }))
    .transform((v) => v.replace(/\/+$/, ''));

const schema = z.object({
  /** How the dashboard reaches the API, on the private network in production. */
  API_INTERNAL_URL: url('http://localhost:8080'),
  /** The API URL that links and `.well-known` advertise. */
  PUBLIC_API_URL: url('http://localhost:8080'),
  /** This dashboard's public URL. */
  PUBLIC_APP_URL: url('http://localhost:3000'),
});

export type Env = z.infer<typeof schema>;

let cached: Env | null = null;

// The localhost defaults are for development. In production a missing public URL
// would advertise localhost to the CLI and drop `Secure` from the session cookie.
const REQUIRED_IN_PRODUCTION = ['PUBLIC_APP_URL', 'PUBLIC_API_URL'] as const;

/**
 * The server's environment, validated on first use (never at import, so
 * `next build` runs without it).
 */
export function env(): Env {
  if (!cached) {
    if (process.env.NODE_ENV === 'production') {
      const missing = REQUIRED_IN_PRODUCTION.filter((name) => !process.env[name]?.trim());
      if (missing.length > 0) {
        throw new Error(`Set ${missing.join(' and ')}: production has no default for them.`);
      }
    }
    const parsed = schema.safeParse({
      API_INTERNAL_URL: process.env.API_INTERNAL_URL,
      PUBLIC_API_URL: process.env.PUBLIC_API_URL,
      PUBLIC_APP_URL: process.env.PUBLIC_APP_URL,
    });
    if (!parsed.success) {
      throw new Error(`Invalid environment: ${z.prettifyError(parsed.error)}`);
    }
    cached = parsed.data;
  }
  return cached;
}

/** Forget the cached environment. Tests only. */
export function resetEnv() {
  cached = null;
}
