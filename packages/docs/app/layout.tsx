import { Analytics } from '@vercel/analytics/next';
import type { Metadata } from 'next';
import { Geist, Geist_Mono } from 'next/font/google';
import { ThemeProvider } from 'next-themes';
import './globals.css';
import { SpeedInsights } from '@vercel/speed-insights/next';

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
    'API testing',
    'HTTP requests',
    'YAML configuration',
    'CLI tool',
    'Bun',
    'REST API',
    'HTTP client',
    'automation',
    'testing framework',
    'parallel execution',
    'response validation',
    'request templating',
    'CI/CD integration',
    'API monitoring',
    'load testing',
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
    google: 'google-site-verification-code',
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
          {children}
        </ThemeProvider>
        <Analytics />
        <SpeedInsights />
      </body>
    </html>
  );
}
