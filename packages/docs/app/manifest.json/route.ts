import { MetadataRoute } from 'next';

export async function GET(): Promise<Response> {
	const manifest: MetadataRoute.Manifest = {
		name: 'curl-runner Documentation',
		short_name: 'curl-runner',
		description: 'A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance.',
		start_url: '/',
		display: 'standalone',
		background_color: '#ffffff',
		theme_color: '#000000',
		icons: [
			{
				src: '/icon-light-192.png',
				sizes: '192x192',
				type: 'image/png',
				purpose: 'maskable'
			},
			{
				src: '/icon-light-512.png',
				sizes: '512x512',
				type: 'image/png',
				purpose: 'maskable'
			},
			{
				src: '/apple-icon-light.png',
				sizes: '192x192',
				type: 'image/png',
				purpose: 'any'
			}
		],
		categories: ['developer', 'utilities', 'productivity'],
		lang: 'en-US',
		dir: 'ltr',
	};

	return Response.json(manifest);
}