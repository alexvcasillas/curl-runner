// Variables page snippets
export const basicVariablesExample = `global:
  variables:
    BASE_URL: "https://api.example.com"
    API_VERSION: "v1"

collection:
  name: "API Tests"
  requests:
    - name: "Get users"
      url: "\${BASE_URL}/\${API_VERSION}/users"
      method: GET`;

export const environmentVariablesExample = `# Set environment variables
export API_KEY="your-secret-key"
export BASE_URL="https://api.staging.com"

# Reference in YAML
global:
  variables:
    API_TOKEN: "\${API_KEY}"     # From environment
    API_BASE: "\${BASE_URL}"     # From environment
    TIMEOUT: "5000"              # Static value`;

export const variablePrecedenceExample = `global:
  variables:
    TIMEOUT: "5000"      # Global level
    ENV: "production"

collection:
  variables:
    TIMEOUT: "10000"     # Overrides global
    COLLECTION_ID: "api_tests"
    
  requests:
    - name: "Test with overrides"
      variables:
        TIMEOUT: "30000"  # Overrides collection and global
        REQUEST_ID: "req_001"
      url: "\${BASE_URL}/test"
      timeout: \${TIMEOUT}  # Uses 30000`;

export const dynamicVariablesExample = `global:
  variables:
    # Dynamic timestamp
    TIMESTAMP: "\${DATE:YYYY-MM-DD}"
    REQUEST_TIME: "\${TIME:HH:mm:ss}"
    
    # UUID generation
    REQUEST_ID: "\${UUID}"
    SESSION_ID: "\${UUID:short}"
    
    # Random values
    RANDOM_NUM: "\${RANDOM:1-1000}"
    RANDOM_STR: "\${RANDOM:string:10}"

collection:
  requests:
    - name: "Request with dynamic values"
      url: "https://api.example.com/requests"
      headers:
        X-Request-ID: "\${REQUEST_ID}"
        X-Timestamp: "\${TIMESTAMP}"
        X-Session: "\${SESSION_ID}"`;

export const conditionalVariablesExample = `global:
  variables:
    # Environment-based variables
    BASE_URL: "\${NODE_ENV:production:https://api.example.com:https://api-staging.example.com}"
    
    # Default value if environment variable not set
    API_TIMEOUT: "\${API_TIMEOUT:5000}"
    
    # Multiple environment sources
    DB_HOST: "\${DATABASE_HOST:\${DB_HOST:localhost}}"

collection:
  requests:
    - name: "Environment aware request"
      url: "\${BASE_URL}/data"
      timeout: \${API_TIMEOUT}`;

export const stringTransformsExample = `global:
  variables:
    ENV: "production"
    RESOURCE: "Users"

    # Transform to uppercase
    UPPER_ENV: "\${ENV:upper}"           # Results in "PRODUCTION"

    # Transform to lowercase
    LOWER_RESOURCE: "\${RESOURCE:lower}" # Results in "users"

collection:
  requests:
    - name: "Request with case transforms"
      url: "https://api.example.com/\${RESOURCE:lower}"
      headers:
        X-Environment: "\${ENV:upper}"
        X-Resource-Type: "\${RESOURCE:lower}"`;

export const computedVariablesExample = `global:
  variables:
    BASE_PATH: "/api/v1"
    RESOURCE: "users"

    # Computed from other variables
    FULL_ENDPOINT: "\${BASE_URL}\${BASE_PATH}/\${RESOURCE}"

    # String manipulation
    UPPER_ENV: "\${ENV:upper}"
    LOWER_RESOURCE: "\${RESOURCE:lower}"

collection:
  requests:
    - name: "Using computed variables"
      url: "\${FULL_ENDPOINT}"
      headers:
        X-Environment: "\${UPPER_ENV}"
        X-Resource-Type: "\${LOWER_RESOURCE}"`;

export const responseVariablesExample = `collection:
  requests:
    - name: "Login and extract token"
      url: "https://api.example.com/login"
      method: POST
      body: |
        {
          "username": "testuser",
          "password": "testpass"
        }
      extract:
        # Extract values from response
        AUTH_TOKEN: "$.token"           # JSON path
        USER_ID: "$.user.id"
        EXPIRES_AT: "$.expiresAt"
        
    - name: "Use extracted token"
      url: "https://api.example.com/protected"
      headers:
        Authorization: "Bearer \${AUTH_TOKEN}"
      expect:
        body:
          userId: \${USER_ID}`;

export const fileVariablesExample = `# Load variables from external files

# variables.json
{
  "BASE_URL": "https://api.example.com",
  "API_VERSION": "v2",
  "ENDPOINTS": {
    "users": "/users",
    "posts": "/posts"
  }
}

# In YAML configuration
global:
  variables:
    # Load from file
    "@file": "variables.json"
    
    # Override specific values
    API_VERSION: "v3"
    
collection:
  requests:
    - name: "Using file variables"
      url: "\${BASE_URL}/\${API_VERSION}\${ENDPOINTS.users}"`;

export const secretsExample = `# .env file (not committed to git)
API_SECRET=your-secret-key
DATABASE_PASSWORD=secret-password
JWT_SECRET=jwt-signing-key

# YAML configuration
global:
  variables:
    # Load secrets from environment
    API_KEY: "\${API_SECRET}"
    DB_PASS: "\${DATABASE_PASSWORD}"
    JWT_KEY: "\${JWT_SECRET}"
    
    # Public configuration
    API_BASE: "https://api.example.com"
    TIMEOUT: "5000"

collection:
  requests:
    - name: "Authenticated request"
      url: "\${API_BASE}/secure"
      headers:
        Authorization: "Bearer \${API_KEY}"
        X-JWT-Token: "\${JWT_KEY}"`;

export const templateVariablesExample = `global:
  variables:
    # Template strings
    API_ENDPOINT: "https://api.example.com/v1"
    REQUEST_TEMPLATE: |
      {
        "timestamp": "\${TIMESTAMP}",
        "requestId": "\${REQUEST_ID}",
        "environment": "\${NODE_ENV}"
      }
      
collection:
  requests:
    - name: "Templated request"
      url: "\${API_ENDPOINT}/requests"
      method: POST
      headers:
        Content-Type: "application/json"
      body: "\${REQUEST_TEMPLATE}"`;

export const complexVariablesExample = `# Complex variable configuration example
global:
  variables:
    # Environment setup
    NODE_ENV: "\${NODE_ENV:development}"
    
    # Dynamic base configuration
    BASE_URL: "\${NODE_ENV:production:https://api.example.com:http://localhost:3000}"
    
    # API configuration
    API_VERSION: "v2"
    CLIENT_VERSION: "1.2.3"
    
    # Authentication
    API_KEY: "\${API_KEY}"
    CLIENT_ID: "\${CLIENT_ID:default-client}"
    
    # Timing and limits
    DEFAULT_TIMEOUT: "5000"
    MAX_RETRIES: "3"
    RATE_LIMIT: "100"
    
    # Dynamic values
    TIMESTAMP: "\${DATE:YYYY-MM-DDTHH:mm:ss}Z"
    REQUEST_ID: "req_\${UUID:short}"
    TRACE_ID: "trace_\${RANDOM:hex:16}"
    
    # Computed endpoints
    AUTH_ENDPOINT: "\${BASE_URL}/\${API_VERSION}/auth"
    USERS_ENDPOINT: "\${BASE_URL}/\${API_VERSION}/users"
    HEALTH_ENDPOINT: "\${BASE_URL}/health"

collection:
  name: "Production API Tests"
  
  variables:
    # Collection-specific overrides
    COLLECTION_ID: "prod_api_tests"
    TEST_SUITE: "comprehensive"
    
  requests:
    - name: "Health Check"
      url: "\${HEALTH_ENDPOINT}"
      method: GET
      headers:
        User-Agent: "curl-runner/\${CLIENT_VERSION}"
        X-Request-ID: "\${REQUEST_ID}"
        X-Trace-ID: "\${TRACE_ID}"
      timeout: \${DEFAULT_TIMEOUT}
      
    - name: "Authenticate"
      url: "\${AUTH_ENDPOINT}/token"
      method: POST
      headers:
        Content-Type: "application/json"
        X-Client-ID: "\${CLIENT_ID}"
      body: |
        {
          "apiKey": "\${API_KEY}",
          "timestamp": "\${TIMESTAMP}",
          "environment": "\${NODE_ENV}"
        }
      extract:
        ACCESS_TOKEN: "$.accessToken"
        TOKEN_EXPIRES: "$.expiresIn"
        
    - name: "Get User Profile"
      url: "\${USERS_ENDPOINT}/profile"
      method: GET
      headers:
        Authorization: "Bearer \${ACCESS_TOKEN}"
        User-Agent: "curl-runner/\${CLIENT_VERSION}"
        X-Environment: "\${NODE_ENV}"
        X-Collection: "\${COLLECTION_ID}"
      timeout: \${DEFAULT_TIMEOUT}
      retry:
        count: \${MAX_RETRIES}
        delay: 1000`;
