// The compose healthcheck. No API call: the dashboard is up even when the API isn't.
export const dynamic = 'force-dynamic';

export async function GET() {
  return new Response('ok', {
    headers: { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store' },
  });
}
