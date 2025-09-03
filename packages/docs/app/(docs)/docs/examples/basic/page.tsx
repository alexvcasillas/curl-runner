import { DocsPageHeader } from "@/components/docs-page-header"
import { CodeBlockServer } from "@/components/code-block-server"
import { Badge } from "@/components/ui/badge"
import { Lightbulb, Globe, Bug, Zap } from "lucide-react"

const examples = [
  {
    title: "Simple GET Request",
    description: "Basic HTTP GET request to fetch data",
    badge: "Basic",
    filename: "get-request.yaml",
    code: `# Simple GET request
request:
  name: Get JSONPlaceholder Post
  url: https://jsonplaceholder.typicode.com/posts/1
  method: GET`
  },
  {
    title: "POST Request with JSON Body",
    description: "Create a new resource with JSON payload",
    badge: "Basic",
    filename: "post-request.yaml",
    code: `# POST request with JSON body
request:
  name: Create New Post
  url: https://jsonplaceholder.typicode.com/posts
  method: POST
  headers:
    Content-Type: application/json
  body:
    title: My New Post
    body: This is the content of my new post
    userId: 1`
  },
  {
    title: "Request with Headers and Authentication",
    description: "Include custom headers and Bearer token authentication",
    badge: "Auth",
    filename: "auth-request.yaml",
    code: `# Request with authentication
request:
  name: Get Protected Resource
  url: https://api.example.com/protected/resource
  method: GET
  headers:
    Authorization: Bearer your-token-here
    Content-Type: application/json
    X-API-Version: "1.0"
    User-Agent: curl-runner/1.0.0`
  },
  {
    title: "PUT Request to Update Resource",
    description: "Update an existing resource completely",
    badge: "Basic",
    filename: "put-request.yaml",
    code: `# PUT request to update resource
request:
  name: Update Post
  url: https://jsonplaceholder.typicode.com/posts/1
  method: PUT
  headers:
    Content-Type: application/json
  body:
    id: 1
    title: Updated Post Title
    body: This post has been updated
    userId: 1`
  },
  {
    title: "PATCH Request for Partial Update",
    description: "Update specific fields of a resource",
    badge: "Basic",
    filename: "patch-request.yaml",
    code: `# PATCH request for partial update
request:
  name: Update Post Title
  url: https://jsonplaceholder.typicode.com/posts/1
  method: PATCH
  headers:
    Content-Type: application/json
  body:
    title: Partially Updated Title`
  },
  {
    title: "DELETE Request",
    description: "Remove a resource from the server",
    badge: "Basic",
    filename: "delete-request.yaml",
    code: `# DELETE request
request:
  name: Delete Post
  url: https://jsonplaceholder.typicode.com/posts/1
  method: DELETE
  headers:
    Authorization: Bearer your-token-here`
  },
  {
    title: "Request with Query Parameters",
    description: "Include URL query parameters in your request",
    badge: "Basic",
    filename: "query-params.yaml",
    code: `# Request with query parameters
request:
  name: Search Posts
  url: https://jsonplaceholder.typicode.com/posts?userId=1&_limit=5
  method: GET
  headers:
    Accept: application/json`
  },
  {
    title: "Form Data Request",
    description: "Send form-encoded data (application/x-www-form-urlencoded)",
    badge: "Forms",
    filename: "form-request.yaml",
    code: `# Form data request
request:
  name: Submit Contact Form
  url: https://httpbin.org/post
  method: POST
  headers:
    Content-Type: application/x-www-form-urlencoded
  body: "name=John+Doe&email=john@example.com&message=Hello+World"`
  },
  {
    title: "Request with Timeout and Retries",
    description: "Configure request timeout and retry behavior",
    badge: "Advanced",
    filename: "timeout-retry.yaml",
    code: `# Request with timeout and retries
request:
  name: Slow API Call
  url: https://httpbin.org/delay/2
  method: GET
  timeout: 5000  # 5 seconds
  retries: 3
  headers:
    Accept: application/json`
  }
]

const runCommands = `# Run a specific example
curl-runner get-request.yaml

# Run with verbose output
curl-runner post-request.yaml -v

# Run with timeout
curl-runner auth-request.yaml --timeout 10000

# Save results to file
curl-runner query-params.yaml --output results.json`

export default function BasicExamplesPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Basic Examples"
          text="Simple HTTP request configurations to get you started with curl-runner. Copy and modify these examples for your own use cases."
        />

        <div className="space-y-8">
          {/* Examples */}
          <section>
            <div className="space-y-8">
              {examples.map((example, index) => (
                <div key={index} className="space-y-3">
                  <div className="flex items-center space-x-2">
                    <h3 className="text-xl font-semibold">{example.title}</h3>
                    <Badge variant={example.badge === "Advanced" ? "default" : "secondary"}>
                      {example.badge}
                    </Badge>
                  </div>
                  <p className="text-muted-foreground">{example.description}</p>
                  <CodeBlockServer language="yaml" filename={example.filename}>
                    {example.code}
                  </CodeBlockServer>
                </div>
              ))}
            </div>
          </section>

          {/* Running Examples */}
          <section>
            <h2 className="text-2xl font-semibold tracking-tight mb-4">Running the Examples</h2>
            <p className="text-muted-foreground mb-4">
              Save any of the above examples to a YAML file and run them with <code className="font-mono">curl-runner</code>:
            </p>
            <CodeBlockServer language="bash">
              {runCommands}
            </CodeBlockServer>
          </section>

          {/* Tips */}
          <section>
            <h2 className="text-2xl font-semibold tracking-tight mb-6">Tips</h2>
            <div className="grid gap-6 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-6">
                <div className="flex items-start gap-4">
                  <div className="rounded-full bg-blue-500/10 p-3">
                    <Globe className="h-5 w-5 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h3 className="font-medium mb-4">Testing APIs</h3>
                    <ul className="space-y-3 text-sm text-muted-foreground list-none">
                      <li className="relative pl-4 before:content-['•'] before:absolute before:left-0 before:text-blue-600 dark:before:text-blue-400">
                        Use <code className="text-xs bg-muted px-2 py-1 rounded">https://jsonplaceholder.typicode.com</code> for testing - it's a free REST API for testing and prototyping
                      </li>
                      <li className="relative pl-4 before:content-['•'] before:absolute before:left-0 before:text-blue-600 dark:before:text-blue-400">
                        Try <code className="text-xs bg-muted px-2 py-1 rounded">https://httpbin.org</code> for testing different HTTP scenarios like delays, status codes, and headers
                      </li>
                    </ul>
                  </div>
                </div>
              </div>
              
              <div className="rounded-lg border bg-card p-6">
                <div className="flex items-start gap-4">
                  <div className="rounded-full bg-green-500/10 p-3">
                    <Lightbulb className="h-5 w-5 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h3 className="font-medium mb-4">Best Practices</h3>
                    <ul className="space-y-3 text-sm text-muted-foreground list-none">
                      <li className="relative pl-4 before:content-['•'] before:absolute before:left-0 before:text-green-600 dark:before:text-green-400">
                        Always include meaningful names for your requests
                      </li>
                      <li className="relative pl-4 before:content-['•'] before:absolute before:left-0 before:text-green-600 dark:before:text-green-400">
                        Use appropriate HTTP methods (GET for reading, POST for creating, etc.)
                      </li>
                      <li className="relative pl-4 before:content-['•'] before:absolute before:left-0 before:text-green-600 dark:before:text-green-400">
                        Include proper Content-Type headers when sending data
                      </li>
                    </ul>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-6 md:col-span-2">
                <div className="flex items-start gap-4">
                  <div className="rounded-full bg-purple-500/10 p-3">
                    <Bug className="h-5 w-5 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <h3 className="font-medium mb-4">Debugging & Troubleshooting</h3>
                    <ul className="space-y-3 text-sm text-muted-foreground list-none">
                      <li className="relative pl-4 before:content-['•'] before:absolute before:left-0 before:text-purple-600 dark:before:text-purple-400">
                        Use the <code className="text-xs bg-muted px-2 py-1 rounded">--verbose</code> flag to see detailed request and response information
                      </li>
                      <li className="relative pl-4 before:content-['•'] before:absolute before:left-0 before:text-purple-600 dark:before:text-purple-400">
                        Check response headers for debugging information and API limits
                      </li>
                      <li className="relative pl-4 before:content-['•'] before:absolute before:left-0 before:text-purple-600 dark:before:text-purple-400">
                        Test with simple requests first, then add complexity gradually
                      </li>
                      <li className="relative pl-4 before:content-['•'] before:absolute before:left-0 before:text-purple-600 dark:before:text-purple-400">
                        Validate your YAML syntax if requests aren't working as expected
                      </li>
                    </ul>
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
          <div className="space-y-2">
            <p className="font-medium">On This Page</p>
            <ul className="m-0 list-none">
              <li className="mt-0 pt-2">
                <a
                  href="#examples-grid"
                  className="inline-block no-underline transition-colors hover:text-foreground text-muted-foreground"
                >
                  Examples
                </a>
              </li>
              <li className="mt-0 pt-2">
                <a
                  href="#running-examples"
                  className="inline-block no-underline transition-colors hover:text-foreground text-muted-foreground"
                >
                  Running Examples
                </a>
              </li>
              <li className="mt-0 pt-2">
                <a
                  href="#tips"
                  className="inline-block no-underline transition-colors hover:text-foreground text-muted-foreground"
                >
                  Tips
                </a>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </main>
  )
}