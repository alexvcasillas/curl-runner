import {
  AlertTriangle,
  ArrowRight,
  BarChart3,
  CheckCircle,
  Code,
  Database,
  Eye,
  FileText,
  Filter,
  Globe,
  Key,
  Layers,
  Lightbulb,
  List,
  PlusCircle,
  Search,
  Shield,
  Target,
  TrendingUp,
  XCircle,
  Zap,
} from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';
import { failureTestingExample } from './snippets';

export const metadata: Metadata = {
  title: 'Validation Rules',
  description:
    'Complete reference for curl-runner validation rules including status code validation, header validation, body validation, and custom validation patterns.',
  keywords: [
    'curl-runner validation rules',
    'response validation',
    'validation patterns',
    'status code validation',
    'header validation',
    'body validation',
    'expect configuration',
    'validation assertions',
    'API testing validation',
    'response checking',
    'validation schema',
    'validation rules reference',
  ],
  openGraph: {
    title: 'Validation Rules | curl-runner API Reference',
    description:
      'Complete reference for curl-runner validation rules including status code validation, header validation, body validation, and custom validation patterns.',
    url: 'https://www.curl-runner.com/docs/api-reference/validation-rules',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Validation Rules | curl-runner API Reference',
    description: 'Learn about curl-runner validation rules for comprehensive API response testing.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/api-reference/validation-rules',
  },
};

const expectConfigExample = `# Complete ExpectConfig Example
request:
  name: "Comprehensive Validation Example"
  url: "https://api.example.com/users/123"
  method: GET
  expect:
    # Failure expectation (optional - for negative testing)
    failure: false    # false = expect success (default), true = expect failure
    
    # Status code validation
    status: 200
    
    # Response headers validation
    headers:
      content-type: "application/json"
      x-api-version: "v2"
      cache-control: "no-cache, no-store"
      etag: "^W/\\"[a-f0-9]+\\""  # Regex pattern for ETag
      
    # Response body validation
    body:
      id: 123
      username: "johndoe"
      email: "john@example.com"
      profile:
        firstName: "John"
        lastName: "Doe"
        createdAt: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$"  # ISO date
        active: true
        roles: ["user", "premium"]  # Array with specific values
        metadata:
          lastLogin: "*"  # Any value (wildcard)
          preferences: "*"  # Any object structure`;

const statusValidationExample = `# Status Code Validation Examples

# Single status code (most common)
request:
  name: "Expect Success"
  url: "https://api.example.com/data"
  expect:
    status: 200

---

# Multiple acceptable status codes
request:
  name: "Accept Multiple Status"
  url: "https://api.example.com/users/123"
  method: GET
  expect:
    status: [200, 304]  # OK or Not Modified

---

# Different scenarios
requests:
  # Successful creation
  - name: "Create Resource"
    url: "https://api.example.com/resources"
    method: POST
    body:
      name: "New Resource"
    expect:
      status: 201  # Created
      
  # Successful deletion
  - name: "Delete Resource"
    url: "https://api.example.com/resources/123"
    method: DELETE
    expect:
      status: 204  # No Content
      
  # Conditional request
  - name: "Conditional Get"
    url: "https://api.example.com/data"
    method: GET
    headers:
      If-None-Match: "some-etag"
    expect:
      status: [200, 304]  # OK or Not Modified
      
  # Error handling
  - name: "Expected Not Found"
    url: "https://api.example.com/users/nonexistent"
    method: GET
    expect:
      status: 404  # Not Found is expected
      
  # Rate limiting tolerance
  - name: "Rate Limited Request"
    url: "https://api.example.com/limited-endpoint"
    method: GET
    expect:
      status: [200, 429]  # OK or Too Many Requests`;

const headerValidationExample = `# Header Validation Examples

# Basic header validation
request:
  name: "Basic Header Check"
  url: "https://api.example.com/data"
  expect:
    headers:
      content-type: "application/json"
      server: "nginx"
      
---

# Case-insensitive header matching
request:
  name: "Case Insensitive Headers"
  url: "https://api.example.com/data"
  expect:
    headers:
      Content-Type: "application/json"      # Works
      content-type: "application/json"      # Also works
      CONTENT-TYPE: "application/json"      # Also works
      
---

# Pattern matching with regex
request:
  name: "Pattern Matching Headers"
  url: "https://api.example.com/data"
  expect:
    headers:
      # Date header pattern
      date: "^[A-Z][a-z]{2}, \\d{2} [A-Z][a-z]{2} \\d{4}"
      
      # ETag pattern
      etag: "^(W/)?\\"[a-f0-9-]+\\""
      
      # Rate limit patterns
      x-rate-limit-limit: "^[0-9]+$"
      x-rate-limit-remaining: "^[0-9]+$"
      
      # Version patterns
      x-api-version: "^v[0-9]+(\\.0-9]+)*$"
      
---

# Security headers validation
request:
  name: "Security Headers Check"
  url: "https://api.secure.com/data"
  expect:
    headers:
      strict-transport-security: "max-age=31536000"
      x-frame-options: "DENY"
      x-content-type-options: "nosniff"
      x-xss-protection: "1; mode=block"
      content-security-policy: "*"  # Any CSP policy present
      
---

# API-specific headers
request:
  name: "API Headers Validation"
  url: "https://api.example.com/data"
  expect:
    headers:
      # Rate limiting info
      x-rate-limit-remaining: "*"
      x-rate-limit-reset: "^[0-9]+$"
      
      # Request tracking
      x-request-id: "^[a-f0-9-]{36}$"  # UUID format
      
      # Performance metrics
      x-response-time: "^[0-9]+ms$"
      
      # Location header for redirects/created resources
      location: "^https://api\\.example\\.com/resources/[0-9]+$"`;

const bodyValidationExample = `# Body Validation Examples

# Exact value matching
request:
  name: "Exact Value Match"
  url: "https://api.example.com/status"
  expect:
    body:
      status: "operational"
      code: 200
      message: "All systems operational"
      
---

# Object structure validation
request:
  name: "Object Structure Validation"
  url: "https://api.example.com/users/123"
  expect:
    body:
      id: 123
      username: "johndoe"
      profile:
        firstName: "John"
        lastName: "Doe"
        email: "john@example.com"
        settings:
          theme: "dark"
          notifications: true
          
---

# Array validation
request:
  name: "Array Validation"
  url: "https://api.example.com/users"
  expect:
    body:
      # Validate array structure
      - id: 1
        username: "user1"
        active: true
      - id: 2
        username: "user2"
        active: true
        
---

# Partial matching (only check specific fields)
request:
  name: "Partial Matching"
  url: "https://api.example.com/users/123"
  expect:
    body:
      id: 123  # Only check ID
      active: true  # Only check active status
      # Other fields ignored
      
---

# Mixed data types
request:
  name: "Mixed Data Types"
  url: "https://api.example.com/metrics"
  expect:
    body:
      # String values
      version: "2.1.0"
      environment: "production"
      
      # Numeric values
      uptime: 86400
      cpu_usage: 0.75
      
      # Boolean values
      healthy: true
      maintenance_mode: false
      
      # Null values
      last_error: null
      
      # Array of strings
      tags: ["api", "production", "monitoring"]
      
      # Array of numbers
      response_times: [120, 95, 200, 150]`;

const patternValidationExample = `# Pattern Validation with Regex

# Common patterns
request:
  name: "Common Pattern Validation"
  url: "https://api.example.com/user/profile"
  expect:
    body:
      # Email pattern
      email: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
      
      # Phone number pattern (US format)
      phone: "^\\+?1?[-\\s]?\\(?[0-9]{3}\\)?[-\\s]?[0-9]{3}[-\\s]?[0-9]{4}$"
      
      # UUID pattern
      user_id: "^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$"
      
      # ISO date pattern
      created_at: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}(\\.\\d{3})?Z$"
      
      # URL pattern
      avatar_url: "^https?://[\\w.-]+\\.[a-zA-Z]{2,}(/.*)?$"
      
---

# Numeric patterns
request:
  name: "Numeric Pattern Validation"
  url: "https://api.example.com/financial/account"
  expect:
    body:
      # Currency amounts (dollars and cents)
      balance: "^[0-9]+\\.[0-9]{2}$"
      
      # Percentage values
      interest_rate: "^[0-9]{1,2}\\.[0-9]{2}%$"
      
      # Account numbers
      account_number: "^[0-9]{10,12}$"
      
      # Credit card last 4 digits
      card_last_four: "^[0-9]{4}$"
      
---

# Custom business patterns
request:
  name: "Business Pattern Validation"
  url: "https://api.example.com/orders/12345"
  expect:
    body:
      # Order ID pattern
      order_id: "^ORD-[0-9]{8}$"
      
      # Product SKU pattern
      sku: "^[A-Z]{3}-[0-9]{4}-[A-Z]{2}$"
      
      # Tracking number pattern
      tracking_number: "^1Z[A-Z0-9]{16}$"
      
      # Status enum validation
      status: "^(pending|processing|shipped|delivered|cancelled)$"
      
---

# Array element patterns
request:
  name: "Array Element Patterns"
  url: "https://api.example.com/products"
  expect:
    body:
      products:
        - id: "^PROD-[0-9]+$"
          name: ".+"  # Any non-empty string
          price: "^[0-9]+\\.[0-9]{2}$"
          category: "^(electronics|clothing|books|home)$"
        - id: "^PROD-[0-9]+$"
          name: ".+"
          price: "^[0-9]+\\.[0-9]{2}$"
          category: "^(electronics|clothing|books|home)$"`;

const wildcardValidationExample = `# Wildcard and Flexible Validation

# Wildcard matching (any value present)
request:
  name: "Wildcard Validation"
  url: "https://api.example.com/user/session"
  expect:
    body:
      user_id: "*"        # Any user ID value
      session_token: "*"  # Any token value
      expires_at: "*"     # Any expiration time
      created_at: "*"     # Any creation time
      
---

# Conditional validation based on response
request:
  name: "Conditional Validation"
  url: "https://api.example.com/user/123"
  expect:
    body:
      id: 123
      username: "johndoe"
      # Premium users have additional fields
      account_type: ["free", "premium", "enterprise"]
      # These fields only present for premium/enterprise
      subscription_id: "*"  # Present if premium/enterprise
      billing_cycle: "*"   # Present if premium/enterprise
      
---

# Nested wildcard validation
request:
  name: "Nested Wildcard"
  url: "https://api.example.com/complex-data"
  expect:
    body:
      metadata:
        created_by: "*"    # Any user info
        tags: "*"          # Any tag structure
        custom_fields: "*" # Any custom field object
      data:
        # Specific validation for critical fields
        id: "^[0-9]+$"
        status: "active"
        # Flexible for other fields
        properties: "*"
        
---

# Array with wildcard elements
request:
  name: "Array Wildcard Elements"
  url: "https://api.example.com/notifications"
  expect:
    body:
      notifications:
        - id: "*"          # Any notification ID
          type: "info"     # Specific type
          message: "*"     # Any message content
          timestamp: "*"   # Any timestamp
        - id: "*"
          type: "*"        # Any type for second notification
          message: "*"
          timestamp: "*"`;

const complexValidationExample = `# Complex Validation Scenarios

# Nested object validation with mixed patterns
request:
  name: "E-commerce Order Validation"
  url: "https://api.ecommerce.com/orders/12345"
  expect:
    status: 200
    headers:
      content-type: "application/json"
      x-order-id: "^ORD-[0-9]{8}$"
    body:
      order:
        id: "^ORD-[0-9]{8}$"
        status: "^(pending|confirmed|shipped|delivered)$"
        customer:
          id: "^CUST-[0-9]{6}$"
          email: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
          shipping_address:
            street: "*"
            city: "*"
            state: "^[A-Z]{2}$"  # US state code
            zip_code: "^[0-9]{5}(-[0-9]{4})?$"  # US ZIP
        items:
          - sku: "^[A-Z0-9-]+$"
            quantity: "^[1-9][0-9]*$"  # Positive integer
            price: "^[0-9]+\\.[0-9]{2}$"
            discount: "^[0-9]*\\.[0-9]{2}$"  # Optional discount
        totals:
          subtotal: "^[0-9]+\\.[0-9]{2}$"
          tax: "^[0-9]+\\.[0-9]{2}$"
          shipping: "^[0-9]+\\.[0-9]{2}$"
          total: "^[0-9]+\\.[0-9]{2}$"
        payment:
          method: "^(credit_card|paypal|bank_transfer)$"
          status: "^(pending|completed|failed)$"
          transaction_id: "*"  # Any transaction ID format
          
---

# API response validation with pagination
request:
  name: "Paginated Response Validation"
  url: "https://api.example.com/products?page=1&limit=20"
  expect:
    status: 200
    headers:
      x-total-count: "^[0-9]+$"
      x-page-count: "^[0-9]+$"
    body:
      data:
        - id: "^[0-9]+$"
          name: ".+"  # Any non-empty name
          price: "^[0-9]+\\.[0-9]{2}$"
          category_id: "^[0-9]+$"
          in_stock: true  # Boolean value
          images: ["*"]   # Array with at least one image
      pagination:
        current_page: 1
        per_page: 20
        total_pages: "^[0-9]+$"
        total_items: "^[0-9]+$"
        has_next: true
        has_previous: false
        
---

# Multi-level validation with error scenarios
request:
  name: "Robust API Validation"
  url: "https://api.banking.com/accounts/123/transactions"
  expect:
    status: [200, 400, 401, 403]  # Multiple valid responses
    # Headers present for all responses
    headers:
      x-request-id: "^[a-f0-9-]{36}$"
      x-rate-limit-remaining: "^[0-9]+$"
    # Body varies by status code
    body:
      # Success response (200)
      transactions:
        - id: "^TXN-[0-9A-Z]{12}$"
          amount: "^-?[0-9]+\\.[0-9]{2}$"  # Can be negative
          type: "^(debit|credit|fee|interest)$"
          date: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$"
          description: "*"
          balance_after: "^[0-9]+\\.[0-9]{2}$"
      # OR error response (400/401/403)
      error:
        code: "^[A-Z_]+$"
        message: "*"
        details: "*"`;

const validationInheritanceExample = `# Validation Inheritance and Overrides

# Global validation defaults
global:
  defaults:
    expect:
      # All requests expect JSON by default
      headers:
        content-type: "application/json"
      # All requests should be fast
      status: [200, 201, 204]

collection:
  name: "API Test Suite"
  
  # Collection-level validation defaults
  defaults:
    expect:
      headers:
        # Inherits content-type from global
        x-api-version: "v2"  # Additional header requirement
        x-rate-limit-remaining: "*"  # Rate limit info required
      
  requests:
    # Inherits all validation from global + collection
    - name: "Standard Request"
      url: "https://api.example.com/users"
      method: GET
      # Uses: content-type: "application/json"
      #       x-api-version: "v2"  
      #       x-rate-limit-remaining: "*"
      #       status: [200, 201, 204]
      
    # Override specific validation rules
    - name: "Custom Validation Request"
      url: "https://api.example.com/upload"
      method: POST
      expect:
        status: 202  # Override global status expectation
        headers:
          # Inherits x-api-version and x-rate-limit-remaining
          content-type: "text/plain"  # Override global content-type
          x-upload-id: "^[a-f0-9-]+$"  # Additional header
          
    # Remove inherited validation  
    - name: "No Validation Request"
      url: "https://api.example.com/webhook"
      method: POST
      expect:
        status: [200, 202, 204]  # Override status
        # No header expectations (removes inherited ones)
        
    # Complex override scenario
    - name: "Special Case Request"
      url: "https://api.example.com/special"
      method: GET
      expect:
        status: [200, 404]  # Allow not found
        headers:
          content-type: ["application/json", "text/html"]  # Multiple types
          x-api-version: "v2"  # Keep collection default
          # Remove rate limit requirement for this endpoint
        body:
          # Only validate if response is 200
          data: "*"  # Any data structure`;

export default function ValidationRulesPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Validation Rules API Reference"
          text="Complete reference for response validation using the expect configuration object. Define validation rules for status codes, headers, and response body content."
        />

        <div className="space-y-8">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground mb-6">
              Validation rules allow you to verify that API responses meet expected criteria. Using
              the <code>expect</code> configuration object, you can validate HTTP status codes,
              response headers, and body content with support for exact matching, pattern matching,
              and wildcard validation.
            </p>

            <CodeBlockServer language="yaml" filename="expect-config-complete.yaml">
              {expectConfigExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-muted/50 p-4">
              <p className="text-sm">
                <strong>Important:</strong> When validation rules are present, curl-runner marks
                requests as failed if any validation rule doesn't match, even if the HTTP request
                itself was successful.
              </p>
            </div>
          </section>

          {/* ExpectConfig Properties */}
          <section>
            <H2 id="expect-properties">ExpectConfig Properties</H2>
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
                    <td className="p-3">
                      <code className="text-sm">failure</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">boolean</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      When <code>true</code>, expect the request to fail with 4xx/5xx status. Used
                      for negative testing.
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">status</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">number | number[]</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Expected HTTP status code(s)
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">headers</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Record&lt;string, string&gt;
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Expected response headers with values or patterns
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">body</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">JsonValue</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Expected response body structure and values
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Status Code Validation */}
          <section>
            <H2 id="status-validation">Status Code Validation</H2>
            <p className="text-muted-foreground mb-6">
              Validate that responses return expected HTTP status codes. You can specify a single
              status code or an array of acceptable codes.
            </p>

            <CodeBlockServer language="yaml" filename="status-validation.yaml">
              {statusValidationExample}
            </CodeBlockServer>

            <div className="mt-6 grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      Success <Badge variant="default">2xx</Badge>
                    </h4>
                    <div className="text-sm text-muted-foreground space-y-1">
                      <p>200 - OK</p>
                      <p>201 - Created</p>
                      <p>204 - No Content</p>
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
                      Redirect <Badge variant="secondary">3xx</Badge>
                    </h4>
                    <div className="text-sm text-muted-foreground space-y-1">
                      <p>301 - Moved</p>
                      <p>302 - Found</p>
                      <p>304 - Not Modified</p>
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
                      Client Error <Badge variant="destructive">4xx</Badge>
                    </h4>
                    <div className="text-sm text-muted-foreground space-y-1">
                      <p>400 - Bad Request</p>
                      <p>401 - Unauthorized</p>
                      <p>404 - Not Found</p>
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
                      Server Error <Badge variant="destructive">5xx</Badge>
                    </h4>
                    <div className="text-sm text-muted-foreground space-y-1">
                      <p>500 - Internal Error</p>
                      <p>502 - Bad Gateway</p>
                      <p>503 - Unavailable</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Failure Testing */}
          <section>
            <H2 id="failure-testing">Failure Testing (Negative Testing)</H2>
            <p className="text-muted-foreground mb-6">
              Use <code>expect.failure: true</code> to test that endpoints correctly fail in
              expected ways. This is useful for testing error handling, authentication failures,
              validation errors, and other scenarios where you expect the request to return a 4xx or
              5xx status code.
            </p>

            <CodeBlockServer language="yaml" filename="failure-testing.yaml">
              {failureTestingExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-4">
              <div className="rounded-lg border bg-blue-500/5 dark:bg-blue-500/10 border-blue-500/20 p-4">
                <h4 className="font-medium mb-2 text-blue-600 dark:text-blue-400 flex items-center gap-2">
                  <Lightbulb className="h-4 w-4" />
                  How Failure Testing Works
                </h4>
                <div className="space-y-2 text-sm">
                  <p>
                    <strong>✅ Success:</strong> <code>expect.failure: true</code> + 4xx/5xx status
                    + validations pass
                  </p>
                  <p>
                    <strong>❌ Failed:</strong> <code>expect.failure: true</code> + 2xx/3xx status
                    (expected failure but got success)
                  </p>
                  <p>
                    <strong>❌ Failed:</strong> <code>expect.failure: true</code> + 4xx/5xx status +
                    validations fail (wrong error details)
                  </p>
                </div>
              </div>

              <div className="rounded-lg border bg-yellow-500/5 dark:bg-yellow-500/10 border-yellow-500/20 p-4">
                <p className="text-sm">
                  <strong className="text-yellow-600 dark:text-yellow-400">Important:</strong> When{' '}
                  <code>expect.failure: true</code> is used, curl-runner inverts the success/failure
                  logic. A 4xx/5xx status code that matches your expectations becomes a "successful"
                  test result.
                </p>
              </div>
            </div>
          </section>

          {/* Header Validation */}
          <section>
            <H2 id="header-validation">Header Validation</H2>
            <p className="text-muted-foreground mb-6">
              Validate response headers using exact values or regular expression patterns. Header
              names are case-insensitive.
            </p>

            <CodeBlockServer language="yaml" filename="header-validation.yaml">
              {headerValidationExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="header-patterns">Common Header Validation Patterns</H3>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <FileText className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Content Headers</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Validate response content metadata
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        content-type, content-length
                      </code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-orange-500/10 p-2">
                      <Shield className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Security Headers</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Verify security policy headers
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        strict-transport-security
                      </code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-green-500/10 p-2">
                      <Globe className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">API Headers</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Check API versioning and tracking
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        x-api-version, x-request-id
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
                      <h4 className="font-medium mb-2">Rate Limiting</h4>
                      <p className="text-sm text-muted-foreground mb-3">Monitor API usage limits</p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        x-rate-limit-remaining
                      </code>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Body Validation */}
          <section>
            <H2 id="body-validation">Body Validation</H2>
            <p className="text-muted-foreground mb-6">
              Validate response body content with support for exact matching, partial matching, and
              complex nested structures.
            </p>

            <CodeBlockServer language="yaml" filename="body-validation.yaml">
              {bodyValidationExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="body-validation-types">Body Validation Types</H3>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <Target className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Exact Matching</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Values must match exactly
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        {'status: "active"'}
                      </code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-green-500/10 p-2">
                      <Code className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Type Validation</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Validates JSON data types
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        {'active: true, count: 42'}
                      </code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-purple-500/10 p-2">
                      <CheckCircle className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Partial Matching</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Only specified fields validated
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        {'id: 123  # Others ignored'}
                      </code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-orange-500/10 p-2">
                      <Layers className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Nested Objects</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Deep structure validation
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        {"profile: {name: 'John'}"}
                      </code>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Pattern Validation */}
          <section>
            <H2 id="pattern-validation">Pattern Validation with Regex</H2>
            <p className="text-muted-foreground mb-6">
              Use regular expressions for flexible validation of dynamic content like IDs,
              timestamps, emails, and custom formats.
            </p>

            <CodeBlockServer language="yaml" filename="pattern-validation.yaml">
              {patternValidationExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-yellow-500/5 dark:bg-yellow-500/10 border-yellow-500/20 p-4">
              <p className="text-sm">
                <strong className="text-yellow-600 dark:text-yellow-400">Regex Escaping:</strong>{' '}
                Remember to properly escape backslashes in YAML strings. Use double backslashes (
                <code>\\\\</code>) for literal backslashes in regex patterns.
              </p>
            </div>
          </section>

          {/* Wildcard Validation */}
          <section>
            <H2 id="wildcard-validation">Wildcard and Flexible Validation</H2>
            <p className="text-muted-foreground mb-6">
              Use wildcards (<code>*</code>) to validate field presence without checking specific
              values, useful for dynamic or sensitive data.
            </p>

            <CodeBlockServer language="yaml" filename="wildcard-validation.yaml">
              {wildcardValidationExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="wildcard-use-cases">Wildcard Use Cases</H3>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <Search className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Dynamic Values</h4>
                      <p className="text-sm text-muted-foreground">
                        Timestamps, UUIDs, tokens that change between requests
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-orange-500/10 p-2">
                      <Shield className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Sensitive Data</h4>
                      <p className="text-sm text-muted-foreground">
                        Check presence without exposing actual values in validation
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-green-500/10 p-2">
                      <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Optional Fields</h4>
                      <p className="text-sm text-muted-foreground">
                        Validate fields that may or may not be present
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-purple-500/10 p-2">
                      <Database className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Complex Structures</h4>
                      <p className="text-sm text-muted-foreground">
                        Allow flexible nested object structures
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Complex Validation Scenarios */}
          <section>
            <H2 id="complex-validation">Complex Validation Scenarios</H2>
            <p className="text-muted-foreground mb-6">
              Advanced validation examples combining multiple techniques for comprehensive API
              testing.
            </p>

            <CodeBlockServer language="yaml" filename="complex-validation.yaml">
              {complexValidationExample}
            </CodeBlockServer>
          </section>

          {/* Validation Inheritance */}
          <section>
            <H2 id="validation-inheritance">Validation Inheritance and Overrides</H2>
            <p className="text-muted-foreground mb-6">
              Understand how validation rules cascade from global to collection to request level,
              and how to override inherited rules.
            </p>

            <CodeBlockServer language="yaml" filename="validation-inheritance.yaml">
              {validationInheritanceExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="validation-precedence">Validation Precedence Order</H3>
              <div className="space-y-4">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <Target className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">1. Request-Level Validation</h4>
                      <p className="text-sm text-muted-foreground">
                        Validation rules defined directly on requests have the highest priority and
                        completely override inherited rules.
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-green-500/10 p-2">
                      <Layers className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">2. Collection-Level Defaults</h4>
                      <p className="text-sm text-muted-foreground">
                        Collection validation defaults override global defaults and merge with
                        request-level rules.
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
                      <h4 className="font-medium mb-2">3. Global Defaults</h4>
                      <p className="text-sm text-muted-foreground">
                        Global validation defaults provide baseline rules applied to all requests
                        unless overridden.
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Common Patterns */}
          <section>
            <H2 id="common-patterns">Common Validation Patterns</H2>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Key className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">API Authentication</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Validate login responses and token formats
                    </p>
                    <code className="text-xs bg-muted px-2 py-1 rounded block">
                      {'token: "^[A-Za-z0-9\\-_]+\\."'}
                    </code>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <PlusCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Data Creation</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Verify new resource creation and IDs
                    </p>
                    <code className="text-xs bg-muted px-2 py-1 rounded block">
                      {'id: "^[0-9]+$", status: 201'}
                    </code>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-red-500/10 p-2">
                    <AlertTriangle className="h-4 w-4 text-red-600 dark:text-red-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Error Handling</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Validate error response structures
                    </p>
                    <code className="text-xs bg-muted px-2 py-1 rounded block">
                      {"error: {code: '*', message: '*'}"}
                    </code>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <List className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Pagination</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Check paginated response metadata
                    </p>
                    <code className="text-xs bg-muted px-2 py-1 rounded block">
                      {'total_pages: "^[0-9]+$"'}
                    </code>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Best Practices */}
          <section>
            <H2 id="best-practices">Validation Best Practices</H2>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <TrendingUp className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Start Simple, Add Complexity</h4>
                    <p className="text-sm text-muted-foreground">
                      Begin with basic status code validation, then gradually add header and body
                      validation as needed.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Filter className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Use Partial Matching</h4>
                    <p className="text-sm text-muted-foreground">
                      Validate only the fields that matter to avoid brittle tests that break with
                      API changes.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Zap className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Leverage Patterns for Dynamic Data</h4>
                    <p className="text-sm text-muted-foreground">
                      Use regex patterns for IDs, timestamps, and other dynamic values that change
                      between requests.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Test Both Success and Error Cases</h4>
                    <p className="text-sm text-muted-foreground">
                      Include validation for expected error responses to ensure proper error
                      handling.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4 md:col-span-2">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-red-500/10 p-2">
                    <Eye className="h-4 w-4 text-red-600 dark:text-red-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Use Wildcards for Sensitive Data</h4>
                    <p className="text-sm text-muted-foreground">
                      Validate presence of tokens and sensitive fields without exposing actual
                      values in test logs.
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
