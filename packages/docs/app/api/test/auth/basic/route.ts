import { type NextRequest, NextResponse } from 'next/server';

/**
 * Basic authentication endpoint
 * Replaces httpbin.org/basic-auth functionality
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
          'WWW-Authenticate': 'Basic realm="Test Realm"',
        },
      },
    );
  }

  // Parse Basic auth
  if (!authHeader.startsWith('Basic ')) {
    return NextResponse.json(
      {
        error: 'Unauthorized',
        message: 'Basic authentication required',
      },
      {
        status: 401,
        headers: {
          'WWW-Authenticate': 'Basic realm="Test Realm"',
        },
      },
    );
  }

  try {
    const base64Credentials = authHeader.split(' ')[1];
    const credentials = Buffer.from(base64Credentials, 'base64').toString('utf-8');
    const [username, password] = credentials.split(':');

    // For testing, only accept specific test credentials
    // Accept 'user:pass' as valid credentials
    if (username === 'user' && password === 'pass') {
      return NextResponse.json({
        authenticated: true,
        user: username,
        message: 'Successfully authenticated',
        timestamp: new Date().toISOString(),
      });
    } else {
      return NextResponse.json(
        {
          error: 'Unauthorized',
          message: 'Invalid credentials',
        },
        {
          status: 401,
          headers: {
            'WWW-Authenticate': 'Basic realm="Test Realm"',
          },
        },
      );
    }
  } catch (_error) {
    return NextResponse.json(
      {
        error: 'Unauthorized',
        message: 'Invalid authorization header',
      },
      {
        status: 401,
        headers: {
          'WWW-Authenticate': 'Basic realm="Test Realm"',
        },
      },
    );
  }
}

export async function POST(request: NextRequest) {
  return GET(request);
}
