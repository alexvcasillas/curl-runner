import type { Info, Me } from '@/lib/types';

export const me: Me = {
  user: { id: 'u1', email: 'ada@acme.com', name: 'Ada Lovelace' },
  teams: [
    { slug: 'acme', name: 'Acme', role: 'owner', plan: 'team', suspended: false },
    { slug: 'globex', name: 'Globex Partners', role: 'member', plan: 'free', suspended: false },
  ],
  admin: false,
};

export const info: Info = {
  version: '2.6.0',
  sign_in: ['email'],
  signup: 'open',
  billing: true,
};
