import {
  ArrowRight,
  Download,
  FileText,
  Github,
  Play,
  Settings,
  Terminal,
  Zap,
} from 'lucide-react';
import Link from 'next/link';
import { CodeBlockServer } from '@/components/code-block-server';
import { Contributors } from '@/components/contributors';
import { CurlRunner } from '@/components/curl-runner';
import { SiteFooter } from '@/components/site-footer';
import { SiteHeader } from '@/components/site-header';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { exampleYaml, installCommand, runCommand } from './snippets';

export default function Home() {
  return (
    <div className="min-h-screen bg-background">
      <SiteHeader />

      {/* Hero Section */}
      <main className="flex-1 relative z-10">
        <section className="space-y-6 pb-8 pt-6 md:pb-12 md:pt-10 lg:py-32">
          <div className="container mx-auto flex max-w-[64rem] flex-col items-center gap-4 text-center px-4 sm:px-6 lg:px-8">
            <Badge className="text-sm bg-cyan-500/10 text-cyan-600 dark:text-cyan-400 border-cyan-500/20">
              Built with Bun for Maximum Performance
            </Badge>

            <h1 className="font-bold text-3xl sm:text-5xl md:text-6xl lg:text-7xl">
              <span className="bg-gradient-to-br from-black to-gray-600 bg-clip-text text-transparent dark:from-white dark:to-gray-300">
                HTTP Request Management
              </span>
              <br />
              <span className="bg-gradient-to-br from-cyan-400 to-cyan-600 bg-clip-text text-transparent dark:from-cyan-300 dark:to-cyan-500">
                Made Simple
              </span>
            </h1>

            <p className="max-w-[42rem] leading-normal text-muted-foreground sm:text-xl sm:leading-8">
              A powerful CLI tool for managing HTTP requests using YAML configuration files. Perfect
              for API testing, automation, and development workflows.
            </p>

            <div className="flex gap-4 items-center flex-col sm:flex-row">
              <Button asChild size="lg" className="h-11 bg-cyan-500 hover:bg-cyan-600 text-white">
                <Link href="/docs/quick-start">
                  <Play className="mr-2 h-4 w-4" />
                  Get Started
                </Link>
              </Button>

              <Button variant="outline" size="lg" className="h-11" asChild>
                <Link href="/docs">
                  View Documentation
                  <ArrowRight className="ml-2 h-4 w-4" />
                </Link>
              </Button>
            </div>
          </div>
        </section>

        {/* Code Preview */}
        <section className="container mx-auto max-w-[64rem] py-8 md:py-12 lg:py-24 px-4 sm:px-6 lg:px-8">
          <div className="grid gap-8 lg:grid-cols-2 lg:gap-12 items-center">
            <div className="space-y-4">
              <h2 className="text-3xl font-bold">Simple YAML Configuration</h2>
              <p className="text-muted-foreground text-lg">
                Define your HTTP requests using clean, readable YAML files. Support for variables,
                templates, parallel execution, and response validation.
              </p>
            </div>
            <div className="space-y-4">
              <CodeBlockServer language="yaml" filename="api-tests.yaml">
                {exampleYaml}
              </CodeBlockServer>
            </div>
          </div>
        </section>

        {/* Installation */}
        <section className="container mx-auto max-w-[64rem] py-8 md:py-12 lg:py-24 px-4 sm:px-6 lg:px-8">
          <div className="grid gap-8 lg:grid-cols-2 lg:gap-12 items-center">
            <div className="space-y-4 order-2 lg:order-1">
              <div className="space-y-4">
                <div>
                  <h3 className="text-lg font-semibold mb-2">Install</h3>
                  <CodeBlockServer language="bash">{installCommand}</CodeBlockServer>
                </div>

                <div>
                  <h3 className="text-lg font-semibold mb-2">Run</h3>
                  <CodeBlockServer language="bash">{runCommand}</CodeBlockServer>
                </div>
              </div>
            </div>
            <div className="space-y-4 order-1 lg:order-2">
              <h2 className="text-3xl font-bold">Quick to Install & Use</h2>
              <p className="text-muted-foreground text-lg">
                Get started in seconds with your preferred package manager. Works on Linux, macOS,
                and Windows with full cross-platform support.
              </p>
            </div>
          </div>
        </section>

        {/* Bento Grid Features */}
        <section className="py-24 sm:py-32">
          <div className="mx-auto max-w-2xl px-6 lg:max-w-7xl lg:px-8">
            <div className="text-center">
              <Badge className="text-sm mb-4 bg-cyan-500/10 text-cyan-600 dark:text-cyan-400 border-cyan-500/20">
                Powerful Features
              </Badge>
              <h2 className="text-4xl font-bold tracking-tight sm:text-5xl md:text-6xl">
                <span className="bg-gradient-to-br from-black to-gray-600 bg-clip-text text-transparent dark:from-white dark:to-gray-300">
                  Everything you need for
                </span>
                <br />
                <span className="bg-gradient-to-br from-cyan-400 to-cyan-600 bg-clip-text text-transparent">
                  HTTP request management
                </span>
              </h2>
              <p className="mx-auto mt-4 max-w-2xl text-lg text-muted-foreground">
                A comprehensive toolkit designed for developers who need reliable API testing and
                automation
              </p>
            </div>

            <div className="mt-16 grid gap-4 lg:grid-cols-3 lg:grid-rows-2">
              {/* YAML Configuration - Large Left Card */}
              <div className="relative lg:row-span-2">
                <div className="absolute inset-px rounded-lg bg-card lg:rounded-l-3xl" />
                <div className="relative flex h-full flex-col overflow-hidden rounded-lg lg:rounded-l-3xl border">
                  <div className="px-8 pt-8 pb-3 sm:px-10 sm:pt-10 sm:pb-0">
                    <div className="flex items-center gap-2 mb-4">
                      <FileText className="h-6 w-6 text-cyan-500" />
                      <Badge className="bg-cyan-500/10 text-cyan-600 dark:text-cyan-400 border-cyan-500/20">
                        YAML Config
                      </Badge>
                    </div>
                    <p className="text-lg font-semibold tracking-tight">Simple Configuration</p>
                    <p className="mt-2 text-sm text-muted-foreground">
                      Define HTTP requests using clean, readable YAML files with support for
                      variables, templates, and validation.
                    </p>
                  </div>
                  <div className="flex-1 px-8 pb-8 sm:px-10 sm:pb-10">
                    <div className="mt-6 rounded-lg bg-slate-100 dark:bg-zinc-950/70 backdrop-blur-sm border border-slate-200 dark:border-zinc-800 p-4 font-mono text-xs">
                      <div className="text-slate-600 dark:text-gray-500"># api-test.yaml</div>
                      <div className="mt-2">
                        <span className="text-cyan-600 dark:text-cyan-400">requests</span>:
                      </div>
                      <div className="ml-2">
                        - <span className="text-cyan-600 dark:text-cyan-400">name</span>:{' '}
                        <span className="text-green-600 dark:text-green-400">Get User</span>
                      </div>
                      <div className="ml-4">
                        <span className="text-cyan-600 dark:text-cyan-400">url</span>:{' '}
                        <span className="text-amber-600 dark:text-yellow-400">
                          ${`{BASE_URL}`}/users/1
                        </span>
                      </div>
                      <div className="ml-4">
                        <span className="text-cyan-600 dark:text-cyan-400">method</span>:{' '}
                        <span className="text-orange-600 dark:text-orange-400">GET</span>
                      </div>
                      <div className="ml-4">
                        <span className="text-cyan-600 dark:text-cyan-400">headers</span>:
                      </div>
                      <div className="ml-6">
                        <span className="text-cyan-600 dark:text-cyan-400">Authorization</span>:{' '}
                        <span className="text-slate-700 dark:text-slate-300">
                          Bearer ${`{TOKEN}`}
                        </span>
                      </div>
                      <div className="ml-4">
                        <span className="text-cyan-600 dark:text-cyan-400">validation</span>:
                      </div>
                      <div className="ml-6">
                        <span className="text-cyan-600 dark:text-cyan-400">status</span>:{' '}
                        <span className="text-purple-600 dark:text-purple-400">200</span>
                      </div>
                      <div className="ml-6">
                        <span className="text-cyan-600 dark:text-cyan-400">body</span>:
                      </div>
                      <div className="ml-8">
                        <span className="text-cyan-600 dark:text-cyan-400">id</span>:{' '}
                        <span className="text-purple-600 dark:text-purple-400">1</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              {/* Performance - Top Middle Card */}
              <div className="relative max-lg:row-start-1">
                <div className="absolute inset-px rounded-lg bg-card max-lg:rounded-t-3xl" />
                <div className="relative flex h-full flex-col overflow-hidden rounded-lg max-lg:rounded-t-3xl border">
                  <div className="px-8 pt-8 sm:px-10 sm:pt-10">
                    <div className="flex items-center gap-2 mb-4">
                      <Zap className="h-6 w-6 text-yellow-500" />
                      <Badge className="bg-yellow-500/10 text-yellow-600 dark:text-yellow-400 border-yellow-500/20">
                        Blazing Fast
                      </Badge>
                    </div>
                    <p className="text-lg font-semibold tracking-tight">Built with Bun</p>
                    <p className="mt-2 text-sm text-muted-foreground">
                      Exceptional performance for sequential and parallel request execution.
                    </p>
                  </div>
                  <div className="flex flex-1 items-center justify-center px-8 pb-8">
                    <div className="text-center">
                      <div className="text-4xl font-bold text-cyan-500">10x</div>
                      <div className="text-sm text-muted-foreground mt-2">
                        Faster than traditional tools
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              {/* CLI Interface - Bottom Middle Card */}
              <div className="relative max-lg:row-start-3 lg:col-start-2 lg:row-start-2">
                <div className="absolute inset-px rounded-lg bg-card" />
                <div className="relative flex h-full flex-col overflow-hidden rounded-lg border">
                  <div className="px-8 pt-8 sm:px-10 sm:pt-10">
                    <div className="flex items-center gap-2 mb-4">
                      <Terminal className="h-6 w-6 text-green-500" />
                      <Badge className="bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20">
                        CLI
                      </Badge>
                    </div>
                    <p className="text-lg font-semibold tracking-tight">Beautiful Output</p>
                    <p className="mt-2 text-sm text-muted-foreground">
                      Progress indicators, colored output, and comprehensive error handling.
                    </p>
                  </div>
                  <div className="flex-1 p-8 flex items-end">
                    <div className="w-full rounded-lg bg-slate-100 dark:bg-zinc-950/70 backdrop-blur-sm border border-slate-200 dark:border-zinc-800 p-3 font-mono text-xs">
                      <div>
                        <span className="text-green-600 dark:text-green-400">✓</span>{' '}
                        <span className="text-slate-700 dark:text-slate-300">Get Users</span>{' '}
                        <span className="text-slate-500 dark:text-gray-500">[200 OK]</span>{' '}
                        <span className="text-cyan-600 dark:text-cyan-400">125ms</span>
                      </div>
                      <div className="mt-1">
                        <span className="text-green-600 dark:text-green-400">✓</span>{' '}
                        <span className="text-slate-700 dark:text-slate-300">Create Post</span>{' '}
                        <span className="text-slate-500 dark:text-gray-500">[201 Created]</span>{' '}
                        <span className="text-cyan-600 dark:text-cyan-400">89ms</span>
                      </div>
                      <div className="mt-1">
                        <span className="text-amber-600 dark:text-yellow-400">⚡</span>{' '}
                        <span className="text-slate-700 dark:text-slate-300">
                          Running 5 parallel requests...
                        </span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              {/* Powerful Features - Large Right Card */}
              <div className="relative lg:row-span-2">
                <div className="absolute inset-px rounded-lg bg-card max-lg:rounded-b-3xl lg:rounded-r-3xl" />
                <div className="relative flex h-full flex-col overflow-hidden rounded-lg max-lg:rounded-b-3xl lg:rounded-r-3xl border">
                  <div className="px-8 pt-8 pb-3 sm:px-10 sm:pt-10 sm:pb-0">
                    <div className="flex items-center gap-2 mb-4">
                      <Settings className="h-6 w-6 text-purple-500" />
                      <Badge className="bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-500/20">
                        Advanced
                      </Badge>
                    </div>
                    <p className="text-lg font-semibold tracking-tight">Flexible & Extensible</p>
                    <p className="mt-2 text-sm text-muted-foreground">
                      Global settings, variable interpolation, retries, and custom output formats.
                    </p>
                  </div>
                  <div className="flex-1 px-8 pb-8 sm:px-10 sm:pb-10">
                    <div className="mt-6 space-y-4">
                      <div className="flex items-center gap-3">
                        <div className="h-8 w-8 rounded-lg bg-cyan-500/10 flex items-center justify-center">
                          <Play className="h-4 w-4 text-cyan-500" />
                        </div>
                        <div className="flex-1">
                          <div className="text-sm font-medium">Parallel Execution</div>
                          <div className="text-xs text-muted-foreground">
                            Run multiple requests simultaneously
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-3">
                        <div className="h-8 w-8 rounded-lg bg-green-500/10 flex items-center justify-center">
                          <ArrowRight className="h-4 w-4 text-green-500" />
                        </div>
                        <div className="flex-1">
                          <div className="text-sm font-medium">Response Validation</div>
                          <div className="text-xs text-muted-foreground">
                            Assert status codes and body content
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-3">
                        <div className="h-8 w-8 rounded-lg bg-purple-500/10 flex items-center justify-center">
                          <Download className="h-4 w-4 text-purple-500" />
                        </div>
                        <div className="flex-1">
                          <div className="text-sm font-medium">Export Results</div>
                          <div className="text-xs text-muted-foreground">
                            Save to JSON, CSV, or custom formats
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-3">
                        <div className="h-8 w-8 rounded-lg bg-yellow-500/10 flex items-center justify-center">
                          <Zap className="h-4 w-4 text-yellow-500" />
                        </div>
                        <div className="flex-1">
                          <div className="text-sm font-medium">Auto Retry</div>
                          <div className="text-xs text-muted-foreground">
                            Configurable retry policies
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Contributors Section */}
        <Contributors />

        {/* CTA */}
        <section className="container mx-auto max-w-[64rem] py-8 md:py-12 lg:py-24 px-4 sm:px-6 lg:px-8">
          <div className="mx-auto flex max-w-[58rem] flex-col items-center justify-center gap-6 text-center">
            <h2 className="font-bold text-3xl leading-[1.1] sm:text-3xl md:text-6xl">
              Ready to get started?
            </h2>
            <p className="max-w-[85%] leading-normal text-muted-foreground sm:text-lg sm:leading-7">
              Join developers who trust <CurlRunner /> for their HTTP request management needs.
            </p>
            <div className="flex gap-4 items-center flex-col sm:flex-row mt-4">
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
