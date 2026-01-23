// Request object API reference snippets
export const basicRequestExample = `requests:
  - name: "Basic GET request"
    url: "https://api.example.com/users"
    method: GET
    
  - name: "Basic POST request"
    url: "https://api.example.com/users"
    method: POST`;

export const completeRequestExample = `requests:
  - name: "Complete request configuration"
    description: "Example showing all available request properties"
    url: "https://api.example.com/users"
    method: POST
    
    # Request-specific variables
    variables:
      USER_ID: "123"
      ACTION: "create"
      
    # Headers
    headers:
      Content-Type: "application/json"
      Authorization: "Bearer \${API_TOKEN}"
      X-Request-ID: "\${REQUEST_ID}"
      User-Agent: "MyApp/1.0"
      
    # Query parameters
    params:
      limit: 10
      offset: 0
      sort: "created_at"
      order: "desc"
      
    # Request body
    body: |
      {
        "name": "John Doe",
        "email": "john@example.com",
        "role": "user"
      }
      
    # Timing and behavior
    timeout: 30000
    
    # Retry configuration
    retry:
      count: 3
      delay: 1000
      backoff: exponential  # Coming Soon
      
    # Response expectations
    expect:
      status: 201
      headers:
        content-type: "application/json"
      body:
        id: "*"
        name: "John Doe"
        email: "john@example.com"
        
    # Extract values from response
    extract:
      USER_ID: "$.id"
      CREATED_AT: "$.createdAt"`;

export const httpMethodsExample = `requests:
  - name: "GET request"
    url: "https://api.example.com/users/123"
    method: GET
    
  - name: "POST request"
    url: "https://api.example.com/users"
    method: POST
    body: '{"name": "New User"}'
    
  - name: "PUT request"
    url: "https://api.example.com/users/123"
    method: PUT
    body: '{"name": "Updated User"}'
    
  - name: "PATCH request"
    url: "https://api.example.com/users/123"
    method: PATCH
    body: '{"name": "Patched User"}'
    
  - name: "DELETE request"
    url: "https://api.example.com/users/123"
    method: DELETE
    
  - name: "HEAD request"
    url: "https://api.example.com/users/123"
    method: HEAD
    
  - name: "OPTIONS request"
    url: "https://api.example.com/users"
    method: OPTIONS`;

export const headersExample = `requests:
  - name: "Request with various headers"
    url: "https://api.example.com/data"
    method: POST
    headers:
      # Content headers
      Content-Type: "application/json"
      Content-Length: "auto"        # Calculated automatically
      Content-Encoding: "gzip"
      
      # Authentication headers
      Authorization: "Bearer \${ACCESS_TOKEN}"
      X-API-Key: "\${API_KEY}"
      
      # Custom headers
      X-Client-Version: "1.2.3"
      X-Request-ID: "\${UUID}"
      X-Correlation-ID: "\${CORRELATION_ID}"
      
      # Cache control
      Cache-Control: "no-cache"
      If-None-Match: "\${ETAG}"
      
      # User agent
      User-Agent: "MyApp/1.0 (curl-runner)"
      
      # Accept headers
      Accept: "application/json"
      Accept-Language: "en-US,en;q=0.9"
      Accept-Encoding: "gzip, deflate"`;

export const queryParametersExample = `requests:
  - name: "Simple query parameters"
    url: "https://api.example.com/search"
    method: GET
    params:
      q: "search term"
      limit: 20
      offset: 0
      
  - name: "Complex query parameters"
    url: "https://api.example.com/users"
    method: GET
    params:
      # Simple values
      active: true
      role: "admin"
      
      # Arrays
      tags: ["user", "admin", "active"]
      
      # Variable substitution
      userId: "\${USER_ID}"
      timestamp: "\${TIMESTAMP}"
      
      # Special characters (will be URL encoded)
      filter: "name contains 'John Doe'"
      
  - name: "Dynamic parameters"
    url: "https://api.example.com/analytics"
    method: GET
    params:
      startDate: "\${START_DATE}"
      endDate: "\${END_DATE}"
      metrics: ["views", "clicks", "conversions"]
      groupBy: "day"`;

export const bodyTypesExample = `requests:
  # JSON body (string format)
  - name: "JSON string body"
    url: "https://api.example.com/users"
    method: POST
    headers:
      Content-Type: "application/json"
    body: |
      {
        "name": "John Doe",
        "email": "john@example.com",
        "age": 30
      }
      
  # JSON body (object format)
  - name: "JSON object body"
    url: "https://api.example.com/users"
    method: POST
    headers:
      Content-Type: "application/json"
    body:
      name: "John Doe"
      email: "john@example.com"
      age: 30
      
  # URL-encoded form data
  - name: "URL-encoded form body"
    url: "https://api.example.com/form"
    method: POST
    headers:
      Content-Type: "application/x-www-form-urlencoded"
    body: "username=johndoe&password=secret123&remember=true"

  # Multipart form data with file upload (use formData instead of body)
  - name: "Multipart form with files"
    url: "https://api.example.com/upload"
    method: POST
    formData:
      # Regular form fields
      description: "File upload"
      category: "document"
      # File attachment
      file:
        file: "./document.pdf"
        filename: "my-document.pdf"
        contentType: "application/pdf"

  # Raw text body
  - name: "Raw text body"
    url: "https://api.example.com/webhook"
    method: POST
    headers:
      Content-Type: "text/plain"
    body: "Raw text content for webhook"
    
  # XML body
  - name: "XML body"
    url: "https://api.example.com/soap"
    method: POST
    headers:
      Content-Type: "application/xml"
    body: |
      <?xml version="1.0" encoding="UTF-8"?>
      <soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
        <soap:Body>
          <request>
            <action>getData</action>
            <userId>123</userId>
          </request>
        </soap:Body>
      </soap:Envelope>`;

export const timeoutExample = `requests:
  - name: "Quick request with short timeout"
    url: "https://api.example.com/quick"
    method: GET
    timeout: 2000          # 2 seconds
    
  - name: "Slow operation with long timeout"
    url: "https://api.example.com/heavy-processing"
    method: POST
    timeout: 60000         # 60 seconds
    
  - name: "No timeout (use global default)"
    url: "https://api.example.com/standard"
    method: GET
    # Uses global or collection default timeout
    
  - name: "Infinite timeout"
    url: "https://api.example.com/streaming"
    method: GET
    timeout: 0             # No timeout limit`;

export const variablesExample = `requests:
  - name: "Request with local variables"
    url: "https://api.example.com/users/\${USER_ID}"
    method: GET
    variables:
      USER_ID: "123"               # Local to this request
      REQUEST_TYPE: "user_fetch"
      CACHE_KEY: "user_\${USER_ID}"
    headers:
      X-Request-Type: "\${REQUEST_TYPE}"
      X-Cache-Key: "\${CACHE_KEY}"
      
  - name: "Override global variables"
    url: "https://api.example.com/data"
    method: GET
    variables:
      TIMEOUT: "30000"    # Override global TIMEOUT for this request
      API_VERSION: "v2"   # Override global API_VERSION
    timeout: \${TIMEOUT}`;

export const extractExample = `# Data Extraction Examples - Coming Soon
requests:
  - name: "Login and extract token"  # Coming Soon
    url: "https://api.example.com/auth/login"
    method: POST
    body: |
      {
        "username": "testuser",
        "password": "testpass"
      }
    extract:  # Coming Soon - data extraction not implemented
      # Extract from response body using JSON path - Coming Soon
      ACCESS_TOKEN: "$.token"  # Coming Soon
      REFRESH_TOKEN: "$.refreshToken"  # Coming Soon
      USER_ID: "$.user.id"  # Coming Soon
      EXPIRES_AT: "$.expiresIn"  # Coming Soon
      
      # Extract from response headers
      REQUEST_ID: "headers['x-request-id']"
      RATE_LIMIT: "headers['x-rate-limit-remaining']"
      
  - name: "Use extracted values"
    url: "https://api.example.com/users/\${USER_ID}"
    method: GET
    headers:
      Authorization: "Bearer \${ACCESS_TOKEN}"
      X-Request-ID: "\${REQUEST_ID}"`;

export const formDataExample = `# Form Data and File Upload Examples
requests:
  # Simple file upload
  - name: "Single file upload"
    url: "https://api.example.com/upload"
    method: POST
    formData:
      document:
        file: "./report.pdf"

  # File with custom filename and content type
  - name: "File with options"
    url: "https://api.example.com/upload"
    method: POST
    formData:
      document:
        file: "./local-file.pdf"
        filename: "quarterly-report.pdf"
        contentType: "application/pdf"

  # Multiple files with form fields
  - name: "Mixed form data"
    url: "https://api.example.com/submit"
    method: POST
    formData:
      # Text fields
      title: "Document Submission"
      description: "Q4 Financial Reports"
      submitted_by: "john@example.com"

      # File attachments
      main_document:
        file: "./report.pdf"
        filename: "financial-report.pdf"
        contentType: "application/pdf"
      supporting_document:
        file: "./appendix.xlsx"
        contentType: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"

  # File upload with variables
  - name: "Dynamic file upload"
    url: "\${API_URL}/users/\${USER_ID}/avatar"
    method: POST
    formData:
      description: "Uploaded on \${DATE:YYYY-MM-DD}"
      request_id: "\${UUID}"
      avatar:
        file: "./profile-photo.jpg"
        contentType: "image/jpeg"`;

export const advancedRequestExample = `requests:
  - name: "Advanced request with all features"
    description: "Comprehensive example showing all request capabilities"
    url: "\${BASE_URL}/\${API_VERSION}/complex-operation"
    method: POST
    
    # Local variables
    variables:
      OPERATION_ID: "\${UUID}"
      BATCH_SIZE: "100"
      PRIORITY: "high"
      
    # Comprehensive headers
    headers:
      Content-Type: "application/json"
      Authorization: "Bearer \${ACCESS_TOKEN}"
      X-Operation-ID: "\${OPERATION_ID}"
      X-Client-Version: "\${CLIENT_VERSION}"
      X-Request-Priority: "\${PRIORITY}"
      X-Batch-Size: "\${BATCH_SIZE}"
      X-Idempotency-Key: "\${OPERATION_ID}"
      
    # Query parameters
    params:
      async: true
      callback: "https://callback.example.com/webhook"
      timeout: \${REQUEST_TIMEOUT}
      
    # Complex JSON body with variables
    body: |
      {
        "operationId": "\${OPERATION_ID}",
        "type": "batch_process",
        "priority": "\${PRIORITY}",
        "config": {
          "batchSize": \${BATCH_SIZE},
          "timeout": \${REQUEST_TIMEOUT},
          "retryPolicy": {
            "maxRetries": 3,
            "backoffStrategy": "exponential"
          }
        },
        "data": {
          "items": [
            {"id": 1, "action": "process"},
            {"id": 2, "action": "validate"},
            {"id": 3, "action": "transform"}
          ],
          "metadata": {
            "source": "api_client",
            "timestamp": "\${TIMESTAMP}",
            "version": "\${API_VERSION}"
          }
        }
      }
      
    # Extended timeout for complex operation
    timeout: 120000
    
    # Robust retry configuration
    retry:
      count: 3
      delay: 5000
      backoff: exponential  # Coming Soon
      maxDelay: 30000  # Coming Soon
      jitter: true  # Coming Soon
      codes: [500, 502, 503, 504]  # Coming Soon
      
    # Comprehensive validation
    expect:
      status: [200, 202]
      headers:
        content-type: "application/json"
        x-operation-id: "\${OPERATION_ID}"
      body:
        operationId: "\${OPERATION_ID}"
        status: ["accepted", "processing", "completed"]
        timestamp: "*"
        
    # Extract response data - Coming Soon
    extract:  # Coming Soon - data extraction not implemented
      JOB_ID: "$.jobId"  # Coming Soon
      STATUS_URL: "$.statusUrl"  # Coming Soon
      ESTIMATED_COMPLETION: "$.estimatedCompletion"  # Coming Soon
      
    # Custom validation - Coming Soon
    custom:  # Coming Soon - custom validation expressions not implemented
      - name: "Operation ID matches"  # Coming Soon
        expression: "response.body.operationId === variables.OPERATION_ID"  # Coming Soon
      - name: "Status URL is valid"  # Coming Soon
        expression: "/^https?:\\/\\/.+/.test(response.body.statusUrl)"  # Coming Soon
      - name: "Response time acceptable"  # Coming Soon
        expression: "response.time < 30000"  # Coming Soon`;

export const validationConfigExample = `# Response Validation Examples
requests:
  # Standard success validation
  - name: "Success Response Validation"
    url: "https://api.example.com/users"
    method: POST
    body:
      username: "newuser"
      email: "newuser@example.com"
    expect:
      failure: false    # Expect success (default)
      status: 201       # Created
      headers:
        content-type: "application/json"
        location: "^/users/[0-9]+$"
      body:
        id: "^[0-9]+$"
        username: "newuser"
        email: "newuser@example.com"
        created_at: "*"   # Any timestamp value
        
  # Negative testing - expect failure
  - name: "Invalid Input Validation"  
    url: "https://api.example.com/users"
    method: POST
    body:
      username: ""      # Invalid: empty username
      email: "invalid"  # Invalid: malformed email
    expect:
      failure: true     # Expect this request to fail
      status: 400       # Bad Request
      body:
        error: "Validation failed"
        details:
          username: "Username is required"
          email: "Invalid email format"
          
  # Authentication failure testing
  - name: "Unauthorized Access Test"
    url: "https://api.example.com/protected"
    method: GET
    headers:
      authorization: "Bearer invalid-token"
    expect:
      failure: true     # Expect authentication to fail
      status: 401       # Unauthorized
      body:
        error: "Invalid token"
        
  # Mixed validation with multiple acceptable statuses
  - name: "Flexible Status Validation"
    url: "https://api.example.com/cached-resource"
    method: GET
    headers:
      if-none-match: "some-etag"
    expect:
      failure: false    # Expect success
      status: [200, 304] # OK or Not Modified
      headers:
        cache-control: "*"  # Any cache control header`;
