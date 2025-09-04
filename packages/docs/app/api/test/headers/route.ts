import { type NextRequest, NextResponse } from 'next/server';

/**
 * Headers endpoint - returns request headers
 * Replaces httpbin.org/headers functionality
 */
export async function GET(request: NextRequest) {
  const headers = Object.fromEntries(request.headers.entries());

  return NextResponse.json({
    headers,
    count: Object.keys(headers).length,
    timestamp: new Date().toISOString(),
  });
}

export async function POST(request: NextRequest) {
  return GET(request);
}
