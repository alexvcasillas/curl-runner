import type { BulkResult } from '@/app/admin/users/actions';
import type { ActionResult } from '@/lib/action-result';

/** The server actions the users list calls. The page passes them in, so tests can stand in. */
export type UserListActions = {
  suspend: (id: string, reason: string, notify: boolean) => Promise<ActionResult>;
  unsuspend: (id: string) => Promise<ActionResult>;
  signOut: (id: string) => Promise<ActionResult>;
  remove: (id: string) => Promise<ActionResult>;
  email: (id: string, subject: string, body: string) => Promise<ActionResult>;
  bulk: (ids: string[], action: 'sign_out' | 'suspend', reason: string) => Promise<BulkResult>;
};

/** The detail page's: the same set, minus bulk, plus devices and sign-in methods. */
export type UserDetailActions = Omit<UserListActions, 'bulk'> & {
  revokeSession: (id: string, sessionId: string) => Promise<ActionResult>;
  unlink: (id: string, provider: string) => Promise<ActionResult>;
};
