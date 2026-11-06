import type { GrantResult } from '@/app/admin/teams/actions';
import type { ActionResult } from '@/lib/action-result';
import type { OverrideInput } from '@/lib/admin-teams';

/** The server actions the team detail page calls. The page passes them in, so tests can stand in. */
export type TeamDetailActions = {
  grant: (slug: string, input: OverrideInput) => Promise<GrantResult>;
  removeOverride: (slug: string) => Promise<ActionResult>;
  suspend: (slug: string, reason: string, notify: boolean) => Promise<ActionResult>;
  unsuspend: (slug: string) => Promise<ActionResult>;
  remove: (slug: string) => Promise<ActionResult>;
  setRole: (slug: string, userId: string, role: 'owner' | 'member') => Promise<ActionResult>;
  stripeSync: (slug: string) => Promise<ActionResult>;
  resendInvoice: (slug: string) => Promise<ActionResult>;
  emailOwners: (slug: string, subject: string, body: string) => Promise<ActionResult>;
};
