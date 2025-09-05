import {
  AlertCircle,
  AlertTriangle,
  ArrowRight,
  BarChart3,
  CheckCircle,
  Code,
  File,
  FileJson,
  FileText,
  Timer,
  Wifi,
  XCircle,
} from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'Response Object',
  description:
    'Complete reference for curl-runner response objects including status codes, headers, body content, metrics, and execution results.',
  keywords: [
    'curl-runner response object',
    'HTTP response structure',
    'response properties',
    'response status codes',
    'response headers',
    'response body',
    'response metrics',
    'execution results',
    'API response format',
    'response schema',
    'HTTP response data',
    'response handling',
  ],
  openGraph: {
    title: 'Response Object | curl-runner API Reference',
    description:
      'Complete reference for curl-runner response objects including status codes, headers, body content, metrics, and execution results.',
    url: 'https://curl-runner.com/docs/api-reference/response-object',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Response Object | curl-runner API Reference',
    description: 'Learn about curl-runner response object structure and available properties.',
  },
  alternates: {
    canonical: 'https://curl-runner.com/docs/api-reference/response-object',
  },
};

const executionResultExample = `# Example ExecutionResult object structure
{
  "request": {
    "name": "Get User Profile",
    "url": "https://api.example.com/users/123",
    "method": "GET",
    "headers": {
      "Authorization": "Bearer token123",
      "Accept": "application/json"
    },
    "timeout": 5000
  },
  "success": true,
  "status": 200,
  "headers": {
    "content-type": "application/json; charset=utf-8",
    "content-length": "256",
    "date": "Wed, 21 Oct 2024 07:28:00 GMT",
    "server": "nginx/1.18.0",
    "x-rate-limit-remaining": "99",
    "x-rate-limit-reset": "1634799480",
    "etag": "W/\\"d41d8cd98f00b204e9800998ecf8427e\\"",
    "cache-control": "no-cache, no-store, must-revalidate"
  },
  "body": {
    "id": 123,
    "username": "johndoe",
    "email": "john@example.com",
    "profile": {
      "firstName": "John",
      "lastName": "Doe",
      "createdAt": "2024-01-15T09:30:00Z",
      "lastLogin": "2024-10-21T07:25:00Z"
    },
    "preferences": {
      "theme": "dark",
      "notifications": true,
      "language": "en"
    }
  },
  "metrics": {
    "duration": 245,
    "size": 256,
    "dnsLookup": 12,
    "tcpConnection": 45,
    "tlsHandshake": 67,
    "firstByte": 189,
    "download": 8
  }
}`;

const errorResultExample = `# Example failed request ExecutionResult
{
  "request": {
    "name": "Failed Request",
    "url": "https://api.unreachable.com/data",
    "method": "GET",
    "timeout": 3000
  },
  "success": false,
  "error": "Connection timeout after 3000ms",
  "metrics": {
    "duration": 3000,
    "dnsLookup": 2500,
    "tcpConnection": 500
  }
}`;

const executionSummaryExample = `# Example ExecutionSummary for a collection run
{
  "total": 5,
  "successful": 3,
  "failed": 2,
  "duration": 2456,
  "results": [
    {
      "request": {
        "name": "Get Users List",
        "url": "https://api.example.com/users",
        "method": "GET"
      },
      "success": true,
      "status": 200,
      "body": [
        {"id": 1, "username": "user1"},
        {"id": 2, "username": "user2"}
      ],
      "metrics": {
        "duration": 234,
        "size": 124
      }
    },
    {
      "request": {
        "name": "Create User",
        "url": "https://api.example.com/users",
        "method": "POST"
      },
      "success": true,
      "status": 201,
      "headers": {
        "location": "/users/3",
        "content-type": "application/json"
      },
      "body": {
        "id": 3,
        "username": "newuser",
        "createdAt": "2024-10-21T07:30:00Z"
      },
      "metrics": {
        "duration": 456,
        "size": 89
      }
    },
    {
      "request": {
        "name": "Get Invalid User",
        "url": "https://api.example.com/users/999",
        "method": "GET"
      },
      "success": false,
      "status": 404,
      "body": {
        "error": "User not found",
        "code": "USER_NOT_FOUND"
      },
      "metrics": {
        "duration": 123,
        "size": 45
      }
    },
    {
      "request": {
        "name": "Update User",
        "url": "https://api.example.com/users/1",
        "method": "PUT"
      },
      "success": true,
      "status": 200,
      "body": {
        "id": 1,
        "username": "updated_user",
        "updatedAt": "2024-10-21T07:31:00Z"
      },
      "metrics": {
        "duration": 345,
        "size": 78
      }
    },
    {
      "request": {
        "name": "Timeout Request",
        "url": "https://api.slow.com/data",
        "method": "GET"
      },
      "success": false,
      "error": "Request timeout after 5000ms",
      "metrics": {
        "duration": 5000
      }
    }
  ]
}`;

const responseBodyTypesExample = `# Different response body types curl-runner handles

# JSON Object Response
{
  "body": {
    "user": {
      "id": 123,
      "name": "John Doe"
    },
    "metadata": {
      "timestamp": "2024-10-21T07:30:00Z"
    }
  }
}

# JSON Array Response  
{
  "body": [
    {"id": 1, "name": "Item 1"},
    {"id": 2, "name": "Item 2"},
    {"id": 3, "name": "Item 3"}
  ]
}

# Plain Text Response
{
  "body": "This is a plain text response from the server"
}

# Number Response
{
  "body": 42
}

# Boolean Response
{
  "body": true
}

# Null Response
{
  "body": null
}

# Empty Response (no body)
{
  "status": 204,
  "headers": {
    "content-length": "0"
  }
  // No body field present
}`;

const metricsDetailsExample = `# Detailed explanation of response metrics

{
  "metrics": {
    "duration": 1245,        // Total request duration in milliseconds
    "size": 2048,           // Response body size in bytes
    "dnsLookup": 23,        // Time to resolve DNS in milliseconds
    "tcpConnection": 45,     // Time to establish TCP connection
    "tlsHandshake": 127,    // Time for TLS/SSL handshake (HTTPS only)
    "firstByte": 856,       // Time to first response byte (TTFB)
    "download": 67          // Time to download response body
  }
}

# Metrics breakdown:
# - Total time = dnsLookup + tcpConnection + tlsHandshake + firstByte + download
# - Some metrics may be 0 or undefined based on request type and caching
# - DNS lookup may be 0 if DNS is cached
# - TLS handshake only present for HTTPS requests
# - Connection time may be 0 if connection is reused`;

const statusCodesExample = `# Common HTTP status codes in responses

# Success responses (2xx)
{
  "success": true,
  "status": 200,    // OK - Request successful
  "body": "data"
}

{
  "success": true,
  "status": 201,    // Created - Resource created successfully
  "headers": {
    "location": "/users/123"
  }
}

{
  "success": true,
  "status": 204,    // No Content - Successful with no body
  "headers": {
    "content-length": "0"
  }
}

# Client error responses (4xx)
{
  "success": false,
  "status": 400,    // Bad Request
  "body": {
    "error": "Invalid request parameters",
    "details": ["Missing required field: email"]
  }
}

{
  "success": false,
  "status": 401,    // Unauthorized
  "body": {
    "error": "Authentication required",
    "code": "UNAUTHORIZED"
  }
}

{
  "success": false,
  "status": 404,    // Not Found
  "body": {
    "error": "Resource not found",
    "code": "NOT_FOUND"
  }
}

{
  "success": false,
  "status": 429,    // Too Many Requests
  "headers": {
    "retry-after": "60",
    "x-rate-limit-remaining": "0"
  },
  "body": {
    "error": "Rate limit exceeded"
  }
}

# Server error responses (5xx)
{
  "success": false,
  "status": 500,    // Internal Server Error
  "body": {
    "error": "Internal server error",
    "code": "INTERNAL_ERROR"
  }
}

{
  "success": false,
  "status": 503,    // Service Unavailable
  "headers": {
    "retry-after": "120"
  },
  "body": {
    "error": "Service temporarily unavailable"
  }
}`;

const headerTypesExample = `# Common response headers and their meanings

{
  "headers": {
    // Content headers
    "content-type": "application/json; charset=utf-8",
    "content-length": "1024",
    "content-encoding": "gzip",
    "content-language": "en-US",
    
    // Caching headers
    "cache-control": "public, max-age=3600",
    "etag": "W/\\"686897696a7c876b7e\\"",
    "expires": "Thu, 22 Oct 2024 07:30:00 GMT",
    "last-modified": "Wed, 21 Oct 2024 07:30:00 GMT",
    
    // Security headers
    "strict-transport-security": "max-age=31536000",
    "content-security-policy": "default-src 'self'",
    "x-frame-options": "DENY",
    "x-content-type-options": "nosniff",
    
    // Rate limiting headers
    "x-rate-limit-limit": "1000",
    "x-rate-limit-remaining": "999",
    "x-rate-limit-reset": "1634799600",
    
    // Server information
    "server": "nginx/1.18.0",
    "x-powered-by": "Express",
    "date": "Wed, 21 Oct 2024 07:28:00 GMT",
    
    // Custom API headers
    "x-api-version": "v2",
    "x-request-id": "req_123456789",
    "x-response-time": "45ms",
    
    // Location header (for redirects/created resources)
    "location": "https://api.example.com/users/123"
  }
}`;

const processingResponsesExample = `# How to process response data programmatically

# Save response to file for processing
request:
  name: "Get Large Dataset"
  url: "https://api.example.com/dataset"
  output: "dataset.json"
  
# Use response validation to check structure
request:
  name: "Validate API Response"
  url: "https://api.example.com/user/profile"
  expect:
    status: 200
    headers:
      content-type: "application/json"
    body:
      id: "^[0-9]+$"        # Validate ID format
      email: ".*@.*\\..*"    # Validate email format
      created_at: "^\\d{4}-\\d{2}-\\d{2}"  # ISO date format
      
# Chain requests using response data
collection:
  name: "Request Chain Example"
  requests:
    # First request - get user ID
    - name: "Login User"
      url: "https://api.example.com/auth/login"
      method: POST
      body:
        username: "testuser"
        password: "password"
      expect:
        status: 200
        body:
          token: "*"        # Any token value
          user_id: "*"      # Any user ID
          
    # Second request - use data from first
    - name: "Get User Data"
      url: "https://api.example.com/users/\${USER_ID}"  # From previous response
      headers:
        Authorization: "Bearer \${TOKEN}"              # From previous response
      expect:
        status: 200`;

export default function ResponseObjectPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Response Object API Reference"
          text="Complete reference for response objects returned by curl-runner, including ExecutionResult and ExecutionSummary interfaces."
        />

        <div className="space-y-8">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground mb-6">
              curl-runner provides detailed response information through structured objects. Every
              request returns an ExecutionResult, and collections return an ExecutionSummary
              containing multiple results with aggregate statistics.
            </p>

            <div className="rounded-lg border bg-muted/50 p-4">
              <p className="text-sm">
                <strong>Note:</strong> Response objects are available in verbose output, saved
                files, and can be used for programmatic processing of API test results.
              </p>
            </div>
          </section>

          {/* ExecutionResult */}
          <section>
            <H2 id="execution-result">ExecutionResult Interface</H2>
            <p className="text-muted-foreground mb-6">
              The ExecutionResult interface contains complete information about a single request
              execution, including the original request configuration, response data, and
              performance metrics.
            </p>

            <CodeBlockServer language="json" filename="execution-result.json">
              {executionResultExample}
            </CodeBlockServer>

            <div className="mt-6 border rounded-lg overflow-hidden">
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
                    <td className="p-3">
                      <code className="text-sm">request</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">RequestConfig</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Original request configuration that was executed
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">success</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">boolean</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Whether the request completed successfully
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">status</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">number?</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      HTTP status code (if response received)
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">headers</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Record&lt;string, string&gt;?
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Response headers (if response received)
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">body</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">JsonValue?</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Response body content (parsed if JSON)
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">error</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">string?</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Error message (if request failed)
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">metrics</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">MetricsObject?</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Performance timing information
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Error Responses */}
          <section>
            <H2 id="error-responses">Error Response Structure</H2>
            <p className="text-muted-foreground mb-6">
              When requests fail due to network issues, timeouts, or other errors, the
              ExecutionResult provides error information.
            </p>

            <CodeBlockServer language="json" filename="error-result.json">
              {errorResultExample}
            </CodeBlockServer>

            <div className="mt-6 grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-red-500/10 p-2">
                    <Wifi className="h-4 w-4 text-red-600 dark:text-red-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Network Errors</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Connection refused, DNS resolution failures, network timeouts
                    </p>
                    <Badge variant="destructive">success: false</Badge>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <AlertCircle className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">HTTP Error Status</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      4xx client errors, 5xx server errors with response body
                    </p>
                    <Badge variant="outline">success: depends on validation</Badge>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* ExecutionSummary */}
          <section>
            <H2 id="execution-summary">ExecutionSummary Interface</H2>
            <p className="text-muted-foreground mb-6">
              The ExecutionSummary provides aggregate information about a collection of requests,
              including overall statistics and all individual results.
            </p>

            <CodeBlockServer language="json" filename="execution-summary.json">
              {executionSummaryExample}
            </CodeBlockServer>

            <div className="mt-6 border rounded-lg overflow-hidden">
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
                    <td className="p-3">
                      <code className="text-sm">total</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">number</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Total number of requests executed
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">successful</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">number</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Number of successful requests
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">failed</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">number</td>
                    <td className="p-3 text-sm text-muted-foreground">Number of failed requests</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">duration</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">number</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Total execution time in milliseconds
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">results</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">ExecutionResult[]</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Array of individual request results
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Response Body Types */}
          <section>
            <H2 id="response-body-types">Response Body Types</H2>
            <p className="text-muted-foreground mb-6">
              curl-runner automatically parses JSON responses and preserves the original data types
              for all response content.
            </p>

            <CodeBlockServer language="json" filename="body-types.json">
              {responseBodyTypesExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="json-parsing">JSON Response Parsing</H3>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <FileJson className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Automatic Parsing</h4>
                      <p className="text-sm text-muted-foreground">
                        JSON responses are automatically parsed into objects/arrays
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-green-500/10 p-2">
                      <Code className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Type Preservation</h4>
                      <p className="text-sm text-muted-foreground">
                        Numbers, booleans, null values maintain original types
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-purple-500/10 p-2">
                      <FileText className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Text Fallback</h4>
                      <p className="text-sm text-muted-foreground">
                        Non-JSON responses stored as plain text strings
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-orange-500/10 p-2">
                      <File className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Empty Responses</h4>
                      <p className="text-sm text-muted-foreground">
                        204 No Content and empty responses omit body field
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Metrics Object */}
          <section>
            <H2 id="metrics-object">Performance Metrics</H2>
            <p className="text-muted-foreground mb-6">
              Detailed timing information for analyzing request performance and identifying
              bottlenecks.
            </p>

            <CodeBlockServer language="json" filename="metrics-details.json">
              {metricsDetailsExample}
            </CodeBlockServer>

            <div className="mt-6 border rounded-lg overflow-hidden">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Metric</th>
                    <th className="text-left p-3 font-medium">Unit</th>
                    <th className="text-left p-3 font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">duration</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">milliseconds</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Total request time from start to finish
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">size</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">bytes</td>
                    <td className="p-3 text-sm text-muted-foreground">Response body size</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">dnsLookup</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">milliseconds</td>
                    <td className="p-3 text-sm text-muted-foreground">DNS resolution time</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">tcpConnection</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">milliseconds</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      TCP connection establishment time
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">tlsHandshake</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">milliseconds</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      TLS/SSL handshake time (HTTPS only)
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">firstByte</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">milliseconds</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Time to first response byte (TTFB)
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">download</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">milliseconds</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Response body download time
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* HTTP Status Codes */}
          <section>
            <H2 id="status-codes">HTTP Status Code Handling</H2>
            <p className="text-muted-foreground mb-6">
              Understanding how curl-runner processes different HTTP status codes and determines
              success/failure.
            </p>

            <CodeBlockServer language="json" filename="status-codes.json">
              {statusCodesExample}
            </CodeBlockServer>

            <div className="mt-6 grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      2xx Success
                      <Badge variant="default">Success</Badge>
                    </h4>
                    <div className="text-sm text-muted-foreground space-y-1">
                      <p>200 OK</p>
                      <p>201 Created</p>
                      <p>204 No Content</p>
                    </div>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <ArrowRight className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      3xx Redirect
                      <Badge variant="secondary">Redirect</Badge>
                    </h4>
                    <div className="text-sm text-muted-foreground space-y-1">
                      <p>301 Moved</p>
                      <p>302 Found</p>
                      <p>304 Not Modified</p>
                    </div>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <AlertTriangle className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      4xx Client
                      <Badge variant="destructive">Error</Badge>
                    </h4>
                    <div className="text-sm text-muted-foreground space-y-1">
                      <p>400 Bad Request</p>
                      <p>401 Unauthorized</p>
                      <p>404 Not Found</p>
                    </div>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-red-500/10 p-2">
                    <XCircle className="h-4 w-4 text-red-600 dark:text-red-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      5xx Server
                      <Badge variant="destructive">Error</Badge>
                    </h4>
                    <div className="text-sm text-muted-foreground space-y-1">
                      <p>500 Internal Error</p>
                      <p>502 Bad Gateway</p>
                      <p>503 Unavailable</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Response Headers */}
          <section>
            <H2 id="response-headers">Response Headers</H2>
            <p className="text-muted-foreground mb-6">
              Common response headers and their significance in API responses.
            </p>

            <CodeBlockServer language="json" filename="response-headers.json">
              {headerTypesExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="header-categories">Header Categories</H3>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <FileText className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Content Headers</h4>
                      <p className="text-sm text-muted-foreground mb-3">Describe response body</p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        content-type, content-length
                      </code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-green-500/10 p-2">
                      <Timer className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Caching Headers</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Control client-side caching
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        cache-control, etag
                      </code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-orange-500/10 p-2">
                      <CheckCircle className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Security Headers</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Enforce security policies
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        strict-transport-security
                      </code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-purple-500/10 p-2">
                      <BarChart3 className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">API Headers</h4>
                      <p className="text-sm text-muted-foreground mb-3">API-specific metadata</p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        x-rate-limit, x-api-version
                      </code>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Processing Responses */}
          <section>
            <H2 id="processing-responses">Processing Response Data</H2>
            <p className="text-muted-foreground mb-6">
              Techniques for working with response data programmatically and in automated workflows.
            </p>

            <CodeBlockServer language="yaml" filename="response-processing.yaml">
              {processingResponsesExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-green-500/5 dark:bg-green-500/10 border-green-500/20 p-4">
              <p className="text-sm">
                <strong className="text-green-600 dark:text-green-400">Pro Tip:</strong> Save
                responses to JSON files for post-processing with tools like jq, or import into data
                analysis tools for comprehensive API testing insights.
              </p>
            </div>
          </section>

          {/* Best Practices */}
          <section>
            <H2 id="best-practices">Response Handling Best Practices</H2>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Validate Critical Fields</h4>
                    <p className="text-sm text-muted-foreground">
                      Use response validation to ensure critical fields are present and correctly
                      formatted, especially for automated testing.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <BarChart3 className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Monitor Performance Metrics</h4>
                    <p className="text-sm text-muted-foreground">
                      Track response times and identify performance bottlenecks using the detailed
                      metrics provided in each response.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <AlertTriangle className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Handle Errors Gracefully</h4>
                    <p className="text-sm text-muted-foreground">
                      Check both the success flag and HTTP status code to properly categorize and
                      handle different types of failures.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <FileText className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Use Response Headers</h4>
                    <p className="text-sm text-muted-foreground">
                      Leverage response headers for pagination, rate limiting, caching, and API
                      versioning information.
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
