import type { ErrorCode } from './errors';

/** What a server action tells the page: done, or the copy to show. */
export type ActionResult = { ok: true } | { ok: false; message: string; code?: ErrorCode };
