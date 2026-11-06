import type { ActionResult } from '@/lib/action-result';

/** The server actions the collections page calls. The page passes them in, so tests can stand in. */
export type CollectionActions = {
  disable: (team: string, slug: string, reason: string) => Promise<ActionResult>;
  enable: (team: string, slug: string) => Promise<ActionResult>;
};
