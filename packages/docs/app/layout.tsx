import { Analytics } from '@vercel/analytics/next';
import type { Metadata } from 'next';
import { Geist, Geist_Mono } from 'next/font/google';
import { ThemeProvider } from 'next-themes';
import './globals.css';
import { SpeedInsights } from '@vercel/speed-insights/next';
import { PulsoraAnalytics } from '@/components/pulsora-analytics';

const geistSans = Geist({
  variable: '--font-geist-sans',
  subsets: ['latin'],
});

const geistMono = Geist_Mono({
  variable: '--font-geist-mono',
  subsets: ['latin'],
});

export const metadata: Metadata = {
  metadataBase: new URL('https://www.curl-runner.com'),
  title: {
    default: 'curl-runner Documentation',
    template: '%s | curl-runner Documentation',
  },
  description:
    'Complete documentation for curl-runner - A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance.',
  keywords: [
    'curl-runner',
    'API testing tool',
    'HTTP client CLI',
    'YAML configuration',
    'CLI tool',
    'Bun runtime',
    'REST API testing',
    'HTTP client',
    'API automation',
    'testing framework',
    'parallel execution',
    'response validation',
    'request templating',
    'CI/CD integration',
    'API monitoring',
    'load testing',
    'alternative to Postman',
    'alternative to curl',
    'alternative to Insomnia',
    'microservices testing',
    'developer tools',
    'command line HTTP client',
    'YAML API testing',
    'HTTP request automation',
  ],
  authors: [{ name: 'curl-runner Team', url: 'https://github.com/alexvcasillas/curl-runner' }],
  creator: 'curl-runner Team',
  publisher: 'curl-runner',
  applicationName: 'curl-runner',
  generator: 'Next.js',
  referrer: 'origin-when-cross-origin',
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      'max-video-preview': -1,
      'max-image-preview': 'large',
      'max-snippet': -1,
    },
  },
  manifest: '/manifest.json',
  alternates: {
    canonical: 'https://www.curl-runner.com',
    types: {
      'application/rss+xml': [{ url: '/rss.xml', title: 'curl-runner Documentation RSS' }],
    },
  },
  openGraph: {
    type: 'website',
    locale: 'en_US',
    url: 'https://www.curl-runner.com',
    title: 'curl-runner Documentation',
    description:
      'Complete documentation for curl-runner - A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance.',
    siteName: 'curl-runner Documentation',
    images: [
      {
        url: '/opengraph-image',
        width: 1200,
        height: 630,
        alt: 'curl-runner Documentation',
        type: 'image/png',
      },
    ],
  },
  twitter: {
    card: 'summary_large_image',
    title: 'curl-runner Documentation',
    description:
      'Complete documentation for curl-runner - A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance.',
    images: ['/opengraph-image'],
    creator: '@curl_runner',
    site: '@curl_runner',
  },
  verification: {
    google: process.env.GOOGLE_SITE_VERIFICATION || 'google-site-verification-placeholder',
    other: {
      'msvalidate.01': process.env.BING_SITE_VERIFICATION || 'bing-verification-placeholder',
    },
  },
  category: 'technology',
  icons: {
    icon: [
      {
        url: '/icon-light-32.png',
        media: '(prefers-color-scheme: light)',
        sizes: '32x32',
        type: 'image/png',
      },
      {
        url: '/icon-dark-32.png',
        media: '(prefers-color-scheme: dark)',
        sizes: '32x32',
        type: 'image/png',
      },
      {
        url: '/icon-light-192.png',
        media: '(prefers-color-scheme: light)',
        sizes: '192x192',
        type: 'image/png',
      },
      {
        url: '/icon-dark-192.png',
        media: '(prefers-color-scheme: dark)',
        sizes: '192x192',
        type: 'image/png',
      },
    ],
    shortcut: [
      {
        url: '/icon-light-192.png',
        media: '(prefers-color-scheme: light)',
        sizes: '192x192',
        type: 'image/png',
      },
      {
        url: '/icon-dark-192.png',
        media: '(prefers-color-scheme: dark)',
        sizes: '192x192',
        type: 'image/png',
      },
    ],
    apple: [
      {
        url: '/apple-icon-light.png',
        media: '(prefers-color-scheme: light)',
        sizes: '192x192',
        type: 'image/png',
      },
      {
        url: '/apple-icon-dark.png',
        media: '(prefers-color-scheme: dark)',
        sizes: '192x192',
        type: 'image/png',
      },
    ],
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="" />
        <link rel="dns-prefetch" href="https://vercel-insights.com" />
        <link rel="dns-prefetch" href="https://vitals.vercel-insights.com" />
        <meta name="theme-color" media="(prefers-color-scheme: light)" content="#ffffff" />
        <meta name="theme-color" media="(prefers-color-scheme: dark)" content="#000000" />
        <meta name="color-scheme" content="light dark" />
      </head>
      <body
        className={`${geistSans.variable} ${geistMono.variable} antialiased`}
        suppressHydrationWarning
      >
        <ThemeProvider
          attribute="class"
          defaultTheme="system"
          enableSystem
          disableTransitionOnChange
        >
          <PulsoraAnalytics>{children}</PulsoraAnalytics>
        </ThemeProvider>
        <Analytics />
        <SpeedInsights />
      </body>
    </html>
  );
}
