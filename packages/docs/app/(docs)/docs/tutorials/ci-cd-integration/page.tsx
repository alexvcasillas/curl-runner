import { ArrowRight, CheckCircle, FileCode, GitBranch, Settings, Terminal } from 'lucide-react';
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
	title: 'CI/CD Integration with GitHub Actions',
	description:
		'Set up curl-runner in your CI/CD pipeline. Configure exit codes, failure thresholds, and integrate with GitHub Actions for automated API testing.',
	keywords: [
		'curl-runner CI/CD',
		'GitHub Actions API testing',
		'automated API testing',
		'CI pipeline integration',
		'exit codes',
		'failure thresholds',
	],
	openGraph: {
		title: 'CI/CD Integration | curl-runner Tutorial',
		description:
			'Set up curl-runner in GitHub Actions and configure exit codes for your pipeline.',
		url: 'https://www.curl-runner.com/docs/tutorials/ci-cd-integration',
	},
	alternates: {
		canonical: 'https://www.curl-runner.com/docs/tutorials/ci-cd-integration',
	},
};

const basicGitHubAction = `# .github/workflows/api-tests.yml
name: API Tests

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  api-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v1

      - name: Install curl-runner
        run: bun install -g @curl-runner/cli

      - name: Run API Tests
        run: curl-runner tests/`;

const withSecretsExample = `# .github/workflows/api-tests.yml
name: API Tests

on:
  push:
    branches: [main]

jobs:
  api-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v1

      - name: Install curl-runner
        run: bun install -g @curl-runner/cli

      - name: Run API Tests
        env:
          API_TOKEN: \${{ secrets.API_TOKEN }}
          BASE_URL: \${{ vars.API_BASE_URL }}
        run: curl-runner tests/ --strict-exit`;

const exitCodesYamlExample = `# tests/api-health.yaml
global:
  ci:
    strictExit: true        # Exit 1 if ANY test fails
    # OR use thresholds:
    # failOn: 5             # Exit 1 if more than 5 failures
    # failOnPercentage: 10  # Exit 1 if >10% fail

requests:
  - name: Health Check
    url: \${BASE_URL}/health
    method: GET
    expect:
      status: 200

  - name: Auth Endpoint
    url: \${BASE_URL}/auth/status
    method: GET
    headers:
      Authorization: Bearer \${API_TOKEN}
    expect:
      status: 200`;

const cliExitCodesExample = `# Strict mode: fail on any error
curl-runner tests/ --strict-exit

# Fail if more than 5 requests fail
curl-runner tests/ --fail-on 5

# Fail if more than 10% of requests fail
curl-runner tests/ --fail-on-percentage 10

# Combine with verbose output
curl-runner tests/ --strict-exit -v`;

const envVarsExample = `# Environment variables curl-runner recognizes
export CURL_RUNNER_TIMEOUT=30000
export CURL_RUNNER_RETRIES=3
export CURL_RUNNER_STRICT_EXIT=true
export CURL_RUNNER_VERBOSE=true

# Your custom variables for tests
export API_TOKEN="your-token"
export BASE_URL="https://api.staging.example.com"`;

const yamlEnvVarsExample = `# tests/staging.yaml
global:
  variables:
    # Reference environment variables
    BASE_URL: \${BASE_URL}
    API_TOKEN: \${API_TOKEN}

    # Or use defaults with fallback
    TIMEOUT: \${CURL_RUNNER_TIMEOUT:-5000}

requests:
  - name: Test Endpoint
    url: \${BASE_URL}/api/test
    method: GET
    headers:
      Authorization: Bearer \${API_TOKEN}
    timeout: \${TIMEOUT}
    expect:
      status: 200`;

const matrixTestingExample = `# .github/workflows/api-tests.yml
name: API Tests - Multi Environment

on:
  push:
    branches: [main]
  schedule:
    - cron: '0 */6 * * *'  # Every 6 hours

jobs:
  api-tests:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        environment: [staging, production]
        include:
          - environment: staging
            base_url: https://api.staging.example.com
          - environment: production
            base_url: https://api.example.com

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v1

      - name: Install curl-runner
        run: bun install -g @curl-runner/cli

      - name: Run API Tests - \${{ matrix.environment }}
        env:
          BASE_URL: \${{ matrix.base_url }}
          API_TOKEN: \${{ secrets[format('API_TOKEN_{0}', matrix.environment)] }}
        run: |
          echo "Testing \${{ matrix.environment }} environment"
          curl-runner tests/ --strict-exit -v`;

const outputArtifactsExample = `# .github/workflows/api-tests.yml
name: API Tests with Artifacts

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

      - name: Run API Tests
        env:
          API_TOKEN: \${{ secrets.API_TOKEN }}
        run: |
          curl-runner tests/ \\
            --strict-exit \\
            --output results.json \\
            --output-format json \\
            -v
        continue-on-error: true

      - name: Upload Test Results
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: api-test-results
          path: results.json
          retention-days: 30`;

const scheduledTestsExample = `# .github/workflows/scheduled-api-tests.yml
name: Scheduled API Health Checks

on:
  schedule:
    - cron: '*/15 * * * *'  # Every 15 minutes
  workflow_dispatch:        # Allow manual triggers

jobs:
  health-check:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v1

      - name: Install curl-runner
        run: bun install -g @curl-runner/cli

      - name: Run Health Checks
        env:
          BASE_URL: \${{ vars.PRODUCTION_URL }}
        run: curl-runner health-checks/ --strict-exit

      - name: Notify on Failure
        if: failure()
        uses: slackapi/slack-github-action@v1
        with:
          payload: |
            {
              "text": "API Health Check Failed!",
              "blocks": [
                {
                  "type": "section",
                  "text": {
                    "type": "mrkdwn",
                    "text": ":warning: *API Health Check Failed*\\nCheck the <\${{ github.server_url }}/\${{ github.repository }}/actions/runs/\${{ github.run_id }}|workflow run> for details."
                  }
                }
              ]
            }
        env:
          SLACK_WEBHOOK_URL: \${{ secrets.SLACK_WEBHOOK }}`;

const prCommentExample = `# .github/workflows/pr-api-tests.yml
name: PR API Tests

on:
  pull_request:
    branches: [main]

jobs:
  api-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v1

      - name: Install curl-runner
        run: bun install -g @curl-runner/cli

      - name: Run API Tests
        id: tests
        env:
          BASE_URL: \${{ vars.STAGING_URL }}
        run: |
          curl-runner tests/ --output results.json --output-format json
          echo "results<<EOF" >> $GITHUB_OUTPUT
          cat results.json >> $GITHUB_OUTPUT
          echo "EOF" >> $GITHUB_OUTPUT
        continue-on-error: true

      - name: Comment on PR
        uses: actions/github-script@v7
        with:
          script: |
            const results = JSON.parse(\`\${{ steps.tests.outputs.results }}\`);
            const passed = results.filter(r => r.success).length;
            const failed = results.filter(r => !r.success).length;

            const body = \`## API Test Results

            | Status | Count |
            |--------|-------|
            | ✅ Passed | \${passed} |
            | ❌ Failed | \${failed} |

            \${failed > 0 ? '⚠️ Some tests failed. Please review before merging.' : '🎉 All tests passed!'}\`;

            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: body
            });`;

export default function CiCdIntegrationPage() {
	return (
		<main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
			<DocumentationArticleSchema
				title="CI/CD Integration with GitHub Actions"
				description="Set up curl-runner in your CI/CD pipeline for automated API testing."
				url="https://www.curl-runner.com/docs/tutorials/ci-cd-integration"
				section="Tutorials"
			/>
			<BreadcrumbSchema
				items={[
					{ name: 'Home', url: 'https://www.curl-runner.com' },
					{ name: 'Documentation', url: 'https://www.curl-runner.com/docs' },
					{ name: 'Tutorials', url: 'https://www.curl-runner.com/docs/tutorials' },
					{
						name: 'CI/CD Integration',
						url: 'https://www.curl-runner.com/docs/tutorials/ci-cd-integration',
					},
				]}
			/>

			<div className="mx-auto w-full min-w-0">
				<DocsPageHeader
					heading="CI/CD Integration"
					text="Set up curl-runner in your CI/CD pipeline for automated API testing"
				/>

				<div className="flex items-center gap-4 mb-8">
					<Badge variant="secondary">Intermediate</Badge>
					<span className="text-sm text-muted-foreground">20 min read</span>
				</div>

				<div className="space-y-8">
					{/* What You'll Learn */}
					<section>
						<H2 id="what-youll-learn">What You'll Learn</H2>
						<div className="grid gap-3 sm:grid-cols-2">
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Set up GitHub Actions workflow</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Configure exit codes for CI</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Use secrets and environment variables</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Set up scheduled tests and notifications</span>
							</div>
						</div>
					</section>

					{/* Basic Setup */}
					<section>
						<H2 id="basic-setup">Basic GitHub Actions Setup</H2>
						<p className="text-muted-foreground mb-4">
							Create a workflow file to run your API tests on every push and pull request:
						</p>
						<CodeBlockServer language="yaml" filename=".github/workflows/api-tests.yml">
							{basicGitHubAction}
						</CodeBlockServer>
					</section>

					{/* With Secrets */}
					<section>
						<H2 id="secrets">Using Secrets and Variables</H2>
						<p className="text-muted-foreground mb-4">
							Use GitHub secrets for sensitive data like API tokens:
						</p>
						<CodeBlockServer language="yaml" filename=".github/workflows/api-tests.yml">
							{withSecretsExample}
						</CodeBlockServer>

						<div className="mt-4 rounded-lg border bg-muted/50 p-4">
							<h4 className="font-medium mb-2">Setting up secrets:</h4>
							<ol className="text-sm text-muted-foreground space-y-1 list-decimal list-inside">
								<li>Go to your repository Settings → Secrets and variables → Actions</li>
								<li>Click "New repository secret"</li>
								<li>Add API_TOKEN and other sensitive values</li>
								<li>Use repository variables for non-sensitive config like BASE_URL</li>
							</ol>
						</div>
					</section>

					{/* Exit Codes */}
					<section>
						<H2 id="exit-codes">Configuring Exit Codes</H2>
						<p className="text-muted-foreground mb-4">
							curl-runner returns exit code 0 by default even if tests fail. Configure exit codes
							to fail your CI pipeline when tests don't pass.
						</p>

						<H3 id="cli-exit-codes">CLI Options</H3>
						<CodeBlockServer language="bash">{cliExitCodesExample}</CodeBlockServer>

						<H3 id="yaml-exit-codes">YAML Configuration</H3>
						<p className="text-muted-foreground mb-4 mt-6">
							Configure exit behavior in your YAML files:
						</p>
						<CodeBlockServer language="yaml" filename="tests/api-health.yaml">
							{exitCodesYamlExample}
						</CodeBlockServer>

						<div className="mt-6 border rounded-lg overflow-hidden">
							<table className="w-full">
								<thead className="bg-muted/50">
									<tr className="border-b">
										<th className="text-left p-3 font-medium">Option</th>
										<th className="text-left p-3 font-medium">Description</th>
									</tr>
								</thead>
								<tbody>
									<tr className="border-b">
										<td className="p-3">
											<code className="text-sm">--strict-exit</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">
											Exit 1 if any request fails validation
										</td>
									</tr>
									<tr className="border-b">
										<td className="p-3">
											<code className="text-sm">--fail-on N</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">
											Exit 1 if more than N requests fail
										</td>
									</tr>
									<tr>
										<td className="p-3">
											<code className="text-sm">--fail-on-percentage N</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">
											Exit 1 if more than N% of requests fail
										</td>
									</tr>
								</tbody>
							</table>
						</div>
					</section>

					{/* Environment Variables */}
					<section>
						<H2 id="environment-variables">Environment Variables</H2>
						<p className="text-muted-foreground mb-4">
							curl-runner reads configuration from environment variables:
						</p>
						<CodeBlockServer language="bash">{envVarsExample}</CodeBlockServer>

						<H3 id="yaml-env-vars">Using in YAML</H3>
						<p className="text-muted-foreground mb-4 mt-6">
							Reference environment variables in your test files:
						</p>
						<CodeBlockServer language="yaml" filename="tests/staging.yaml">
							{yamlEnvVarsExample}
						</CodeBlockServer>
					</section>

					{/* Matrix Testing */}
					<section>
						<H2 id="matrix-testing">Multi-Environment Testing</H2>
						<p className="text-muted-foreground mb-4">
							Use GitHub Actions matrix to test against multiple environments:
						</p>
						<CodeBlockServer language="yaml" filename=".github/workflows/api-tests.yml">
							{matrixTestingExample}
						</CodeBlockServer>
					</section>

					{/* Artifacts */}
					<section>
						<H2 id="artifacts">Saving Test Results</H2>
						<p className="text-muted-foreground mb-4">
							Save test results as artifacts for debugging and reporting:
						</p>
						<CodeBlockServer language="yaml" filename=".github/workflows/api-tests.yml">
							{outputArtifactsExample}
						</CodeBlockServer>
					</section>

					{/* Scheduled Tests */}
					<section>
						<H2 id="scheduled-tests">Scheduled Health Checks</H2>
						<p className="text-muted-foreground mb-4">
							Run periodic health checks and get notified on failures:
						</p>
						<CodeBlockServer language="yaml" filename=".github/workflows/scheduled-api-tests.yml">
							{scheduledTestsExample}
						</CodeBlockServer>
					</section>

					{/* PR Comments */}
					<section>
						<H2 id="pr-comments">PR Comments with Results</H2>
						<p className="text-muted-foreground mb-4">
							Automatically comment on PRs with test results:
						</p>
						<CodeBlockServer language="yaml" filename=".github/workflows/pr-api-tests.yml">
							{prCommentExample}
						</CodeBlockServer>
					</section>

					{/* Best Practices */}
					<section>
						<H2 id="best-practices">Best Practices</H2>
						<div className="grid gap-4 md:grid-cols-2">
							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-green-500/10 p-2">
										<Settings className="h-4 w-4 text-green-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Use Timeouts</h4>
										<p className="text-sm text-muted-foreground">
											Set appropriate timeouts to avoid hanging CI jobs.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-blue-500/10 p-2">
										<GitBranch className="h-4 w-4 text-blue-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Separate Test Files</h4>
										<p className="text-sm text-muted-foreground">
											Organize tests by feature or endpoint for easier maintenance.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-purple-500/10 p-2">
										<FileCode className="h-4 w-4 text-purple-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Version Your Tests</h4>
										<p className="text-sm text-muted-foreground">
											Keep test configurations in version control alongside your code.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-orange-500/10 p-2">
										<Terminal className="h-4 w-4 text-orange-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Use Verbose Mode</h4>
										<p className="text-sm text-muted-foreground">
											Enable <code className="bg-muted px-1 rounded">-v</code> in CI for better
											debugging.
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
								<h4 className="font-medium mb-1">Parallel Execution</h4>
								<p className="text-sm text-muted-foreground mb-2">
									Speed up your CI tests with parallel requests
								</p>
								<Button asChild variant="ghost" size="sm" className="p-0">
									<Link href="/docs/tutorials/parallel-execution" className="flex items-center">
										Read Tutorial <ArrowRight className="ml-1 h-3 w-3" />
									</Link>
								</Button>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<h4 className="font-medium mb-1">Environment Variables</h4>
								<p className="text-sm text-muted-foreground mb-2">
									Learn all configuration options
								</p>
								<Button asChild variant="ghost" size="sm" className="p-0">
									<Link href="/docs/environment-variables" className="flex items-center">
										View Reference <ArrowRight className="ml-1 h-3 w-3" />
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
