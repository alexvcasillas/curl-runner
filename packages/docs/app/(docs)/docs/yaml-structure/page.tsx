import { DocsPageHeader } from "@/components/docs-page-header"
import { CodeBlockServer } from "@/components/code-block-server"
import { Badge } from "@/components/ui/badge"
import { TableOfContents } from "@/components/toc"
import { H2, H3 } from "@/components/docs-heading"

const singleRequestExample = `# Single HTTP request
request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET
  headers:
    Authorization: Bearer $\{API_TOKEN}
    Content-Type: application/json`

const multipleRequestsExample = `# Multiple requests collection
requests:
  - name: Create User
    url: https://api.example.com/users
    method: POST
    headers:
      Content-Type: application/json
    body:
      name: John Doe
      email: john@example.com
      
  - name: Get Created User
    url: https://api.example.com/users/\${USER_ID}
    method: GET
    headers:
      Authorization: Bearer $\{API_TOKEN}`

const collectionExample = `# Advanced collection with global settings
global:
  variables:
    BASE_URL: https://api.example.com
    API_TOKEN: your-api-token-here
  execution: parallel
  continueOnError: true
  output:
    verbose: true
    saveToFile: results.json

collection:
  name: User Management API Tests
  variables:
    USER_ID: 123
  defaults:
    headers:
      Authorization: Bearer $\{API_TOKEN}
      Content-Type: application/json
  
  requests:
    - name: List Users
      url: $\{BASE_URL}/users
      method: GET
      
    - name: Get Specific User
      url: $\{BASE_URL}/users/$\{USER_ID}
      method: GET
      
    - name: Update User
      url: $\{BASE_URL}/users/$\{USER_ID}
      method: PATCH
      body:
        name: Updated Name`

const validationExample = `# Request with response validation
request:
  name: API Health Check
  url: https://api.example.com/health
  method: GET
  validation:
    status: 200
    headers:
      content-type: application/json
    body:
      status: ok
      version: "^1.0.0"
  timeout: 5000
  retries: 3`

export default function YamlStructurePage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="YAML Structure"
          text="Learn the structure and syntax of curl-runner YAML configuration files."
        />

        <div className="space-y-8">
          {/* Basic Structure */}
          <section>
            <H2 id="basic-structure">Basic Structure</H2>
            <p className="text-muted-foreground mb-6">
              <code className="font-mono">curl-runner</code> uses YAML files to define HTTP requests. There are several ways to structure your configuration files.
            </p>

            <div className="space-y-8">
              {/* Single Request */}
              <div className="space-y-3">
                <H3 id="single-request">
                  Single Request
                  <Badge variant="secondary" className="ml-2">Basic</Badge>
                </H3>
                <p className="text-sm text-muted-foreground">
                  The simplest form - define a single HTTP request.
                </p>
                <CodeBlockServer language="yaml" filename="single-request.yaml">
                  {singleRequestExample}
                </CodeBlockServer>
              </div>

              {/* Multiple Requests */}
              <div className="space-y-3">
                <H3 id="multiple-requests">Multiple Requests</H3>
                <p className="text-sm text-muted-foreground">
                  Execute multiple HTTP requests in sequence or parallel.
                </p>
                <CodeBlockServer language="yaml" filename="multiple-requests.yaml">
                  {multipleRequestsExample}
                </CodeBlockServer>
              </div>

              {/* Collection */}
              <div className="space-y-3">
                <H3 id="collection">
                  Collection
                  <Badge className="bg-cyan-500/10 text-cyan-600 dark:text-cyan-400 border-cyan-500/20 ml-2">Advanced</Badge>
                </H3>
                <p className="text-sm text-muted-foreground">
                  Advanced structure with global settings, variables, and defaults.
                </p>
                <CodeBlockServer language="yaml" filename="collection.yaml">
                  {collectionExample}
                </CodeBlockServer>
              </div>
            </div>
          </section>

          {/* Request Properties */}
          <section>
            <H2 id="request-properties">Request Properties</H2>
            <p className="text-muted-foreground mb-6">
              Each request can have the following properties:
            </p>
            
            <div className="space-y-6">
              <div className="space-y-3">
                <H3 id="required-properties">Required Properties</H3>
                <div className="border rounded-lg overflow-hidden">
                  <table className="w-full">
                    <thead className="bg-muted/50">
                      <tr className="border-b">
                        <th className="text-left p-3 font-medium">Property</th>
                        <th className="text-left p-3 font-medium">Type</th>
                        <th className="text-left p-3 font-medium">Description</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr className="border-b">
                        <td className="p-3"><code className="text-sm">url</code></td>
                        <td className="p-3 text-sm text-muted-foreground">string</td>
                        <td className="p-3 text-sm text-muted-foreground">The URL to send the request to</td>
                      </tr>
                      <tr>
                        <td className="p-3"><code className="text-sm">method</code></td>
                        <td className="p-3 text-sm text-muted-foreground">string</td>
                        <td className="p-3 text-sm text-muted-foreground">HTTP method (GET, POST, PUT, DELETE, etc.)</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>

              <div className="space-y-3">
                <H3 id="optional-properties">Optional Properties</H3>
                <div className="border rounded-lg overflow-hidden">
                  <table className="w-full">
                    <thead className="bg-muted/50">
                      <tr className="border-b">
                        <th className="text-left p-3 font-medium">Property</th>
                        <th className="text-left p-3 font-medium">Type</th>
                        <th className="text-left p-3 font-medium">Description</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr className="border-b">
                        <td className="p-3"><code className="text-sm">name</code></td>
                        <td className="p-3 text-sm text-muted-foreground">string</td>
                        <td className="p-3 text-sm text-muted-foreground">Display name for the request</td>
                      </tr>
                      <tr className="border-b">
                        <td className="p-3"><code className="text-sm">headers</code></td>
                        <td className="p-3 text-sm text-muted-foreground">object</td>
                        <td className="p-3 text-sm text-muted-foreground">HTTP headers to send with the request</td>
                      </tr>
                      <tr className="border-b">
                        <td className="p-3"><code className="text-sm">body</code></td>
                        <td className="p-3 text-sm text-muted-foreground">object | string</td>
                        <td className="p-3 text-sm text-muted-foreground">Request body data</td>
                      </tr>
                      <tr className="border-b">
                        <td className="p-3"><code className="text-sm">timeout</code></td>
                        <td className="p-3 text-sm text-muted-foreground">number</td>
                        <td className="p-3 text-sm text-muted-foreground">Request timeout in milliseconds</td>
                      </tr>
                      <tr className="border-b">
                        <td className="p-3"><code className="text-sm">retries</code></td>
                        <td className="p-3 text-sm text-muted-foreground">number</td>
                        <td className="p-3 text-sm text-muted-foreground">Number of retry attempts</td>
                      </tr>
                      <tr>
                        <td className="p-3"><code className="text-sm">validation</code></td>
                        <td className="p-3 text-sm text-muted-foreground">object</td>
                        <td className="p-3 text-sm text-muted-foreground">Response validation rules</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          </section>

          {/* Global Configuration */}
          <section>
            <H2 id="global-configuration">Global Configuration</H2>
            <p className="text-muted-foreground mb-6">
              The <code className="text-sm bg-muted px-1 py-0.5 rounded">global</code> section allows you to configure settings that apply to all requests in the file.
            </p>
            
            <div className="border rounded-lg overflow-hidden">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Setting</th>
                    <th className="text-left p-3 font-medium">Type</th>
                    <th className="text-left p-3 font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">variables</code></td>
                    <td className="p-3 text-sm text-muted-foreground">object</td>
                    <td className="p-3 text-sm text-muted-foreground">Global variables available to all requests</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">execution</code></td>
                    <td className="p-3 text-sm text-muted-foreground">string</td>
                    <td className="p-3 text-sm text-muted-foreground">"sequential" or "parallel" execution mode</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">continueOnError</code></td>
                    <td className="p-3 text-sm text-muted-foreground">boolean</td>
                    <td className="p-3 text-sm text-muted-foreground">Continue execution if a request fails</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">output.verbose</code></td>
                    <td className="p-3 text-sm text-muted-foreground">boolean</td>
                    <td className="p-3 text-sm text-muted-foreground">Show detailed output during execution</td>
                  </tr>
                  <tr>
                    <td className="p-3"><code className="text-sm">output.saveToFile</code></td>
                    <td className="p-3 text-sm text-muted-foreground">string</td>
                    <td className="p-3 text-sm text-muted-foreground">Save results to a JSON file</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Response Validation */}
          <section>
            <H2 id="response-validation">Response Validation</H2>
            <p className="text-muted-foreground mb-4">
              Add validation rules to verify that responses meet your expectations.
            </p>
            
            <CodeBlockServer language="yaml" filename="validation-example.yaml">
              {validationExample}
            </CodeBlockServer>
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