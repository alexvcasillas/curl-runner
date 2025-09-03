// Response validation page snippets
export const basicValidationExample = `collection:
  name: "Basic Response Validation"
  requests:
    - name: "Validate status code"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: GET
      expect:
        status: 200        # Exact status code match
        
    - name: "Validate multiple status codes"
      url: "https://jsonplaceholder.typicode.com/users/999"
      method: GET
      expect:
        status: [404, 410]  # Accept either 404 or 410`;

export const headerValidationExample = `collection:
  name: "Header Validation"
  requests:
    - name: "Validate response headers"
      url: "https://jsonplaceholder.typicode.com/users"
      method: GET
      expect:
        headers:
          content-type: "application/json; charset=utf-8"
          cache-control: "*"                    # Any value
          x-powered-by: "Express"
          access-control-allow-origin: "*"
          
    - name: "Validate header patterns"
      url: "https://api.example.com/data"
      method: GET
      expect:
        headers:
          x-request-id: "^req_[a-zA-Z0-9]{16}$"  # Regex pattern
          x-rate-limit-remaining: "^\\d+$"        # Numbers only
          content-length: "^[1-9]\\d*$"           # Positive integer`;

export const bodyValidationExample = `collection:
  name: "Body Validation"
  requests:
    - name: "Validate JSON response body"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: GET
      expect:
        body:
          # Exact value matching
          id: 1
          name: "Leanne Graham"
          username: "Bret"
          
          # Wildcard matching (any value)
          email: "*"
          phone: "*"
          website: "*"
          
          # Nested object validation
          address:
            street: "Kulas Light"
            suite: "Apt. 556"
            city: "Gwenborough"
            zipcode: "92998-3874"
            geo:
              lat: "*"     # Any latitude value
              lng: "*"     # Any longitude value
              
          # Company object
          company:
            name: "Romaguera-Crona"
            catchPhrase: "*"
            bs: "*"`;

export const patternValidationExample = `collection:
  name: "Pattern Validation"
  requests:
    - name: "Validate with regex patterns"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: GET
      expect:
        body:
          # Email pattern validation
          email: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"
          
          # Phone pattern validation
          phone: "^[\\d\\-\\.\\s\\(\\)x]+$"
          
          # Website URL pattern
          website: "^[\\w\\.-]+\\.[a-zA-Z]{2,}$"
          
          # Zipcode pattern
          address:
            zipcode: "^\\d{5}-\\d{4}$"
            
          # Coordinate patterns
            geo:
              lat: "^-?\\d+\\.\\d+$"
              lng: "^-?\\d+\\.\\d+$"`;

export const arrayValidationExample = `collection:
  name: "Array Validation"
  requests:
    - name: "Validate array response"
      url: "https://jsonplaceholder.typicode.com/users"
      method: GET
      expect:
        body:
          # Array length validation
          length: 10
          
          # First item validation
          [0]:
            id: 1
            name: "Leanne Graham"
            email: "*"
            
          # Validate all items have required fields
          "*":
            id: "*"
            name: "*"
            email: "*"
            
    - name: "Validate nested arrays"
      url: "https://api.example.com/orders"
      method: GET
      expect:
        body:
          orders:
            # Each order must have these fields
            "*":
              id: "*"
              status: ["pending", "processing", "completed"]
              items:
                # Each item must have these fields
                "*":
                  productId: "*"
                  quantity: "^[1-9]\\d*$"
                  price: "^\\d+\\.\\d{2}$"`;

export const conditionalValidationExample = `collection:
  name: "Conditional Validation"
  requests:
    - name: "Validate based on conditions"
      url: "https://api.example.com/user/profile"
      method: GET
      expect:
        body:
          # Basic fields always required
          id: "*"
          username: "*"
          
          # Conditional validation
          if:
            condition: "response.body.type === 'premium'"
            then:
              subscriptionLevel: "premium"
              features: ["advanced", "priority_support"]
            else:
              subscriptionLevel: "basic"
              
    - name: "Environment-based validation"
      url: "https://api.example.com/config"
      method: GET
      expect:
        body:
          environment: "\${NODE_ENV:development}"
          debug: "\${NODE_ENV:development:true:false}"
          apiUrl: "\${NODE_ENV:production:https://api.prod.com:http://localhost:3000}"`;

export const customValidationExample = `collection:
  name: "Custom Validation Logic"
  requests:
    - name: "Custom validation functions"
      url: "https://api.example.com/analytics"
      method: GET
      expect:
        # Custom validation expressions
        custom:
          - name: "Response time is reasonable"
            expression: "response.time < 2000"
            
          - name: "User count is positive"
            expression: "response.body.userCount > 0"
            
          - name: "Revenue is numeric"
            expression: "typeof response.body.revenue === 'number'"
            
          - name: "Date format is valid"
            expression: "/^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}/.test(response.body.timestamp)"
            
          - name: "Array is not empty"
            expression: "Array.isArray(response.body.items) && response.body.items.length > 0"`;

export const responseTimeValidationExample = `collection:
  name: "Response Time Validation"
  requests:
    - name: "Fast endpoint"
      url: "https://api.example.com/quick"
      method: GET
      expect:
        responseTime: "<1000"    # Must respond within 1 second
        
    - name: "Slow endpoint"
      url: "https://api.example.com/heavy"
      method: GET
      expect:
        responseTime: "<5000"    # Must respond within 5 seconds
        
    - name: "Response time range"
      url: "https://api.example.com/moderate"
      method: GET
      expect:
        responseTime: ">500,<3000"  # Between 500ms and 3 seconds`;

export const advancedValidationExample = `collection:
  name: "Advanced Validation Scenarios"
  requests:
    - name: "Complex API response validation"
      url: "https://api.example.com/orders/123"
      method: GET
      expect:
        status: 200
        headers:
          content-type: "application/json"
          x-request-id: "^req_[a-zA-Z0-9]+$"
          
        body:
          # Order details
          id: 123
          status: ["pending", "processing", "shipped", "delivered"]
          createdAt: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}"
          
          # Customer information
          customer:
            id: "*"
            email: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"
            name: "*"
            
          # Shipping address
          shipping:
            street: "*"
            city: "*"
            state: "^[A-Z]{2}$"        # Two-letter state code
            zipcode: "^\\d{5}(-\\d{4})?$"  # ZIP or ZIP+4
            country: "US"
            
          # Order items
          items:
            length: ">0"               # At least one item
            "*":                       # Each item must have:
              productId: "*"
              name: "*"
              quantity: "^[1-9]\\d*$"  # Positive integer
              price: "^\\d+\\.\\d{2}$" # Price format
              
          # Totals
          subtotal: "^\\d+\\.\\d{2}$"
          tax: "^\\d+\\.\\d{2}$"
          shipping: "^\\d+\\.\\d{2}$"
          total: "^\\d+\\.\\d{2}$"
          
        # Custom business logic validation
        custom:
          - name: "Total equals sum of components"
            expression: |
              Math.abs(
                response.body.total - 
                (response.body.subtotal + response.body.tax + response.body.shipping)
              ) < 0.01
              
          - name: "Subtotal matches item sum"
            expression: |
              Math.abs(
                response.body.subtotal - 
                response.body.items.reduce((sum, item) => 
                  sum + (item.quantity * item.price), 0)
              ) < 0.01`;

export const errorValidationExample = `collection:
  name: "Error Response Validation"
  requests:
    - name: "Validate 404 error"
      url: "https://api.example.com/users/999999"
      method: GET
      expect:
        status: 404
        body:
          error: "Not Found"
          message: "User not found"
          code: 404
          
    - name: "Validate validation errors"
      url: "https://api.example.com/users"
      method: POST
      headers:
        Content-Type: "application/json"
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
              code: "*"
              
    - name: "Validate rate limit error"
      url: "https://api.example.com/data"
      method: GET
      expect:
        status: 429
        headers:
          x-rate-limit-remaining: "0"
          retry-after: "^\\d+$"
        body:
          error: "Rate Limit Exceeded"
          retryAfter: "*"`;

export const schemaValidationExample = `collection:
  name: "Schema-based Validation"
  requests:
    - name: "Validate against JSON schema"
      url: "https://api.example.com/users/1"
      method: GET
      expect:
        schema: |
          {
            "type": "object",
            "required": ["id", "name", "email"],
            "properties": {
              "id": {"type": "integer", "minimum": 1},
              "name": {"type": "string", "minLength": 1},
              "email": {"type": "string", "format": "email"},
              "age": {"type": "integer", "minimum": 0, "maximum": 150},
              "address": {
                "type": "object",
                "properties": {
                  "street": {"type": "string"},
                  "city": {"type": "string"},
                  "zipcode": {"type": "string", "pattern": "^\\d{5}$"}
                }
              }
            }
          }`;
