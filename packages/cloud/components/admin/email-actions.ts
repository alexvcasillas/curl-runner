import type { ActionResult } from '@/lib/action-result';
import type { Audience, BroadcastResult, DraftInput, Preview } from '@/lib/admin-emails';

type Failure = Extract<ActionResult, { ok: false }>;

/** The server actions the broadcasts page calls. The page passes them in, so tests can stand in. */
export type EmailActions = {
  create: (input?: Partial<DraftInput>) => Promise<BroadcastResult>;
  save: (id: string, input: DraftInput) => Promise<BroadcastResult>;
  test: (id: string) => Promise<{ ok: true; to: string } | Failure>;
  send: (id: string, at?: string | null) => Promise<BroadcastResult>;
  preview: (id: string, asUser: string) => Promise<{ ok: true; preview: Preview } | Failure>;
  count: (audience: Audience) => Promise<{ ok: true; count: number } | Failure>;
};
