import { CheckCircle, Download, ExternalLink, ShieldCheck } from 'lucide-react';
import type { Metadata } from 'next';
import Link from 'next/link';
import { DownloadsSection } from '@/components/downloads-section';
import { SiteFooter } from '@/components/site-footer';
import { SiteHeader } from '@/components/site-header';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

export const metadata: Metadata = {
  title: 'Download curl-runner - Pre-compiled Binaries for All Platforms',
  description:
    'Download pre-compiled binaries of curl-runner for macOS, Linux, and Windows. Standalone executables with no dependencies required. Get started in seconds.',
  keywords: [
    'curl-runner download',
    'download curl-runner',
    'curl-runner binary',
    'curl-runner macOS',
    'curl-runner Linux',
    'curl-runner Windows',
    'curl-runner executable',
    'curl-runner standalone',
    'API testing tool download',
    'HTTP client download',
  ],
  openGraph: {
    title: 'Download curl-runner - Pre-compiled Binaries',
    description:
      'Download standalone executables for macOS, Linux, and Windows. No dependencies required.',
    url: 'https://www.curl-runner.com/downloads',
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Download curl-runner - Pre-compiled Binaries',
    description: 'Standalone executables for macOS, Linux, and Windows. No dependencies required.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/downloads',
  },
};

export default function DownloadsPage() {
  return (
    <div className="min-h-screen bg-background">
      <SiteHeader />

      <main className="flex-1 relative z-10">
        {/* Hero Section */}
        <section className="space-y-6 pb-8 pt-6 md:pb-12 md:pt-10 lg:py-32">
          <div className="container mx-auto flex max-w-[64rem] flex-col items-center gap-4 text-center px-4 sm:px-6 lg:px-8">
            <Badge className="text-sm bg-cyan-500/10 text-cyan-600 dark:text-cyan-400 border-cyan-500/20">
              No Dependencies Required
            </Badge>

            <h1 className="font-bold text-3xl sm:text-5xl md:text-6xl lg:text-7xl">
              <span className="bg-gradient-to-br from-black to-gray-600 bg-clip-text text-transparent dark:from-white dark:to-gray-300">
                Download
              </span>
              <br />
              <span className="bg-gradient-to-br from-cyan-400 to-cyan-600 bg-clip-text text-transparent dark:from-cyan-300 dark:to-cyan-500">
                curl-runner
              </span>
            </h1>

            <p className="max-w-[42rem] leading-normal text-muted-foreground sm:text-xl sm:leading-8">
              Pre-compiled binaries for macOS, Linux, and Windows. Get started in seconds with
              standalone executables that include everything you need.
            </p>

            <div className="flex gap-4 items-center flex-col sm:flex-row">
              <Button asChild size="lg" className="h-11 bg-cyan-500 hover:bg-cyan-600 text-white">
                <a href="#downloads">
                  <Download className="mr-2 h-4 w-4" />
                  Download Now
                </a>
              </Button>

              <Button variant="outline" size="lg" className="h-11" asChild>
                <Link href="/docs/installation">
                  View Installation Guide
                  <ExternalLink className="ml-2 h-4 w-4" />
                </Link>
              </Button>
            </div>
          </div>
        </section>

        {/* Downloads Section */}
        <section
          id="downloads"
          className="container mx-auto max-w-[64rem] py-8 md:py-12 lg:py-24 px-4 sm:px-6 lg:px-8"
        >
          <DownloadsSection />
        </section>

        {/* Features Grid */}
        <section className="container mx-auto max-w-[64rem] py-8 md:py-12 lg:py-24 px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-12">
            <h2 className="text-2xl sm:text-3xl font-bold mb-4">Why Download Binaries?</h2>
            <p className="text-muted-foreground max-w-2xl mx-auto">
              Pre-compiled binaries offer the fastest way to get started with curl-runner
            </p>
          </div>

          <div className="grid gap-6 md:grid-cols-3">
            <div className="rounded-lg border bg-card p-6">
              <div className="flex items-start gap-3">
                <div className="rounded-full bg-green-500/10 p-2">
                  <CheckCircle className="h-5 w-5 text-green-600 dark:text-green-400" />
                </div>
                <div className="flex-1">
                  <h3 className="font-semibold mb-2">No Dependencies</h3>
                  <p className="text-sm text-muted-foreground">
                    Standalone executables with Bun runtime bundled. No need to install Node.js or
                    any other dependencies.
                  </p>
                </div>
              </div>
            </div>

            <div className="rounded-lg border bg-card p-6">
              <div className="flex items-start gap-3">
                <div className="rounded-full bg-cyan-500/10 p-2">
                  <Download className="h-5 w-5 text-cyan-600 dark:text-cyan-400" />
                </div>
                <div className="flex-1">
                  <h3 className="font-semibold mb-2">Fast Setup</h3>
                  <p className="text-sm text-muted-foreground">
                    Download, extract, and run. Get started in seconds without complex installation
                    procedures.
                  </p>
                </div>
              </div>
            </div>

            <div className="rounded-lg border bg-card p-6">
              <div className="flex items-start gap-3">
                <div className="rounded-full bg-purple-500/10 p-2">
                  <ShieldCheck className="h-5 w-5 text-purple-600 dark:text-purple-400" />
                </div>
                <div className="flex-1">
                  <h3 className="font-semibold mb-2">Verified & Secure</h3>
                  <p className="text-sm text-muted-foreground">
                    All binaries include SHA256 checksums for verification. Download with
                    confidence.
                  </p>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Installation Guide CTA */}
        <section className="container mx-auto max-w-[64rem] py-8 md:py-12 lg:py-24 px-4 sm:px-6 lg:px-8">
          <div className="rounded-lg border bg-gradient-to-br from-cyan-500/10 to-cyan-600/10 p-8 md:p-12 text-center">
            <h2 className="text-2xl sm:text-3xl font-bold mb-4">Need Help Getting Started?</h2>
            <p className="text-muted-foreground mb-6 max-w-2xl mx-auto">
              Check out our comprehensive installation guide with step-by-step instructions for all
              platforms, including checksum verification and troubleshooting tips.
            </p>
            <div className="flex gap-4 justify-center flex-col sm:flex-row">
              <Button asChild size="lg" className="h-11">
                <Link href="/docs/installation">View Installation Guide</Link>
              </Button>
              <Button asChild variant="outline" size="lg" className="h-11">
                <Link href="/docs/quick-start">Quick Start Tutorial</Link>
              </Button>
            </div>
          </div>
        </section>

        {/* Alternative Methods */}
        <section className="container mx-auto max-w-[64rem] py-8 md:py-12 lg:py-24 px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-12">
            <h2 className="text-2xl sm:text-3xl font-bold mb-4">Other Installation Methods</h2>
            <p className="text-muted-foreground">
              Prefer using a package manager? We've got you covered.
            </p>
          </div>

          <div className="max-w-xl mx-auto">
            <div className="rounded-lg border bg-card p-6">
              <h3 className="font-semibold mb-2 text-lg">Package Managers</h3>
              <p className="text-sm text-muted-foreground mb-4">
                Install via npm, bun, or your favorite package manager
              </p>
              <code className="block bg-muted p-3 rounded text-sm mb-4">
                bun install -g @curl-runner/cli
              </code>
              <Button asChild variant="outline" size="sm">
                <Link href="/docs/installation#installation-methods">
                  View All Package Managers
                </Link>
              </Button>
            </div>
          </div>
        </section>
      </main>

      <SiteFooter />
    </div>
  );
}
