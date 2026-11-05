import { redirect } from 'next/navigation';
import { api } from '@/lib/api';
import { ApiError } from '@/lib/errors';
import { isSlug } from '@/lib/params';
import { routes } from '@/lib/routes';
import type { BundleFile } from '@/lib/types';
import { UnsafePathError, zipFiles } from '@/lib/zip';

type Params = { params: Promise<{ team: string; slug: string; n: string }> };

function text(status: number, body: string): Response {
  return new Response(body, {
    status,
    headers: { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store' },
  });
}

/** The version's files as one zip. The session decides who may read them, as for the pages. */
export async function GET(_request: Request, { params }: Params): Promise<Response> {
  const { team, slug, n } = await params;
  if (!isSlug(team) || !isSlug(slug) || !/^[1-9]\d{0,8}$/.test(n)) {
    return text(404, 'Not found.');
  }
  let files: BundleFile[];
  try {
    // `optional`: a dead session comes back as a 401 here, so we can choose where to send them.
    files = await api<BundleFile[]>(`/v1/teams/${team}/collections/${slug}/versions/${n}/files`, {
      auth: 'optional',
    });
  } catch (e) {
    if (e instanceof ApiError) {
      if (e.status === 401) {
        redirect(`/sign-in/expired?return_to=${encodeURIComponent(routes.collection(team, slug))}`);
      }
      if (e.status === 404) {
        return text(404, 'Not found.');
      }
      if (e.status === 403) {
        return text(403, 'You can’t download this.');
      }
      return text(503, 'Cloud isn’t answering right now. Try again in a minute.');
    }
    throw e;
  }
  let bytes: Uint8Array;
  try {
    bytes = zipFiles(files);
  } catch (e) {
    if (e instanceof UnsafePathError) {
      console.error(e.message);
      return text(500, 'This version has a file path that can’t be zipped safely.');
    }
    throw e;
  }
  const name = `${slug}-v${n}.zip`;
  return new Response(bytes as BodyInit, {
    headers: {
      'content-type': 'application/zip',
      'content-disposition': `attachment; filename="${name}"`,
      'content-length': String(bytes.byteLength),
      'cache-control': 'private, no-store',
      'x-content-type-options': 'nosniff',
    },
  });
}
