import { NextRequest, NextResponse } from 'next/server'

/**
 * GET-only endpoint - only accepts GET method
 * Used for testing method not allowed scenarios
 */
export async function GET(request: NextRequest) {
	const headers = Object.fromEntries(request.headers.entries())
	const url = new URL(request.url)
	
	return NextResponse.json({
		method: 'GET',
		url: request.url,
		message: 'This endpoint only accepts GET requests',
		timestamp: new Date().toISOString()
	})
}

// No POST, PUT, DELETE, or PATCH handlers defined
// Next.js will automatically return 405 Method Not Allowed for these