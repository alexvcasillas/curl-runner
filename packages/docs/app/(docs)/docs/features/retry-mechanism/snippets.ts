// Retry mechanism page snippets
export const basicRetryExample = `collection:
  name: "Basic Retry Example"
  requests:
    - name: "Request with retries"
      url: "https://httpbin.org/status/500"
      method: GET
      retry:
        count: 3        # Retry up to 3 times
        delay: 1000     # Wait 1 second between retries
      expect:
        status: [200, 500]  # Accept success or failure after retries`;

export const retryConfigurationExample = `collection:
  name: "Retry Configuration Options"
  requests:
    - name: "Fixed delay retry (no backoff)"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 3
        delay: 2000         # Fixed 2s delay between retries
        # backoff defaults to 1 (no increase)

    - name: "Exponential backoff retry (2x)"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 5
        delay: 1000
        backoff: 2          # 1s, 2s, 4s, 8s, 16s

    - name: "Gentle backoff retry (1.5x)"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 4
        delay: 1000
        backoff: 1.5        # 1s, 1.5s, 2.25s, 3.375s`;

export const statusCodeRetryExample = `collection:
  name: "Retry with Validation"
  requests:
    - name: "Retry until success status"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 3
        delay: 1000
        backoff: 2
      expect:
        status: 200           # Retry if status is not 200

    - name: "Retry with timeout handling"
      url: "https://httpbin.org/delay/10"
      method: GET
      timeout: 5000           # 5 second timeout
      retry:
        count: 3
        delay: 2000
        backoff: 1.5
      expect:
        status: [200, 201]    # Accept success statuses

    - name: "Critical endpoint with exponential backoff"
      url: "https://api.example.com/critical"
      method: GET
      retry:
        count: 5
        delay: 1000
        backoff: 2            # 1s, 2s, 4s, 8s, 16s
      expect:
        status: 200`;

export const backoffExamplesSnippet = `collection:
  name: "Backoff Multiplier Examples"
  requests:
    - name: "Aggressive backoff (3x multiplier)"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 4
        delay: 500
        backoff: 3          # 500ms, 1.5s, 4.5s, 13.5s

    - name: "Conservative backoff (1.2x multiplier)"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 5
        delay: 1000
        backoff: 1.2        # 1s, 1.2s, 1.44s, 1.73s, 2.07s

    - name: "Standard exponential (2x multiplier)"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 4
        delay: 1000
        backoff: 2          # 1s, 2s, 4s, 8s`;

export const retryWithExpectExample = `collection:
  name: "Retry with Expect Validation"
  requests:
    - name: "Retry until body matches"
      url: "https://api.example.com/status"
      method: GET
      retry:
        count: 5
        delay: 2000
        backoff: 1.5
      expect:
        status: 200
        body:
          status: "ready"     # Retry until status is "ready"

    - name: "Retry with response time check"
      url: "https://api.example.com/fast"
      method: GET
      retry:
        count: 3
        delay: 1000
        backoff: 2
      expect:
        status: 200
        responseTime: "< 1000"  # Retry if response takes > 1s

    - name: "Retry with header validation"
      url: "https://api.example.com/data"
      method: GET
      retry:
        count: 3
        delay: 1500
        backoff: 2
      expect:
        status: [200, 201]
        headers:
          content-type: "application/json"`;

export const globalRetryExample = `global:
  defaults:
    # Apply retry settings to all requests
    retry:
      count: 3
      delay: 1000
      backoff: 2        # 2x exponential backoff

collection:
  name: "Global Retry Settings"
  requests:
    - name: "Uses global retry settings"
      url: "https://httpbin.org/status/500"
      method: GET
      # Inherits global retry: count=3, delay=1000, backoff=2

    - name: "Override global retry settings"
      url: "https://httpbin.org/status/503"
      method: GET
      retry:
        count: 5        # Override: more retries
        delay: 2000     # Override: longer initial delay
        backoff: 1.5    # Override: gentler backoff

    - name: "Disable retry for this request"
      url: "https://httpbin.org/status/404"
      method: GET
      retry:
        count: 0        # No retries for this request`;

export const retryWithStoreExample = `collection:
  name: "Retry with Response Storage"
  requests:
    - name: "Login with retry and backoff"
      url: "https://api.example.com/auth/login"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "username": "testuser",
          "password": "testpass"
        }
      retry:
        count: 3
        delay: 2000
        backoff: 2            # 2s, 4s, 8s delays
      store:
        AUTH_TOKEN: "body.token"  # Store token after successful retry
      expect:
        status: 200

    - name: "Use stored token with retry"
      url: "https://api.example.com/protected"
      method: GET
      headers:
        Authorization: "Bearer \${store.AUTH_TOKEN}"
      retry:
        count: 2
        delay: 1000
        backoff: 2
      expect:
        status: 200`;

export const advancedRetryExample = `collection:
  name: "Advanced Retry Scenarios"
  requests:
    - name: "Payment processing with exponential backoff"
      url: "https://payments.api.example.com/charge"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "amount": 2999,
          "currency": "USD",
          "source": "tok_visa"
        }
      retry:
        count: 3
        delay: 1000
        backoff: 2          # 1s, 2s, 4s delays
      expect:
        status: [200, 201]

    - name: "File upload with progressive retry"
      url: "https://api.example.com/upload"
      method: POST
      timeout: 30000
      retry:
        count: 5
        delay: 2000
        backoff: 2          # 2s, 4s, 8s, 16s, 32s delays
      expect:
        status: 200

    - name: "Slow database query with gentle backoff"
      url: "https://api.example.com/database/query"
      method: POST
      timeout: 60000
      retry:
        count: 3
        delay: 5000
        backoff: 1.5        # 5s, 7.5s, 11.25s delays
      expect:
        status: 200
        body:
          success: true`;

export const retryBestPracticesExample = `# Best practices for retry configuration
collection:
  name: "Retry Best Practices"
  requests:
    - name: "Idempotent GET request - safe to retry"
      url: "https://api.example.com/users/123"
      method: GET
      retry:
        count: 5
        delay: 1000
        backoff: 2            # Exponential backoff: 1s, 2s, 4s, 8s, 16s
      expect:
        status: 200

    - name: "Idempotent PUT request - safe to retry"
      url: "https://api.example.com/users/123"
      method: PUT
      headers:
        Content-Type: "application/json"
      body: |
        {
          "name": "Updated Name",
          "email": "new@example.com"
        }
      retry:
        count: 3
        delay: 2000
        backoff: 2            # 2s, 4s, 8s delays
      expect:
        status: [200, 204]

    - name: "Non-idempotent POST - conservative retry"
      url: "https://api.example.com/orders"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "productId": "123",
          "quantity": 1
        }
      retry:
        count: 2              # Fewer retries for POST
        delay: 3000
        backoff: 1.5          # Gentler backoff: 3s, 4.5s
      expect:
        status: [200, 201]

    - name: "DELETE request - no retry"
      url: "https://api.example.com/users/123"
      method: DELETE
      retry:
        count: 0              # Don't retry DELETE operations`;
