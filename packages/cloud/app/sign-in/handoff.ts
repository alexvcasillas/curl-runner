'use server';

import { api } from '@/lib/api';
import { env } from '@/lib/env';
import { ApiError, messageFor } from '@/lib/errors';

export type HandoffResult =
  | { ok: true; code: string; url: string }
  | { ok: false; message: string };

async function mint(label: string): Promise<{ code: string } | { message: string }> {
  try {
    return await api<{ code: string }>('/v1/auth/handoff', {
      method: 'POST',
      json: { session_label: label },
    });
  } catch (e) {
    if (e instanceof ApiError) {
      return { message: messageFor(e) };
    }
    throw e;
  }
}

/** The CLI's code, minted only when the person asks (a POST), never on page load. */
export async function mintDeviceCode(): Promise<HandoffResult> {
  const r = await mint('CLI');
  return 'code' in r ? { ok: true, code: r.code, url: '' } : { ok: false, message: r.message };
}

/** The studio's code and its `curl-runner://` link, minted on the person's click. */
export async function mintStudioLink(): Promise<HandoffResult> {
  const r = await mint('Studio');
  if (!('code' in r)) {
    return { ok: false, message: r.message };
  }
  const url = `curl-runner://auth/callback?code=${encodeURIComponent(r.code)}&server=${encodeURIComponent(env().PUBLIC_API_URL)}`;
  return { ok: true, code: r.code, url };
}
