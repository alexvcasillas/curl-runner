import { type NextRequest, NextResponse } from 'next/server';

/**
 * Status code endpoint - returns specified HTTP status code
 * Replaces httpbin.org/status functionality
 */
export async function GET(request: NextRequest, { params }: { params: Promise<{ code: string }> }) {
  const { code } = await params;
  const statusCode = parseInt(code, 10);

  // Validate status code
  if (isNaN(statusCode) || statusCode < 100 || statusCode > 599) {
    return NextResponse.json(
      { error: 'Invalid status code. Must be between 100-599' },
      { status: 400 },
    );
  }

  // Return appropriate response based on status code
  if (statusCode >= 200 && statusCode < 300) {
    return NextResponse.json(
      {
        message: 'Success',
        statusCode,
        timestamp: new Date().toISOString(),
      },
      { status: statusCode },
    );
  } else if (statusCode >= 300 && statusCode < 400) {
    // Redirects
    const response = NextResponse.json(
      {
        message: 'Redirect',
        statusCode,
        timestamp: new Date().toISOString(),
      },
      { status: statusCode },
    );
    if (statusCode === 301 || statusCode === 302) {
      response.headers.set('Location', '/api/test/echo');
    }
    return response;
  } else if (statusCode >= 400 && statusCode < 500) {
    return NextResponse.json(
      {
        error: 'Client Error',
        message: getErrorMessage(statusCode),
        statusCode,
        timestamp: new Date().toISOString(),
      },
      { status: statusCode },
    );
  } else {
    return NextResponse.json(
      {
        error: 'Server Error',
        message: getErrorMessage(statusCode),
        statusCode,
        timestamp: new Date().toISOString(),
      },
      { status: statusCode },
    );
  }
}

export async function POST(request: NextRequest, { params }: { params: Promise<{ code: string }> }) {
  return GET(request, { params });
}

export async function PUT(request: NextRequest, { params }: { params: Promise<{ code: string }> }) {
  return GET(request, { params });
}

export async function DELETE(request: NextRequest, { params }: { params: Promise<{ code: string }> }) {
  return GET(request, { params });
}

function getErrorMessage(statusCode: number): string {
  const messages: Record<number, string> = {
    400: 'Bad Request',
    401: 'Unauthorized',
    403: 'Forbidden',
    404: 'Not Found',
    405: 'Method Not Allowed',
    429: 'Too Many Requests',
    500: 'Internal Server Error',
    502: 'Bad Gateway',
    503: 'Service Unavailable',
  };
  return messages[statusCode] || 'Error';
}
