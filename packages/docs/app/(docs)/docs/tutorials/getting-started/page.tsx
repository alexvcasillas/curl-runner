import { ArrowRight, CheckCircle, FileText, Play, Terminal } from 'lucide-react';
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
	title: 'Getting Started: Your First API Test',
	description:
		'Learn the basics of curl-runner by creating your first API test. This tutorial covers creating YAML files, running requests, and validating responses.',
	keywords: [
		'curl-runner tutorial',
		'API testing beginner',
		'first API test',
		'YAML configuration',
		'HTTP request tutorial',
		'curl-runner getting started',
	],
	openGraph: {
		title: 'Getting Started: Your First API Test | curl-runner',
		description:
			'Learn the basics of curl-runner by creating your first API test with step-by-step instructions.',
		url: 'https://www.curl-runner.com/docs/tutorials/getting-started',
	},
	alternates: {
		canonical: 'https://www.curl-runner.com/docs/tutorials/getting-started',
	},
};

const installExample = `# Install globally with bun
bun install -g @curl-runner/cli

# Or with npm
npm install -g @curl-runner/cli`;

const firstRequestExample = `# my-first-test.yaml
request:
  name: Get a Post
  url: https://jsonplaceholder.typicode.com/posts/1
  method: GET`;

const runCommandExample = `curl-runner my-first-test.yaml`;

const outputExample = `ℹ Found 1 YAML file(s)
ℹ Processing: my-first-test.yaml

✓ Get a Post [my-first-test]
   ├─ GET: https://jsonplaceholder.typicode.com/posts/1
   ├─ ✓ Status: 200
   └─ Duration: 156ms | 292.00 B

Summary: 1 request completed successfully`;

const validationExample = `# validated-test.yaml
request:
  name: Get a Post with Validation
  url: https://jsonplaceholder.typicode.com/posts/1
  method: GET
  expect:
    status: 200
    body:
      id: 1
      userId: 1
      title: "*"      # Accept any title
      body: "*"       # Accept any body`;

const postRequestExample = `# create-post.yaml
request:
  name: Create a New Post
  url: https://jsonplaceholder.typicode.com/posts
  method: POST
  headers:
    Content-Type: application/json
  body:
    title: "My First Post"
    body: "This is the content of my post."
    userId: 1
  expect:
    status: 201
    body:
      id: "*"
      title: "My First Post"`;

const variablesExample = `# with-variables.yaml
global:
  variables:
    BASE_URL: https://jsonplaceholder.typicode.com
    USER_ID: 1

request:
  name: Get User Posts
  url: \${BASE_URL}/users/\${USER_ID}/posts
  method: GET
  expect:
    status: 200`;

const multipleRequestsExample = `# multiple-requests.yaml
global:
  variables:
    BASE_URL: https://jsonplaceholder.typicode.com

requests:
  - name: Get User
    url: \${BASE_URL}/users/1
    method: GET
    expect:
      status: 200

  - name: Get User Posts
    url: \${BASE_URL}/users/1/posts
    method: GET
    expect:
      status: 200

  - name: Get User Todos
    url: \${BASE_URL}/users/1/todos
    method: GET
    expect:
      status: 200`;

export default function GettingStartedTutorialPage() {
	return (
		<main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
			<DocumentationArticleSchema
				title="Getting Started: Your First API Test"
				description="Learn the basics of curl-runner by creating your first API test with step-by-step instructions."
				url="https://www.curl-runner.com/docs/tutorials/getting-started"
				section="Tutorials"
			/>
			<BreadcrumbSchema
				items={[
					{ name: 'Home', url: 'https://www.curl-runner.com' },
					{ name: 'Documentation', url: 'https://www.curl-runner.com/docs' },
					{ name: 'Tutorials', url: 'https://www.curl-runner.com/docs/tutorials' },
					{
						name: 'Getting Started',
						url: 'https://www.curl-runner.com/docs/tutorials/getting-started',
					},
				]}
			/>

			<div className="mx-auto w-full min-w-0">
				<DocsPageHeader
					heading="Getting Started: Your First API Test"
					text="Learn the basics of curl-runner by creating your first API test"
				/>

				<div className="flex items-center gap-4 mb-8">
					<Badge variant="secondary">Beginner</Badge>
					<span className="text-sm text-muted-foreground">10 min read</span>
				</div>

				<div className="space-y-8">
					{/* What You'll Learn */}
					<section>
						<H2 id="what-youll-learn">What You'll Learn</H2>
						<div className="grid gap-3 sm:grid-cols-2">
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Install curl-runner</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Create YAML configuration files</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Run HTTP requests</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Validate API responses</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Use variables</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Run multiple requests</span>
							</div>
						</div>
					</section>

					{/* Step 1: Installation */}
					<section>
						<H2 id="installation">Step 1: Install curl-runner</H2>
						<p className="text-muted-foreground mb-4">
							First, install curl-runner globally so you can use it from any directory.
						</p>
						<CodeBlockServer language="bash">{installExample}</CodeBlockServer>
						<p className="text-sm text-muted-foreground mt-4">
							Verify the installation by running{' '}
							<code className="bg-muted px-1 py-0.5 rounded">curl-runner --version</code>.
						</p>
					</section>

					{/* Step 2: First Request */}
					<section>
						<H2 id="first-request">Step 2: Create Your First Request</H2>
						<p className="text-muted-foreground mb-4">
							Create a new file called <code className="bg-muted px-1 py-0.5 rounded">my-first-test.yaml</code>{' '}
							with the following content:
						</p>
						<CodeBlockServer language="yaml" filename="my-first-test.yaml">
							{firstRequestExample}
						</CodeBlockServer>

						<div className="mt-6 rounded-lg border bg-muted/50 p-4">
							<h4 className="font-medium mb-2">Understanding the structure:</h4>
							<ul className="text-sm text-muted-foreground space-y-1">
								<li>
									<code className="bg-muted px-1 rounded">request:</code> - Defines a single HTTP request
								</li>
								<li>
									<code className="bg-muted px-1 rounded">name:</code> - A descriptive name for the request
								</li>
								<li>
									<code className="bg-muted px-1 rounded">url:</code> - The endpoint to call
								</li>
								<li>
									<code className="bg-muted px-1 rounded">method:</code> - HTTP method (GET, POST, PUT, DELETE, etc.)
								</li>
							</ul>
						</div>
					</section>

					{/* Step 3: Run the Request */}
					<section>
						<H2 id="run-request">Step 3: Run the Request</H2>
						<p className="text-muted-foreground mb-4">Run your test using the curl-runner command:</p>
						<CodeBlockServer language="bash">{runCommandExample}</CodeBlockServer>

						<p className="text-muted-foreground mt-4 mb-4">You should see output like this:</p>
						<CodeBlockServer language="text">{outputExample}</CodeBlockServer>

						<div className="mt-4 flex items-center gap-2 text-sm text-green-600">
							<CheckCircle className="h-4 w-4" />
							<span>Congratulations! You've run your first API test.</span>
						</div>
					</section>

					{/* Step 4: Add Validation */}
					<section>
						<H2 id="validation">Step 4: Add Response Validation</H2>
						<p className="text-muted-foreground mb-4">
							Now let's validate that the response contains expected data using the{' '}
							<code className="bg-muted px-1 py-0.5 rounded">expect</code> property:
						</p>
						<CodeBlockServer language="yaml" filename="validated-test.yaml">
							{validationExample}
						</CodeBlockServer>

						<div className="mt-6 rounded-lg border bg-blue-500/5 border-blue-500/20 p-4">
							<h4 className="font-medium text-blue-600 dark:text-blue-400 mb-2">Validation Features</h4>
							<ul className="text-sm text-muted-foreground space-y-1">
								<li>
									<code className="bg-muted px-1 rounded">status: 200</code> - Validates the HTTP status code
								</li>
								<li>
									<code className="bg-muted px-1 rounded">body:</code> - Validates response body fields
								</li>
								<li>
									<code className="bg-muted px-1 rounded">"*"</code> - Wildcard accepts any value
								</li>
							</ul>
						</div>
					</section>

					{/* Step 5: POST Request */}
					<section>
						<H2 id="post-request">Step 5: Make a POST Request</H2>
						<p className="text-muted-foreground mb-4">
							Create a request that sends data to an API:
						</p>
						<CodeBlockServer language="yaml" filename="create-post.yaml">
							{postRequestExample}
						</CodeBlockServer>
					</section>

					{/* Step 6: Using Variables */}
					<section>
						<H2 id="variables">Step 6: Using Variables</H2>
						<p className="text-muted-foreground mb-4">
							Variables help you avoid repetition and make your tests more maintainable:
						</p>
						<CodeBlockServer language="yaml" filename="with-variables.yaml">
							{variablesExample}
						</CodeBlockServer>

						<p className="text-sm text-muted-foreground mt-4">
							Variables are defined in the <code className="bg-muted px-1 rounded">global.variables</code> section
							and referenced using <code className="bg-muted px-1 rounded">{'${VARIABLE_NAME}'}</code> syntax.
						</p>
					</section>

					{/* Step 7: Multiple Requests */}
					<section>
						<H2 id="multiple-requests">Step 7: Running Multiple Requests</H2>
						<p className="text-muted-foreground mb-4">
							Use <code className="bg-muted px-1 py-0.5 rounded">requests:</code> (plural) to run multiple
							requests in sequence:
						</p>
						<CodeBlockServer language="yaml" filename="multiple-requests.yaml">
							{multipleRequestsExample}
						</CodeBlockServer>

						<p className="text-sm text-muted-foreground mt-4">
							By default, requests run sequentially. You can add{' '}
							<code className="bg-muted px-1 rounded">execution: parallel</code> to the global section to run
							them simultaneously.
						</p>
					</section>

					{/* Summary */}
					<section>
						<H2 id="summary">Summary</H2>
						<p className="text-muted-foreground mb-4">In this tutorial, you learned how to:</p>
						<ul className="space-y-2 text-muted-foreground">
							<li className="flex items-start gap-2">
								<CheckCircle className="h-4 w-4 text-green-600 mt-1" />
								<span>Install curl-runner using bun or npm</span>
							</li>
							<li className="flex items-start gap-2">
								<CheckCircle className="h-4 w-4 text-green-600 mt-1" />
								<span>Create YAML configuration files for HTTP requests</span>
							</li>
							<li className="flex items-start gap-2">
								<CheckCircle className="h-4 w-4 text-green-600 mt-1" />
								<span>Add validation rules to verify response data</span>
							</li>
							<li className="flex items-start gap-2">
								<CheckCircle className="h-4 w-4 text-green-600 mt-1" />
								<span>Use variables for cleaner, more maintainable tests</span>
							</li>
							<li className="flex items-start gap-2">
								<CheckCircle className="h-4 w-4 text-green-600 mt-1" />
								<span>Run multiple requests in a single file</span>
							</li>
						</ul>
					</section>

					{/* Next Steps */}
					<section>
						<H2 id="next-steps">Next Steps</H2>
						<div className="grid gap-4 sm:grid-cols-2">
							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-blue-500/10 p-2">
										<FileText className="h-4 w-4 text-blue-600 dark:text-blue-400" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Authentication Workflows</h4>
										<p className="text-sm text-muted-foreground mb-2">
											Learn to chain requests and store tokens
										</p>
										<Button asChild variant="ghost" size="sm" className="p-0">
											<Link
												href="/docs/tutorials/authentication-workflows"
												className="flex items-center"
											>
												Read Tutorial <ArrowRight className="ml-1 h-3 w-3" />
											</Link>
										</Button>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-purple-500/10 p-2">
										<Terminal className="h-4 w-4 text-purple-600 dark:text-purple-400" />
									</div>
									<div>
										<h4 className="font-medium mb-1">CLI Options</h4>
										<p className="text-sm text-muted-foreground mb-2">
											Explore all command-line options
										</p>
										<Button asChild variant="ghost" size="sm" className="p-0">
											<Link href="/docs/cli-options" className="flex items-center">
												View Reference <ArrowRight className="ml-1 h-3 w-3" />
											</Link>
										</Button>
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
	);
}
