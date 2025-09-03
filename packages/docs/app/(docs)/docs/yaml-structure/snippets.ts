// YAML structure page snippets
export const basicStructureExample = `# Top-level structure
global:
  # Global configuration options
  
collection:
  # Collection definition
  
# Or multiple collections
collections:
  - name: "Collection 1"
    # Collection 1 definition
  - name: "Collection 2" 
    # Collection 2 definition`;

export const globalSectionExample = `global:
  # Execution behavior
  execution: sequential  # or 'parallel'
  continueOnError: false
  
  # Global variables
  variables:
    BASE_URL: "https://api.example.com"
    API_VERSION: "v1"
    
  # Default request settings
  defaults:
    timeout: 5000
    headers:
      User-Agent: "curl-runner/1.0"
      
  # Output configuration
  output:
    verbose: true
    format: pretty`;

export const collectionSectionExample = `collection:
  name: "My API Tests"
  description: "Testing the user management API"
  
  # Collection-level variables
  variables:
    COLLECTION_ID: "user_tests"
    
  # Collection-level defaults
  defaults:
    headers:
      X-Collection: "user-tests"
      
  # List of requests
  requests:
    - name: "Get all users"
      url: "\${BASE_URL}/users"
      method: GET
      
    - name: "Create user"
      url: "\${BASE_URL}/users"
      method: POST`;

export const requestStructureExample = `requests:
  - name: "Complete request example"
    description: "Shows all available request options"
    url: "\${BASE_URL}/users"
    method: POST
    
    # Request-specific variables
    variables:
      USER_NAME: "testuser"
      
    # Headers
    headers:
      Content-Type: "application/json"
      Authorization: "Bearer \${API_TOKEN}"
      
    # Query parameters
    params:
      limit: 10
      offset: 0
      
    # Request body
    body: |
      {
        "name": "\${USER_NAME}",
        "email": "test@example.com"
      }
      
    # Timeout for this request
    timeout: 10000
    
    # Expected response
    expect:
      status: 201
      headers:
        content-type: "application/json"
      body:
        name: "\${USER_NAME}"
        id: "*"`;

export const multipleCollectionsExample = `# Configuration with multiple collections
global:
  variables:
    BASE_URL: "https://api.example.com"
    
collections:
  - name: "User Tests"
    description: "Test user management endpoints"
    requests:
      - name: "List users"
        url: "\${BASE_URL}/users"
        method: GET
        
      - name: "Create user"
        url: "\${BASE_URL}/users"
        method: POST
        
  - name: "Post Tests"
    description: "Test blog post endpoints"
    requests:
      - name: "List posts"
        url: "\${BASE_URL}/posts"
        method: GET
        
      - name: "Create post"
        url: "\${BASE_URL}/posts"
        method: POST`;

export const variablesExample = `# Variable usage at different levels
global:
  variables:
    # Global variables available everywhere
    BASE_URL: "https://api.example.com"
    API_VERSION: "v2"
    GLOBAL_TIMEOUT: "5000"
    
collection:
  variables:
    # Collection variables override global
    COLLECTION_ID: "user_collection"
    
  requests:
    - name: "Request with variables"
      variables:
        # Request variables override collection and global
        LOCAL_VAR: "request_specific"
        
      url: "\${BASE_URL}/\${API_VERSION}/users"
      timeout: \${GLOBAL_TIMEOUT}
      headers:
        X-Collection-ID: "\${COLLECTION_ID}"
        X-Request-Context: "\${LOCAL_VAR}"`;

export const expectationsExample = `requests:
  - name: "Response validation examples"
    url: "\${BASE_URL}/users/1"
    method: GET
    expect:
      # Status code validation
      status: 200
      # Multiple acceptable status codes
      # status: [200, 201, 202]
      
      # Header validation
      headers:
        content-type: "application/json"
        x-api-version: "v2"
        
      # Body validation
      body:
        # Exact matches
        id: 1
        active: true
        
        # Wildcard matches (any value)
        name: "*"
        email: "*"
        
        # Regex patterns
        phone: "^\\\\+?[1-9]\\\\d{1,14}$"
        
        # Nested object validation
        address:
          city: "*"
          country: "US"
          
        # Array validation
        tags: ["user", "active"]`;

export const advancedYamlExample = `# Advanced YAML configuration example
global:
  execution: sequential
  continueOnError: false
  
  variables:
    BASE_URL: "https://api.production.com"
    API_KEY: "\${API_KEY}"  # From environment
    TIMEOUT_MS: "30000"
    
  defaults:
    timeout: \${TIMEOUT_MS}
    headers:
      User-Agent: "curl-runner-ci/1.0"
      Authorization: "Bearer \${API_KEY}"
      Accept: "application/json"
    retry:
      count: 3
      delay: 1000
      
  output:
    verbose: true
    format: json
    saveToFile: "test-results.json"
    
collections:
  - name: "Health Checks"
    description: "System health monitoring"
    
    defaults:
      expect:
        status: 200
        headers:
          content-type: "application/json"
          
    requests:
      - name: "API Health"
        url: "\${BASE_URL}/health"
        method: GET
        expect:
          body:
            status: "healthy"
            
      - name: "Database Health"
        url: "\${BASE_URL}/health/db"
        method: GET
        timeout: 60000  # Override default timeout
        
  - name: "User Management"
    description: "User CRUD operations"
    
    variables:
      USER_ENDPOINT: "/api/v1/users"
      
    requests:
      - name: "Create Test User"
        url: "\${BASE_URL}\${USER_ENDPOINT}"
        method: POST
        headers:
          Content-Type: "application/json"
        body: |
          {
            "name": "Test User",
            "email": "test@example.com",
            "role": "user"
          }
        expect:
          status: 201
          body:
            name: "Test User"
            email: "test@example.com"
            id: "*"
            
      - name: "Get Created User"
        url: "\${BASE_URL}\${USER_ENDPOINT}/\${CREATED_USER_ID}"
        method: GET
        expect:
          body:
            name: "Test User"
            active: true`;
