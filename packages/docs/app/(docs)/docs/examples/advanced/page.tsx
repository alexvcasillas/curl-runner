import {
  Activity,
  BarChart3,
  CheckCircle,
  Database,
  Globe,
  Layers,
  Server,
  Settings,
  Target,
  TrendingUp,
  Users,
  Zap,
} from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';

export const metadata: Metadata = {
  title: 'Advanced Examples',
  description:
    'Complex curl-runner configurations for advanced use cases. Learn about variable interpolation, authentication, validation, and real-world API testing scenarios.',
  keywords: [
    'curl-runner advanced examples',
    'complex HTTP requests',
    'variable interpolation examples',
    'authentication examples',
    'response validation examples',
    'retry mechanism examples',
    'parallel execution examples',
    'advanced API testing',
    'production testing',
    'complex configurations',
    'enterprise examples',
    'advanced YAML configuration',
  ],
  openGraph: {
    title: 'Advanced Examples | curl-runner Documentation',
    description:
      'Complex curl-runner configurations for advanced use cases. Learn about variable interpolation, authentication, validation, and real-world API testing scenarios.',
    url: 'https://www.curl-runner.com/docs/examples/advanced',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Advanced Examples | curl-runner Documentation',
    description: 'Master curl-runner with advanced examples for complex API testing scenarios.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/examples/advanced',
  },
};

const variableInterpolationExample = `# Variable interpolation with environment loading
global:
  variables:
    BASE_URL: https://api.production.com
    API_VERSION: v2
    TIMEOUT: 5000
  defaults:
    timeout: \${TIMEOUT}
    headers:
      X-API-Version: "\${API_VERSION}"
      User-Agent: "curl-runner/1.0.0"

collection:
  name: Production API Tests
  variables:
    # Override global variables
    USER_ID: 12345
    RESOURCE_ID: abc-123
    
  requests:
    - name: Get User Profile
      url: \${BASE_URL}/\${API_VERSION}/users/\${USER_ID}
      method: GET
      headers:
        Authorization: "Bearer \${API_TOKEN}"  # From environment
      expect:
        status: 200
        body:
          id: \${USER_ID}
          active: true
          
    - name: Update Resource
      url: \${BASE_URL}/\${API_VERSION}/resources/\${RESOURCE_ID}
      method: PUT
      headers:
        Content-Type: application/json
        Authorization: "Bearer \${API_TOKEN}"
      body:
        name: "Updated Resource"
        updatedBy: \${USER_ID}
        timestamp: "\${CURRENT_TIME}"  # Dynamic variable
      expect:
        status: 200
        headers:
          etag: "*"  # Any ETag value
        body:
          id: \${RESOURCE_ID}
          updatedBy: \${USER_ID}`;

const authenticationExample = `# Advanced authentication patterns
requests:
  # Basic Authentication
  - name: Basic Auth Example
    url: https://httpbin.org/basic-auth/user/pass
    method: GET
    auth:
      type: basic
      username: user
      password: pass
      
  # Bearer Token Authentication
  - name: Bearer Token Example
    url: https://api.github.com/user
    method: GET
    auth:
      type: bearer
      token: \${GITHUB_TOKEN}
      
  # Custom Authorization Headers
  - name: Custom Auth Example
    url: https://api.example.com/protected
    method: GET
    headers:
      Authorization: "Custom \${CUSTOM_TOKEN}"
      X-API-Key: "\${API_KEY}"
      X-Client-ID: "\${CLIENT_ID}"
      
  # OAuth2-style with refresh
  - name: OAuth2 Request
    url: https://api.oauth-provider.com/data
    method: GET
    headers:
      Authorization: "Bearer \${ACCESS_TOKEN}"
      X-Refresh-Token: "\${REFRESH_TOKEN}"
    expect:
      status: [200, 401]  # Handle token expiry`;

const errorHandlingExample = `# Advanced error handling and retry logic
global:
  continueOnError: true  # Continue even if requests fail
  execution: sequential   # Process one at a time

collection:
  name: Robust API Testing
  
  requests:
    # Retry with exponential backoff
    - name: Flaky Service Call
      url: https://api.flaky-service.com/data
      method: GET
      timeout: 3000
      retry:
        count: 5
        delay: 1000  # Start with 1 second, exponential backoff
      expect:
        status: [200, 503]  # Accept service unavailable
        
    # Handle multiple possible responses
    - name: Resource Check
      url: https://api.example.com/resource/status
      method: GET
      expect:
        status: [200, 404, 410]  # OK, Not Found, or Gone
        body:
          # Different valid response structures
          status: ["active", "inactive", "deleted"]
          
    # Conditional request based on previous
    - name: Conditional Update
      url: https://api.example.com/resource/123
      method: PUT
      headers:
        If-Match: "*"  # Only if resource exists
        Content-Type: application/json
      body:
        status: "updated"
      expect:
        status: [200, 412]  # OK or Precondition Failed
        
    # Fallback request if first fails
    - name: Primary Endpoint
      url: https://api-primary.example.com/data
      method: GET
      timeout: 2000
      
    - name: Fallback Endpoint
      url: https://api-backup.example.com/data
      method: GET
      timeout: 5000
      # Only runs if previous request failed`;

const parallelExecutionExample = `# Parallel execution with dependencies
global:
  execution: parallel  # Run requests concurrently
  continueOnError: false
  
collection:
  name: High-Performance API Tests
  variables:
    BATCH_SIZE: 10
    CONCURRENT_REQUESTS: 5
    
  requests:
    # Batch of parallel requests
    - name: Health Check 1
      url: https://api1.example.com/health
      method: GET
      timeout: 1000
      
    - name: Health Check 2
      url: https://api2.example.com/health
      method: GET
      timeout: 1000
      
    - name: Health Check 3
      url: https://api3.example.com/health
      method: GET
      timeout: 1000
      
    # Load testing pattern
    - name: Load Test Create User 1
      url: https://api.example.com/users
      method: POST
      headers:
        Content-Type: application/json
      body:
        username: "user1_\${TIMESTAMP}"
        email: "user1_\${TIMESTAMP}@example.com"
        
    - name: Load Test Create User 2
      url: https://api.example.com/users
      method: POST
      headers:
        Content-Type: application/json
      body:
        username: "user2_\${TIMESTAMP}"
        email: "user2_\${TIMESTAMP}@example.com"
        
    # Concurrent API validation
    - name: Validate Endpoint A
      url: https://api.example.com/endpoint-a
      method: GET
      expect:
        status: 200
        headers:
          x-rate-limit-remaining: "*"
          
    - name: Validate Endpoint B
      url: https://api.example.com/endpoint-b  
      method: GET
      expect:
        status: 200
        headers:
          x-rate-limit-remaining: "*"`;

const complexValidationExample = `# Complex response validation patterns
requests:
  # Deep object validation with nested structures
  - name: Complex JSON Validation
    url: https://api.example.com/complex-data
    method: GET
    expect:
      status: 200
      headers:
        content-type: application/json
        x-total-count: "^[0-9]+$"  # Regex pattern
      body:
        metadata:
          version: "2.1"
          timestamp: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$"
          pagination:
            page: 1
            per_page: 20
            total: "*"  # Any value present
        data:
          - id: "^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$"  # UUID
            name: ".*"  # Any string
            status: ["active", "inactive", "pending"]  # One of these values
            tags: ["*"]  # Array with at least one element
            created_at: "^\\d{4}-\\d{2}-\\d{2}"  # Date format
            
  # Array validation with specific patterns
  - name: Array Response Validation
    url: https://api.example.com/items
    method: GET
    expect:
      status: 200
      body:
        # Validate each item in array matches pattern
        items:
          - id: "^[0-9]+$"
            type: "product"
            price: "^[0-9]+(\\.[0-9]{2})?$"  # Currency format
            in_stock: true
          - id: "^[0-9]+$"
            type: "service" 
            price: "^[0-9]+(\\.[0-9]{2})?$"
            available: true
            
  # Conditional validation based on response content
  - name: Conditional Validation
    url: https://api.example.com/user/profile
    method: GET
    expect:
      status: 200
      body:
        # If premium user, expect additional fields
        account_type: ["free", "premium", "enterprise"]
        # These fields only present for premium/enterprise
        subscription:
          plan: "^(premium|enterprise)$"
          expires_at: "^\\d{4}-\\d{2}-\\d{2}"
          features: ["*"]  # Array of features
          
  # Multi-format response validation
  - name: XML Response Validation
    url: https://api.example.com/data.xml
    method: GET
    headers:
      Accept: application/xml
    expect:
      status: 200
      headers:
        content-type: "application/xml"
      # XML is parsed to JSON structure for validation
      body:
        root:
          version: "1.0"
          items:
            item:
              - id: "^[0-9]+$"
                name: ".*"`;

const integrationTestExample = `# Full integration test suite
global:
  variables:
    BASE_URL: https://api.example.com
    API_VERSION: v1
  defaults:
    timeout: 10000
    headers:
      User-Agent: "curl-runner-integration-test"
      Accept: application/json
  continueOnError: false
  execution: sequential

collection:
  name: E2E Integration Test Suite
  description: "Complete workflow testing from user creation to deletion"
  
  variables:
    TEST_USER_EMAIL: "test-\${TIMESTAMP}@example.com"
    TEST_USER_NAME: "Test User \${TIMESTAMP}"
    
  requests:
    # 1. Create test user
    - name: Create User Account
      url: \${BASE_URL}/\${API_VERSION}/users
      method: POST
      headers:
        Content-Type: application/json
      body:
        email: \${TEST_USER_EMAIL}
        name: \${TEST_USER_NAME}
        password: "TempPassword123!"
      expect:
        status: 201
        headers:
          location: "^/\${API_VERSION}/users/[0-9]+$"
        body:
          id: "^[0-9]+$"
          email: \${TEST_USER_EMAIL}
          name: \${TEST_USER_NAME}
          created_at: "^\\d{4}-\\d{2}-\\d{2}"
          
    # 2. Login with created user
    - name: User Login
      url: \${BASE_URL}/\${API_VERSION}/auth/login
      method: POST
      headers:
        Content-Type: application/json
      body:
        email: \${TEST_USER_EMAIL}
        password: "TempPassword123!"
      expect:
        status: 200
        body:
          token: "^[A-Za-z0-9\\-_]+\\.[A-Za-z0-9\\-_]+\\.[A-Za-z0-9\\-_]+$"  # JWT
          user:
            id: "^[0-9]+$"
            email: \${TEST_USER_EMAIL}
            
    # 3. Access protected resource
    - name: Get User Profile
      url: \${BASE_URL}/\${API_VERSION}/users/me
      method: GET
      headers:
        Authorization: "Bearer \${LOGIN_TOKEN}"  # From previous response
      expect:
        status: 200
        body:
          email: \${TEST_USER_EMAIL}
          name: \${TEST_USER_NAME}
          profile:
            created_at: "^\\d{4}-\\d{2}-\\d{2}"
            last_login: "^\\d{4}-\\d{2}-\\d{2}"
            
    # 4. Update user profile
    - name: Update Profile
      url: \${BASE_URL}/\${API_VERSION}/users/me
      method: PUT
      headers:
        Authorization: "Bearer \${LOGIN_TOKEN}"
        Content-Type: application/json
      body:
        name: "Updated \${TEST_USER_NAME}"
        preferences:
          theme: "dark"
          notifications: true
      expect:
        status: 200
        body:
          name: "Updated \${TEST_USER_NAME}"
          preferences:
            theme: "dark"
            notifications: true
            
    # 5. Create user resource
    - name: Create User Resource
      url: \${BASE_URL}/\${API_VERSION}/resources
      method: POST
      headers:
        Authorization: "Bearer \${LOGIN_TOKEN}"
        Content-Type: application/json
      body:
        title: "Test Resource"
        description: "Created during integration test"
        type: "document"
      expect:
        status: 201
        body:
          id: "^[0-9]+$"
          title: "Test Resource"
          owner_id: "^[0-9]+$"
          
    # 6. List user resources
    - name: List User Resources  
      url: \${BASE_URL}/\${API_VERSION}/resources?owner=me
      method: GET
      headers:
        Authorization: "Bearer \${LOGIN_TOKEN}"
      expect:
        status: 200
        body:
          items:
            - id: "^[0-9]+$"
              title: "Test Resource"
              owner_id: "^[0-9]+$"
          metadata:
            total: 1
            
    # 7. Delete user resource
    - name: Delete User Resource
      url: \${BASE_URL}/\${API_VERSION}/resources/\${RESOURCE_ID}
      method: DELETE
      headers:
        Authorization: "Bearer \${LOGIN_TOKEN}"
      expect:
        status: 204
        
    # 8. Verify resource deleted
    - name: Verify Resource Deleted
      url: \${BASE_URL}/\${API_VERSION}/resources/\${RESOURCE_ID}
      method: GET
      headers:
        Authorization: "Bearer \${LOGIN_TOKEN}"
      expect:
        status: 404
        
    # 9. Logout user
    - name: User Logout
      url: \${BASE_URL}/\${API_VERSION}/auth/logout
      method: POST
      headers:
        Authorization: "Bearer \${LOGIN_TOKEN}"
      expect:
        status: 200
        
    # 10. Clean up - delete test user
    - name: Delete Test User
      url: \${BASE_URL}/\${API_VERSION}/admin/users/\${USER_ID}
      method: DELETE
      headers:
        Authorization: "Bearer \${ADMIN_TOKEN}"  # Admin privileges required
      expect:
        status: 204`;

const performanceTestingExample = `# Performance and load testing scenarios
global:
  execution: parallel
  continueOnError: true
  variables:
    TARGET_RPS: 100  # Requests per second target
    DURATION: 60     # Test duration in seconds
    
collection:
  name: Performance Test Suite
  
  requests:
    # Warmup requests
    - name: Warmup Request 1
      url: https://api.example.com/health
      method: GET
      timeout: 1000
      
    - name: Warmup Request 2
      url: https://api.example.com/health
      method: GET
      timeout: 1000
      
    # Load testing - multiple identical requests
    - name: Load Test - Get Users Page 1
      url: https://api.example.com/users?page=1&limit=50
      method: GET
      timeout: 2000
      expect:
        status: 200
        headers:
          x-response-time: "^[0-9]+ms$"
          
    - name: Load Test - Get Users Page 2  
      url: https://api.example.com/users?page=2&limit=50
      method: GET
      timeout: 2000
      expect:
        status: 200
        
    - name: Load Test - Get Users Page 3
      url: https://api.example.com/users?page=3&limit=50
      method: GET
      timeout: 2000
      expect:
        status: 200
        
    # Stress testing - resource creation
    - name: Stress Test - Create Resource 1
      url: https://api.example.com/resources
      method: POST
      headers:
        Content-Type: application/json
        Authorization: "Bearer \${TEST_TOKEN}"
      body:
        name: "Load Test Resource 1"
        data: "Performance test data"
      timeout: 5000
      expect:
        status: [201, 429]  # Created or Rate Limited
        
    - name: Stress Test - Create Resource 2
      url: https://api.example.com/resources
      method: POST
      headers:
        Content-Type: application/json
        Authorization: "Bearer \${TEST_TOKEN}"
      body:
        name: "Load Test Resource 2"
        data: "Performance test data"
      timeout: 5000
      expect:
        status: [201, 429]
        
    # Database performance testing
    - name: DB Performance - Complex Query
      url: https://api.example.com/analytics/report
      method: POST
      headers:
        Content-Type: application/json
        Authorization: "Bearer \${TEST_TOKEN}"
      body:
        dateRange:
          start: "2024-01-01"
          end: "2024-12-31"
        metrics: ["users", "revenue", "conversions"]
        groupBy: ["month", "region"]
      timeout: 30000  # Allow longer for complex queries
      expect:
        status: 200
        headers:
          x-query-time: "*"  # Check query timing header
          
    # CDN/Cache performance
    - name: Cache Performance Test
      url: https://cdn.example.com/static/large-file.json
      method: GET
      headers:
        Accept-Encoding: "gzip, deflate"
      timeout: 10000
      expect:
        status: 200
        headers:
          cache-control: "max-age=3600"
          etag: "*"`;

export default function AdvancedExamplesPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Advanced Examples"
          text="Complex real-world scenarios showcasing the full power of curl-runner. These examples demonstrate advanced patterns for production environments."
        />

        <div className="space-y-8">
          {/* Overview */}
          <section>
            <p className="text-muted-foreground mb-6">
              These advanced examples demonstrate sophisticated curl-runner patterns for complex
              workflows, integration testing, performance testing, and production environments. Each
              example is production-ready and includes comprehensive error handling and validation.
            </p>

            <div className="rounded-lg border bg-muted/50 p-4">
              <p className="text-sm">
                <strong>Note:</strong> Advanced examples often use environment variables and dynamic
                values. Make sure to set up your environment properly with required tokens and
                endpoints before running these configurations.
              </p>
            </div>
          </section>

          {/* Variable Interpolation */}
          <section>
            <H2 id="variable-interpolation">Variable Interpolation & Environment Management</H2>
            <p className="text-muted-foreground mb-6">
              Advanced variable usage with environment loading, nested interpolation, and dynamic
              values.
            </p>

            <CodeBlockServer language="yaml" filename="advanced-variables.yaml">
              {variableInterpolationExample}
            </CodeBlockServer>

            <div className="mt-6 grid gap-4 sm:grid-cols-2">
              <Card>
                <CardHeader>
                  <CardTitle className="text-base flex items-center gap-2">
                    Environment Variables
                    <Badge variant="outline">External</Badge>
                  </CardTitle>
                </CardHeader>
                <CardContent className="text-sm space-y-2">
                  <p>
                    Load variables from <code>.env</code> files or system environment.
                  </p>
                  <code className="text-xs bg-muted px-2 py-1 rounded block">
                    API_TOKEN=your_token_here
                  </code>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="text-base flex items-center gap-2">
                    Dynamic Values
                    <Badge variant="outline">Runtime</Badge>
                  </CardTitle>
                </CardHeader>
                <CardContent className="text-sm space-y-2">
                  <p>Generate values at runtime like timestamps or UUIDs.</p>
                  <code className="text-xs bg-muted px-2 py-1 rounded block">
                    {'$'}
                    {'{CURRENT_TIME}'}
                  </code>
                </CardContent>
              </Card>
            </div>
          </section>

          {/* Advanced Authentication */}
          <section>
            <H2 id="advanced-authentication">Advanced Authentication Patterns</H2>
            <p className="text-muted-foreground mb-6">
              Multiple authentication methods, token refresh, and custom auth headers.
            </p>

            <CodeBlockServer language="yaml" filename="auth-patterns.yaml">
              {authenticationExample}
            </CodeBlockServer>
          </section>

          {/* Error Handling */}
          <section>
            <H2 id="error-handling">Error Handling & Retry Logic</H2>
            <p className="text-muted-foreground mb-6">
              Robust error handling with retry mechanisms, fallbacks, and conditional logic.
            </p>

            <CodeBlockServer language="yaml" filename="error-handling.yaml">
              {errorHandlingExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-orange-500/5 dark:bg-orange-500/10 border-orange-500/20 p-4">
              <p className="text-sm">
                <strong className="text-orange-600 dark:text-orange-400">Best Practice:</strong> Use{' '}
                <code>continueOnError: true</code> for test suites where you want to see all
                failures, but <code>false</code> for critical workflows where early termination is
                desired.
              </p>
            </div>
          </section>

          {/* Parallel Execution */}
          <section>
            <H2 id="parallel-execution">High-Performance Parallel Execution</H2>
            <p className="text-muted-foreground mb-6">
              Concurrent request execution for load testing and performance optimization.
            </p>

            <CodeBlockServer language="yaml" filename="parallel-execution.yaml">
              {parallelExecutionExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="parallel-considerations">Parallel Execution Considerations</H3>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-orange-500/10 p-2">
                      <Server className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Resource Limits</h4>
                      <p className="text-sm text-muted-foreground">
                        Monitor system resources and API rate limits when running many concurrent
                        requests.
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <Layers className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Request Independence</h4>
                      <p className="text-sm text-muted-foreground">
                        Ensure parallel requests don't depend on each other's results or side
                        effects.
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Complex Validation */}
          <section>
            <H2 id="complex-validation">Complex Response Validation</H2>
            <p className="text-muted-foreground mb-6">
              Advanced validation patterns for complex data structures, arrays, and conditional
              logic.
            </p>

            <CodeBlockServer language="yaml" filename="complex-validation.yaml">
              {complexValidationExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="validation-patterns">Validation Patterns</H3>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-purple-500/10 p-2">
                      <Target className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Regex Patterns</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Use regex for flexible format validation
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">^[a-f0-9-]{36}$</code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-green-500/10 p-2">
                      <Layers className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Array Validation</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Validate array items and structure
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">{'items: ["*"]'}</code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <CheckCircle className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Conditional Fields</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Different validation based on content
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        {'type: ["free", "premium"]'}
                      </code>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-cyan-500/10 p-2">
                      <Database className="h-4 w-4 text-cyan-600 dark:text-cyan-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Nested Objects</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Deep object structure validation
                      </p>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        metadata.pagination.*
                      </code>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Integration Testing */}
          <section>
            <H2 id="integration-testing">Full Integration Testing</H2>
            <p className="text-muted-foreground mb-6">
              End-to-end workflow testing with user lifecycle management and cleanup.
            </p>

            <CodeBlockServer language="yaml" filename="integration-test.yaml">
              {integrationTestExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-green-500/5 dark:bg-green-500/10 border-green-500/20 p-4">
              <p className="text-sm">
                <strong className="text-green-600 dark:text-green-400">Integration Testing:</strong>{' '}
                This example demonstrates a complete user lifecycle test including creation,
                authentication, resource management, and cleanup. Perfect for CI/CD pipelines.
              </p>
            </div>
          </section>

          {/* Performance Testing */}
          <section>
            <H2 id="performance-testing">Performance & Load Testing</H2>
            <p className="text-muted-foreground mb-6">
              Load testing scenarios with concurrent requests and performance metrics validation.
            </p>

            <CodeBlockServer language="yaml" filename="performance-test.yaml">
              {performanceTestingExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="performance-tips">Performance Testing Tips</H3>
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <Zap className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Warmup Phase</h4>
                      <p className="text-sm text-muted-foreground">
                        Always include warmup requests to establish connections and prime caches
                        before measuring performance.
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-green-500/10 p-2">
                      <Activity className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Realistic Scenarios</h4>
                      <p className="text-sm text-muted-foreground">
                        Mix read and write operations in realistic proportions that match your
                        production traffic patterns.
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4 md:col-span-2 lg:col-span-1">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-orange-500/10 p-2">
                      <BarChart3 className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Monitor Resources</h4>
                      <p className="text-sm text-muted-foreground">
                        Watch for rate limiting (429 responses) and adjust concurrency levels
                        accordingly.
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Running Advanced Examples */}
          <section>
            <H2 id="running-examples">Running Advanced Examples</H2>
            <p className="text-muted-foreground mb-4">
              Advanced examples often require additional setup and environment configuration:
            </p>

            <CodeBlockServer language="bash">
              {`# Set up environment variables
export API_TOKEN="your_api_token"
export BASE_URL="https://api.example.com"
export ADMIN_TOKEN="admin_token"

# Run with environment loading
curl-runner advanced-example.yaml --env production

# Run with custom timeout for load testing
curl-runner performance-test.yaml --timeout 60000

# Run with detailed metrics output
curl-runner integration-test.yaml --verbose --show-metrics

# Save detailed results for analysis
curl-runner complex-validation.yaml --output results.json --format json`}
            </CodeBlockServer>
          </section>

          {/* Best Practices */}
          <section>
            <H2 id="best-practices">Advanced Usage Best Practices</H2>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Globe className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Environment Separation</h4>
                    <p className="text-sm text-muted-foreground">
                      Use different variable files for development, staging, and production
                      environments to avoid accidental cross-environment requests.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Users className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Test Data Isolation</h4>
                    <p className="text-sm text-muted-foreground">
                      Always use unique identifiers (timestamps, UUIDs) for test data to avoid
                      conflicts in concurrent test runs.
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
                    <h4 className="font-medium mb-2">Resource Cleanup</h4>
                    <p className="text-sm text-muted-foreground">
                      Include cleanup steps in your test suites to remove test data and prevent
                      resource accumulation.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-yellow-500/10 p-2">
                    <TrendingUp className="h-4 w-4 text-yellow-600 dark:text-yellow-500" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Performance Baselines</h4>
                    <p className="text-sm text-muted-foreground">
                      Establish performance baselines and validate response times to catch
                      performance regressions early.
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
