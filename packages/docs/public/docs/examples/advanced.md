---
title: "Advanced Examples"
description: "Complex real-world scenarios showcasing the full power of curl-runner. These examples demonstrate advanced patterns for production environments."
category: "Documentation"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - advanced
  - examples
  - yaml
  - variables
  - authentication
  - parallel
  - sequential
  - validation
  - retry
  - timeout
  - headers
  - response
  - request
  - collection
  - cli
  - environment
slug: "/docs/advanced"
toc: true
date: "2026-01-24T16:01:46.342Z"
lastModified: "2026-01-24T16:01:46.342Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Advanced Examples"
  category: "Documentation"
tags:
  - documentation
  - documentation
og:
  title: "Advanced Examples - curl-runner Documentation"
  description: "Complex real-world scenarios showcasing the full power of curl-runner. These examples demonstrate advanced patterns for production environments."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Advanced Examples"
  description: "Complex real-world scenarios showcasing the full power of curl-runner. These examples demonstrate advanced patterns for production environments."
  datePublished: "2026-01-24T16:01:46.342Z"
  dateModified: "2026-01-24T16:01:46.342Z"
---

# Advanced Examples

Complex real-world scenarios showcasing the full power of curl-runner. These examples demonstrate advanced patterns for production environments.

These advanced examples demonstrate sophisticated curl-runner patterns for complex workflows, integration testing, performance testing, and production environments. Each example is production-ready and includes comprehensive error handling and validation.

## Variable Interpolation & Environment Management

Advanced variable usage with environment loading, nested interpolation, and dynamic values.

**advanced-variables.yaml**

```yaml
# Variable interpolation with environment loading
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
          updatedBy: \${USER_ID}
```

## Advanced Authentication Patterns

Multiple authentication methods, token refresh, and custom auth headers.

**auth-patterns.yaml**

```yaml
# Advanced authentication patterns
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
      status: [200, 401]  # Handle token expiry
```

## Error Handling & Retry Logic

Robust error handling with retry mechanisms, fallbacks, and conditional logic.

**error-handling.yaml**

```yaml
# Advanced error handling and retry logic
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
      # Only runs if previous request failed
```

## High-Performance Parallel Execution

Concurrent request execution for load testing and performance optimization.

**parallel-execution.yaml**

```yaml
# Parallel execution with dependencies
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
          x-rate-limit-remaining: "*"
```

## Complex Response Validation

Advanced validation patterns for complex data structures, arrays, and conditional logic.

**complex-validation.yaml**

```yaml
# Complex response validation patterns
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
                name: ".*"
```

## Full Integration Testing

End-to-end workflow testing with user lifecycle management and cleanup.

**integration-test.yaml**

```yaml
# Full integration test suite
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
        status: 204
```

## Performance & Load Testing

Load testing scenarios with concurrent requests and performance metrics validation.

**performance-test.yaml**

```yaml
# Performance and load testing scenarios
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
          etag: "*"
```

## Running Advanced Examples

Advanced examples often require additional setup and environment configuration:

```bash
# Set up environment variables
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
curl-runner complex-validation.yaml --output results.json --format json
```

## Advanced Usage Best Practices
