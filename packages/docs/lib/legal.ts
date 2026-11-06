/** Shared facts for the privacy policy and the terms of service. */

export const LEGAL_UPDATED = 'November 6, 2026';

export const CONTACT_EMAIL = 'hello@curl-runner.com';

export const CLOUD_URL = 'https://cloud.curl-runner.com';

export const PROCESSORS: { name: string; purpose: string }[] = [
  { name: 'Hetzner', purpose: 'Servers for curl-runner Cloud and its database, in Germany' },
  { name: 'Stripe', purpose: 'Checkout, subscriptions, invoices, tax and the billing portal' },
  { name: 'Amazon SES', purpose: 'Sends Cloud emails: sign-in codes, invites and billing notices' },
  { name: 'GitHub', purpose: 'Sign-in with GitHub, if you use it' },
  { name: 'Google', purpose: 'Sign-in with Google, if you use it' },
  { name: 'Vercel', purpose: 'Hosts this website and its cookieless page statistics' },
  { name: 'Pulsora', purpose: 'Cookieless page statistics for this website' },
];
