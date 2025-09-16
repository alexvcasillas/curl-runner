import { Code, FileText, Globe, Lightbulb, List, Settings, Timer, Type } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { validationConfigExample } from './snippets';

export const metadata: Metadata = {
  title: 'Request Object',
  description:
    'Complete reference for curl-runner request objects including all properties, methods, headers, body formats, and configuration options.',
  keywords: [
    'curl-runner request object',
    'HTTP request configuration',
    'request properties',
    'request methods',
    'request headers',
    'request body',
    'URL parameters',
    'request timeout',
    'request validation',
    'API request structure',
    'HTTP request schema',
    'request format',
  ],
  openGraph: {
    title: 'Request Object | curl-runner API Reference',
    description:
      'Complete reference for curl-runner request objects including all properties, methods, headers, body formats, and configuration options.',
    url: 'https://curl-runner.com/docs/api-reference/request-object',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Request Object | curl-runner API Reference',
    description: 'Learn about curl-runner request object structure and all available properties.',
  },
  alternates: {
    canonical: 'https://curl-runner.com/docs/api-reference/request-object',
  },
};

const requestObjectExample = `# Complete Request Object Example
request:
  name: "Complete API Request"
  url: "https://api.example.com/users"
  method: POST
  headers:
    Content-Type: "application/json"
    Authorization: "Bearer \${API_TOKEN}"
    X-API-Version: "v2"
    User-Agent: "curl-runner/1.0.0"
  params:
    include: "profile,settings"
    sort: "created_at"
    limit: "10"
  body:
    username: "john_doe"
    email: "john@example.com"
    profile:
      firstName: "John"
      lastName: "Doe"
      age: 30
  timeout: 5000
  followRedirects: true
  maxRedirects: 3
  auth:
    type: bearer
    token: "\${API_TOKEN}"
  proxy: "http://proxy.company.com:8080"
  insecure: false
  output: "user-creation-result.json"
  retry:
    count: 3
    delay: 1000
  variables:
    USER_TYPE: "premium"
    REGION: "us-east"
  expect:
    failure: false  # Expect this request to succeed (default)
    status: 201
    headers:
      location: "^/users/[0-9]+$"
      content-type: "application/json"
    body:
      id: "^[0-9]+$"
      username: "john_doe"
      email: "john@example.com"`;

const methodsExample = `# HTTP Methods Examples
requests:
  # GET - Retrieve data
  - name: "Get User Profile"
    url: "https://api.example.com/users/123"
    method: GET
    
  # POST - Create new resource
  - name: "Create New User"
    url: "https://api.example.com/users"
    method: POST
    body:
      username: "newuser"
      email: "newuser@example.com"
      
  # PUT - Complete resource replacement
  - name: "Update User (Complete)"
    url: "https://api.example.com/users/123"
    method: PUT
    body:
      id: 123
      username: "updated_user"
      email: "updated@example.com"
      profile:
        firstName: "Updated"
        lastName: "User"
        
  # PATCH - Partial resource update
  - name: "Update User (Partial)"
    url: "https://api.example.com/users/123"
    method: PATCH
    body:
      email: "new_email@example.com"
      
  # DELETE - Remove resource
  - name: "Delete User"
    url: "https://api.example.com/users/123"
    method: DELETE
    
  # HEAD - Get headers only
  - name: "Check Resource Exists"
    url: "https://api.example.com/users/123"
    method: HEAD
    
  # OPTIONS - Get allowed methods
  - name: "Check Allowed Methods"
    url: "https://api.example.com/users"
    method: OPTIONS`;

const headersExample = `# Headers Configuration Examples
requests:
  # Basic headers
  - name: "JSON API Request"
    url: "https://api.example.com/data"
    method: POST
    headers:
      Content-Type: "application/json"
      Accept: "application/json"
      
  # Authentication headers
  - name: "Authenticated Request"
    url: "https://api.example.com/protected"
    method: GET
    headers:
      Authorization: "Bearer \${ACCESS_TOKEN}"
      X-API-Key: "\${API_KEY}"
      
  # Custom headers
  - name: "Custom Headers Request"
    url: "https://api.example.com/custom"
    method: GET
    headers:
      X-Request-ID: "\${REQUEST_UUID}"
      X-Client-Version: "1.2.3"
      X-Environment: "production"
      User-Agent: "MyApp/1.0.0"
      
  # Content encoding
  - name: "Compressed Request"
    url: "https://api.example.com/upload"
    method: POST
    headers:
      Content-Type: "application/json"
      Content-Encoding: "gzip"
      Accept-Encoding: "gzip, deflate"
      
  # Cache control
  - name: "Cache-Controlled Request"
    url: "https://api.example.com/cached-data"
    method: GET
    headers:
      Cache-Control: "no-cache"
      If-None-Match: "\${ETAG_VALUE}"
      If-Modified-Since: "Wed, 21 Oct 2024 07:28:00 GMT"`;

const parametersExample = `# URL Parameters Examples
requests:
  # Query parameters as object
  - name: "Search with Parameters"
    url: "https://api.example.com/search"
    method: GET
    params:
      q: "search term"
      page: "1"
      limit: "20"
      sort: "created_at"
      order: "desc"
      include: "author,tags"
      
  # Parameters with arrays (multiple values)
  - name: "Multi-value Parameters"
    url: "https://api.example.com/items"
    method: GET
    params:
      category: "electronics,books"  # Comma-separated
      status: "active"
      tags: "new,featured,sale"
      
  # Parameters with special characters
  - name: "Special Character Parameters"
    url: "https://api.example.com/search"
    method: GET
    params:
      q: "hello world"      # Spaces
      filter: "price>100"   # Special characters
      location: "New York, NY"
      
  # Parameters with variables
  - name: "Variable Parameters"
    url: "https://api.example.com/users"
    method: GET
    params:
      userId: "\${USER_ID}"
      region: "\${USER_REGION}"
      locale: "\${USER_LOCALE}"
      apiVersion: "\${API_VERSION}"`;

const bodyExample = `# Request Body Examples
requests:
  # JSON object body
  - name: "JSON Object Body"
    url: "https://api.example.com/users"
    method: POST
    headers:
      Content-Type: "application/json"
    body:
      username: "johndoe"
      email: "john@example.com"
      profile:
        firstName: "John"
        lastName: "Doe"
        preferences:
          theme: "dark"
          notifications: true
          
  # JSON array body
  - name: "JSON Array Body"
    url: "https://api.example.com/batch-create"
    method: POST
    headers:
      Content-Type: "application/json"
    body:
      - name: "Item 1"
        type: "document"
      - name: "Item 2"
        type: "image"
      - name: "Item 3"
        type: "video"
        
  # Form data body (URL encoded)
  - name: "Form Data Body"
    url: "https://api.example.com/form-submit"
    method: POST
    headers:
      Content-Type: "application/x-www-form-urlencoded"
    body: "name=John+Doe&email=john%40example.com&message=Hello+World"
    
  # Plain text body
  - name: "Plain Text Body"
    url: "https://api.example.com/text-content"
    method: POST
    headers:
      Content-Type: "text/plain"
    body: "This is plain text content that will be sent as the request body."
    
  # Empty body (common for GET, DELETE)
  - name: "No Body Request"
    url: "https://api.example.com/resource/123"
    method: DELETE
    # No body field needed
    
  # Body with variables
  - name: "Variable Body"
    url: "https://api.example.com/dynamic"
    method: POST
    headers:
      Content-Type: "application/json"
    body:
      userId: "\${USER_ID}"
      timestamp: "\${CURRENT_TIME}"
      environment: "\${ENV}"
      data:
        value: "\${DYNAMIC_VALUE}"
        reference: "\${REF_ID}"`;

const authenticationExample = `# Authentication Examples
requests:
  # Basic authentication
  - name: "Basic Auth Request"
    url: "https://api.example.com/protected"
    method: GET
    auth:
      type: basic
      username: "\${BASIC_USER}"
      password: "\${BASIC_PASS}"
      
  # Bearer token authentication
  - name: "Bearer Token Request"
    url: "https://api.github.com/user"
    method: GET
    auth:
      type: bearer
      token: "\${GITHUB_TOKEN}"
      
  # Manual authorization header (alternative to auth object)
  - name: "Manual Auth Header"
    url: "https://api.example.com/protected"
    method: GET
    headers:
      Authorization: "Bearer \${ACCESS_TOKEN}"
      
  # API key authentication
  - name: "API Key Request"
    url: "https://api.example.com/data"
    method: GET
    headers:
      X-API-Key: "\${API_KEY}"
      X-API-Secret: "\${API_SECRET}"
      
  # Custom authentication scheme
  - name: "Custom Auth Request"
    url: "https://api.example.com/custom-auth"
    method: GET
    headers:
      Authorization: "Custom \${CUSTOM_TOKEN}"
      X-Signature: "\${REQUEST_SIGNATURE}"
      X-Timestamp: "\${REQUEST_TIMESTAMP}"`;

const timeoutRetryExample = `# Timeout and Retry Configuration
requests:
  # Basic timeout
  - name: "Quick Request"
    url: "https://fast-api.example.com/data"
    method: GET
    timeout: 3000  # 3 seconds
    
  # Long timeout for slow endpoints
  - name: "Slow Processing Request"
    url: "https://api.example.com/heavy-processing"
    method: POST
    timeout: 60000  # 60 seconds
    body:
      processLargeDataset: true
      
  # Retry configuration
  - name: "Reliable Request with Retries"
    url: "https://unreliable-api.example.com/data"
    method: GET
    timeout: 5000
    retry:
      count: 5      # Retry up to 5 times
      delay: 2000   # Wait 2 seconds between retries
      
  # No retries (default behavior)
  - name: "One-Shot Request"
    url: "https://api.example.com/one-time-action"
    method: POST
    timeout: 10000
    retry:
      count: 0      # Explicitly no retries
      
  # Quick retry for flaky services
  - name: "Flaky Service Request"
    url: "https://flaky-service.example.com/endpoint"
    method: GET
    timeout: 2000
    retry:
      count: 3
      delay: 500    # Quick retry interval`;

const redirectsExample = `# Redirect Handling Configuration
requests:
  # Follow redirects (default behavior)
  - name: "Follow Redirects"
    url: "https://example.com/redirect-me"
    method: GET
    followRedirects: true
    maxRedirects: 5
    
  # Don't follow redirects
  - name: "No Redirect Following"
    url: "https://example.com/redirect-me"
    method: GET
    followRedirects: false
    expect:
      status: [301, 302, 307, 308]  # Expect redirect status
      
  # Limited redirect following
  - name: "Limited Redirects"
    url: "https://example.com/redirect-chain"
    method: GET
    followRedirects: true
    maxRedirects: 2  # Only follow up to 2 redirects
    
  # POST request with redirect handling
  - name: "POST with Redirect"
    url: "https://example.com/form-submit"
    method: POST
    followRedirects: true
    maxRedirects: 3
    body:
      action: "submit"`;

const proxyExample = `# Proxy and Security Configuration
requests:
  # HTTP proxy
  - name: "HTTP Proxy Request"
    url: "https://api.example.com/data"
    method: GET
    proxy: "http://proxy.company.com:8080"
    
  # SOCKS proxy - Coming Soon
  - name: "SOCKS Proxy Request"
    url: "https://api.example.com/data"
    method: GET
    proxy: "socks5://proxy.company.com:1080"  # Coming Soon
    
  # Proxy with authentication - Coming Soon  
  - name: "Authenticated Proxy Request"
    url: "https://api.example.com/data"
    method: GET
    proxy: "http://username:password@proxy.company.com:8080"  # Coming Soon
    
  # Disable SSL verification (insecure)
  - name: "Insecure SSL Request"
    url: "https://self-signed-cert.example.com/api"
    method: GET
    insecure: true  # Skip SSL certificate verification
    
  # Secure request (default behavior)
  - name: "Secure Request"
    url: "https://api.example.com/secure-endpoint"
    method: GET
    insecure: false  # Verify SSL certificates (default)`;

const variablesExample = `# Request-Level Variables
requests:
  # Variables defined at request level
  - name: "Request with Local Variables"
    url: "https://api.example.com/users/\${LOCAL_USER_ID}"
    method: GET
    variables:
      LOCAL_USER_ID: "12345"
      REQUEST_TYPE: "profile"
      CACHE_TTL: "300"
    headers:
      X-Request-Type: "\${REQUEST_TYPE}"
      X-Cache-TTL: "\${CACHE_TTL}"
      
  # Variables override global ones
  - name: "Override Global Variables"
    url: "\${BASE_URL}/api/v2/data"  # Uses global BASE_URL
    method: GET
    variables:
      API_VERSION: "v2"  # Overrides global API_VERSION
      TIMEOUT: "8000"    # Local timeout override
    timeout: \${TIMEOUT}
    
  # Variables with complex values
  - name: "Complex Variable Values"
    url: "https://api.example.com/search"
    method: POST
    variables:
      SEARCH_FILTERS:
        status: "active"
        category: "electronics"
        price_range:
          min: 10
          max: 100
    body: \${SEARCH_FILTERS}`;

const outputExample = `# Output Configuration
requests:
  # Save response to file
  - name: "Save Response to File"
    url: "https://api.example.com/large-dataset"
    method: GET
    output: "dataset.json"
    
  # Save with dynamic filename
  - name: "Dynamic Output Filename"
    url: "https://api.example.com/reports/daily"
    method: GET
    output: "daily-report-\${DATE}.json"
    
  # Save to directory structure
  - name: "Organized Output"
    url: "https://api.example.com/user/\${USER_ID}/profile"
    method: GET
    output: "data/users/\${USER_ID}/profile.json"
    
  # No output file (default - display in console)
  - name: "Console Output"
    url: "https://api.example.com/status"
    method: GET
    # No output field - results shown in console`;

export default function RequestObjectPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Request Object API Reference"
          text="Complete reference for the RequestConfig interface and all available options for configuring HTTP requests."
        />

        <div className="space-y-8">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground mb-6">
              The Request Object defines the configuration for individual HTTP requests in
              curl-runner. It supports all standard HTTP methods, authentication, headers, body
              content, error handling, and response validation.
            </p>

            <CodeBlockServer language="yaml" filename="complete-request.yaml">
              {requestObjectExample}
            </CodeBlockServer>
          </section>

          {/* Properties Table */}
          <section>
            <H2 id="properties">Properties Reference</H2>
            <div className="border rounded-lg overflow-hidden">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Property</th>
                    <th className="text-left p-3 font-medium">Type</th>
                    <th className="text-left p-3 font-medium">Required</th>
                    <th className="text-left p-3 font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">name</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">string</td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Human-readable name for the request
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">url</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">string</td>
                    <td className="p-3 text-sm">
                      <Badge variant="destructive">Required</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Target URL for the HTTP request
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">method</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">HTTPMethod</td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">headers</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Record&lt;string, string&gt;
                    </td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">HTTP headers to include</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">params</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Record&lt;string, string&gt;
                    </td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">URL query parameters</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">body</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">JsonValue</td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">Request body content</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">timeout</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">number</td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Request timeout in milliseconds
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">followRedirects</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">boolean</td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Whether to follow HTTP redirects
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">maxRedirects</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">number</td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Maximum number of redirects to follow
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">auth</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">AuthConfig</td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Authentication configuration
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">proxy</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">string</td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">Proxy server URL</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">insecure</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">boolean</td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Skip SSL certificate verification
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">output</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">string</td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      File path to save response
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">retry</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">RetryConfig</td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">Retry configuration</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">variables</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Record&lt;string, string&gt;
                    </td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">Request-level variables</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">expect</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">ExpectConfig</td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Response validation rules including negative testing
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">sourceOutputConfig</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">OutputConfig</td>
                    <td className="p-3 text-sm">
                      <Badge variant="outline">Optional</Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Per-request output configuration (internal use)
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* HTTP Methods */}
          <section>
            <H2 id="http-methods">HTTP Methods</H2>
            <p className="text-muted-foreground mb-6">
              curl-runner supports all standard HTTP methods. The default method is GET if not
              specified.
            </p>

            <CodeBlockServer language="yaml" filename="http-methods.yaml">
              {methodsExample}
            </CodeBlockServer>

            <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              <Card>
                <CardHeader>
                  <CardTitle className="text-base flex items-center gap-2">
                    GET <Badge variant="secondary">Safe</Badge>
                  </CardTitle>
                </CardHeader>
                <CardContent className="text-sm text-muted-foreground">
                  Retrieve data without side effects
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="text-base flex items-center gap-2">
                    POST <Badge variant="default">Create</Badge>
                  </CardTitle>
                </CardHeader>
                <CardContent className="text-sm text-muted-foreground">
                  Create new resources or submit data
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="text-base flex items-center gap-2">
                    PUT <Badge variant="default">Replace</Badge>
                  </CardTitle>
                </CardHeader>
                <CardContent className="text-sm text-muted-foreground">
                  Complete resource replacement
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="text-base flex items-center gap-2">
                    PATCH <Badge variant="default">Update</Badge>
                  </CardTitle>
                </CardHeader>
                <CardContent className="text-sm text-muted-foreground">
                  Partial resource updates
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="text-base flex items-center gap-2">
                    DELETE <Badge variant="destructive">Remove</Badge>
                  </CardTitle>
                </CardHeader>
                <CardContent className="text-sm text-muted-foreground">
                  Delete resources
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="text-base flex items-center gap-2">
                    HEAD <Badge variant="secondary">Metadata</Badge>
                  </CardTitle>
                </CardHeader>
                <CardContent className="text-sm text-muted-foreground">
                  Get headers without body
                </CardContent>
              </Card>
            </div>
          </section>

          {/* Headers */}
          <section>
            <H2 id="headers">Headers Configuration</H2>
            <p className="text-muted-foreground mb-6">
              Configure HTTP headers for authentication, content type, custom metadata, and more.
            </p>

            <CodeBlockServer language="yaml" filename="headers-config.yaml">
              {headersExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-blue-500/5 dark:bg-blue-500/10 border-blue-500/20 p-4">
              <p className="text-sm">
                <strong className="text-blue-600 dark:text-blue-400">Note:</strong> Header names are
                case-insensitive according to HTTP specification, but values are case-sensitive. Use
                variables for dynamic header values.
              </p>
            </div>
          </section>

          {/* URL Parameters */}
          <section>
            <H2 id="url-parameters">URL Parameters</H2>
            <p className="text-muted-foreground mb-6">
              Add query parameters to URLs using the params object. Parameters are automatically
              URL-encoded.
            </p>

            <CodeBlockServer language="yaml" filename="url-parameters.yaml">
              {parametersExample}
            </CodeBlockServer>
          </section>

          {/* Request Body */}
          <section>
            <H2 id="request-body">Request Body</H2>
            <p className="text-muted-foreground mb-6">
              Configure request body content for POST, PUT, PATCH, and other methods that support
              body data.
            </p>

            <CodeBlockServer language="yaml" filename="request-body.yaml">
              {bodyExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="body-types">Body Content Types</H3>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <FileText className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">JSON Object</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Structured data as objects
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        {"body: {key: 'value'}"}
                      </code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-green-500/10 p-2">
                      <List className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">JSON Array</h4>
                      <p className="text-sm text-muted-foreground mb-3">Array of data items</p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        {'body: [item1, item2]'}
                      </code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-purple-500/10 p-2">
                      <Code className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2 flex items-center gap-2">
                        Form Data
                        <Badge variant="secondary">Coming Soon</Badge>
                      </h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Structured multipart/form-data
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        Raw URL-encoded strings supported
                      </code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-orange-500/10 p-2">
                      <Type className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Plain Text</h4>
                      <p className="text-sm text-muted-foreground mb-3">Raw text content</p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        {'body: "plain text"'}
                      </code>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Authentication */}
          <section>
            <H2 id="authentication">Authentication</H2>
            <p className="text-muted-foreground mb-6">
              Configure authentication using the auth object or manual headers.
            </p>

            <CodeBlockServer language="yaml" filename="authentication.yaml">
              {authenticationExample}
            </CodeBlockServer>

            <div className="mt-6 border rounded-lg overflow-hidden">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Auth Type</th>
                    <th className="text-left p-3 font-medium">Required Fields</th>
                    <th className="text-left p-3 font-medium">Generated Header</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">basic</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">username, password</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Authorization: Basic &lt;base64&gt;
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">bearer</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">token</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Authorization: Bearer &lt;token&gt;
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Timeout and Retry */}
          <section>
            <H2 id="timeout-retry">Timeout & Retry Configuration</H2>
            <p className="text-muted-foreground mb-6">
              Configure request timeouts and retry behavior for handling network issues and
              unreliable services.
            </p>

            <CodeBlockServer language="yaml" filename="timeout-retry.yaml">
              {timeoutRetryExample}
            </CodeBlockServer>

            <div className="mt-6 grid gap-4 sm:grid-cols-2">
              <Card>
                <CardHeader>
                  <CardTitle className="text-base">Timeout Values</CardTitle>
                </CardHeader>
                <CardContent className="text-sm space-y-2">
                  <p>
                    <strong>Quick APIs:</strong> 1-5 seconds
                  </p>
                  <p>
                    <strong>Standard APIs:</strong> 5-15 seconds
                  </p>
                  <p>
                    <strong>Heavy processing:</strong> 30-120 seconds
                  </p>
                  <p>
                    <strong>File uploads:</strong> 300+ seconds
                  </p>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="text-base">Retry Strategy</CardTitle>
                </CardHeader>
                <CardContent className="text-sm space-y-2">
                  <p>
                    <strong>count:</strong> Number of retry attempts
                  </p>
                  <p>
                    <strong>delay:</strong> Wait time between retries
                  </p>
                  <p>
                    <Badge variant="secondary" className="mr-2">
                      Coming Soon
                    </Badge>
                    Uses exponential backoff automatically
                  </p>
                </CardContent>
              </Card>
            </div>
          </section>

          {/* Redirects */}
          <section>
            <H2 id="redirects">Redirect Handling</H2>
            <p className="text-muted-foreground mb-6">
              Control how curl-runner handles HTTP redirects (3xx status codes).
            </p>

            <CodeBlockServer language="yaml" filename="redirects.yaml">
              {redirectsExample}
            </CodeBlockServer>
          </section>

          {/* Proxy and Security */}
          <section>
            <H2 id="proxy-security">Proxy & Security Configuration</H2>
            <p className="text-muted-foreground mb-6">
              Configure proxy settings and SSL/TLS security options.
            </p>

            <div className="mb-6 flex gap-2 flex-wrap">
              <Badge variant="secondary">Coming Soon</Badge>
              <span className="text-sm text-muted-foreground">
                Advanced proxy settings (SOCKS, authentication)
              </span>
            </div>
            <div className="mb-6 flex gap-2 flex-wrap">
              <Badge variant="secondary">Coming Soon</Badge>
              <span className="text-sm text-muted-foreground">
                SSL/TLS configuration (custom certificates, client certs)
              </span>
            </div>

            <CodeBlockServer language="yaml" filename="proxy-security.yaml">
              {proxyExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-yellow-500/5 dark:bg-yellow-500/10 border-yellow-500/20 p-4">
              <p className="text-sm">
                <strong className="text-yellow-600 dark:text-yellow-400">Security Warning:</strong>{' '}
                Only use <code>insecure: true</code> for testing with self-signed certificates.
                Never use in production environments.
              </p>
            </div>
          </section>

          {/* Variables */}
          <section>
            <H2 id="variables">Request-Level Variables</H2>
            <p className="text-muted-foreground mb-6">
              Define variables at the request level that can be used within that specific request.
            </p>

            <CodeBlockServer language="yaml" filename="request-variables.yaml">
              {variablesExample}
            </CodeBlockServer>
          </section>

          {/* Validation Configuration */}
          <section>
            <H2 id="validation-configuration">Response Validation</H2>
            <p className="text-muted-foreground mb-6">
              Configure response validation using the expect object to verify status codes, headers,
              and body content. Includes support for negative testing with expect.failure.
            </p>

            <CodeBlockServer language="yaml" filename="validation-config.yaml">
              {validationConfigExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-4">
              <div className="rounded-lg border bg-blue-500/5 dark:bg-blue-500/10 border-blue-500/20 p-4">
                <h4 className="font-medium mb-2 text-blue-600 dark:text-blue-400 flex items-center gap-2">
                  <Lightbulb className="h-4 w-4" />
                  Negative Testing with expect.failure
                </h4>
                <div className="space-y-2 text-sm text-muted-foreground">
                  <p>
                    Set <code>expect.failure: true</code> to test scenarios where you expect the
                    request to fail:
                  </p>
                  <ul className="list-disc pl-4 space-y-1">
                    <li>Authentication failures (401, 403)</li>
                    <li>Validation errors (400, 422)</li>
                    <li>Resource not found (404)</li>
                    <li>Rate limiting (429)</li>
                    <li>Server errors (5xx)</li>
                  </ul>
                </div>
              </div>

              <div className="rounded-lg border bg-green-500/5 dark:bg-green-500/10 border-green-500/20 p-4">
                <p className="text-sm text-muted-foreground">
                  <strong>Validation Priority:</strong> Request-level validation rules override
                  collection and global defaults. Use partial matching to validate only the fields
                  that matter to your test.
                </p>
              </div>
            </div>
          </section>

          {/* Output Configuration */}
          <section>
            <H2 id="output-configuration">Output Configuration</H2>
            <p className="text-muted-foreground mb-6">
              Configure where to save response data from individual requests.
            </p>

            <CodeBlockServer language="yaml" filename="output-config.yaml">
              {outputExample}
            </CodeBlockServer>
          </section>

          {/* Best Practices */}
          <section>
            <H2 id="best-practices">Best Practices</H2>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <FileText className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Use Descriptive Names</h4>
                    <p className="text-sm text-muted-foreground">
                      Always include meaningful names for your requests to make logs and output
                      easier to understand.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <Timer className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Set Appropriate Timeouts</h4>
                    <p className="text-sm text-muted-foreground">
                      Configure timeouts based on expected response times. Too short causes
                      failures, too long delays error detection.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Settings className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Use Variables for Dynamic Content</h4>
                    <p className="text-sm text-muted-foreground">
                      Leverage variables for API endpoints, tokens, and user IDs to make
                      configurations reusable across environments.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Globe className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Include Content-Type Headers</h4>
                    <p className="text-sm text-muted-foreground">
                      Always specify Content-Type headers when sending body data to ensure proper
                      server-side processing.
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
