// Validation rules API reference snippets
export const statusValidationExample = `requests:
  - name: "Exact status code"
    url: "https://api.example.com/users"
    method: GET
    expect:
      status: 200           # Must be exactly 200
      
  - name: "Multiple acceptable status codes"
    url: "https://api.example.com/users/999"
    method: GET
    expect:
      status: [404, 410]    # Either 404 or 410 is acceptable
      
  - name: "Status code range"
    url: "https://api.example.com/upload"
    method: POST
    expect:
      status: "2xx"         # Any 2xx status code (200-299)
      
  - name: "Status code patterns"
    url: "https://api.example.com/process"
    method: POST
    expect:
      status: ">= 200, < 300"  # Range specification`;

export const headerValidationExample = `requests:
  - name: "Header validation examples"
    url: "https://api.example.com/data"
    method: GET
    expect:
      headers:
        # Exact value matching
        content-type: "application/json; charset=utf-8"
        
        # Case-insensitive header names (automatically handled)
        Content-Type: "application/json; charset=utf-8"
        
        # Wildcard matching (any value)
        x-request-id: "*"
        
        # Regex pattern matching
        x-rate-limit-remaining: "^\\d+$"        # Numbers only
        etag: "^"[a-zA-Z0-9]+"$"             # Quoted string
        
        # Presence check (header must exist)
        cache-control: "*"
        
        # Multiple acceptable values
        accept-encoding: ["gzip", "deflate", "br"]
        
        # Pattern with specific format
        date: "^[A-Za-z]{3}, \\d{2} [A-Za-z]{3} \\d{4}"  # HTTP date format`;

export const bodyValidationExample = `requests:
  - name: "JSON body validation"
    url: "https://api.example.com/users/1"
    method: GET
    expect:
      body:
        # Exact value matching
        id: 1
        active: true
        role: "user"
        
        # Wildcard matching (any value)
        name: "*"
        email: "*"
        createdAt: "*"
        
        # Pattern matching with regex
        email: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"
        phone: "^\\+?[1-9]\\d{1,14}$"
        
        # Multiple acceptable values
        status: ["active", "pending", "suspended"]
        
        # Nested object validation
        profile:
          firstName: "*"
          lastName: "*"
          age: ">= 18, <= 120"
          
        # Address validation with patterns
        address:
          street: "*"
          city: "*"
          state: "^[A-Z]{2}$"           # Two-letter state code
          zipcode: "^\\d{5}(-\\d{4})?$"  # ZIP or ZIP+4 format
          country: ["US", "CA", "MX"]   # North America only`;

export const arrayValidationExample = `requests:
  - name: "Array validation"
    url: "https://api.example.com/users"
    method: GET
    expect:
      body:
        # Array length validation
        length: 10                    # Exactly 10 items
        # length: "> 5"               # More than 5 items
        # length: ">= 1, <= 100"      # Between 1 and 100 items
        
        # Validate first item
        [0]:
          id: 1
          name: "*"
          
        # Validate specific item by index
        [2]:
          id: 3
          active: true
          
        # Validate last item
        [-1]:
          id: "*"
          name: "*"
          
        # Validate all items must have certain fields
        "*":
          id: "*"                     # Every item must have id
          name: "*"                   # Every item must have name
          email: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"  # Valid email format
          
        # Validate subset of items
        "slice(0,3)":                 # First 3 items
          "*":
            active: true
            
  - name: "Nested array validation"
    url: "https://api.example.com/orders"
    method: GET
    expect:
      body:
        orders:
          "*":                        # Each order
            id: "*"
            items:
              length: "> 0"           # Must have at least one item
              "*":                    # Each item in the order
                productId: "*"
                quantity: "^[1-9]\\d*$"  # Positive integer
                price: "^\\d+\\.\\d{2}$"  # Currency format`;

export const numericValidationExample = `requests:
  - name: "Numeric validation patterns"
    url: "https://api.example.com/analytics"
    method: GET
    expect:
      body:
        # Exact numbers
        count: 100
        percentage: 85.5
        
        # Numeric ranges
        views: "> 1000"              # Greater than 1000
        clicks: ">= 50"              # Greater than or equal to 50
        conversion: "< 10.5"         # Less than 10.5
        bounce: "<= 25.0"            # Less than or equal to 25.0
        
        # Range specifications
        score: ">= 0, <= 100"        # Between 0 and 100 inclusive
        rating: "> 1, < 5"           # Between 1 and 5 exclusive
        
        # Multiple acceptable values
        status_code: [200, 201, 202]
        
        # Pattern matching for numbers
        revenue: "^\\d+\\.\\d{2}$"   # Currency format (dollars.cents)
        phone: "^\\d{10}$"           # Exactly 10 digits
        
        # Complex numeric validation
        metrics:
          daily_active_users: "> 1000"
          monthly_revenue: ">= 10000, <= 1000000"
          churn_rate: "< 5.0"`;

export const stringValidationExample = `requests:
  - name: "String validation patterns"
    url: "https://api.example.com/validation-demo"
    method: GET
    expect:
      body:
        # Exact string matching
        status: "success"
        environment: "production"
        
        # Case-sensitive matching
        code: "ABC123"
        
        # Wildcard (any string value)
        message: "*"
        timestamp: "*"
        
        # Pattern matching with regex
        email: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"
        uuid: "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
        url: "^https?:\\/\\/(www\\.)?[-a-zA-Z0-9@:%._\\+~#=]{1,256}\\.[a-zA-Z0-9()]{1,6}\\b([-a-zA-Z0-9()@:%_\\+.~#?&//=]*)$"
        
        # String length validation
        username: "^.{3,20}$"        # Between 3 and 20 characters
        password: "^.{8,}$"          # At least 8 characters
        
        # Multiple acceptable values
        type: ["user", "admin", "guest"]
        language: ["en", "es", "fr", "de"]
        
        # Complex patterns
        phone: "^\\+?[1-9]\\d{1,14}$"           # International phone format
        credit_card: "^\\d{4}-\\d{4}-\\d{4}-\\d{4}$"  # Credit card format
        ip_address: "^(?:[0-9]{1,3}\\.){3}[0-9]{1,3}$"  # IPv4 address
        
        # Date/time patterns
        iso_date: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}(\\.\\d{3})?Z?$"
        date_only: "^\\d{4}-\\d{2}-\\d{2}$"
        time_only: "^\\d{2}:\\d{2}:\\d{2}$"`;

export const booleanValidationExample = `requests:
  - name: "Boolean validation"
    url: "https://api.example.com/settings"
    method: GET
    expect:
      body:
        # Exact boolean values
        enabled: true
        active: false
        
        # Multiple acceptable boolean values
        visible: [true, false]        # Either true or false
        
        # String representations of booleans
        feature_flag: ["true", "false", "1", "0"]
        
        # Truthy/falsy validation (using custom expressions)
        settings:
          notifications: true
          dark_mode: false
          auto_save: true`;

export const nullValidationExample = `requests:
  - name: "Null and undefined validation"
    url: "https://api.example.com/optional-data"
    method: GET
    expect:
      body:
        # Explicit null values
        optional_field: null
        
        # Field can be null or have a value
        nullable_field: ["null", "*"]
        
        # Field must exist but can be null
        required_but_nullable: "*"   # Accepts null, string, number, etc.
        
        # Optional fields (may not exist)
        profile:
          optional_bio: "*"          # May be present or absent
          optional_avatar: "*"       # May be present or absent`;

export const customValidationExample = `requests:
  - name: "Custom validation expressions"
    url: "https://api.example.com/complex"
    method: GET
    expect:
      custom:
        # Simple comparisons
        - name: "User count is positive"
          expression: "response.body.userCount > 0"
          
        # Type checking
        - name: "Revenue is a number"
          expression: "typeof response.body.revenue === 'number'"
          
        # Array operations
        - name: "Has active users"
          expression: "response.body.users.some(user => user.active === true)"
          
        # String operations
        - name: "Email domain is valid"
          expression: "response.body.email.includes('@') && response.body.email.endsWith('.com')"
          
        # Date operations
        - name: "Created recently"
          expression: "Date.now() - new Date(response.body.createdAt).getTime() < 86400000"
          
        # Complex business logic
        - name: "Order total is correct"
          expression: |
            Math.abs(
              response.body.total - 
              response.body.items.reduce((sum, item) => sum + (item.price * item.quantity), 0)
            ) < 0.01
            
        # Conditional validation
        - name: "Premium users have extra features"
          expression: |
            response.body.plan !== 'premium' || 
            (response.body.features && response.body.features.includes('advanced_analytics'))
            
        # Cross-field validation
        - name: "End date after start date"
          expression: |
            new Date(response.body.endDate) > new Date(response.body.startDate)`;

export const responseTimeValidationExample = `requests:
  - name: "Response time validation"
    url: "https://api.example.com/performance"
    method: GET
    expect:
      # Simple response time limits
      responseTime: "< 1000"        # Must respond within 1 second
      
  - name: "Response time ranges"
    url: "https://api.example.com/moderate"
    method: GET
    expect:
      responseTime: "> 100, < 2000" # Between 100ms and 2 seconds
      
  - name: "Performance validation with custom logic"
    url: "https://api.example.com/heavy"
    method: GET
    expect:
      custom:
        - name: "Acceptable response time"
          expression: "response.time < 5000"
          
        - name: "Performance SLA met"
          expression: "response.time < 3000"
          
        - name: "Not too fast (caching check)"
          expression: "response.time > 50"`;

export const conditionalValidationExample = `requests:
  - name: "Conditional validation based on response"
    url: "https://api.example.com/user/profile"
    method: GET
    expect:
      body:
        id: "*"
        username: "*"
        type: ["free", "premium", "enterprise"]
        
      # Conditional validation using custom expressions
      custom:
        # If user is premium, validate premium features
        - name: "Premium users have subscription"
          expression: |
            response.body.type !== 'premium' || 
            (response.body.subscription && response.body.subscription.active === true)
            
        # If user is enterprise, validate additional fields
        - name: "Enterprise users have company info"
          expression: |
            response.body.type !== 'enterprise' || 
            (response.body.company && response.body.company.id)
            
        # Age-based validation
        - name: "Adult users can have credit cards"
          expression: |
            response.body.age < 18 || 
            !response.body.creditCards ||
            response.body.creditCards.length === 0
            
  - name: "Environment-based validation"
    url: "https://api.example.com/config"
    method: GET
    expect:
      body:
        environment: "\${NODE_ENV}"
        
      custom:
        # Different validation rules for different environments
        - name: "Production has HTTPS"
          expression: |
            response.body.environment !== 'production' ||
            response.body.apiUrl.startsWith('https://')
            
        - name: "Development allows debug"
          expression: |
            response.body.environment !== 'development' ||
            response.body.debug === true`;

export const advancedValidationExample = `requests:
  - name: "Comprehensive validation example"
    url: "https://api.example.com/orders/12345"
    method: GET
    expect:
      # Status validation
      status: 200
      
      # Header validation
      headers:
        content-type: "application/json"
        x-request-id: "^req_[a-zA-Z0-9]{16}$"
        cache-control: "max-age=300"
        
      # Response time validation
      responseTime: "< 2000"
      
      # Body structure validation
      body:
        # Order identification
        id: 12345
        orderNumber: "^ORD-\\d{8}$"
        status: ["pending", "processing", "shipped", "delivered", "cancelled"]
        
        # Timestamps
        createdAt: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}"
        updatedAt: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}"
        
        # Customer information
        customer:
          id: "*"
          email: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"
          name: "*"
          phone: "^\\+?[1-9]\\d{1,14}$"
          
        # Shipping address
        shipping:
          street: "*"
          city: "*"
          state: "^[A-Z]{2}$"
          zipcode: "^\\d{5}(-\\d{4})?$"
          country: ["US", "CA", "MX"]
          
        # Order items
        items:
          length: "> 0"
          "*":
            productId: "^prod_[a-zA-Z0-9]+$"
            name: "*"
            quantity: "^[1-9]\\d*$"
            price: "^\\d+\\.\\d{2}$"
            
        # Financial information
        totals:
          subtotal: "^\\d+\\.\\d{2}$"
          tax: "^\\d+\\.\\d{2}$"
          shipping: "^\\d+\\.\\d{2}$"
          discount: "^\\d+\\.\\d{2}$"
          total: "^\\d+\\.\\d{2}$"
          
      # Complex business logic validation
      custom:
        # Mathematical validation
        - name: "Order total calculation is correct"
          expression: |
            Math.abs(
              response.body.totals.total - 
              (response.body.totals.subtotal + response.body.totals.tax + 
               response.body.totals.shipping - response.body.totals.discount)
            ) < 0.01
            
        # Inventory validation
        - name: "All items have positive quantities"
          expression: |
            response.body.items.every(item => parseInt(item.quantity) > 0)
            
        # Status consistency
        - name: "Timestamps are logical"
          expression: |
            new Date(response.body.updatedAt) >= new Date(response.body.createdAt)
            
        # Business rules
        - name: "Shipped orders have tracking"
          expression: |
            response.body.status !== 'shipped' || 
            (response.body.tracking && response.body.tracking.number)
            
        # Security validation
        - name: "No sensitive data in response"
          expression: |
            !JSON.stringify(response.body).toLowerCase().includes('password') &&
            !JSON.stringify(response.body).toLowerCase().includes('secret') &&
            !JSON.stringify(response.body).toLowerCase().includes('key')
            
        # Data integrity
        - name: "Customer email matches order email"
          expression: |
            response.body.customer.email === response.body.billing.email ||
            !response.body.billing.email`;

export const failureTestingExample = `# Failure Testing Examples

# Test authentication failure
request:
  name: "Test Invalid Authentication"
  url: "https://api.example.com/protected-resource"
  method: GET
  auth:
    type: bearer
    token: "invalid-token"
  expect:
    failure: true    # Expect this request to fail
    status: 401      # Should return Unauthorized
    body:
      error: "Invalid token"
      code: "AUTH_ERROR"

---

# Test validation failure
request:
  name: "Test Invalid Input"
  url: "https://api.example.com/users"
  method: POST
  body:
    name: ""         # Invalid: empty name
    email: "invalid" # Invalid: malformed email
  expect:
    failure: true    # Expect validation to fail
    status: 400      # Should return Bad Request
    body:
      error: "Validation failed"
      details:
        name: "Name is required"
        email: "Invalid email format"

---

# Test resource not found
request:
  name: "Test Non-existent Resource"
  url: "https://api.example.com/users/999999"
  method: GET
  expect:
    failure: true    # Expect resource not found
    status: 404      # Should return Not Found
    body:
      error: "User not found"
      
---

# Test rate limiting
request:
  name: "Test Rate Limit Exceeded"  
  url: "https://api.example.com/limited-endpoint"
  method: GET
  expect:
    failure: true    # Expect rate limit error
    status: 429      # Should return Too Many Requests
    headers:
      retry-after: "*"  # Should include retry information
      
---

# Mixed success and failure testing
requests:
  # Normal success case
  - name: "Valid Request"
    url: "https://api.example.com/data"
    method: GET
    expect:
      status: 200    # Normal success expectation
      
  # Test failure case  
  - name: "Invalid Request"
    url: "https://api.example.com/data"
    method: GET
    headers:
      authorization: "invalid"
    expect:
      failure: true  # Expect this to fail
      status: 403    # Should be Forbidden`;
