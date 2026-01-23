import {
	AlertTriangle,
	ArrowRight,
	CheckCircle,
	Database,
	Key,
	Link2,
	RefreshCw,
	Workflow,
} from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';
import {
	authFlowExample,
	basicStorageExample,
	crudWorkflowExample,
	dataExtractionExample,
	mixedVariablesExample,
	nestedPathsExample,
	pathSyntaxReference,
	realWorldExample,
} from './snippets';

export const metadata: Metadata = {
	title: 'Response Storage',
	description:
		'Store response values from one request to use in subsequent requests. Chain API calls together by extracting tokens, IDs, and other data.',
	keywords: [
		'curl-runner response storage',
		'API request chaining',
		'store response values',
		'dynamic variables',
		'token extraction',
		'request sequencing',
		'API workflow',
		'response data extraction',
		'sequential requests',
		'API testing workflow',
		'authentication flow',
		'data pipelines',
	],
	openGraph: {
		title: 'Response Storage | curl-runner Documentation',
		description:
			'Store response values from one request to use in subsequent requests. Chain API calls together with curl-runner.',
		url: 'https://www.curl-runner.com/docs/features/response-storage',
		type: 'article',
	},
	twitter: {
		card: 'summary_large_image',
		title: 'Response Storage | curl-runner Documentation',
		description:
			'Learn how to chain API requests by storing and reusing response data with curl-runner.',
	},
	alternates: {
		canonical: 'https://www.curl-runner.com/docs/features/response-storage',
	},
};

export default function ResponseStoragePage() {
	return (
		<main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
			<div className="mx-auto w-full min-w-0">
				<DocsPageHeader
					heading="Response Storage"
					text="Store response values from one request to use in subsequent requests."
				/>

				<div className="space-y-8">
					{/* Overview */}
					<section>
						<H2 id="overview">Overview</H2>
						<p className="text-muted-foreground mb-6">
							Response storage allows you to extract data from API responses and use it in
							subsequent requests. This is essential for building realistic API test workflows
							where requests depend on each other, such as authentication flows, CRUD operations,
							and data pipelines.
						</p>

						<div className="grid gap-4 sm:grid-cols-2">
							<div className="rounded-lg border p-4">
								<div className="flex items-center gap-2 mb-2">
									<Badge className="bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20">
										Flexible
									</Badge>
								</div>
								<h4 className="font-medium mb-1">JSON Path Extraction</h4>
								<p className="text-sm text-muted-foreground">
									Extract values from any part of the response using dot-notation paths
								</p>
							</div>

							<div className="rounded-lg border p-4">
								<div className="flex items-center gap-2 mb-2">
									<Badge className="bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20">
										Chainable
									</Badge>
								</div>
								<h4 className="font-medium mb-1">Request Chaining</h4>
								<p className="text-sm text-muted-foreground">
									Build complex workflows by passing data between sequential requests
								</p>
							</div>
						</div>
					</section>

					{/* Basic Usage */}
					<section>
						<H2 id="basic-usage">Basic Usage</H2>
						<p className="text-muted-foreground mb-6">
							Use the{' '}
							<code className="text-sm bg-muted px-1 py-0.5 rounded">store</code> property in
							your request to extract and save response values. Reference stored values using the{' '}
							<code className="text-sm bg-muted px-1 py-0.5 rounded">{'${store.variableName}'}</code>{' '}
							syntax.
						</p>

						<CodeBlockServer language="yaml" filename="basic-storage.yaml">
							{basicStorageExample}
						</CodeBlockServer>

						<div className="mt-6 rounded-lg border bg-muted/50 p-4">
							<h4 className="text-sm font-medium mb-2">How it works:</h4>
							<ol className="text-sm text-muted-foreground space-y-1 list-decimal list-inside">
								<li>First request executes and receives a response</li>
								<li>Values are extracted based on the <code>store</code> configuration</li>
								<li>Stored values are available for subsequent requests</li>
								<li>Use <code>{'${store.variableName}'}</code> to reference stored values</li>
							</ol>
						</div>
					</section>

					{/* Path Syntax */}
					<section>
						<H2 id="path-syntax">Path Syntax</H2>
						<p className="text-muted-foreground mb-6">
							The store configuration uses dot-notation paths to extract values from different
							parts of the response.
						</p>

						<div className="border rounded-lg overflow-hidden mb-6">
							<table className="w-full">
								<thead className="bg-muted/50">
									<tr className="border-b">
										<th className="text-left p-3 font-medium">Path</th>
										<th className="text-left p-3 font-medium">Extracts From</th>
										<th className="text-left p-3 font-medium">Example</th>
									</tr>
								</thead>
								<tbody>
									<tr className="border-b">
										<td className="p-3">
											<code className="text-sm">body.field</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">Response body</td>
										<td className="p-3 text-sm text-muted-foreground">
											<code>body.id</code>, <code>body.user.name</code>
										</td>
									</tr>
									<tr className="border-b">
										<td className="p-3">
											<code className="text-sm">body.array.N</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">Array element</td>
										<td className="p-3 text-sm text-muted-foreground">
											<code>body.items.0.id</code>
										</td>
									</tr>
									<tr className="border-b">
										<td className="p-3">
											<code className="text-sm">headers.name</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">Response header</td>
										<td className="p-3 text-sm text-muted-foreground">
											<code>headers.content-type</code>
										</td>
									</tr>
									<tr className="border-b">
										<td className="p-3">
											<code className="text-sm">status</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">HTTP status code</td>
										<td className="p-3 text-sm text-muted-foreground">
											<code>status</code> → "200"
										</td>
									</tr>
									<tr>
										<td className="p-3">
											<code className="text-sm">metrics.field</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">Response metrics</td>
										<td className="p-3 text-sm text-muted-foreground">
											<code>metrics.duration</code>
										</td>
									</tr>
								</tbody>
							</table>
						</div>

						<CodeBlockServer language="yaml" filename="path-reference.yaml">
							{pathSyntaxReference}
						</CodeBlockServer>
					</section>

					{/* Common Use Cases */}
					<section>
						<H2 id="use-cases">Common Use Cases</H2>

						<div className="space-y-6">
							<div>
								<H3 id="authentication-flow">Authentication Flow</H3>
								<p className="text-muted-foreground mb-4">
									Store authentication tokens from login responses to use in subsequent
									authenticated requests.
								</p>
								<CodeBlockServer language="yaml" filename="auth-flow.yaml">
									{authFlowExample}
								</CodeBlockServer>
							</div>

							<div>
								<H3 id="crud-workflow">CRUD Workflow</H3>
								<p className="text-muted-foreground mb-4">
									Create, read, update, and delete resources by passing IDs between requests.
								</p>
								<CodeBlockServer language="yaml" filename="crud-workflow.yaml">
									{crudWorkflowExample}
								</CodeBlockServer>
							</div>

							<div>
								<H3 id="data-extraction">Data Extraction</H3>
								<p className="text-muted-foreground mb-4">
									Extract specific data from list responses to use in follow-up requests.
								</p>
								<CodeBlockServer language="yaml" filename="data-extraction.yaml">
									{dataExtractionExample}
								</CodeBlockServer>
							</div>
						</div>
					</section>

					{/* Nested Paths */}
					<section>
						<H2 id="nested-paths">Extracting Nested Values</H2>
						<p className="text-muted-foreground mb-6">
							Store values from deeply nested response structures using dot-notation paths.
						</p>

						<CodeBlockServer language="yaml" filename="nested-paths.yaml">
							{nestedPathsExample}
						</CodeBlockServer>
					</section>

					{/* Mixing Variables */}
					<section>
						<H2 id="mixed-variables">Combining with Other Variables</H2>
						<p className="text-muted-foreground mb-6">
							Response storage works seamlessly with static variables, dynamic variables, and
							date/time formatting.
						</p>

						<CodeBlockServer language="yaml" filename="mixed-variables.yaml">
							{mixedVariablesExample}
						</CodeBlockServer>

						<div className="mt-6 rounded-lg border bg-blue-500/5 dark:bg-blue-500/10 border-blue-500/20 p-4">
							<p className="text-sm">
								<strong className="text-blue-600 dark:text-blue-400">Variable Resolution Order:</strong>{' '}
								Store variables → Dynamic variables (UUID, TIMESTAMP) → Static variables (from YAML)
							</p>
						</div>
					</section>

					{/* Real World Example */}
					<section>
						<H2 id="real-world">Real-World Example</H2>
						<p className="text-muted-foreground mb-6">
							A complete e-commerce checkout flow demonstrating how response storage enables
							complex, realistic API test scenarios.
						</p>

						<CodeBlockServer language="yaml" filename="checkout-flow.yaml">
							{realWorldExample}
						</CodeBlockServer>
					</section>

					{/* Important Notes */}
					<section>
						<H2 id="important-notes">Important Notes</H2>

						<div className="space-y-4">
							<div className="rounded-lg border bg-yellow-500/5 dark:bg-yellow-500/10 border-yellow-500/20 p-4">
								<div className="flex items-start gap-3">
									<AlertTriangle className="h-5 w-5 text-yellow-600 dark:text-yellow-400 mt-0.5" />
									<div>
										<h4 className="font-medium text-yellow-600 dark:text-yellow-400 mb-1">
											Sequential Execution Required
										</h4>
										<p className="text-sm text-muted-foreground">
											Response storage only works in sequential execution mode (the default). In
											parallel mode, request order is not guaranteed, so stored values may not be
											available when expected.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-muted/50 p-4">
								<div className="flex items-start gap-3">
									<Database className="h-5 w-5 text-muted-foreground mt-0.5" />
									<div>
										<h4 className="font-medium mb-1">Values are Stored as Strings</h4>
										<p className="text-sm text-muted-foreground">
											All stored values are converted to strings. Numbers become "123", booleans
											become "true"/"false", and objects/arrays are JSON stringified.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-muted/50 p-4">
								<div className="flex items-start gap-3">
									<RefreshCw className="h-5 w-5 text-muted-foreground mt-0.5" />
									<div>
										<h4 className="font-medium mb-1">Scope and Persistence</h4>
										<p className="text-sm text-muted-foreground">
											Stored values persist for all subsequent requests within the same execution
											run. They are not persisted between separate curl-runner invocations.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-muted/50 p-4">
								<div className="flex items-start gap-3">
									<Link2 className="h-5 w-5 text-muted-foreground mt-0.5" />
									<div>
										<h4 className="font-medium mb-1">Missing Values</h4>
										<p className="text-sm text-muted-foreground">
											If a path doesn't exist in the response, the stored value will be an empty
											string. The original{' '}
											<code className="text-sm bg-muted px-1 py-0.5 rounded">
												{'${store.variableName}'}
											</code>{' '}
											syntax remains unchanged if the variable was never stored.
										</p>
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
										<CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
									</div>
									<div>
										<h4 className="font-medium mb-2">Use Descriptive Names</h4>
										<p className="text-sm text-muted-foreground">
											Name stored variables clearly: <code>authToken</code> instead of{' '}
											<code>t</code>, <code>createdUserId</code> instead of <code>id</code>.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-blue-500/10 p-2">
										<Key className="h-4 w-4 text-blue-600 dark:text-blue-400" />
									</div>
									<div>
										<h4 className="font-medium mb-2">Store Only What You Need</h4>
										<p className="text-sm text-muted-foreground">
											Only store values you'll actually use. This keeps your configuration clean
											and makes it easier to understand the data flow.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-purple-500/10 p-2">
										<Workflow className="h-4 w-4 text-purple-600 dark:text-purple-400" />
									</div>
									<div>
										<h4 className="font-medium mb-2">Plan Your Workflow</h4>
										<p className="text-sm text-muted-foreground">
											Design your request sequence carefully. Ensure data-producing requests come
											before data-consuming requests.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-orange-500/10 p-2">
										<ArrowRight className="h-4 w-4 text-orange-600 dark:text-orange-400" />
									</div>
									<div>
										<h4 className="font-medium mb-2">Add Validation</h4>
										<p className="text-sm text-muted-foreground">
											Use <code>expect</code> to validate responses before storing values. This
											ensures you're extracting data from successful responses.
										</p>
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
