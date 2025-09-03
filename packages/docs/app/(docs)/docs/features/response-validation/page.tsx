import { DocsPageHeader } from "@/components/docs-page-header"
import { CodeBlockServer } from "@/components/code-block-server"
import { TableOfContents } from "@/components/toc"
import { H2, H3 } from "@/components/docs-heading"
import { Badge } from "@/components/ui/badge"
import { Target, Search, Zap, Eye, Shield, CheckCircle, AlertTriangle, Lightbulb } from "lucide-react"

const statusValidation = `# Validate response status codes
request:
  name: API Health Check
  url: https://api.example.com/health
  method: GET
  expect:
    status: 200  # Expect exactly 200
    
# Multiple accepted status codes
request:
  name: Get User
  url: https://api.example.com/users/123
  method: GET
  expect:
    status: [200, 304]  # Accept either 200 or 304`

const headerValidation = `# Validate response headers
request:
  name: API Response Headers
  url: https://api.example.com/data
  method: GET
  expect:
    status: 200
    headers:
      content-type: application/json
      x-api-version: "v2"
      cache-control: "no-cache"`

const bodyValidation = `# Validate response body content
request:
  name: Get User Profile
  url: https://api.example.com/users/1
  method: GET
  expect:
    status: 200
    body:
      id: 1
      username: "johndoe"
      email: "john@example.com"
      active: true
      
# Partial matching
request:
  name: Check API Response
  url: https://api.example.com/status
  method: GET
  expect:
    body:
      status: "operational"  # Only check specific fields`

const complexValidation = `# Complex validation scenarios
requests:
  - name: Create Resource
    url: https://api.example.com/resources
    method: POST
    headers:
      Content-Type: application/json
    body:
      name: "Test Resource"
      type: "document"
    expect:
      status: 201
      headers:
        location: "^/resources/[0-9]+$"  # Regex pattern
      body:
        id: "^[0-9]+$"  # Validate format
        name: "Test Resource"
        createdAt: "^\\d{4}-\\d{2}-\\d{2}"  # Date format
        
  - name: Validate Array Response
    url: https://api.example.com/items
    method: GET
    expect:
      status: 200
      body:
        - id: 1
          name: "Item 1"
        - id: 2
          name: "Item 2"`

const collectionValidation = `# Validation in collections
global:
  variables:
    BASE_URL: https://api.example.com
  continueOnError: false  # Stop on validation failure

collection:
  name: API Test Suite
  defaults:
    expect:
      headers:
        content-type: application/json  # Default for all requests
  
  requests:
    - name: Login
      url: \${BASE_URL}/auth/login
      method: POST
      body:
        username: "testuser"
        password: "testpass"
      expect:
        status: 200
        body:
          token: "^[A-Za-z0-9-_]+\\.[A-Za-z0-9-_]+\\.[A-Za-z0-9-_]+$"  # JWT format
          
    - name: Get Protected Resource
      url: \${BASE_URL}/protected
      headers:
        Authorization: "Bearer \${token}"
      expect:
        status: 200
        body:
          data: "*"  # Any non-null value`

export default function ResponseValidationPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Response Validation"
          text="Validate HTTP responses to ensure your APIs return expected results."
        />

        <div className="space-y-8">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground mb-6">
              Response validation allows you to verify that your API responses meet expected criteria. curl-runner can validate status codes, headers, and response body content, making it perfect for API testing and monitoring.
            </p>
            
            <div className="rounded-lg border bg-muted/50 p-4">
              <p className="text-sm">
                <strong>Tip:</strong> Use the <code className="text-xs bg-background px-1 py-0.5 rounded">expect</code> field in your request configuration to define validation rules. If validation fails, curl-runner will report the mismatch and mark the request as failed.
              </p>
            </div>
          </section>

          {/* Status Code Validation */}
          <section>
            <H2 id="status-validation">Status Code Validation</H2>
            <p className="text-muted-foreground mb-6">
              Validate that responses return expected HTTP status codes.
            </p>
            
            <CodeBlockServer language="yaml" filename="status-validation.yaml">
              {statusValidation}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="status-patterns">Common Status Patterns</H3>
              <div className="grid gap-3 sm:grid-cols-2">
                <div className="rounded-lg border p-3">
                  <div className="flex items-center gap-2 mb-1">
                    <Badge variant="outline">2xx Success</Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">200, 201, 204</p>
                </div>
                <div className="rounded-lg border p-3">
                  <div className="flex items-center gap-2 mb-1">
                    <Badge variant="outline">3xx Redirect</Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">301, 302, 304</p>
                </div>
                <div className="rounded-lg border p-3">
                  <div className="flex items-center gap-2 mb-1">
                    <Badge variant="outline">4xx Client Error</Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">400, 401, 403, 404</p>
                </div>
                <div className="rounded-lg border p-3">
                  <div className="flex items-center gap-2 mb-1">
                    <Badge variant="outline">5xx Server Error</Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">500, 502, 503</p>
                </div>
              </div>
            </div>
          </section>

          {/* Header Validation */}
          <section>
            <H2 id="header-validation">Header Validation</H2>
            <p className="text-muted-foreground mb-6">
              Verify that responses include expected headers with correct values.
            </p>
            
            <CodeBlockServer language="yaml" filename="header-validation.yaml">
              {headerValidation}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-blue-500/5 dark:bg-blue-500/10 border-blue-500/20 p-4">
              <p className="text-sm">
                <strong className="text-blue-600 dark:text-blue-400">Note:</strong> Header names are case-insensitive, but values are case-sensitive. Use exact values or regex patterns for flexible matching.
              </p>
            </div>
          </section>

          {/* Body Validation */}
          <section>
            <H2 id="body-validation">Body Validation</H2>
            <p className="text-muted-foreground mb-6">
              Validate response body content for JSON, text, or other formats.
            </p>
            
            <CodeBlockServer language="yaml" filename="body-validation.yaml">
              {bodyValidation}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="validation-types">Validation Types</H3>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <Target className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Exact Match</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Specify exact values that must match in the response.
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">{"body: { status: \"ok\" }"}</code>
                    </div>
                  </div>
                </div>
                
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-green-500/10 p-2">
                      <Eye className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Partial Match</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Only validate specific fields, ignore others.
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">{"body: { id: 123 } # Other fields ignored"}</code>
                    </div>
                  </div>
                </div>
                
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-purple-500/10 p-2">
                      <Search className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Pattern Match</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Use regex patterns for flexible validation.
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">{"body: { email: \"^[a-z]+@[a-z]+\\\\.[a-z]+$\" }"}</code>
                    </div>
                  </div>
                </div>
                
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-yellow-500/10 p-2">
                      <Zap className="h-4 w-4 text-yellow-600 dark:text-yellow-500" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Wildcard Match</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Use "*" to check for presence of a field with any value.
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">{"body: { token: \"*\" } # Field must exist"}</code>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Complex Validation */}
          <section>
            <H2 id="complex-validation">Complex Validation</H2>
            <p className="text-muted-foreground mb-6">
              Combine multiple validation rules for comprehensive testing.
            </p>
            
            <CodeBlockServer language="yaml" filename="complex-validation.yaml">
              {complexValidation}
            </CodeBlockServer>
          </section>

          {/* Collection Validation */}
          <section>
            <H2 id="collection-validation">Collection Validation</H2>
            <p className="text-muted-foreground mb-6">
              Apply validation rules across collections with defaults and overrides.
            </p>
            
            <CodeBlockServer language="yaml" filename="collection-validation.yaml">
              {collectionValidation}
            </CodeBlockServer>
          </section>

          {/* Validation Reference */}
          <section>
            <H2 id="validation-reference">Validation Reference</H2>
            <div className="border rounded-lg overflow-hidden">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Field</th>
                    <th className="text-left p-3 font-medium">Type</th>
                    <th className="text-left p-3 font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">expect.status</code></td>
                    <td className="p-3 text-sm text-muted-foreground">number | number[]</td>
                    <td className="p-3 text-sm text-muted-foreground">Expected HTTP status code(s)</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">expect.headers</code></td>
                    <td className="p-3 text-sm text-muted-foreground">object</td>
                    <td className="p-3 text-sm text-muted-foreground">Expected response headers</td>
                  </tr>
                  <tr>
                    <td className="p-3"><code className="text-sm">expect.body</code></td>
                    <td className="p-3 text-sm text-muted-foreground">any</td>
                    <td className="p-3 text-sm text-muted-foreground">Expected response body content</td>
                  </tr>
                </tbody>
              </table>
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
                    <h4 className="font-medium mb-2">Start Simple</h4>
                    <p className="text-sm text-muted-foreground">
                      Begin with basic status code validation, then add more complex rules as needed.
                    </p>
                  </div>
                </div>
              </div>
              
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Eye className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Use Partial Matching</h4>
                    <p className="text-sm text-muted-foreground">
                      Don't validate entire response bodies unless necessary. Focus on critical fields to avoid brittle tests.
                    </p>
                  </div>
                </div>
              </div>
              
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Search className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Leverage Patterns</h4>
                    <p className="text-sm text-muted-foreground">
                      Use regex patterns for dynamic values like IDs, timestamps, or tokens that change between requests.
                    </p>
                  </div>
                </div>
              </div>
              
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-cyan-500/10 p-2">
                    <Shield className="h-4 w-4 text-cyan-600 dark:text-cyan-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Group Related Validations</h4>
                    <p className="text-sm text-muted-foreground">
                      Use collection defaults to apply common validation rules across multiple requests.
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
  )
}