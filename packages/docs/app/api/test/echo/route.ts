import { type NextRequest, NextResponse } from 'next/server';

/**
 * Echo endpoint - returns request details back to client
 * Replaces httpbin.org/anything functionality
 */
export async function GET(request: NextRequest) {
  const headers = Object.fromEntries(request.headers.entries());
  const url = new URL(request.url);

  return NextResponse.json({
    method: 'GET',
    url: request.url,
    headers,
    query: Object.fromEntries(url.searchParams.entries()),
    origin: headers['x-forwarded-for'] || headers['x-real-ip'] || 'unknown',
    timestamp: new Date().toISOString(),
  });
}

export async function POST(request: NextRequest) {
  const headers = Object.fromEntries(request.headers.entries());
  const url = new URL(request.url);
  let body = null;

  try {
    const contentType = headers['content-type'] || '';
    if (contentType.includes('application/json')) {
      body = await request.json();
    } else {
      body = await request.text();
    }
  } catch (e) {
    body = null;
  }

  return NextResponse.json({
    method: 'POST',
    url: request.url,
    headers,
    query: Object.fromEntries(url.searchParams.entries()),
    body,
    origin: headers['x-forwarded-for'] || headers['x-real-ip'] || 'unknown',
    timestamp: new Date().toISOString(),
  });
}

export async function PUT(request: NextRequest) {
  const headers = Object.fromEntries(request.headers.entries());
  const url = new URL(request.url);
  let body = null;

  try {
    const contentType = headers['content-type'] || '';
    if (contentType.includes('application/json')) {
      body = await request.json();
    } else {
      body = await request.text();
    }
  } catch (e) {
    body = null;
  }

  return NextResponse.json({
    method: 'PUT',
    url: request.url,
    headers,
    query: Object.fromEntries(url.searchParams.entries()),
    body,
    origin: headers['x-forwarded-for'] || headers['x-real-ip'] || 'unknown',
    timestamp: new Date().toISOString(),
  });
}

export async function DELETE(request: NextRequest) {
  const headers = Object.fromEntries(request.headers.entries());
  const url = new URL(request.url);

  return NextResponse.json({
    method: 'DELETE',
    url: request.url,
    headers,
    query: Object.fromEntries(url.searchParams.entries()),
    origin: headers['x-forwarded-for'] || headers['x-real-ip'] || 'unknown',
    timestamp: new Date().toISOString(),
  });
}

export async function PATCH(request: NextRequest) {
  const headers = Object.fromEntries(request.headers.entries());
  const url = new URL(request.url);
  let body = null;

  try {
    const contentType = headers['content-type'] || '';
    if (contentType.includes('application/json')) {
      body = await request.json();
    } else {
      body = await request.text();
    }
  } catch (e) {
    body = null;
  }

  return NextResponse.json({
    method: 'PATCH',
    url: request.url,
    headers,
    query: Object.fromEntries(url.searchParams.entries()),
    body,
    origin: headers['x-forwarded-for'] || headers['x-real-ip'] || 'unknown',
    timestamp: new Date().toISOString(),
  });
}
