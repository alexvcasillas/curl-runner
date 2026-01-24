import { ArrowRight, CheckCircle, Clock, Gauge, Layers, Zap } from 'lucide-react';
import type { Metadata } from 'next';
import Link from 'next/link';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { BreadcrumbSchema, DocumentationArticleSchema } from '@/components/structured-data';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

export const metadata: Metadata = {
	title: 'Performance Testing with Parallel Execution',
	description:
		'Speed up your test suites by running requests in parallel. Learn when to use parallel vs sequential execution and how to configure performance thresholds.',
	keywords: [
		'curl-runner parallel',
		'parallel execution',
		'performance testing',
		'load testing',
		'concurrent requests',
		'API performance',
	],
	openGraph: {
		title: 'Parallel Execution | curl-runner Tutorial',
		description:
			'Speed up your test suites by running requests in parallel with curl-runner.',
		url: 'https://www.curl-runner.com/docs/tutorials/parallel-execution',
	},
	alternates: {
		canonical: 'https://www.curl-runner.com/docs/tutorials/parallel-execution',
	},
};

const sequentialExample = `# sequential.yaml (default behavior)
global:
  execution: sequential  # This is the default

requests:
  - name: Request 1
    url: https://api.example.com/endpoint1
    method: GET

  - name: Request 2
    url: https://api.example.com/endpoint2
    method: GET

  - name: Request 3
    url: https://api.example.com/endpoint3
    method: GET

# Runs in order: 1 → 2 → 3
# Total time ≈ sum of all request times`;

const parallelExample = `# parallel.yaml
global:
  execution: parallel

requests:
  - name: Request 1
    url: https://api.example.com/endpoint1
    method: GET

  - name: Request 2
    url: https://api.example.com/endpoint2
    method: GET

  - name: Request 3
    url: https://api.example.com/endpoint3
    method: GET

# Runs simultaneously: 1, 2, 3
# Total time ≈ longest request time`;

const cliParallelExample = `# Run in parallel using CLI flag
curl-runner tests/ -p

# Or with long form
curl-runner tests/ --execution parallel

# Force sequential (override YAML)
curl-runner tests/ -s
curl-runner tests/ --sequential`;

const healthCheckExample = `# health-checks.yaml
global:
  execution: parallel
  continueOnError: true  # Don't stop on first failure

requests:
  - name: Main API
    url: https://api.example.com/health
    method: GET
    expect:
      status: 200
      responseTime: "< 500"

  - name: Auth Service
    url: https://auth.example.com/health
    method: GET
    expect:
      status: 200
      responseTime: "< 500"

  - name: Database Proxy
    url: https://db.example.com/health
    method: GET
    expect:
      status: 200
      responseTime: "< 1000"

  - name: Cache Service
    url: https://cache.example.com/health
    method: GET
    expect:
      status: 200
      responseTime: "< 200"

  - name: Search Service
    url: https://search.example.com/health
    method: GET
    expect:
      status: 200
      responseTime: "< 800"`;

const performanceValidationExample = `# performance-test.yaml
global:
  execution: parallel

requests:
  - name: Fast Endpoint
    url: https://api.example.com/fast
    method: GET
    expect:
      status: 200
      responseTime: "< 100"   # Must respond in under 100ms

  - name: Normal Endpoint
    url: https://api.example.com/data
    method: GET
    expect:
      status: 200
      responseTime: "< 500"   # Must respond in under 500ms

  - name: Heavy Endpoint
    url: https://api.example.com/report
    method: GET
    expect:
      status: 200
      responseTime: "< 2000"  # Allow up to 2 seconds`;

const loadPatternExample = `# load-test.yaml
global:
  execution: parallel
  variables:
    BASE_URL: https://api.example.com

# Simulate multiple concurrent users
requests:
  - name: User 1 - Browse
    url: \${BASE_URL}/products
    method: GET
    expect:
      status: 200

  - name: User 2 - Browse
    url: \${BASE_URL}/products
    method: GET
    expect:
      status: 200

  - name: User 3 - Search
    url: \${BASE_URL}/search?q=test
    method: GET
    expect:
      status: 200

  - name: User 4 - View Product
    url: \${BASE_URL}/products/1
    method: GET
    expect:
      status: 200

  - name: User 5 - Add to Cart
    url: \${BASE_URL}/cart
    method: POST
    headers:
      Content-Type: application/json
    body:
      productId: 1
      quantity: 1
    expect:
      status: [200, 201]`;

const mixedExample = `# mixed-workflow.yaml
# Use sequential for the setup, parallel for the tests

# File 1: setup.yaml
global:
  execution: sequential

requests:
  - name: Login
    url: https://api.example.com/auth/login
    method: POST
    body:
      email: test@example.com
      password: testpass
    store:
      token: body.token

  - name: Create Test Data
    url: https://api.example.com/test-data
    method: POST
    headers:
      Authorization: Bearer \${store.token}
    store:
      testId: body.id`;

const parallelTestsExample = `# File 2: parallel-tests.yaml
global:
  execution: parallel
  variables:
    TOKEN: \${AUTH_TOKEN}  # From environment

requests:
  - name: Test Endpoint A
    url: https://api.example.com/a
    method: GET
    headers:
      Authorization: Bearer \${TOKEN}
    expect:
      status: 200

  - name: Test Endpoint B
    url: https://api.example.com/b
    method: GET
    headers:
      Authorization: Bearer \${TOKEN}
    expect:
      status: 200

  - name: Test Endpoint C
    url: https://api.example.com/c
    method: GET
    headers:
      Authorization: Bearer \${TOKEN}
    expect:
      status: 200`;

const metricsExample = `# View detailed metrics
curl-runner tests/ -p --show-metrics -v

# Save metrics to file
curl-runner tests/ -p --output results.json --show-metrics

# Example output includes:
# - duration: Total request time
# - dnsLookup: DNS resolution time
# - tcpConnection: TCP connection time
# - tlsHandshake: TLS handshake time
# - firstByte: Time to first byte
# - download: Download time`;

const ciParallelExample = `# .github/workflows/parallel-tests.yml
name: Parallel API Tests

on: [push, pull_request]

jobs:
  api-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v1

      - name: Install curl-runner
        run: bun install -g @curl-runner/cli

      - name: Run Health Checks (Parallel)
        run: curl-runner health-checks/ -p --strict-exit

      - name: Run Integration Tests (Sequential)
        run: curl-runner integration/ -s --strict-exit`;

export default function ParallelExecutionPage() {
	return (
		<main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
			<DocumentationArticleSchema
				title="Performance Testing with Parallel Execution"
				description="Speed up your test suites by running requests in parallel with curl-runner."
				url="https://www.curl-runner.com/docs/tutorials/parallel-execution"
				section="Tutorials"
			/>
			<BreadcrumbSchema
				items={[
					{ name: 'Home', url: 'https://www.curl-runner.com' },
					{ name: 'Documentation', url: 'https://www.curl-runner.com/docs' },
					{ name: 'Tutorials', url: 'https://www.curl-runner.com/docs/tutorials' },
					{
						name: 'Parallel Execution',
						url: 'https://www.curl-runner.com/docs/tutorials/parallel-execution',
					},
				]}
			/>

			<div className="mx-auto w-full min-w-0">
				<DocsPageHeader
					heading="Parallel Execution"
					text="Speed up your test suites by running requests in parallel"
				/>

				<div className="flex items-center gap-4 mb-8">
					<Badge variant="secondary">Intermediate</Badge>
					<span className="text-sm text-muted-foreground">15 min read</span>
				</div>

				<div className="space-y-8">
					{/* What You'll Learn */}
					<section>
						<H2 id="what-youll-learn">What You'll Learn</H2>
						<div className="grid gap-3 sm:grid-cols-2">
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Sequential vs parallel execution</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Configure parallel mode</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Performance validation</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Load testing patterns</span>
							</div>
						</div>
					</section>

					{/* Sequential vs Parallel */}
					<section>
						<H2 id="comparison">Sequential vs Parallel</H2>

						<div className="grid gap-6 md:grid-cols-2 mb-6">
							<div className="rounded-lg border p-4">
								<div className="flex items-center gap-2 mb-3">
									<Layers className="h-5 w-5 text-blue-600" />
									<h3 className="font-semibold">Sequential (Default)</h3>
								</div>
								<ul className="text-sm text-muted-foreground space-y-1">
									<li>• Requests run one after another</li>
									<li>• Response storage works</li>
									<li>• Predictable execution order</li>
									<li>• Use for dependent requests</li>
								</ul>
							</div>

							<div className="rounded-lg border p-4">
								<div className="flex items-center gap-2 mb-3">
									<Zap className="h-5 w-5 text-green-600" />
									<h3 className="font-semibold">Parallel</h3>
								</div>
								<ul className="text-sm text-muted-foreground space-y-1">
									<li>• Requests run simultaneously</li>
									<li>• Much faster execution</li>
									<li>• Response storage won't work</li>
									<li>• Use for independent requests</li>
								</ul>
							</div>
						</div>

						<H3 id="sequential-mode">Sequential Mode</H3>
						<CodeBlockServer language="yaml" filename="sequential.yaml">
							{sequentialExample}
						</CodeBlockServer>

						<H3 id="parallel-mode">Parallel Mode</H3>
						<p className="text-muted-foreground mb-4 mt-6">
							Enable parallel execution in your YAML:
						</p>
						<CodeBlockServer language="yaml" filename="parallel.yaml">
							{parallelExample}
						</CodeBlockServer>
					</section>

					{/* CLI Usage */}
					<section>
						<H2 id="cli-usage">CLI Options</H2>
						<p className="text-muted-foreground mb-4">
							Override execution mode from the command line:
						</p>
						<CodeBlockServer language="bash">{cliParallelExample}</CodeBlockServer>
					</section>

					{/* Health Checks */}
					<section>
						<H2 id="health-checks">Health Check Pattern</H2>
						<p className="text-muted-foreground mb-4">
							Parallel execution is perfect for checking multiple services at once:
						</p>
						<CodeBlockServer language="yaml" filename="health-checks.yaml">
							{healthCheckExample}
						</CodeBlockServer>

						<div className="mt-4 rounded-lg border bg-green-500/5 border-green-500/20 p-4">
							<div className="flex items-start gap-2">
								<Clock className="h-5 w-5 text-green-600 mt-0.5" />
								<div>
									<h4 className="font-medium text-green-600 dark:text-green-400">
										Performance Benefit
									</h4>
									<p className="text-sm text-muted-foreground mt-1">
										5 requests taking 500ms each: Sequential = ~2.5s, Parallel = ~500ms
									</p>
								</div>
							</div>
						</div>
					</section>

					{/* Performance Validation */}
					<section>
						<H2 id="performance-validation">Performance Validation</H2>
						<p className="text-muted-foreground mb-4">
							Validate that endpoints respond within acceptable time limits:
						</p>
						<CodeBlockServer language="yaml" filename="performance-test.yaml">
							{performanceValidationExample}
						</CodeBlockServer>

						<div className="mt-6 border rounded-lg overflow-hidden">
							<table className="w-full">
								<thead className="bg-muted/50">
									<tr className="border-b">
										<th className="text-left p-3 font-medium">Validation</th>
										<th className="text-left p-3 font-medium">Meaning</th>
									</tr>
								</thead>
								<tbody>
									<tr className="border-b">
										<td className="p-3">
											<code className="text-sm">responseTime: "{'<'} 100"</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">
											Must complete in under 100ms
										</td>
									</tr>
									<tr className="border-b">
										<td className="p-3">
											<code className="text-sm">responseTime: "{'<='} 500"</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">
											Must complete in 500ms or less
										</td>
									</tr>
									<tr>
										<td className="p-3">
											<code className="text-sm">responseTime: "100-500"</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">
											Must complete between 100-500ms
										</td>
									</tr>
								</tbody>
							</table>
						</div>
					</section>

					{/* Load Testing */}
					<section>
						<H2 id="load-testing">Load Testing Patterns</H2>
						<p className="text-muted-foreground mb-4">
							Simulate multiple concurrent users accessing your API:
						</p>
						<CodeBlockServer language="yaml" filename="load-test.yaml">
							{loadPatternExample}
						</CodeBlockServer>

						<div className="mt-4 rounded-lg border bg-yellow-500/5 border-yellow-500/20 p-4">
							<p className="text-sm text-muted-foreground">
								<strong className="text-yellow-600 dark:text-yellow-400">Note:</strong> curl-runner
								is designed for functional testing with moderate load. For extensive load testing
								with thousands of concurrent users, consider specialized tools like k6 or Artillery.
							</p>
						</div>
					</section>

					{/* Mixed Workflows */}
					<section>
						<H2 id="mixed-workflows">Mixed Workflows</H2>
						<p className="text-muted-foreground mb-4">
							Combine sequential setup with parallel tests using separate files:
						</p>

						<H3 id="setup-file">Setup File (Sequential)</H3>
						<CodeBlockServer language="yaml" filename="setup.yaml">
							{mixedExample}
						</CodeBlockServer>

						<H3 id="test-file">Test File (Parallel)</H3>
						<CodeBlockServer language="yaml" filename="parallel-tests.yaml">
							{parallelTestsExample}
						</CodeBlockServer>

						<p className="text-sm text-muted-foreground mt-4">
							Run them in order:{' '}
							<code className="bg-muted px-1 rounded">curl-runner setup.yaml && curl-runner parallel-tests.yaml -p</code>
						</p>
					</section>

					{/* Metrics */}
					<section>
						<H2 id="metrics">Viewing Performance Metrics</H2>
						<p className="text-muted-foreground mb-4">
							Get detailed timing information for performance analysis:
						</p>
						<CodeBlockServer language="bash">{metricsExample}</CodeBlockServer>
					</section>

					{/* CI/CD */}
					<section>
						<H2 id="ci-integration">CI/CD Integration</H2>
						<p className="text-muted-foreground mb-4">
							Use parallel execution in your CI pipeline for faster test runs:
						</p>
						<CodeBlockServer language="yaml" filename=".github/workflows/parallel-tests.yml">
							{ciParallelExample}
						</CodeBlockServer>
					</section>

					{/* When to Use */}
					<section>
						<H2 id="when-to-use">When to Use Each Mode</H2>
						<div className="grid gap-4 md:grid-cols-2">
							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-blue-500/10 p-2">
										<Layers className="h-4 w-4 text-blue-600" />
									</div>
									<div>
										<h4 className="font-medium mb-2">Use Sequential When:</h4>
										<ul className="text-sm text-muted-foreground space-y-1">
											<li>• Requests depend on each other</li>
											<li>• Using response storage</li>
											<li>• Testing specific workflows</li>
											<li>• Order matters (CRUD operations)</li>
										</ul>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-green-500/10 p-2">
										<Zap className="h-4 w-4 text-green-600" />
									</div>
									<div>
										<h4 className="font-medium mb-2">Use Parallel When:</h4>
										<ul className="text-sm text-muted-foreground space-y-1">
											<li>• Requests are independent</li>
											<li>• Running health checks</li>
											<li>• Performance testing</li>
											<li>• Simulating concurrent users</li>
										</ul>
									</div>
								</div>
							</div>
						</div>
					</section>

					{/* Best Practices */}
					<section>
						<H2 id="best-practices">Best Practices</H2>
						<div className="grid gap-4 md:grid-cols-2">
							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-green-500/10 p-2">
										<Gauge className="h-4 w-4 text-green-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Set Realistic Thresholds</h4>
										<p className="text-sm text-muted-foreground">
											Base response time limits on actual performance requirements.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-blue-500/10 p-2">
										<Clock className="h-4 w-4 text-blue-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Use continueOnError</h4>
										<p className="text-sm text-muted-foreground">
											In parallel mode, continue checking all services even if one fails.
										</p>
									</div>
								</div>
							</div>
						</div>
					</section>

					{/* Next Steps */}
					<section>
						<H2 id="next-steps">Next Steps</H2>
						<div className="grid gap-4 sm:grid-cols-2">
							<div className="rounded-lg border bg-card p-4">
								<h4 className="font-medium mb-1">Advanced Validation</h4>
								<p className="text-sm text-muted-foreground mb-2">
									Learn complex validation patterns
								</p>
								<Button asChild variant="ghost" size="sm" className="p-0">
									<Link href="/docs/tutorials/advanced-validation" className="flex items-center">
										Read Tutorial <ArrowRight className="ml-1 h-3 w-3" />
									</Link>
								</Button>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<h4 className="font-medium mb-1">CI/CD Integration</h4>
								<p className="text-sm text-muted-foreground mb-2">
									Set up automated testing pipelines
								</p>
								<Button asChild variant="ghost" size="sm" className="p-0">
									<Link href="/docs/tutorials/ci-cd-integration" className="flex items-center">
										Read Tutorial <ArrowRight className="ml-1 h-3 w-3" />
									</Link>
								</Button>
							</div>
						</div>
					</section>
				</div>
			</div>

			{/* Table of Contents */}
			<div className="hidden text-sm xl:block">
				<div className="sticky top-16 -mt-10 pt-4">
					<TableOfContents />
				</div>
			</div>
		</main>
	);
}
