// Parallel execution page snippets
export const basicParallelExample = `global:
  execution: parallel  # Run all requests simultaneously

collection:
  name: "Parallel Execution Example"
  requests:
    - name: "Request 1"
      url: "https://httpbin.org/delay/2"
      method: GET
      
    - name: "Request 2"
      url: "https://httpbin.org/delay/2"
      method: GET
      
    - name: "Request 3"
      url: "https://httpbin.org/delay/2"
      method: GET
      
# Total time: ~2 seconds instead of ~6 seconds`;

export const sequentialVsParallelExample = `# Sequential execution (default)
global:
  execution: sequential

collection:
  requests:
    - name: "First request"
      url: "https://httpbin.org/delay/1"
      # Waits 1 second
      
    - name: "Second request"
      url: "https://httpbin.org/delay/1"
      # Waits another 1 second (total: 2 seconds)
      
    - name: "Third request"
      url: "https://httpbin.org/delay/1"
      # Waits another 1 second (total: 3 seconds)

---

# Parallel execution
global:
  execution: parallel

collection:
  requests:
    - name: "First request"
      url: "https://httpbin.org/delay/1"
      # Starts immediately
      
    - name: "Second request"
      url: "https://httpbin.org/delay/1"
      # Starts immediately
      
    - name: "Third request"
      url: "https://httpbin.org/delay/1"
      # Starts immediately (total: ~1 second)`;

export const loadTestingExample = `global:
  execution: parallel
  continueOnError: true

collection:
  name: "Load Testing Example"
  requests:
    # Simulate 10 concurrent users
    - name: "User 1 - Get homepage"
      url: "https://httpbin.org/status/200"
      method: GET
      
    - name: "User 2 - Get homepage"
      url: "https://httpbin.org/status/200"
      method: GET
      
    - name: "User 3 - Get homepage"
      url: "https://httpbin.org/status/200"
      method: GET
      
    - name: "User 4 - Search products"
      url: "https://httpbin.org/status/200"
      method: GET
      params:
        q: "laptop"
        
    - name: "User 5 - View product"
      url: "https://httpbin.org/status/200"
      method: GET
      
    - name: "User 6 - Add to cart"
      url: "https://httpbin.org/status/200"
      method: POST
      
    - name: "User 7 - Login"
      url: "https://httpbin.org/status/200"
      method: POST
      
    - name: "User 8 - Checkout"
      url: "https://httpbin.org/status/200"
      method: POST
      
    - name: "User 9 - View orders"
      url: "https://httpbin.org/status/200"
      method: GET
      
    - name: "User 10 - Logout"
      url: "https://httpbin.org/status/200"
      method: POST`;

export const healthCheckExample = `global:
  execution: parallel

collection:
  name: "System Health Checks"
  requests:
    - name: "API Server Health"
      url: "https://api.example.com/health"
      method: GET
      expect:
        status: 200
        body:
          status: "healthy"
          
    - name: "Database Health"
      url: "https://api.example.com/health/database"
      method: GET
      expect:
        status: 200
        body:
          database: "connected"
          
    - name: "Cache Health"
      url: "https://api.example.com/health/cache"
      method: GET
      expect:
        status: 200
        
    - name: "External Service Health"
      url: "https://api.example.com/health/external"
      method: GET
      expect:
        status: 200
        
    - name: "File System Health"
      url: "https://api.example.com/health/storage"
      method: GET
      expect:
        status: 200

# All health checks run simultaneously for faster monitoring`;

export const performanceTestingExample = `global:
  execution: parallel
  continueOnError: true

collection:
  name: "Performance Testing"
  requests:
    # Test different endpoints under load
    - name: "Load test endpoint 1"
      url: "https://api.example.com/heavy-operation"
      method: GET
      timeout: 30000
      expect:
        responseTime: "<5000"  # Must respond within 5 seconds
        
    - name: "Load test endpoint 2"
      url: "https://api.example.com/heavy-operation"
      method: GET
      timeout: 30000
      expect:
        responseTime: "<5000"
        
    - name: "Load test endpoint 3"
      url: "https://api.example.com/heavy-operation"
      method: GET
      timeout: 30000
      expect:
        responseTime: "<5000"
        
    - name: "Stress test database"
      url: "https://api.example.com/database-intensive"
      method: GET
      timeout: 45000
      expect:
        status: 200
        responseTime: "<10000"
        
    - name: "Test file upload"
      url: "https://api.example.com/upload"
      method: POST
      timeout: 60000
      expect:
        status: 200
        responseTime: "<15000"`;

export const parallelWithErrorHandlingExample = `global:
  execution: parallel
  continueOnError: true  # Continue even if some requests fail

collection:
  name: "Resilient Parallel Testing"
  requests:
    - name: "Stable endpoint"
      url: "https://httpbin.org/status/200"
      method: GET
      expect:
        status: 200
        
    - name: "Unstable endpoint"
      url: "https://httpbin.org/status/500"
      method: GET
      expect:
        status: [200, 500]  # Accept either success or failure
        
    - name: "Flaky endpoint"
      url: "https://httpbin.org/unstable"
      method: GET
      retry:
        count: 3
        delay: 1000
      expect:
        status: [200, 500, 503]
        
    - name: "Timeout test"
      url: "https://httpbin.org/delay/10"
      method: GET
      timeout: 5000
      expect:
        status: [200, 408]  # Accept success or timeout`;

export const parallelMultipleServicesExample = `global:
  execution: parallel

collection:
  name: "Multi-Service Integration Test"
  requests:
    # Test multiple microservices simultaneously
    - name: "User Service Health"
      url: "https://users.api.example.com/health"
      method: GET
      
    - name: "Product Service Health"
      url: "https://products.api.example.com/health"
      method: GET
      
    - name: "Order Service Health"
      url: "https://orders.api.example.com/health"
      method: GET
      
    - name: "Payment Service Health"
      url: "https://payments.api.example.com/health"
      method: GET
      
    - name: "Notification Service Health"
      url: "https://notifications.api.example.com/health"
      method: GET
      
    - name: "Analytics Service Health"
      url: "https://analytics.api.example.com/health"
      method: GET

# All services checked in parallel for comprehensive system status`;

export const concurrencyControlExample = `# Using CLI options to control concurrency
# curl-runner --parallel --max-concurrent 5 tests.yaml

global:
  execution: parallel

collection:
  name: "Controlled Concurrency"
  requests:
    # Even with 20 requests, only 5 will run concurrently
    - name: "Request 1"
      url: "https://httpbin.org/delay/1"
    - name: "Request 2"
      url: "https://httpbin.org/delay/1"
    - name: "Request 3"
      url: "https://httpbin.org/delay/1"
    - name: "Request 4"
      url: "https://httpbin.org/delay/1"
    - name: "Request 5"
      url: "https://httpbin.org/delay/1"
    - name: "Request 6"
      url: "https://httpbin.org/delay/1"
    - name: "Request 7"
      url: "https://httpbin.org/delay/1"
    - name: "Request 8"
      url: "https://httpbin.org/delay/1"
    - name: "Request 9"
      url: "https://httpbin.org/delay/1"
    - name: "Request 10"
      url: "https://httpbin.org/delay/1"
    # ... up to 20 requests`;

export const mixedExecutionExample = `# Collections can have different execution modes
global:
  execution: sequential  # Default for all collections

collections:
  - name: "Setup Tasks"
    # Uses global setting: sequential
    requests:
      - name: "Initialize database"
        url: "https://api.example.com/setup/db"
        method: POST
        
      - name: "Seed test data"
        url: "https://api.example.com/setup/seed"
        method: POST
        
  - name: "Parallel Health Checks"
    execution: parallel  # Override global setting
    requests:
      - name: "Check API"
        url: "https://api.example.com/health"
        method: GET
        
      - name: "Check Database"
        url: "https://api.example.com/health/db"
        method: GET
        
      - name: "Check Cache"
        url: "https://api.example.com/health/cache"
        method: GET
        
  - name: "Cleanup Tasks"
    # Uses global setting: sequential
    requests:
      - name: "Clear test data"
        url: "https://api.example.com/cleanup"
        method: DELETE`;
