import { type NextRequest, NextResponse } from 'next/server';

/**
 * Delay endpoint - responds after specified delay
 * Replaces httpbin.org/delay functionality
 */
export async function GET(request: NextRequest, { params }: { params: Promise<{ seconds: string }> }) {
  const { seconds } = await params;
  const delay = parseFloat(seconds);

  // Validate delay
  if (isNaN(delay) || delay < 0 || delay > 10) {
    return NextResponse.json(
      { error: 'Invalid delay. Must be between 0-10 seconds' },
      { status: 400 },
    );
  }

  // Wait for the specified delay
  await new Promise((resolve) => setTimeout(resolve, delay * 1000));

  return NextResponse.json({
    delay: delay,
    message: `Delayed response after ${delay} seconds`,
    timestamp: new Date().toISOString(),
  });
}

export async function POST(request: NextRequest, { params }: { params: Promise<{ seconds: string }> }) {
  return GET(request, { params });
}
