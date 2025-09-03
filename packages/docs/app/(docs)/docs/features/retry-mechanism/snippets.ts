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
    - name: "Linear backoff retry"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 5
        delay: 2000
        backoff: linear     # 2s, 4s, 6s, 8s, 10s
        maxDelay: 10000
        
    - name: "Exponential backoff retry"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 5
        delay: 1000
        backoff: exponential  # 1s, 2s, 4s, 8s, 16s
        maxDelay: 30000
        
    - name: "Fixed delay retry"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 3
        delay: 5000
        backoff: fixed      # 5s, 5s, 5s`;

export const statusCodeRetryExample = `collection:
  name: "Status Code Based Retry"
  requests:
    - name: "Retry on server errors only"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 3
        delay: 1000
        codes: [500, 502, 503, 504]  # Only retry server errors
        
    - name: "Retry on timeouts and server errors"
      url: "https://httpbin.org/delay/10"
      method: GET
      timeout: 5000
      retry:
        count: 2
        delay: 2000
        codes: [408, 500, 502, 503, 504]  # Include timeout errors
        
    - name: "Custom retry codes"
      url: "https://api.example.com/flaky-endpoint"
      method: GET
      retry:
        count: 3
        delay: 1000
        codes: [429, 500, 503]  # Rate limit + server errors`;

export const jitterRetryExample = `collection:
  name: "Retry with Jitter"
  requests:
    - name: "Retry with jitter to avoid thundering herd"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 5
        delay: 1000
        backoff: exponential
        jitter: true        # Add random variation to delays
        maxDelay: 30000
        
    - name: "Custom jitter range"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 3
        delay: 2000
        jitter: true
        jitterRange: 0.5    # ±50% variation in delay`;

export const conditionalRetryExample = `collection:
  name: "Conditional Retry Logic"
  requests:
    - name: "Retry based on response content"
      url: "https://api.example.com/status"
      method: GET
      retry:
        count: 3
        delay: 1000
        condition: "response.body.status === 'processing'"
        
    - name: "Retry based on headers"
      url: "https://api.example.com/data"
      method: GET
      retry:
        count: 5
        delay: 2000
        condition: "response.headers['x-retry-after']"
        
    - name: "Complex retry condition"
      url: "https://api.example.com/complex"
      method: GET
      retry:
        count: 3
        delay: 1000
        condition: |
          response.status >= 500 || 
          (response.status === 429 && response.headers['retry-after']) ||
          response.body.error === 'temporary_failure'`;

export const globalRetryExample = `global:
  defaults:
    # Apply retry settings to all requests
    retry:
      count: 2
      delay: 1000
      backoff: exponential
      codes: [500, 502, 503, 504]

collection:
  name: "Global Retry Settings"
  requests:
    - name: "Uses global retry settings"
      url: "https://httpbin.org/status/500"
      method: GET
      # Inherits global retry configuration
      
    - name: "Override global retry settings"
      url: "https://httpbin.org/status/503"
      method: GET
      retry:
        count: 5        # Override global count
        delay: 2000     # Override global delay
        # Other settings inherited from global
        
    - name: "Disable retry for this request"
      url: "https://httpbin.org/status/404"
      method: GET
      retry:
        count: 0        # No retries for this request`;

export const retryWithExtractExample = `collection:
  name: "Retry with Data Extraction"
  requests:
    - name: "Login with retry"
      url: "https://api.example.com/auth/login"
      method: POST
      body: |
        {
          "username": "testuser",
          "password": "testpass"
        }
      retry:
        count: 3
        delay: 2000
        codes: [500, 502, 503]
      extract:
        AUTH_TOKEN: "$.token"  # Extract even after retries
        
    - name: "Use extracted token"
      url: "https://api.example.com/protected"
      method: GET
      headers:
        Authorization: "Bearer \${AUTH_TOKEN}"
      retry:
        count: 2
        delay: 1000`;

export const advancedRetryExample = `collection:
  name: "Advanced Retry Scenarios"
  requests:
    - name: "Payment processing with smart retry"
      url: "https://payments.api.example.com/charge"
      method: POST
      headers:
        Content-Type: "application/json"
        Idempotency-Key: "\${UUID}"  # Prevent duplicate charges
      body: |
        {
          "amount": 2999,
          "currency": "USD",
          "source": "tok_visa"
        }
      retry:
        count: 3
        delay: 1000
        backoff: exponential
        maxDelay: 10000
        jitter: true
        codes: [500, 502, 503, 504, 429]
        condition: |
          response.status >= 500 || 
          response.status === 429 ||
          (response.body && response.body.error && 
           response.body.error.type === 'api_connection_error')
           
    - name: "File upload with progressive retry"
      url: "https://api.example.com/upload"
      method: POST
      timeout: 30000
      retry:
        count: 5
        delay: 2000
        backoff: exponential
        maxDelay: 60000
        jitter: true
        codes: [408, 500, 502, 503, 504]
        
    - name: "Database operation with circuit breaker"
      url: "https://api.example.com/database/query"
      method: POST
      retry:
        count: 3
        delay: 5000
        backoff: exponential
        maxDelay: 30000
        codes: [500, 503, 504]
        condition: |
          response.status >= 500 && 
          !response.headers['x-circuit-breaker-open']`;

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
        backoff: exponential
        jitter: true
        codes: [500, 502, 503, 504, 408]
        
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
        backoff: exponential
        codes: [500, 502, 503, 504]
        
    - name: "Non-idempotent POST - careful retry"
      url: "https://api.example.com/orders"
      method: POST
      headers:
        Content-Type: "application/json"
        Idempotency-Key: "\${UUID}"  # Make it safe to retry
      body: |
        {
          "productId": "123",
          "quantity": 1
        }
      retry:
        count: 2              # Fewer retries for POST
        delay: 3000
        codes: [500, 502, 503, 504]  # Don't retry client errors
        condition: |
          response.status >= 500 && 
          response.body.error !== 'duplicate_order'
          
    - name: "DELETE request - no retry"
      url: "https://api.example.com/users/123"
      method: DELETE
      retry:
        count: 0              # Don't retry DELETE operations`;
