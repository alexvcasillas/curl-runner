import { ArrowRight, CheckCircle, Code, Hash, List, Regex, Shield } from 'lucide-react';
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
	title: 'Advanced Validation Patterns',
	description:
		'Master complex validation techniques including regex patterns, numeric ranges, array selectors, wildcards, and negative testing scenarios.',
	keywords: [
		'curl-runner validation',
		'regex validation',
		'API testing patterns',
		'response validation',
		'negative testing',
		'array validation',
	],
	openGraph: {
		title: 'Advanced Validation Patterns | curl-runner Tutorial',
		description:
			'Master complex validation techniques including regex, ranges, and wildcards.',
		url: 'https://www.curl-runner.com/docs/tutorials/advanced-validation',
	},
	alternates: {
		canonical: 'https://www.curl-runner.com/docs/tutorials/advanced-validation',
	},
};

const wildcardExample = `# wildcard-validation.yaml
request:
  name: Get User
  url: https://api.example.com/users/1
  method: GET
  expect:
    status: 200
    body:
      id: 1              # Exact match
      name: "*"          # Any string value
      email: "*"         # Any email value
      createdAt: "*"     # Any timestamp
      updatedAt: "*"     # Any timestamp`;

const regexExample = `# regex-validation.yaml
request:
  name: Get User
  url: https://api.example.com/users/1
  method: GET
  expect:
    status: 200
    body:
      # Numeric ID
      id: "^[0-9]+$"

      # Email format
      email: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\\\.[a-zA-Z]{2,}$"

      # UUID format
      uuid: "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"

      # ISO date format (YYYY-MM-DD)
      date: "^\\\\d{4}-\\\\d{2}-\\\\d{2}$"

      # URL format
      profileUrl: "^https?://"`;

const commonRegexPatterns = `# Common regex patterns reference
body:
  # IDs and codes
  numericId: "^[0-9]+$"
  alphanumeric: "^[a-zA-Z0-9]+$"
  slug: "^[a-z0-9-]+$"

  # Dates and times
  isoDate: "^\\\\d{4}-\\\\d{2}-\\\\d{2}$"
  isoDateTime: "^\\\\d{4}-\\\\d{2}-\\\\d{2}T\\\\d{2}:\\\\d{2}:\\\\d{2}"
  timestamp: "^\\\\d{10,13}$"

  # Formats
  email: "^[\\\\w.+-]+@[\\\\w.-]+\\\\.[a-zA-Z]{2,}$"
  uuid: "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
  phone: "^\\\\+?[1-9]\\\\d{1,14}$"
  url: "^https?://"

  # Versions
  semver: "^\\\\d+\\\\.\\\\d+\\\\.\\\\d+$"`;

const numericRangesExample = `# numeric-ranges.yaml
request:
  name: Get Product
  url: https://api.example.com/products/1
  method: GET
  expect:
    status: 200
    body:
      price: "> 0"            # Greater than 0
      stock: ">= 0"           # Greater than or equal to 0
      discount: "<= 100"      # Less than or equal to 100
      rating: "1-5"           # Between 1 and 5 inclusive
      quantity: ">= 1, <= 1000"  # Combined constraints`;

const multipleValuesExample = `# multiple-values.yaml
request:
  name: Get Order
  url: https://api.example.com/orders/1
  method: GET
  expect:
    status: 200
    body:
      # Accept any of these values
      status: [pending, processing, shipped, delivered, cancelled]
      priority: [1, 2, 3]
      type: [standard, express, overnight]
      paymentMethod: [credit_card, paypal, bank_transfer]`;

const arrayValidationExample = `# array-validation.yaml
request:
  name: Get Posts
  url: https://api.example.com/posts
  method: GET
  expect:
    status: 200
    body:
      # First element
      "[0]":
        id: "*"
        title: "*"

      # Last element
      "[-1]":
        id: "*"
        title: "*"

      # Specific indices
      "[0].author.name": "*"
      "[1].author.name": "*"`;

const allElementsExample = `# all-elements.yaml
request:
  name: Get Users
  url: https://api.example.com/users
  method: GET
  expect:
    status: 200
    body:
      # All elements must match this pattern
      "*":
        id: "^[0-9]+$"
        email: "*"
        active: true`;

const nestedValidationExample = `# nested-validation.yaml
request:
  name: Get User Profile
  url: https://api.example.com/users/1/profile
  method: GET
  expect:
    status: 200
    body:
      user:
        id: 1
        profile:
          firstName: "*"
          lastName: "*"
          address:
            street: "*"
            city: "*"
            country: "^[A-Z]{2}$"    # ISO country code
            postalCode: "^[0-9]{5}$"
        settings:
          notifications:
            email: [true, false]
            push: [true, false]
          theme: [light, dark, auto]`;

const headerValidationExample = `# header-validation.yaml
request:
  name: Check Headers
  url: https://api.example.com/data
  method: GET
  expect:
    status: 200
    headers:
      content-type: application/json; charset=utf-8
      cache-control: "*"              # Any value
      x-request-id: "^[a-f0-9-]+$"   # UUID pattern
      x-rate-limit-remaining: ">= 0"`;

const negativeTestingExample = `# negative-testing.yaml
global:
  variables:
    BASE_URL: https://api.example.com

requests:
  # Test missing authentication
  - name: Test Missing Auth
    url: \${BASE_URL}/protected
    method: GET
    # No Authorization header
    expect:
      failure: true     # Expect 4xx or 5xx
      status: 401
      body:
        error: "*"

  # Test invalid token
  - name: Test Invalid Token
    url: \${BASE_URL}/protected
    method: GET
    headers:
      Authorization: Bearer invalid-token
    expect:
      failure: true
      status: 401

  # Test not found
  - name: Test Not Found
    url: \${BASE_URL}/users/99999999
    method: GET
    expect:
      failure: true
      status: 404

  # Test invalid input
  - name: Test Invalid Input
    url: \${BASE_URL}/users
    method: POST
    headers:
      Content-Type: application/json
    body:
      email: not-an-email
    expect:
      failure: true
      status: 400
      body:
        error: "*"
        message: "*"`;

const comprehensiveExample = `# comprehensive-validation.yaml
global:
  variables:
    BASE_URL: https://api.example.com

request:
  name: Create and Validate Order
  url: \${BASE_URL}/orders
  method: POST
  headers:
    Content-Type: application/json
    Authorization: Bearer \${API_TOKEN}
  body:
    customerId: 123
    items:
      - productId: 1
        quantity: 2
      - productId: 2
        quantity: 1
  expect:
    status: 201
    headers:
      content-type: application/json; charset=utf-8
      x-request-id: "^[0-9a-f-]+$"
    body:
      id: "^[0-9]+$"
      status: pending
      customerId: 123
      total: "> 0"
      currency: [USD, EUR, GBP]
      items:
        "[0]":
          productId: 1
          quantity: 2
          price: "> 0"
        "[1]":
          productId: 2
          quantity: 1
          price: "> 0"
      createdAt: "^\\\\d{4}-\\\\d{2}-\\\\d{2}T"
      updatedAt: "*"
    responseTime: "< 1000"`;

const partialValidationExample = `# Only validate what matters
expect:
  status: 200
  body:
    id: "*"         # Must exist
    status: active  # Must be active
    # Other fields in response are ignored`;

export default function AdvancedValidationPage() {
	return (
		<main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
			<DocumentationArticleSchema
				title="Advanced Validation Patterns"
				description="Master complex validation techniques including regex, ranges, and wildcards."
				url="https://www.curl-runner.com/docs/tutorials/advanced-validation"
				section="Tutorials"
			/>
			<BreadcrumbSchema
				items={[
					{ name: 'Home', url: 'https://www.curl-runner.com' },
					{ name: 'Documentation', url: 'https://www.curl-runner.com/docs' },
					{ name: 'Tutorials', url: 'https://www.curl-runner.com/docs/tutorials' },
					{
						name: 'Advanced Validation',
						url: 'https://www.curl-runner.com/docs/tutorials/advanced-validation',
					},
				]}
			/>

			<div className="mx-auto w-full min-w-0">
				<DocsPageHeader
					heading="Advanced Validation Patterns"
					text="Master complex validation techniques for robust API testing"
				/>

				<div className="flex items-center gap-4 mb-8">
					<Badge variant="secondary">Advanced</Badge>
					<span className="text-sm text-muted-foreground">20 min read</span>
				</div>

				<div className="space-y-8">
					{/* What You'll Learn */}
					<section>
						<H2 id="what-youll-learn">What You'll Learn</H2>
						<div className="grid gap-3 sm:grid-cols-2">
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Wildcard matching</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Regex pattern validation</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Numeric ranges</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Array validation</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Negative testing</span>
							</div>
							<div className="flex items-center gap-2">
								<CheckCircle className="h-4 w-4 text-green-600" />
								<span className="text-sm">Header validation</span>
							</div>
						</div>
					</section>

					{/* Wildcards */}
					<section>
						<H2 id="wildcards">Wildcard Matching</H2>
						<p className="text-muted-foreground mb-4">
							Use <code className="bg-muted px-1 py-0.5 rounded">"*"</code> to accept any value for a
							field. This validates that the field exists but doesn't check its specific value.
						</p>
						<CodeBlockServer language="yaml" filename="wildcard-validation.yaml">
							{wildcardExample}
						</CodeBlockServer>
					</section>

					{/* Regex */}
					<section>
						<H2 id="regex">Regex Pattern Validation</H2>
						<p className="text-muted-foreground mb-4">
							Validate values against regular expression patterns:
						</p>
						<CodeBlockServer language="yaml" filename="regex-validation.yaml">
							{regexExample}
						</CodeBlockServer>

						<H3 id="common-patterns">Common Regex Patterns</H3>
						<CodeBlockServer language="yaml">{commonRegexPatterns}</CodeBlockServer>

						<div className="mt-4 rounded-lg border bg-yellow-500/5 border-yellow-500/20 p-4">
							<p className="text-sm text-muted-foreground">
								<strong className="text-yellow-600 dark:text-yellow-400">Note:</strong> Backslashes
								must be double-escaped in YAML (
								<code className="bg-muted px-1 rounded">\\\\d</code> instead of{' '}
								<code className="bg-muted px-1 rounded">\\d</code>).
							</p>
						</div>
					</section>

					{/* Numeric Ranges */}
					<section>
						<H2 id="numeric-ranges">Numeric Ranges</H2>
						<p className="text-muted-foreground mb-4">
							Validate numeric values are within expected ranges:
						</p>
						<CodeBlockServer language="yaml" filename="numeric-ranges.yaml">
							{numericRangesExample}
						</CodeBlockServer>

						<div className="mt-6 border rounded-lg overflow-hidden">
							<table className="w-full">
								<thead className="bg-muted/50">
									<tr className="border-b">
										<th className="text-left p-3 font-medium">Operator</th>
										<th className="text-left p-3 font-medium">Example</th>
										<th className="text-left p-3 font-medium">Description</th>
									</tr>
								</thead>
								<tbody>
									<tr className="border-b">
										<td className="p-3">
											<code className="text-sm">{'>'}</code>
										</td>
										<td className="p-3">
											<code className="text-sm">{'">'} 0"</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">Greater than</td>
									</tr>
									<tr className="border-b">
										<td className="p-3">
											<code className="text-sm">{'>='}</code>
										</td>
										<td className="p-3">
											<code className="text-sm">"{'>='} 1"</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">Greater than or equal</td>
									</tr>
									<tr className="border-b">
										<td className="p-3">
											<code className="text-sm">{'<'}</code>
										</td>
										<td className="p-3">
											<code className="text-sm">"{'<'} 100"</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">Less than</td>
									</tr>
									<tr className="border-b">
										<td className="p-3">
											<code className="text-sm">{'<='}</code>
										</td>
										<td className="p-3">
											<code className="text-sm">"{'<='} 50"</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">Less than or equal</td>
									</tr>
									<tr>
										<td className="p-3">
											<code className="text-sm">n-m</code>
										</td>
										<td className="p-3">
											<code className="text-sm">"1-10"</code>
										</td>
										<td className="p-3 text-sm text-muted-foreground">Between (inclusive)</td>
									</tr>
								</tbody>
							</table>
						</div>
					</section>

					{/* Multiple Values */}
					<section>
						<H2 id="multiple-values">Multiple Accepted Values</H2>
						<p className="text-muted-foreground mb-4">
							Accept one of several possible values using an array:
						</p>
						<CodeBlockServer language="yaml" filename="multiple-values.yaml">
							{multipleValuesExample}
						</CodeBlockServer>
					</section>

					{/* Array Validation */}
					<section>
						<H2 id="array-validation">Array Validation</H2>

						<H3 id="specific-elements">Specific Elements</H3>
						<p className="text-muted-foreground mb-4">
							Validate specific array elements by index:
						</p>
						<CodeBlockServer language="yaml" filename="array-validation.yaml">
							{arrayValidationExample}
						</CodeBlockServer>

						<H3 id="all-elements">All Elements</H3>
						<p className="text-muted-foreground mb-4 mt-6">
							Validate that all elements in an array match a pattern:
						</p>
						<CodeBlockServer language="yaml" filename="all-elements.yaml">
							{allElementsExample}
						</CodeBlockServer>
					</section>

					{/* Nested Validation */}
					<section>
						<H2 id="nested-validation">Nested Object Validation</H2>
						<p className="text-muted-foreground mb-4">
							Validate deeply nested structures:
						</p>
						<CodeBlockServer language="yaml" filename="nested-validation.yaml">
							{nestedValidationExample}
						</CodeBlockServer>
					</section>

					{/* Header Validation */}
					<section>
						<H2 id="header-validation">Header Validation</H2>
						<p className="text-muted-foreground mb-4">
							Validate response headers (case-insensitive matching):
						</p>
						<CodeBlockServer language="yaml" filename="header-validation.yaml">
							{headerValidationExample}
						</CodeBlockServer>
					</section>

					{/* Negative Testing */}
					<section>
						<H2 id="negative-testing">Negative Testing</H2>
						<p className="text-muted-foreground mb-4">
							Test that endpoints correctly reject invalid requests using{' '}
							<code className="bg-muted px-1 py-0.5 rounded">failure: true</code>:
						</p>
						<CodeBlockServer language="yaml" filename="negative-testing.yaml">
							{negativeTestingExample}
						</CodeBlockServer>

						<div className="mt-4 rounded-lg border bg-blue-500/5 border-blue-500/20 p-4">
							<div className="flex items-start gap-2">
								<Shield className="h-5 w-5 text-blue-600 mt-0.5" />
								<div>
									<h4 className="font-medium text-blue-600 dark:text-blue-400">Security Testing</h4>
									<p className="text-sm text-muted-foreground mt-1">
										Negative tests are essential for validating authentication, authorization, and
										input validation.
									</p>
								</div>
							</div>
						</div>
					</section>

					{/* Comprehensive Example */}
					<section>
						<H2 id="comprehensive-example">Comprehensive Example</H2>
						<p className="text-muted-foreground mb-4">
							Combining multiple validation techniques in a real-world scenario:
						</p>
						<CodeBlockServer language="yaml" filename="comprehensive-validation.yaml">
							{comprehensiveExample}
						</CodeBlockServer>
					</section>

					{/* Partial Validation */}
					<section>
						<H2 id="partial-validation">Partial Validation</H2>
						<p className="text-muted-foreground mb-4">
							You don't need to validate every field—only specify what matters:
						</p>
						<CodeBlockServer language="yaml">{partialValidationExample}</CodeBlockServer>
					</section>

					{/* Tips */}
					<section>
						<H2 id="tips">Validation Tips</H2>
						<div className="grid gap-4 md:grid-cols-2">
							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-green-500/10 p-2">
										<Code className="h-4 w-4 text-green-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Start Simple</h4>
										<p className="text-sm text-muted-foreground">
											Begin with status validation, then add body checks as needed.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-blue-500/10 p-2">
										<Regex className="h-4 w-4 text-blue-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Use Wildcards for Dynamic Data</h4>
										<p className="text-sm text-muted-foreground">
											Use <code className="bg-muted px-1 rounded">"*"</code> for timestamps, IDs,
											and generated values.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-purple-500/10 p-2">
										<List className="h-4 w-4 text-purple-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Validate Structure</h4>
										<p className="text-sm text-muted-foreground">
											Focus on ensuring the response has the right shape, not every exact value.
										</p>
									</div>
								</div>
							</div>

							<div className="rounded-lg border bg-card p-4">
								<div className="flex items-start gap-3">
									<div className="rounded-full bg-orange-500/10 p-2">
										<Shield className="h-4 w-4 text-orange-600" />
									</div>
									<div>
										<h4 className="font-medium mb-1">Include Negative Tests</h4>
										<p className="text-sm text-muted-foreground">
											Test that your API rejects invalid requests properly.
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
								<h4 className="font-medium mb-1">Authentication Workflows</h4>
								<p className="text-sm text-muted-foreground mb-2">
									Apply validation to auth flows
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

							<div className="rounded-lg border bg-card p-4">
								<h4 className="font-medium mb-1">Validation Reference</h4>
								<p className="text-sm text-muted-foreground mb-2">
									Complete validation rules documentation
								</p>
								<Button asChild variant="ghost" size="sm" className="p-0">
									<Link
										href="/docs/api-reference/validation-rules"
										className="flex items-center"
									>
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
