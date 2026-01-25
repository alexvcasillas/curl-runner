import { ArrowRight, CheckCircle, Download, Github } from 'lucide-react';
import type { Metadata } from 'next';
import Link from 'next/link';
import { notFound } from 'next/navigation';
import { CodeBlockServer } from '@/components/code-block-server';
import { SiteFooter } from '@/components/site-footer';
import { SiteHeader } from '@/components/site-header';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { featuresData, getAllFeatures } from '@/lib/features-data';

interface FeaturePageProps {
  params: Promise<{
    slug: string;
  }>;
}

export function generateStaticParams() {
  return getAllFeatures().map((feature) => ({
    slug: feature.slug,
  }));
}

export async function generateMetadata({ params }: FeaturePageProps): Promise<Metadata> {
  const { slug } = await params;
  const feature = featuresData[slug];

  if (!feature) {
    return {
      title: 'Feature Not Found',
    };
  }

  return {
    title: `${feature.title} - curl-runner Feature`,
    description: feature.description,
    keywords: [
      'curl-runner',
      feature.title,
      ...feature.keywords,
      'API testing',
      'HTTP client',
      'CLI tool',
    ],
    openGraph: {
      title: `${feature.title} - curl-runner Feature`,
      description: feature.description,
      url: `https://www.curl-runner.com/features/${feature.slug}`,
      type: 'website',
    },
    twitter: {
      card: 'summary_large_image',
      title: `${feature.title} - curl-runner`,
      description: feature.shortDescription,
    },
    alternates: {
      canonical: `https://www.curl-runner.com/features/${slug}`,
    },
  };
}

export default async function FeaturePage({ params }: FeaturePageProps) {
  const { slug } = await params;
  const feature = featuresData[slug];

  if (!feature) {
    notFound();
  }

  const Icon = feature.icon;
  const colorClass = {
    cyan: 'text-cyan-600 dark:text-cyan-400',
    green: 'text-green-600 dark:text-green-400',
    purple: 'text-purple-600 dark:text-purple-400',
    orange: 'text-orange-600 dark:text-orange-400',
    blue: 'text-blue-600 dark:text-blue-400',
    pink: 'text-pink-600 dark:text-pink-400',
    yellow: 'text-yellow-600 dark:text-yellow-400',
    indigo: 'text-indigo-600 dark:text-indigo-400',
    violet: 'text-violet-600 dark:text-violet-400',
    emerald: 'text-emerald-600 dark:text-emerald-400',
    teal: 'text-teal-600 dark:text-teal-400',
    rose: 'text-rose-600 dark:text-rose-400',
  }[feature.color];

  const bgClass = {
    cyan: 'bg-cyan-500/10 border-cyan-500/20',
    green: 'bg-green-500/10 border-green-500/20',
    purple: 'bg-purple-500/10 border-purple-500/20',
    orange: 'bg-orange-500/10 border-orange-500/20',
    blue: 'bg-blue-500/10 border-blue-500/20',
    pink: 'bg-pink-500/10 border-pink-500/20',
    yellow: 'bg-yellow-500/10 border-yellow-500/20',
    indigo: 'bg-indigo-500/10 border-indigo-500/20',
    violet: 'bg-violet-500/10 border-violet-500/20',
    emerald: 'bg-emerald-500/10 border-emerald-500/20',
    teal: 'bg-teal-500/10 border-teal-500/20',
    rose: 'bg-rose-500/10 border-rose-500/20',
  }[feature.color];

  return (
    <div className="min-h-screen bg-background">
      <SiteHeader />

      {/* Hero Section */}
      <main className="flex-1 relative z-10">
        <section className="space-y-6 pb-8 pt-6 md:pb-12 md:pt-10 lg:py-32">
          <div className="container mx-auto flex max-w-[64rem] flex-col items-center gap-4 text-center px-4 sm:px-6 lg:px-8">
            <div className={`rounded-2xl ${bgClass} p-4 mb-2`}>
              <Icon className={`h-12 w-12 ${colorClass}`} />
            </div>

            <Badge className={`text-sm ${bgClass} ${colorClass}`}>{feature.title}</Badge>

            <h1 className="font-bold text-3xl sm:text-5xl md:text-6xl lg:text-7xl">
              <span className="bg-gradient-to-br from-black to-gray-600 bg-clip-text text-transparent dark:from-white dark:to-gray-300">
                {feature.shortDescription.split(' ').slice(0, -2).join(' ')}
              </span>
              <br />
              <span
                className={`bg-gradient-to-br ${feature.gradient} bg-clip-text text-transparent`}
              >
                {feature.shortDescription.split(' ').slice(-2).join(' ')}
              </span>
            </h1>

            <p className="max-w-[42rem] leading-normal text-muted-foreground sm:text-xl sm:leading-8">
              {feature.description}
            </p>

            <div className="flex gap-4 items-center flex-col sm:flex-row mt-4">
              <Button asChild size="lg" className="h-11 bg-cyan-500 hover:bg-cyan-600 text-white">
                <Link href={`/docs/features/${feature.slug}`}>
                  View Documentation
                  <ArrowRight className="ml-2 h-4 w-4" />
                </Link>
              </Button>

              <Button variant="outline" size="lg" className="h-11" asChild>
                <Link href="/downloads">
                  <Download className="mr-2 h-4 w-4" />
                  Download curl-runner
                </Link>
              </Button>
            </div>
          </div>
        </section>

        {/* Code Example */}
        <section className="container mx-auto max-w-[64rem] py-8 md:py-12 lg:py-24 px-4 sm:px-6 lg:px-8 overflow-hidden">
          <div className="space-y-6">
            <div className="text-center space-y-3">
              <h2 className="text-2xl sm:text-3xl font-bold">{feature.codeExample.title}</h2>
            </div>
            <CodeBlockServer language="yaml" filename={`${feature.slug}.yaml`}>
              {feature.codeExample.code}
            </CodeBlockServer>
          </div>
        </section>

        {/* Benefits */}
        <section className="py-12 sm:py-24 lg:py-32 bg-muted/30">
          <div className="mx-auto max-w-2xl px-4 sm:px-6 lg:max-w-7xl lg:px-8">
            <div className="text-center mb-12">
              <Badge className={`text-sm mb-4 ${bgClass} ${colorClass}`}>Key Benefits</Badge>
              <h2 className="text-2xl font-bold tracking-tight sm:text-4xl md:text-5xl">
                <span className="bg-gradient-to-br from-black to-gray-600 bg-clip-text text-transparent dark:from-white dark:to-gray-300">
                  Why use {feature.title}?
                </span>
              </h2>
            </div>

            <div className="grid gap-6 md:grid-cols-3">
              {feature.benefits.map((benefit, idx) => (
                <div key={idx} className="rounded-lg border bg-card p-6">
                  <div className="flex items-start gap-3">
                    <div className={`rounded-full ${bgClass} p-2 mt-1`}>
                      <CheckCircle className={`h-5 w-5 ${colorClass}`} />
                    </div>
                    <div className="flex-1">
                      <h3 className="font-semibold text-lg mb-2">{benefit.title}</h3>
                      <p className="text-sm text-muted-foreground">{benefit.description}</p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Use Cases */}
        <section className="container mx-auto max-w-[64rem] py-8 md:py-12 lg:py-24 px-4 sm:px-6 lg:px-8">
          <div className="space-y-6 text-center">
            <Badge className={`text-sm ${bgClass} ${colorClass}`}>Use Cases</Badge>
            <h2 className="text-2xl sm:text-3xl font-bold">Perfect for these scenarios</h2>
            <div className="grid gap-4 md:grid-cols-2 text-left mt-8">
              {feature.useCases.map((useCase, idx) => (
                <div key={idx} className="flex items-start gap-3 p-4 rounded-lg border bg-card">
                  <CheckCircle className={`h-5 w-5 ${colorClass} mt-0.5 flex-shrink-0`} />
                  <span className="text-sm">{useCase}</span>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* CTA */}
        <section className="container mx-auto max-w-[64rem] py-8 md:py-12 lg:py-24 px-4 sm:px-6 lg:px-8">
          <div className="mx-auto flex max-w-[58rem] flex-col items-center justify-center gap-4 sm:gap-6 text-center">
            <h2 className="font-bold text-2xl leading-[1.1] sm:text-3xl md:text-5xl lg:text-6xl">
              Ready to try {feature.title}?
            </h2>
            <p className="max-w-[85%] leading-normal text-muted-foreground text-sm sm:text-lg sm:leading-7">
              Get started with curl-runner and unlock powerful {feature.title.toLowerCase()}{' '}
              capabilities for your API testing workflow.
            </p>
            <div className="flex gap-3 sm:gap-4 items-center flex-col sm:flex-row mt-2 sm:mt-4">
              <Button asChild size="lg" className="h-11 bg-cyan-500 hover:bg-cyan-600 text-white">
                <Link href="/docs/installation">
                  <Download className="mr-2 h-4 w-4" />
                  Install Now
                </Link>
              </Button>

              <Button variant="outline" size="lg" className="h-11" asChild>
                <a
                  href="https://github.com/alexvcasillas/curl-runner"
                  target="_blank"
                  rel="noreferrer"
                >
                  <Github className="mr-2 h-4 w-4" />
                  View on GitHub
                </a>
              </Button>
            </div>
          </div>
        </section>
      </main>

      <SiteFooter />
    </div>
  );
}
