import { type NextRequest, NextResponse } from 'next/server';

/**
 * Bearer token authentication endpoint
 * Replaces httpbin.org/bearer functionality
 */
export async function GET(request: NextRequest) {
  const authHeader = request.headers.get('authorization');

  // Check if authorization header exists
  if (!authHeader) {
    return NextResponse.json(
      {
        error: 'Unauthorized',
        message: 'Authorization header required',
      },
      {
        status: 401,
        headers: {
          'WWW-Authenticate': 'Bearer realm="Test Realm"',
        },
      },
    );
  }

  // Parse Bearer token
  if (!authHeader.startsWith('Bearer ')) {
    return NextResponse.json(
      {
        error: 'Unauthorized',
        message: 'Bearer token required',
      },
      {
        status: 401,
        headers: {
          'WWW-Authenticate': 'Bearer realm="Test Realm"',
        },
      },
    );
  }

  const token = authHeader.split(' ')[1];

  // For testing, accept any non-empty token
  // In real scenarios, you'd validate the JWT or token against a database
  if (token && token.length > 0) {
    return NextResponse.json({
      authenticated: true,
      token: token.substring(0, 10) + '...', // Don't echo full token
      message: 'Successfully authenticated with bearer token',
      timestamp: new Date().toISOString(),
    });
  } else {
    return NextResponse.json(
      {
        error: 'Unauthorized',
        message: 'Invalid or empty token',
      },
      {
        status: 401,
        headers: {
          'WWW-Authenticate': 'Bearer realm="Test Realm"',
        },
      },
    );
  }
}

export async function POST(request: NextRequest) {
  return GET(request);
}
