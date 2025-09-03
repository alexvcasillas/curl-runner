// Advanced examples page snippets
export const authenticationFlowExample = `global:
  variables:
    BASE_URL: "https://api.example.com"
    USERNAME: "testuser"
    PASSWORD: "testpass"

collection:
  name: "Authentication Flow"
  requests:
    - name: "Login and get token"
      url: "\${BASE_URL}/auth/login"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "username": "\${USERNAME}",
          "password": "\${PASSWORD}"
        }
      expect:
        status: 200
        body:
          token: "*"
      extract:
        AUTH_TOKEN: "$.token"
        
    - name: "Access protected resource"
      url: "\${BASE_URL}/protected/profile"
      method: GET
      headers:
        Authorization: "Bearer \${AUTH_TOKEN}"
      expect:
        status: 200
        body:
          username: "\${USERNAME}"`;

export const fileUploadExample = `collection:
  name: "File Upload Example"
  requests:
    - name: "Upload file"
      url: "https://httpbin.org/post"
      method: POST
      headers:
        Content-Type: "multipart/form-data"
      body:
        type: "form-data"
        fields:
          file: "@/path/to/file.txt"
          description: "Test file upload"
          category: "documents"
      expect:
        status: 200
        body:
          files:
            file: "*"`;

export const dynamicDataExample = `global:
  variables:
    TIMESTAMP: "\${DATE:YYYY-MM-DDTHH:mm:ssZ}"
    REQUEST_ID: "\${UUID}"

collection:
  name: "Dynamic Data Example"
  requests:
    - name: "Create record with dynamic data"
      url: "https://httpbin.org/post"
      method: POST
      headers:
        Content-Type: "application/json"
        X-Request-ID: "\${REQUEST_ID}"
      body: |
        {
          "timestamp": "\${TIMESTAMP}",
          "requestId": "\${REQUEST_ID}",
          "data": {
            "value": \${RANDOM:1-1000},
            "category": "test"
          }
        }
      expect:
        status: 200`;

export const conditionalRequestsExample = `global:
  variables:
    ENVIRONMENT: "\${NODE_ENV:development}"
    
collection:
  name: "Conditional Requests"
  requests:
    - name: "Environment specific request"
      url: "\${ENVIRONMENT:production:https://api.example.com:http://localhost:3000}/data"
      method: GET
      headers:
        X-Environment: "\${ENVIRONMENT}"
      expect:
        status: 200
        
    - name: "Conditional body"
      url: "https://httpbin.org/post"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "environment": "\${ENVIRONMENT}",
          "debug": \${ENVIRONMENT:development:true:false},
          "apiUrl": "\${ENVIRONMENT:production:https://api.prod.com:http://localhost:3000}"
        }
      expect:
        status: 200`;

export const chainedRequestsExample = `collection:
  name: "Chained Requests Example"
  requests:
    - name: "Create user"
      url: "https://jsonplaceholder.typicode.com/users"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "name": "Test User",
          "email": "test@example.com"
        }
      expect:
        status: 201
      extract:
        USER_ID: "$.id"
        
    - name: "Create post for user"
      url: "https://jsonplaceholder.typicode.com/posts"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "title": "User's First Post",
          "body": "This is a post by the newly created user",
          "userId": \${USER_ID}
        }
      expect:
        status: 201
        body:
          userId: \${USER_ID}
      extract:
        POST_ID: "$.id"
        
    - name: "Add comment to post"
      url: "https://jsonplaceholder.typicode.com/comments"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "postId": \${POST_ID},
          "name": "Test Comment",
          "email": "commenter@example.com",
          "body": "This is a test comment"
        }
      expect:
        status: 201
        body:
          postId: \${POST_ID}`;

export const retryMechanismExample = `collection:
  name: "Retry Mechanism Example"
  requests:
    - name: "Request with retry"
      url: "https://httpbin.org/status/500"
      method: GET
      retry:
        count: 3
        delay: 1000
        backoff: exponential
        codes: [500, 502, 503, 504]
      expect:
        status: [200, 500]  # Accept either success or continued failure
        
    - name: "Request with custom retry logic"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 5
        delay: 2000
        backoff: linear
        maxDelay: 10000
        jitter: true
      expect:
        status: 200`;

export const complexValidationExample = `collection:
  name: "Complex Validation Example"
  requests:
    - name: "Comprehensive response validation"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: GET
      expect:
        status: 200
        headers:
          content-type: "application/json; charset=utf-8"
          cache-control: "*"
        body:
          # Exact value matching
          id: 1
          name: "Leanne Graham"
          
          # Pattern matching
          email: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"
          phone: "^[\\d\\-\\.\\s\\(\\)x]+$"
          
          # Nested object validation
          address:
            street: "*"
            suite: "*"
            city: "Gwenborough"
            zipcode: "^\\d{5}-\\d{4}$"
            geo:
              lat: "^-?\\d+\\.\\d+$"
              lng: "^-?\\d+\\.\\d+$"
              
          # Company object
          company:
            name: "*"
            catchPhrase: "*"
            bs: "*"
            
        # Custom validations
        custom:
          - name: "User ID is positive"
            expression: "response.body.id > 0"
          - name: "Name is not empty"
            expression: "response.body.name.length > 0"`;

export const loadTestingExample = `global:
  execution: parallel
  continueOnError: true
  
  variables:
    BASE_URL: "https://httpbin.org"
    CONCURRENT_USERS: "10"

collection:
  name: "Load Testing Example"
  requests:
    # Generate multiple similar requests for load testing
    - name: "Load Test Request 1"
      url: "\${BASE_URL}/delay/1"
      method: GET
      timeout: 5000
      
    - name: "Load Test Request 2"
      url: "\${BASE_URL}/delay/1"
      method: GET
      timeout: 5000
      
    - name: "Load Test Request 3"
      url: "\${BASE_URL}/delay/1" 
      method: GET
      timeout: 5000
      
    - name: "Load Test Request 4"
      url: "\${BASE_URL}/delay/1"
      method: GET
      timeout: 5000
      
    - name: "Load Test Request 5"
      url: "\${BASE_URL}/delay/1"
      method: GET
      timeout: 5000
      
    - name: "Performance monitoring"
      url: "\${BASE_URL}/status/200"
      method: GET
      expect:
        responseTime: "<2000"  # Must respond within 2 seconds`;

export const environmentSwitchingExample = `# Production environment
global:
  variables:
    ENVIRONMENT: "production"
    BASE_URL: "https://api.production.com"
    API_KEY: "\${PROD_API_KEY}"
    TIMEOUT: "30000"
    
collection:
  name: "Environment Switching Example"
  requests:
    - name: "Health check"
      url: "\${BASE_URL}/health"
      method: GET
      headers:
        Authorization: "Bearer \${API_KEY}"
        X-Environment: "\${ENVIRONMENT}"
      timeout: \${TIMEOUT}
      expect:
        status: 200
        body:
          status: "healthy"
          environment: "\${ENVIRONMENT}"

---

# Development environment  
global:
  variables:
    ENVIRONMENT: "development"
    BASE_URL: "http://localhost:3000"
    API_KEY: "dev-key-123"
    TIMEOUT: "5000"
    
collection:
  name: "Environment Switching Example"
  requests:
    - name: "Health check"
      url: "\${BASE_URL}/health"
      method: GET
      headers:
        Authorization: "Bearer \${API_KEY}"
        X-Environment: "\${ENVIRONMENT}"
      timeout: \${TIMEOUT}
      expect:
        status: 200
        body:
          status: "healthy"
          environment: "\${ENVIRONMENT}"`;
