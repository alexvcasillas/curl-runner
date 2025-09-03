// Response object API reference snippets
export const basicResponseExample = `# Example response object structure
{
  "status": 200,
  "statusText": "OK",
  "headers": {
    "content-type": "application/json; charset=utf-8",
    "content-length": "156",
    "cache-control": "no-cache",
    "x-request-id": "req_123456789"
  },
  "body": {
    "id": 1,
    "name": "John Doe",
    "email": "john@example.com"
  },
  "time": 234,
  "size": 156
}`;

export const responsePropertiesExample = `# Complete response object with all properties
{
  # HTTP status information
  "status": 200,
  "statusText": "OK",
  
  # Response headers
  "headers": {
    "content-type": "application/json; charset=utf-8",
    "content-length": "1024",
    "cache-control": "max-age=3600",
    "etag": ""abc123"",
    "x-rate-limit-remaining": "99",
    "x-rate-limit-reset": "1640995200"
  },
  
  # Response body (parsed based on content-type)
  "body": {
    "data": {...},
    "metadata": {...}
  },
  
  # Timing information (milliseconds)
  "time": 456,
  
  # Response size (bytes)
  "size": 1024,
  
  # Request information
  "request": {
    "method": "GET",
    "url": "https://api.example.com/users/1",
    "headers": {...}
  },
  
  # Additional metadata
  "timestamp": "2024-01-01T12:00:00Z",
  "redirects": 0,
  "certificates": {...}
}`;

export const statusCodeHandlingExample = `requests:
  - name: "Success status codes"
    url: "https://api.example.com/users"
    method: GET
    expect:
      status: 200         # Exact match
      
  - name: "Multiple acceptable status codes"
    url: "https://api.example.com/users/999"
    method: GET
    expect:
      status: [404, 410]  # Either 404 or 410 is acceptable
      
  - name: "Status code ranges"
    url: "https://api.example.com/upload"
    method: POST
    expect:
      status: "2xx"       # Any 2xx status code
      
  - name: "Custom status validation"
    url: "https://api.example.com/process"
    method: POST
    expect:
      custom:
        - name: "Successful or accepted"
          expression: "response.status >= 200 && response.status < 300"`;

export const headerAccessExample = `requests:
  - name: "Access response headers"
    url: "https://api.example.com/data"
    method: GET
    expect:
      headers:
        content-type: "application/json"
        cache-control: "*"          # Any value
        x-rate-limit-remaining: "^\\d+$"  # Regex pattern
        
    custom:
      - name: "Rate limit check"
        expression: "parseInt(response.headers['x-rate-limit-remaining']) > 0"
        
      - name: "Response is cacheable"
        expression: "response.headers['cache-control'].includes('max-age')"
        
    extract:
      RATE_LIMIT: "headers['x-rate-limit-remaining']"
      ETAG: "headers['etag']"
      REQUEST_ID: "headers['x-request-id']"`;

export const bodyParsingExample = `requests:
  # JSON response body
  - name: "JSON response"
    url: "https://api.example.com/users/1"
    method: GET
    expect:
      headers:
        content-type: "application/json"
      body:
        id: 1
        name: "John Doe"
        email: "john@example.com"
        
  # XML response body
  - name: "XML response"
    url: "https://api.example.com/data.xml"
    method: GET
    expect:
      headers:
        content-type: "application/xml"
      # XML is parsed and accessible via xpath-like syntax
      body:
        "/response/status": "success"
        "/response/data/item[1]/name": "First Item"
        
  # Text response body
  - name: "Text response"
    url: "https://api.example.com/health"
    method: GET
    expect:
      headers:
        content-type: "text/plain"
      body: "OK"
      
  # HTML response body
  - name: "HTML response"
    url: "https://example.com"
    method: GET
    expect:
      headers:
        content-type: "text/html"
      # HTML can be validated using CSS selectors
      body:
        "title": "*"
        "meta[name='description']": "*"`;

export const timingExample = `requests:
  - name: "Performance monitoring"
    url: "https://api.example.com/heavy-operation"
    method: GET
    expect:
      # Response time validation
      responseTime: "<5000"        # Must be under 5 seconds
      
    custom:
      - name: "Response time acceptable"
        expression: "response.time < 3000"
        
      - name: "Fast enough for SLA"
        expression: "response.time < 2000"
        
  - name: "Timing thresholds"
    url: "https://api.example.com/search"
    method: GET
    expect:
      responseTime: ">100,<1000"   # Between 100ms and 1 second
      
    custom:
      - name: "Not too fast (caching check)"
        expression: "response.time > 50"
        
      - name: "Performance baseline"
        expression: "response.time < 800"`;

export const extractionExample = `requests:
  - name: "Extract data from response"
    url: "https://api.example.com/auth/login"
    method: POST
    body: |
      {
        "username": "testuser",
        "password": "testpass"
      }
    extract:
      # Extract from JSON body using JSONPath
      ACCESS_TOKEN: "$.token"
      USER_ID: "$.user.id"
      USER_NAME: "$.user.name"
      EXPIRES_AT: "$.expiresAt"
      
      # Extract from headers
      REQUEST_ID: "headers['x-request-id']"
      RATE_LIMIT: "headers['x-rate-limit-remaining']"
      
      # Extract using expressions
      DOMAIN: "response.body.user.email.split('@')[1]"
      IS_ADMIN: "response.body.user.roles.includes('admin')"
      
      # Extract computed values
      TOKEN_EXPIRES_MINUTES: "(response.body.expiresAt - Date.now()) / 60000"
      
  - name: "Use extracted values"
    url: "https://api.example.com/users/\${USER_ID}/profile"
    method: GET
    headers:
      Authorization: "Bearer \${ACCESS_TOKEN}"
      X-Request-ID: "\${REQUEST_ID}"
    expect:
      body:
        id: \${USER_ID}
        name: "\${USER_NAME}"
        domain: "\${DOMAIN}"`;

export const arrayResponseExample = `requests:
  - name: "Array response validation"
    url: "https://api.example.com/users"
    method: GET
    expect:
      body:
        # Array length validation
        length: 10
        
        # Validate first item
        [0]:
          id: 1
          name: "*"
          email: "*"
          
        # Validate specific item by index
        [5]:
          id: 6
          active: true
          
        # Validate all items have required fields
        "*":
          id: "*"
          name: "*"
          email: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"
          
    extract:
      FIRST_USER_ID: "$[0].id"
      LAST_USER_ID: "$[-1].id"
      USER_COUNT: "$.length"
      ADMIN_USERS: "$.filter(user => user.role === 'admin').length"`;

export const errorResponseExample = `requests:
  - name: "Handle error response"
    url: "https://api.example.com/users/999999"
    method: GET
    expect:
      status: 404
      headers:
        content-type: "application/json"
      body:
        error: "Not Found"
        message: "User not found"
        code: 404
        timestamp: "*"
        path: "/users/999999"
        
    extract:
      ERROR_CODE: "$.code"
      ERROR_MESSAGE: "$.message"
      
  - name: "Validation error response"
    url: "https://api.example.com/users"
    method: POST
    body: |
      {
        "email": "invalid-email"
      }
    expect:
      status: 422
      body:
        error: "Validation Failed"
        details:
          "*":                    # Each validation error
            field: "*"
            message: "*"
            code: "*"`;

export const customValidationExample = `requests:
  - name: "Custom response validation"
    url: "https://api.example.com/complex-data"
    method: GET
    expect:
      custom:
        # Validate response structure
        - name: "Response has required fields"
          expression: |
            response.body.hasOwnProperty('data') &&
            response.body.hasOwnProperty('metadata') &&
            response.body.hasOwnProperty('pagination')
            
        # Validate data integrity
        - name: "Pagination matches data"
          expression: |
            response.body.data.length === response.body.pagination.itemsPerPage ||
            response.body.pagination.currentPage === response.body.pagination.totalPages
            
        # Validate business rules
        - name: "User balance is valid"
          expression: |
            response.body.user.balance >= 0 &&
            typeof response.body.user.balance === 'number'
            
        # Validate timestamps
        - name: "Timestamps are recent"
          expression: |
            Date.now() - new Date(response.body.metadata.timestamp).getTime() < 300000
            
        # Validate security
        - name: "No sensitive data leaked"
          expression: |
            !JSON.stringify(response.body).includes('password') &&
            !JSON.stringify(response.body).includes('secret')`;

export const responseTransformationExample = `requests:
  - name: "Transform response data"
    url: "https://api.example.com/users"
    method: GET
    transform:
      # Transform response before validation
      body: |
        response.body.map(user => ({
          ...user,
          fullName: user.firstName + ' ' + user.lastName,
          isActive: user.status === 'active'
        }))
        
    expect:
      body:
        "*":
          fullName: "*"
          isActive: "*"
          
    extract:
      ACTIVE_USERS: "response.body.filter(user => user.isActive).length"
      USER_NAMES: "response.body.map(user => user.fullName)"`;

export const comprehensiveResponseExample = `requests:
  - name: "Comprehensive response handling"
    url: "https://api.example.com/orders/123"
    method: GET
    expect:
      # Status validation
      status: 200
      
      # Header validation
      headers:
        content-type: "application/json"
        x-request-id: "^req_[a-zA-Z0-9]+$"
        cache-control: "*"
        
      # Body structure validation
      body:
        id: 123
        status: ["pending", "processing", "shipped", "delivered"]
        createdAt: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}"
        updatedAt: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}"
        
        customer:
          id: "*"
          email: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"
          
        items:
          length: ">0"
          "*":
            productId: "*"
            quantity: "^[1-9]\\d*$"
            price: "^\\d+\\.\\d{2}$"
            
        totals:
          subtotal: "^\\d+\\.\\d{2}$"
          tax: "^\\d+\\.\\d{2}$"
          shipping: "^\\d+\\.\\d{2}$"
          total: "^\\d+\\.\\d{2}$"
          
      # Performance validation
      responseTime: "<2000"
      
      # Custom business logic validation
      custom:
        - name: "Order total is correct"
          expression: |
            Math.abs(
              response.body.totals.total - 
              (response.body.totals.subtotal + response.body.totals.tax + response.body.totals.shipping)
            ) < 0.01
            
        - name: "Items have valid data"
          expression: |
            response.body.items.every(item => 
              item.quantity > 0 && item.price > 0
            )
            
        - name: "Timestamps are logical"
          expression: |
            new Date(response.body.updatedAt) >= new Date(response.body.createdAt)
            
    # Extract data for subsequent requests
    extract:
      ORDER_STATUS: "$.status"
      CUSTOMER_ID: "$.customer.id"
      ORDER_TOTAL: "$.totals.total"
      ITEM_COUNT: "$.items.length"
      LAST_UPDATED: "$.updatedAt"`;
