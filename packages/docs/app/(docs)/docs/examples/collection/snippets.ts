// Collection examples page snippets
export const singleCollectionExample = `collection:
  name: "User Management API"
  description: "Complete CRUD operations for user management"
  
  variables:
    USER_ID: "123"
    COLLECTION_VERSION: "v1"
    
  defaults:
    headers:
      Accept: "application/json"
      X-Collection: "user-management"
      
  requests:
    - name: "List all users"
      url: "https://api.example.com/users"
      method: GET
      
    - name: "Get specific user"
      url: "https://api.example.com/users/\${USER_ID}"
      method: GET
      
    - name: "Create new user"
      url: "https://api.example.com/users"
      method: POST
      
    - name: "Update user"
      url: "https://api.example.com/users/\${USER_ID}"
      method: PUT
      
    - name: "Delete user"
      url: "https://api.example.com/users/\${USER_ID}"
      method: DELETE`;

export const multipleCollectionsExample = `global:
  variables:
    BASE_URL: "https://api.example.com"
    API_VERSION: "v1"
    
collections:
  - name: "User Management"
    description: "User CRUD operations"
    
    variables:
      RESOURCE: "users"
      
    defaults:
      headers:
        X-Resource-Type: "user"
        
    requests:
      - name: "List users"
        url: "\${BASE_URL}/\${API_VERSION}/\${RESOURCE}"
        method: GET
        
      - name: "Create user"
        url: "\${BASE_URL}/\${API_VERSION}/\${RESOURCE}"
        method: POST
        
  - name: "Post Management"
    description: "Blog post operations"
    
    variables:
      RESOURCE: "posts"
      
    defaults:
      headers:
        X-Resource-Type: "post"
        
    requests:
      - name: "List posts"
        url: "\${BASE_URL}/\${API_VERSION}/\${RESOURCE}"
        method: GET
        
      - name: "Create post"
        url: "\${BASE_URL}/\${API_VERSION}/\${RESOURCE}"
        method: POST`;

export const collectionWithSetupsExample = `collection:
  name: "E-commerce API Tests"
  description: "Complete e-commerce workflow testing"
  
  # Setup requests run before main requests
  setup:
    - name: "Authenticate"
      url: "https://api.shop.com/auth/login"
      method: POST
      body: |
        {
          "username": "testuser",
          "password": "testpass"
        }
      extract:
        AUTH_TOKEN: "$.token"
        
    - name: "Create test product"
      url: "https://api.shop.com/products"
      method: POST
      headers:
        Authorization: "Bearer \${AUTH_TOKEN}"
      body: |
        {
          "name": "Test Product",
          "price": 29.99
        }
      extract:
        PRODUCT_ID: "$.id"
  
  requests:
    - name: "Add to cart"
      url: "https://api.shop.com/cart/items"
      method: POST
      headers:
        Authorization: "Bearer \${AUTH_TOKEN}"
      body: |
        {
          "productId": \${PRODUCT_ID},
          "quantity": 2
        }
        
    - name: "Checkout"
      url: "https://api.shop.com/checkout"
      method: POST
      headers:
        Authorization: "Bearer \${AUTH_TOKEN}"
        
  # Cleanup requests run after main requests
  teardown:
    - name: "Clear cart"
      url: "https://api.shop.com/cart/clear"
      method: DELETE
      headers:
        Authorization: "Bearer \${AUTH_TOKEN}"
        
    - name: "Delete test product"
      url: "https://api.shop.com/products/\${PRODUCT_ID}"
      method: DELETE
      headers:
        Authorization: "Bearer \${AUTH_TOKEN}"`;

export const conditionalCollectionExample = `global:
  variables:
    ENVIRONMENT: "\${NODE_ENV:development}"
    
collections:
  - name: "Development Tests"
    condition: "\${ENVIRONMENT} == 'development'"
    
    variables:
      BASE_URL: "http://localhost:3000"
      
    requests:
      - name: "Dev health check"
        url: "\${BASE_URL}/health"
        method: GET
        
      - name: "Dev debug endpoint"
        url: "\${BASE_URL}/debug"
        method: GET
        
  - name: "Production Tests"
    condition: "\${ENVIRONMENT} == 'production'"
    
    variables:
      BASE_URL: "https://api.production.com"
      
    requests:
      - name: "Prod health check"
        url: "\${BASE_URL}/health"
        method: GET
        expect:
          status: 200
          body:
            status: "healthy"`;

export const parallelCollectionExample = `global:
  execution: parallel
  
collections:
  - name: "Health Checks"
    description: "Independent health monitoring"
    
    requests:
      - name: "API Health"
        url: "https://api.example.com/health"
        method: GET
        
      - name: "Database Health"
        url: "https://api.example.com/health/db"
        method: GET
        
      - name: "Cache Health"
        url: "https://api.example.com/health/cache"
        method: GET
        
  - name: "External Services"
    description: "Third-party service checks"
    
    requests:
      - name: "Payment Gateway"
        url: "https://payments.example.com/health"
        method: GET
        
      - name: "Email Service"
        url: "https://email.example.com/health"
        method: GET
        
      - name: "File Storage"
        url: "https://storage.example.com/health"
        method: GET`;

export const sequentialCollectionExample = `global:
  execution: sequential
  continueOnError: false
  
collections:
  - name: "Environment Setup"
    description: "Prepare test environment"
    
    requests:
      - name: "Clear test data"
        url: "https://api.example.com/test/clear"
        method: DELETE
        
      - name: "Seed initial data"
        url: "https://api.example.com/test/seed"
        method: POST
        
  - name: "Core API Tests"
    description: "Main functionality tests"
    
    requests:
      - name: "User registration"
        url: "https://api.example.com/users/register"
        method: POST
        
      - name: "User login"
        url: "https://api.example.com/users/login"
        method: POST
        
      - name: "Create content"
        url: "https://api.example.com/content"
        method: POST
        
  - name: "Cleanup"
    description: "Clean up test environment"
    
    requests:
      - name: "Remove test data"
        url: "https://api.example.com/test/cleanup"
        method: DELETE`;

export const collectionInheritanceExample = `global:
  variables:
    BASE_URL: "https://api.example.com"
    
  defaults:
    timeout: 5000
    headers:
      User-Agent: "curl-runner/1.0"
      Accept: "application/json"
      
collections:
  - name: "API v1 Tests"
    description: "Tests for API version 1"
    
    variables:
      API_VERSION: "v1"
      
    defaults:
      headers:
        X-API-Version: "\${API_VERSION}"
        # Inherits global defaults + adds version header
        
    requests:
      - name: "V1 Users endpoint"
        url: "\${BASE_URL}/\${API_VERSION}/users"
        method: GET
        # Inherits: timeout, User-Agent, Accept, X-API-Version
        
  - name: "API v2 Tests"
    description: "Tests for API version 2"
    
    variables:
      API_VERSION: "v2"
      
    defaults:
      timeout: 10000  # Override global timeout
      headers:
        X-API-Version: "\${API_VERSION}"
        Content-Type: "application/json"
        # Inherits: User-Agent, Accept + overrides timeout
        
    requests:
      - name: "V2 Users endpoint"
        url: "\${BASE_URL}/\${API_VERSION}/users"
        method: GET
        # Uses: 10000ms timeout, all inherited headers`;

export const dynamicCollectionExample = `global:
  variables:
    ENDPOINTS: '["users", "posts", "comments"]'
    BASE_URL: "https://jsonplaceholder.typicode.com"
    
collection:
  name: "Dynamic Endpoint Testing"
  description: "Test multiple endpoints dynamically"
  
  requests:
    # Generate requests for each endpoint
    - name: "Test users endpoint"
      url: "\${BASE_URL}/users"
      method: GET
      variables:
        CURRENT_ENDPOINT: "users"
      expect:
        status: 200
        
    - name: "Test posts endpoint"
      url: "\${BASE_URL}/posts"
      method: GET
      variables:
        CURRENT_ENDPOINT: "posts"
      expect:
        status: 200
        
    - name: "Test comments endpoint"
      url: "\${BASE_URL}/comments"
      method: GET
      variables:
        CURRENT_ENDPOINT: "comments"
      expect:
        status: 200`;

export const complexCollectionExample = `# Complex multi-collection configuration
global:
  execution: sequential
  continueOnError: false
  
  variables:
    BASE_URL: "https://api.complex-app.com"
    TEST_ENV: "\${TEST_ENVIRONMENT:staging}"
    
  defaults:
    timeout: 15000
    retry:
      count: 2
      delay: 1000
      
collections:
  - name: "Authentication & Authorization"
    description: "User authentication and role-based access"
    
    variables:
      AUTH_ENDPOINT: "/auth"
      
    setup:
      - name: "Clear auth cache"
        url: "\${BASE_URL}\${AUTH_ENDPOINT}/cache/clear"
        method: DELETE
        
    requests:
      - name: "User login"
        url: "\${BASE_URL}\${AUTH_ENDPOINT}/login"
        method: POST
        body: |
          {
            "email": "test@example.com",
            "password": "testpass123"
          }
        extract:
          ACCESS_TOKEN: "$.accessToken"
          REFRESH_TOKEN: "$.refreshToken"
          USER_ID: "$.user.id"
          
      - name: "Verify token"
        url: "\${BASE_URL}\${AUTH_ENDPOINT}/verify"
        method: GET
        headers:
          Authorization: "Bearer \${ACCESS_TOKEN}"
          
      - name: "Access protected resource"
        url: "\${BASE_URL}/protected/profile"
        method: GET
        headers:
          Authorization: "Bearer \${ACCESS_TOKEN}"
          
  - name: "User Management"
    description: "CRUD operations for user data"
    
    variables:
      USERS_ENDPOINT: "/users"
      
    defaults:
      headers:
        Authorization: "Bearer \${ACCESS_TOKEN}"
        
    requests:
      - name: "Get user profile"
        url: "\${BASE_URL}\${USERS_ENDPOINT}/\${USER_ID}"
        method: GET
        
      - name: "Update user profile"
        url: "\${BASE_URL}\${USERS_ENDPOINT}/\${USER_ID}"
        method: PUT
        body: |
          {
            "firstName": "Updated",
            "lastName": "Name"
          }
          
      - name: "Get updated profile"
        url: "\${BASE_URL}\${USERS_ENDPOINT}/\${USER_ID}"
        method: GET
        expect:
          body:
            firstName: "Updated"
            lastName: "Name"
            
  - name: "Data Operations"
    description: "Business data management"
    
    variables:
      DATA_ENDPOINT: "/data"
      
    defaults:
      headers:
        Authorization: "Bearer \${ACCESS_TOKEN}"
        Content-Type: "application/json"
        
    requests:
      - name: "Create data record"
        url: "\${BASE_URL}\${DATA_ENDPOINT}/records"
        method: POST
        body: |
          {
            "title": "Test Record",
            "category": "testing",
            "userId": \${USER_ID}
          }
        extract:
          RECORD_ID: "$.id"
          
      - name: "Retrieve data record"
        url: "\${BASE_URL}\${DATA_ENDPOINT}/records/\${RECORD_ID}"
        method: GET
        
      - name: "Update data record"
        url: "\${BASE_URL}\${DATA_ENDPOINT}/records/\${RECORD_ID}"
        method: PATCH
        body: |
          {
            "status": "processed"
          }
          
      - name: "List user records"
        url: "\${BASE_URL}\${DATA_ENDPOINT}/records"
        method: GET
        params:
          userId: \${USER_ID}
          limit: 10
          
    teardown:
      - name: "Cleanup test record"
        url: "\${BASE_URL}\${DATA_ENDPOINT}/records/\${RECORD_ID}"
        method: DELETE
        headers:
          Authorization: "Bearer \${ACCESS_TOKEN}"`;
