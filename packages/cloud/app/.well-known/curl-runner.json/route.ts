import { env } from '@/lib/env';

// Read at request time, so one image serves any API URL.
export const dynamic = 'force-dynamic';

/** Tells the CLI and the studio where this dashboard's API lives. */
export async function GET() {
  return Response.json(
    { api: env().PUBLIC_API_URL },
    { headers: { 'cache-control': 'public, max-age=300' } },
  );
}
