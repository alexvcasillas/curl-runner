import { api } from '@/lib/api';
import { ApiError } from '@/lib/errors';
import type { BundleFile } from '@/lib/types';
import { zipFiles } from '@/lib/zip';

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
  if (!/^[1-9]\d{0,8}$/.test(n)) {
    return text(404, 'Not found.');
  }
  let files: BundleFile[];
  try {
    files = await api<BundleFile[]>(
      `/v1/teams/${encodeURIComponent(team)}/collections/${encodeURIComponent(slug)}/versions/${n}/files`,
    );
  } catch (e) {
    if (e instanceof ApiError) {
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
  const bytes = zipFiles(files);
  const name = `${slug.replace(/[^\w.-]/g, '-')}-v${n}.zip`;
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
