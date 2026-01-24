import { ArrowRight, CheckCircle, Key, Link2, RefreshCw, Shield } from 'lucide-react';
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
	title: 'Authentication Workflows with Response Storage',
	description:
		'Build authentication flows that extract tokens from login responses and use them in subsequent requests. Master response storage for chaining API calls.',
	keywords: [
		'curl-runner authentication',
		'API token management',
		'response storage',
		'request chaining',
		'OAuth testing',
		'JWT tokens',
	],
	openGraph: {
		title: 'Authentication Workflows | curl-runner Tutorial',
		description:
			'Learn to build authentication flows with response storage for chaining API calls.',
		url: 'https://www.curl-runner.com/docs/tutorials/authentication-workflows',
	},
	alternates: {
		canonical: 'https://www.curl-runner.com/docs/tutorials/authentication-workflows',
	},
};

const basicAuthExample = `# basic-auth.yaml
request:
  name: Access Protected Resource
  url: https://api.example.com/protected
  method: GET
  auth:
    type: basic
    username: myuser
    password: mypassword
  expect:
    status: 200`;

const bearerTokenExample = `# bearer-auth.yaml
global:
  variables:
    API_TOKEN: your-api-token-here

request:
  name: Access with Bearer Token
  url: https://api.example.com/protected
  method: GET
  auth:
    type: bearer
    token: \${API_TOKEN}
  expect:
    status: 200`;

const loginFlowExample = `# login-flow.yaml
global:
  variables:
    BASE_URL: https://api.example.com
  execution: sequential  # Required for response storage

requests:
  # Step 1: Login and get token
  - name: Login
    url: \${BASE_URL}/auth/login
    method: POST
    headers:
      Content-Type: application/json
    body:
      email: user@example.com
      password: secretpassword
    expect:
      status: 200
      body:
        token: "*"
    store:
      authToken: body.token        # Store the token
      userId: body.user.id         # Store user ID

  # Step 2: Use the token for authenticated request
  - name: Get User Profile
    url: \${BASE_URL}/users/\${store.userId}
    method: GET
    headers:
      Authorization: Bearer \${store.authToken}
    expect:
      status: 200
      body:
        id: "*"
        email: user@example.com`;

const refreshTokenExample = `# refresh-token-flow.yaml
global:
  variables:
    BASE_URL: https://api.example.com
  execution: sequential

requests:
  # Initial login
  - name: Login
    url: \${BASE_URL}/auth/login
    method: POST
    headers:
      Content-Type: application/json
    body:
      email: user@example.com
      password: secretpassword
    expect:
      status: 200
    store:
      accessToken: body.accessToken
      refreshToken: body.refreshToken

  # Use access token
  - name: Get Protected Data
    url: \${BASE_URL}/api/data
    method: GET
    headers:
      Authorization: Bearer \${store.accessToken}
    expect:
      status: 200

  # Refresh the token
  - name: Refresh Token
    url: \${BASE_URL}/auth/refresh
    method: POST
    headers:
      Content-Type: application/json
    body:
      refreshToken: \${store.refreshToken}
    expect:
      status: 200
    store:
      accessToken: body.accessToken  # Update with new token

  # Use new access token
  - name: Get More Data
    url: \${BASE_URL}/api/more-data
    method: GET
    headers:
      Authorization: Bearer \${store.accessToken}
    expect:
      status: 200`;

const crudWorkflowExample = `# crud-workflow.yaml
global:
  variables:
    BASE_URL: https://api.example.com
    AUTH_TOKEN: your-token
  execution: sequential

requests:
  # CREATE
  - name: Create Resource
    url: \${BASE_URL}/resources
    method: POST
    headers:
      Authorization: Bearer \${AUTH_TOKEN}
      Content-Type: application/json
    body:
      name: "New Resource"
      description: "Created via curl-runner"
    expect:
      status: 201
    store:
      resourceId: body.id           # Store the created ID
      resourceName: body.name

  # READ
  - name: Get Created Resource
    url: \${BASE_URL}/resources/\${store.resourceId}
    method: GET
    headers:
      Authorization: Bearer \${AUTH_TOKEN}
    expect:
      status: 200
      body:
        id: \${store.resourceId}
        name: "New Resource"

  # UPDATE
  - name: Update Resource
    url: \${BASE_URL}/resources/\${store.resourceId}
    method: PUT
    headers:
      Authorization: Bearer \${AUTH_TOKEN}
      Content-Type: application/json
    body:
      name: "Updated Resource"
      description: "Modified via curl-runner"
    expect:
      status: 200
      body:
        name: "Updated Resource"

  # DELETE
  - name: Delete Resource
    url: \${BASE_URL}/resources/\${store.resourceId}
    method: DELETE
    headers:
      Authorization: Bearer \${AUTH_TOKEN}
    expect:
      status: [200, 204]

  # VERIFY DELETION
  - name: Verify Deleted
    url: \${BASE_URL}/resources/\${store.resourceId}
    method: GET
    headers:
      Authorization: Bearer \${AUTH_TOKEN}
    expect:
      failure: true
      status: 404`;

const multiServiceExample = `# multi-service-auth.yaml
global:
  variables:
    AUTH_SERVICE: https://auth.example.com
    API_SERVICE: https://api.example.com
    ANALYTICS_SERVICE: https://analytics.example.com
  execution: sequential

requests:
  # Authenticate with auth service
  - name: Authenticate
    url: \${AUTH_SERVICE}/oauth/token
    method: POST
    headers:
      Content-Type: application/x-www-form-urlencoded
    body:
      grant_type: client_credentials
      client_id: my-client
      client_secret: my-secret
      scope: "read write"
    expect:
      status: 200
    store:
      token: body.access_token
      tokenType: body.token_type
      expiresIn: body.expires_in

  # Call API service
  - name: Get API Data
    url: \${API_SERVICE}/v1/data
    method: GET
    headers:
      Authorization: \${store.tokenType} \${store.token}
    expect:
      status: 200
    store:
      dataId: body.items.0.id

  # Call Analytics service with same token
  - name: Track Event
    url: \${ANALYTICS_SERVICE}/events
    method: POST
    headers:
      Authorization: \${store.tokenType} \${store.token}
      Content-Type: application/json
    body:
      event: "data_accessed"
      dataId: \${store.dataId}
    expect:
      status: [200, 201, 202]`;

const storagePathsExample = `# Storage paths reference
store:
  # Response body
  token: body.token                    # Top-level field
  userId: body.user.id                 # Nested field
  firstName: body.user.profile.name    # Deeply nested
  firstItem: body.items.0              # First array element
  lastItem: body.items.-1              # Last array element (if supported)

  # Response headers
  contentType: headers.content-type
  requestId: headers.x-request-id
  rateLimit: headers.x-rate-limit-remaining

  # Other response data
  statusCode: status                   # HTTP status code
  duration: metrics.duration           # Request duration`;

export default function AuthenticationWorkflowsPage() {
	return (
		<main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
			<DocumentationArticleSchema
				title="Authentication Workflows with Response Storage"
				description="Build authentication flows that extract tokens and chain API calls together."
				url="https://www.curl-runner.com/docs/tutorials/authentication-workflows"
				section="Tutorials"
			/>
			<BreadcrumbSchema
				items={[
					{ name: 'Home', url: 'https://www.curl-runner.com' },
					{ name: 'Documentation', url: 'https://www.curl-runner.com/docs' },
					{ name: 'Tutorials', url: 'https://www.curl-runner.com/docs/tutorials' },
					{
						name: 'Authentication Workflows',
						url: 'https://www.curl-runner.com/docs/tutorials/authentication-workflows',
					},
				]}
			/>

			<div className="mx-auto w-full min-w-0">
				<DocsPageHeader
					heading="Authentication Workflows"
					text="Build authentication flows with response storage for chaining API calls"
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
								<span className="text-sm">Basic and Bearer authentication</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Extract and store tokens</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Chain authenticated requests</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">CRUD workflows with auth</span>
							</div>
						</div>
					</section>

					{/* Prerequisites */}
					<section className="rounded-lg border bg-muted/50 p-4">
						<h3 className="font-medium mb-2">Prerequisites</h3>
						<p className="text-sm text-muted-foreground">
							This tutorial assumes you've completed the{' '}
							<Link href="/docs/tutorials/getting-started" className="text-primary hover:underline">
								Getting Started tutorial
							</Link>{' '}
							and understand basic YAML configuration.
						</p>
					</section>

					{/* Built-in Authentication */}
					<section>
						<H2 id="built-in-auth">Built-in Authentication</H2>
						<p className="text-muted-foreground mb-6">
							curl-runner supports two built-in authentication methods that automatically handle
							header formatting.
						</p>

						<H3 id="basic-auth">Basic Authentication</H3>
						<p className="text-muted-foreground mb-4">
							Use Basic auth for simple username/password authentication:
						</p>
						<CodeBlockServer language="yaml" filename="basic-auth.yaml">
							{basicAuthExample}
						</CodeBlockServer>

						<H3 id="bearer-auth">Bearer Token Authentication</H3>
						<p className="text-muted-foreground mb-4 mt-6">
							Use Bearer auth when you have an API token:
						</p>
						<CodeBlockServer language="yaml" filename="bearer-auth.yaml">
							{bearerTokenExample}
						</CodeBlockServer>
					</section>

					{/* Response Storage */}
					<section>
						<H2 id="response-storage">Response Storage Basics</H2>
						<p className="text-muted-foreground mb-4">
							Response storage lets you extract values from one request and use them in subsequent
							requests. This is essential for authentication flows where you need to capture tokens.
						</p>

						<div className="rounded-lg border bg-yellow-500/5 border-yellow-500/20 p-4 mb-6">
							<div className="flex items-start gap-2">
								<RefreshCw className="h-5 w-5 text-yellow-600 mt-0.5" />
								<div>
									<h4 className="font-medium text-yellow-600 dark:text-yellow-400">
										Sequential Execution Required
									</h4>
									<p className="text-sm text-muted-foreground mt-1">
										Response storage only works with{' '}
										<code className="bg-muted px-1 rounded">execution: sequential</code> (the default).
										In parallel mode, request order isn't guaranteed.
									</p>
								</div>
							</div>
						</div>

						<H3 id="storage-paths">Storage Path Reference</H3>
						<CodeBlockServer language="yaml">{storagePathsExample}</CodeBlockServer>
					</section>

					{/* Login Flow */}
					<section>
						<H2 id="login-flow">Complete Login Flow</H2>
						<p className="text-muted-foreground mb-4">
							This example shows a typical login flow: authenticate, store the token, and use it for
							subsequent requests.
						</p>
						<CodeBlockServer language="yaml" filename="login-flow.yaml">
							{loginFlowExample}
						</CodeBlockServer>

						<div className="mt-6 rounded-lg border bg-muted/50 p-4">
							<h4 className="font-medium mb-2">How it works:</h4>
							<ol className="text-sm text-muted-foreground space-y-2 list-decimal list-inside">
								<li>Login request sends credentials and receives a token</li>
								<li>
									<code className="bg-muted px-1 rounded">store:</code> extracts the token and user ID
								</li>
								<li>
									Second request uses{' '}
									<code className="bg-muted px-1 rounded">{'${store.authToken}'}</code> in the header
								</li>
								<li>The user ID is also available for dynamic URL construction</li>
							</ol>
						</div>
					</section>

					{/* Refresh Token Flow */}
					<section>
						<H2 id="refresh-tokens">Token Refresh Flow</H2>
						<p className="text-muted-foreground mb-4">
							Handle token refresh scenarios by storing both access and refresh tokens:
						</p>
						<CodeBlockServer language="yaml" filename="refresh-token-flow.yaml">
							{refreshTokenExample}
						</CodeBlockServer>
					</section>

					{/* CRUD Workflow */}
					<section>
						<H2 id="crud-workflow">CRUD Workflow</H2>
						<p className="text-muted-foreground mb-4">
							A complete Create-Read-Update-Delete workflow that passes the resource ID between
							requests:
						</p>
						<CodeBlockServer language="yaml" filename="crud-workflow.yaml">
							{crudWorkflowExample}
						</CodeBlockServer>
					</section>

					{/* Multi-Service Authentication */}
					<section>
						<H2 id="multi-service">Multi-Service Authentication</H2>
						<p className="text-muted-foreground mb-4">
							Use a single token across multiple services in a microservices architecture:
						</p>
						<CodeBlockServer language="yaml" filename="multi-service-auth.yaml">
							{multiServiceExample}
						</CodeBlockServer>
					</section>

					{/* Best Practices */}
					<section>
						<H2 id="best-practices">Best Practices</H2>
						<div className="grid gap-4 md:grid-cols-2">
							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-green-500/10 p-2">
										<Key className="h-4 w-4 text-green-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Use Environment Variables</h4>
										<p className="text-sm text-muted-foreground">
											Never hardcode credentials. Use environment variables or a separate config
											file.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-blue-500/10 p-2">
										<Shield className="h-4 w-4 text-blue-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Validate Before Storing</h4>
										<p className="text-sm text-muted-foreground">
											Use <code className="bg-muted px-1 rounded">expect:</code> to verify the
											response before storing values.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-purple-500/10 p-2">
										<Link2 className="h-4 w-4 text-purple-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Descriptive Variable Names</h4>
										<p className="text-sm text-muted-foreground">
											Use clear names like <code className="bg-muted px-1 rounded">authToken</code>{' '}
											not <code className="bg-muted px-1 rounded">t</code>.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-orange-500/10 p-2">
										<RefreshCw className="h-4 w-4 text-orange-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Test Token Expiration</h4>
										<p className="text-sm text-muted-foreground">
											Include refresh token flows to test your API's token lifecycle handling.
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
								<h4 className="font-medium mb-1">CI/CD Integration</h4>
								<p className="text-sm text-muted-foreground mb-2">
									Run auth flows in your CI/CD pipeline
								</p>
								<Button asChild variant="ghost" size="sm" className="p-0">
									<Link href="/docs/tutorials/ci-cd-integration" className="flex items-center">
										Read Tutorial <ArrowRight className="ml-1 h-3 w-3" />
									</Link>
								</Button>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<h4 className="font-medium mb-1">Advanced Validation</h4>
								<p className="text-sm text-muted-foreground mb-2">
									Validate auth responses with regex and patterns
								</p>
								<Button asChild variant="ghost" size="sm" className="p-0">
									<Link href="/docs/tutorials/advanced-validation" className="flex items-center">
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
