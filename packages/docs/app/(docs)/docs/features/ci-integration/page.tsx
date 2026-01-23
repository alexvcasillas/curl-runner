import {
	AlertTriangle,
	CheckCircle,
	GitBranch,
	PlayCircle,
	Settings,
	Terminal,
} from 'lucide-react'
import type { Metadata } from 'next'
import { CodeBlockServer } from '@/components/code-block-server'
import { H2, H3 } from '@/components/docs-heading'
import { DocsPageHeader } from '@/components/docs-page-header'
import { TableOfContents } from '@/components/toc'
import { Badge } from '@/components/ui/badge'

export const metadata: Metadata = {
	title: 'CI/CD Integration',
	description:
		'Integrate curl-runner with CI/CD pipelines using CI-friendly exit codes and configurable failure thresholds.',
	keywords: [
		'curl-runner CI/CD',
		'CI-friendly exit codes',
		'GitHub Actions API testing',
		'GitLab CI API testing',
		'Jenkins API testing',
		'CircleCI API testing',
		'strict exit mode',
		'failure thresholds',
		'pipeline integration',
		'automated API testing',
		'exit code configuration',
		'test automation',
	],
	openGraph: {
		title: 'CI/CD Integration | curl-runner Documentation',
		description:
			'Integrate curl-runner with CI/CD pipelines using CI-friendly exit codes and configurable failure thresholds.',
		url: 'https://www.curl-runner.com/docs/features/ci-integration',
		type: 'article',
	},
	twitter: {
		card: 'summary_large_image',
		title: 'CI/CD Integration | curl-runner Documentation',
		description:
			'Learn how to integrate curl-runner with CI/CD pipelines for automated API testing.',
	},
	alternates: {
		canonical: 'https://www.curl-runner.com/docs/features/ci-integration',
	},
}

const cliOptions = `# CI/CD exit code options

# Strict mode - exit 1 if ANY validation fails
curl-runner tests/ --strict-exit

# Run all tests, then fail if any failed
curl-runner tests/ --continue-on-error --strict-exit

# Allow up to 2 failures before failing
curl-runner tests/ --fail-on 2

# Allow up to 10% of tests to fail
curl-runner tests/ --fail-on-percentage 10

# Combine options for comprehensive CI setup
curl-runner tests/ \\
  --continue-on-error \\
  --strict-exit \\
  --output-format json \\
  --output results.json`

const envVars = `# Environment variables for CI/CD

# Enable strict exit mode
export CURL_RUNNER_STRICT_EXIT=true

# Set failure threshold (count)
export CURL_RUNNER_FAIL_ON=5

# Set failure threshold (percentage)
export CURL_RUNNER_FAIL_ON_PERCENTAGE=10

# Combine with other settings
export CURL_RUNNER_STRICT_EXIT=true
export CURL_RUNNER_CONTINUE_ON_ERROR=true
export CURL_RUNNER_OUTPUT_FORMAT=json
curl-runner tests/`

const githubActions = `name: API Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v2

      - name: Install curl-runner
        run: bun install -g curl-runner

      - name: Run API Tests
        env:
          CURL_RUNNER_STRICT_EXIT: true
          CURL_RUNNER_CONTINUE_ON_ERROR: true
        run: curl-runner tests/ --output-format json --output results.json

      - name: Upload Results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: test-results
          path: results.json`

const gitlabCi = `stages:
  - test

api-tests:
  stage: test
  image: oven/bun:latest
  variables:
    CURL_RUNNER_STRICT_EXIT: "true"
    CURL_RUNNER_CONTINUE_ON_ERROR: "true"
    CURL_RUNNER_OUTPUT_FORMAT: json
  before_script:
    - bun install -g curl-runner
  script:
    - curl-runner tests/ --output test-results.json
  artifacts:
    when: always
    paths:
      - test-results.json`

const jenkinsPipeline = `pipeline {
    agent any

    environment {
        CURL_RUNNER_STRICT_EXIT = 'true'
        CURL_RUNNER_CONTINUE_ON_ERROR = 'true'
        CURL_RUNNER_OUTPUT_FORMAT = 'json'
    }

    stages {
        stage('Install') {
            steps {
                sh 'npm install -g curl-runner'
            }
        }

        stage('Test') {
            steps {
                sh 'curl-runner tests/ --output test-results.json'
            }
        }
    }

    post {
        always {
            archiveArtifacts artifacts: 'test-results.json', allowEmptyArchive: true
        }
    }
}`

const circleciConfig = `version: 2.1

executors:
  node:
    docker:
      - image: cimg/node:20.0

jobs:
  test:
    executor: node
    environment:
      CURL_RUNNER_STRICT_EXIT: "true"
      CURL_RUNNER_CONTINUE_ON_ERROR: "true"
    steps:
      - checkout
      - run:
          name: Install curl-runner
          command: npm install -g curl-runner
      - run:
          name: Run API Tests
          command: curl-runner tests/ --output-format json --output test-results.json
      - store_artifacts:
          path: test-results.json
          destination: test-results

workflows:
  test:
    jobs:
      - test`

export default function CIIntegrationPage() {
	return (
		<main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
			<div className="mx-auto w-full min-w-0">
				<DocsPageHeader
					heading="CI/CD Integration"
					text="Integrate curl-runner with CI/CD pipelines using CI-friendly exit codes and configurable failure thresholds."
				/>

				<div className="space-y-8">
					{/* Overview */}
					<section>
						<H2 id="overview">Overview</H2>
						<p className="text-muted-foreground mb-6">
							<code className="font-mono">curl-runner</code> provides CI-friendly exit codes
							that integrate seamlessly with popular CI/CD platforms. By default, the tool
							follows standard conventions where exit code 0 indicates success and exit code
							1 indicates failure.
						</p>

						<div className="grid gap-4 sm:grid-cols-3">
							<div className="rounded-lg border p-4">
								<Badge className="mb-2">--strict-exit</Badge>
								<h4 className="font-medium mb-1">Strict Mode</h4>
								<p className="text-sm text-muted-foreground">
									Exit 1 if any validation fails
								</p>
							</div>

							<div className="rounded-lg border p-4">
								<Badge variant="secondary" className="mb-2">
									--fail-on
								</Badge>
								<h4 className="font-medium mb-1">Count Threshold</h4>
								<p className="text-sm text-muted-foreground">
									Allow N failures before failing
								</p>
							</div>

							<div className="rounded-lg border p-4">
								<Badge variant="outline" className="mb-2">
									--fail-on-percentage
								</Badge>
								<h4 className="font-medium mb-1">Percentage Threshold</h4>
								<p className="text-sm text-muted-foreground">Allow N% failures before failing</p>
							</div>
						</div>
					</section>

					{/* Exit Code Options */}
					<section>
						<H2 id="exit-code-options">Exit Code Options</H2>
						<p className="text-muted-foreground mb-6">
							Control how curl-runner determines its exit code based on test results.
						</p>

						<CodeBlockServer language="bash" filename="terminal">
							{cliOptions}
						</CodeBlockServer>

						<div className="mt-6 space-y-3">
							<H3 id="option-reference">Option Reference</H3>
							<div className="border rounded-lg overflow-hidden">
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
												Exit with code 1 if any validation fails, regardless of --continue-on-error
											</td>
										</tr>
										<tr className="border-b">
											<td className="p-3">
												<code className="text-sm">--fail-on &lt;count&gt;</code>
											</td>
											<td className="p-3 text-sm text-muted-foreground">
												Exit with code 1 only if failures exceed the specified count
											</td>
										</tr>
										<tr>
											<td className="p-3">
												<code className="text-sm">--fail-on-percentage &lt;pct&gt;</code>
											</td>
											<td className="p-3 text-sm text-muted-foreground">
												Exit with code 1 only if failure percentage exceeds the threshold (0-100)
											</td>
										</tr>
									</tbody>
								</table>
							</div>
						</div>
					</section>

					{/* Exit Code Behavior */}
					<section>
						<H2 id="exit-code-behavior">Exit Code Behavior</H2>
						<p className="text-muted-foreground mb-6">
							Understanding how different configurations affect the exit code.
						</p>

						<div className="border rounded-lg overflow-hidden">
							<table className="w-full">
								<thead className="bg-muted/50">
									<tr className="border-b">
										<th className="text-left p-3 font-medium">Configuration</th>
										<th className="text-left p-3 font-medium">No Failures</th>
										<th className="text-left p-3 font-medium">Failures Exist</th>
									</tr>
								</thead>
								<tbody>
									<tr className="border-b">
										<td className="p-3 text-sm">Default (no options)</td>
										<td className="p-3 text-sm text-green-600 dark:text-green-400">Exit 0</td>
										<td className="p-3 text-sm text-red-600 dark:text-red-400">Exit 1</td>
									</tr>
									<tr className="border-b">
										<td className="p-3 text-sm">
											<code>--continue-on-error</code>
										</td>
										<td className="p-3 text-sm text-green-600 dark:text-green-400">Exit 0</td>
										<td className="p-3 text-sm text-green-600 dark:text-green-400">Exit 0</td>
									</tr>
									<tr className="border-b">
										<td className="p-3 text-sm">
											<code>--strict-exit</code>
										</td>
										<td className="p-3 text-sm text-green-600 dark:text-green-400">Exit 0</td>
										<td className="p-3 text-sm text-red-600 dark:text-red-400">Exit 1</td>
									</tr>
									<tr className="border-b">
										<td className="p-3 text-sm">
											<code>--continue-on-error --strict-exit</code>
										</td>
										<td className="p-3 text-sm text-green-600 dark:text-green-400">Exit 0</td>
										<td className="p-3 text-sm text-red-600 dark:text-red-400">Exit 1</td>
									</tr>
									<tr className="border-b">
										<td className="p-3 text-sm">
											<code>--fail-on N</code>
										</td>
										<td className="p-3 text-sm text-green-600 dark:text-green-400">Exit 0</td>
										<td className="p-3 text-sm text-muted-foreground">
											Exit 0 if ≤N, Exit 1 if &gt;N
										</td>
									</tr>
									<tr>
										<td className="p-3 text-sm">
											<code>--fail-on-percentage P</code>
										</td>
										<td className="p-3 text-sm text-green-600 dark:text-green-400">Exit 0</td>
										<td className="p-3 text-sm text-muted-foreground">
											Exit 0 if ≤P%, Exit 1 if &gt;P%
										</td>
									</tr>
								</tbody>
							</table>
						</div>

						<div className="mt-6 rounded-lg border bg-blue-500/5 dark:bg-blue-500/10 border-blue-500/20 p-4">
							<p className="text-sm">
								<strong className="text-blue-600 dark:text-blue-400">Tip:</strong> Use{' '}
								<code className="text-xs bg-background px-1 py-0.5 rounded">
									--continue-on-error --strict-exit
								</code>{' '}
								together to run all tests before determining the final exit code. This ensures
								you get complete test results while still failing the pipeline when needed.
							</p>
						</div>
					</section>

					{/* Environment Variables */}
					<section>
						<H2 id="environment-variables">Environment Variables</H2>
						<p className="text-muted-foreground mb-6">
							All CI options can be configured via environment variables for easy integration
							with CI/CD platforms.
						</p>

						<CodeBlockServer language="bash" filename="terminal">
							{envVars}
						</CodeBlockServer>

						<div className="mt-6 space-y-3">
							<H3 id="env-reference">Environment Variable Reference</H3>
							<div className="border rounded-lg overflow-hidden">
								<table className="w-full">
									<thead className="bg-muted/50">
										<tr className="border-b">
											<th className="text-left p-3 font-medium">Variable</th>
											<th className="text-left p-3 font-medium">Type</th>
											<th className="text-left p-3 font-medium">Description</th>
										</tr>
									</thead>
									<tbody>
										<tr className="border-b">
											<td className="p-3">
												<code className="text-sm">CURL_RUNNER_STRICT_EXIT</code>
											</td>
											<td className="p-3 text-sm text-muted-foreground">boolean</td>
											<td className="p-3 text-sm text-muted-foreground">
												Enable strict exit mode (true/false)
											</td>
										</tr>
										<tr className="border-b">
											<td className="p-3">
												<code className="text-sm">CURL_RUNNER_FAIL_ON</code>
											</td>
											<td className="p-3 text-sm text-muted-foreground">number</td>
											<td className="p-3 text-sm text-muted-foreground">
												Maximum allowed failures
											</td>
										</tr>
										<tr>
											<td className="p-3">
												<code className="text-sm">CURL_RUNNER_FAIL_ON_PERCENTAGE</code>
											</td>
											<td className="p-3 text-sm text-muted-foreground">number</td>
											<td className="p-3 text-sm text-muted-foreground">
												Maximum allowed failure percentage (0-100)
											</td>
										</tr>
									</tbody>
								</table>
							</div>
						</div>
					</section>

					{/* Platform Examples */}
					<section>
						<H2 id="platform-examples">Platform Examples</H2>

						<div className="space-y-6">
							<div>
								<H3 id="github-actions">GitHub Actions</H3>
								<p className="text-muted-foreground mb-4">
									Complete workflow for running API tests with artifact upload.
								</p>
								<CodeBlockServer language="yaml" filename=".github/workflows/api-tests.yml">
									{githubActions}
								</CodeBlockServer>
							</div>

							<div>
								<H3 id="gitlab-ci">GitLab CI</H3>
								<p className="text-muted-foreground mb-4">
									GitLab CI configuration with test artifacts.
								</p>
								<CodeBlockServer language="yaml" filename=".gitlab-ci.yml">
									{gitlabCi}
								</CodeBlockServer>
							</div>

							<div>
								<H3 id="jenkins">Jenkins Pipeline</H3>
								<p className="text-muted-foreground mb-4">
									Declarative Jenkins pipeline with artifact archiving.
								</p>
								<CodeBlockServer language="groovy" filename="Jenkinsfile">
									{jenkinsPipeline}
								</CodeBlockServer>
							</div>

							<div>
								<H3 id="circleci">CircleCI</H3>
								<p className="text-muted-foreground mb-4">
									CircleCI configuration with test artifact storage.
								</p>
								<CodeBlockServer language="yaml" filename=".circleci/config.yml">
									{circleciConfig}
								</CodeBlockServer>
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
										<CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
									</div>
									<div>
										<h4 className="font-medium mb-2">Run All Tests First</h4>
										<p className="text-sm text-muted-foreground">
											Use <code>--continue-on-error</code> with <code>--strict-exit</code> to
											get complete test results before failing.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-blue-500/10 p-2">
										<Terminal className="h-4 w-4 text-blue-600 dark:text-blue-400" />
									</div>
									<div>
										<h4 className="font-medium mb-2">Use JSON Output</h4>
										<p className="text-sm text-muted-foreground">
											JSON format is easier to parse and integrate with CI reporting tools.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-purple-500/10 p-2">
										<GitBranch className="h-4 w-4 text-purple-600 dark:text-purple-400" />
									</div>
									<div>
										<h4 className="font-medium mb-2">Archive Test Results</h4>
										<p className="text-sm text-muted-foreground">
											Always save results as artifacts, even when tests fail, for debugging.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-yellow-500/10 p-2">
										<Settings className="h-4 w-4 text-yellow-600 dark:text-yellow-500" />
									</div>
									<div>
										<h4 className="font-medium mb-2">Use Environment Variables</h4>
										<p className="text-sm text-muted-foreground">
											Configure via env vars for flexibility across environments.
										</p>
									</div>
								</div>
							</div>
						</div>
					</section>

					{/* Troubleshooting */}
					<section>
						<H2 id="troubleshooting">Troubleshooting</H2>
						<div className="space-y-4">
							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-yellow-500/10 p-2">
										<AlertTriangle className="h-4 w-4 text-yellow-600 dark:text-yellow-500" />
									</div>
									<div>
										<h4 className="font-medium mb-2">Exit Code Always 0</h4>
										<p className="text-sm text-muted-foreground mb-2">
											If the exit code is always 0 even with failures, check that you&apos;re
											using CI options correctly:
										</p>
										<code className="text-xs bg-muted px-2 py-1 rounded block">
											# Wrong: exits 0 with failures when using -c alone
											<br />
											curl-runner tests/ -c
											<br />
											<br /># Right: exits 1 if any failures occur
											<br />
											curl-runner tests/ -c --strict-exit
										</code>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-yellow-500/10 p-2">
										<PlayCircle className="h-4 w-4 text-yellow-600 dark:text-yellow-500" />
									</div>
									<div>
										<h4 className="font-medium mb-2">Tests Pass Locally but Fail in CI</h4>
										<p className="text-sm text-muted-foreground">
											Common causes include network issues, missing environment variables, or
											timing differences. Try increasing timeout and retry settings:
										</p>
										<code className="text-xs bg-muted px-2 py-1 rounded block mt-2">
											curl-runner tests/ --timeout 30000 --retries 3 --retry-delay 2000
										</code>
									</div>
								</div>
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
	)
}
