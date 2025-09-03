// Basic examples page snippets
export const simpleGetExample = `collection:
  name: "Simple GET Request"
  requests:
    - name: "Get user"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: GET
      expect:
        status: 200`;

export const simplePostExample = `collection:
  name: "Simple POST Request"
  requests:
    - name: "Create post"
      url: "https://jsonplaceholder.typicode.com/posts"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "title": "My Post",
          "body": "This is my post content",
          "userId": 1
        }
      expect:
        status: 201`;

export const withHeadersExample = `collection:
  name: "Request with Headers"
  requests:
    - name: "API request with custom headers"
      url: "https://jsonplaceholder.typicode.com/posts"
      method: GET
      headers:
        Accept: "application/json"
        User-Agent: "MyApp/1.0"
        X-Custom-Header: "custom-value"
      expect:
        status: 200
        headers:
          content-type: "application/json; charset=utf-8"`;

export const queryParametersExample = `collection:
  name: "Request with Query Parameters"
  requests:
    - name: "Search posts"
      url: "https://jsonplaceholder.typicode.com/posts"
      method: GET
      params:
        userId: 1
        _limit: 5
        _sort: "id"
        _order: "desc"
      expect:
        status: 200`;

export const responseValidationExample = `collection:
  name: "Response Validation"
  requests:
    - name: "Validate user response"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: GET
      expect:
        status: 200
        headers:
          content-type: "application/json; charset=utf-8"
        body:
          id: 1
          name: "Leanne Graham"
          username: "Bret"
          email: "Sincere@april.biz"
          address:
            street: "Kulas Light"
            suite: "Apt. 556"
            city: "Gwenborough"
            zipcode: "92998-3874"`;

export const basicVariablesExample = `global:
  variables:
    BASE_URL: "https://jsonplaceholder.typicode.com"
    USER_ID: "1"

collection:
  name: "Basic Variables Example"
  requests:
    - name: "Get user with variable"
      url: "\${BASE_URL}/users/\${USER_ID}"
      method: GET
      expect:
        status: 200
        body:
          id: \${USER_ID}`;

export const multipleRequestsExample = `collection:
  name: "Multiple Requests Example"
  requests:
    - name: "Get all users"
      url: "https://jsonplaceholder.typicode.com/users"
      method: GET
      expect:
        status: 200
        
    - name: "Get specific user"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: GET
      expect:
        status: 200
        body:
          id: 1
          
    - name: "Get user posts"
      url: "https://jsonplaceholder.typicode.com/users/1/posts"
      method: GET
      expect:
        status: 200`;

export const basicErrorHandlingExample = `collection:
  name: "Basic Error Handling"
  requests:
    - name: "Valid request"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: GET
      expect:
        status: 200
        
    - name: "Not found request"
      url: "https://jsonplaceholder.typicode.com/users/999"
      method: GET
      expect:
        status: 404
        
    - name: "Invalid method"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: DELETE
      expect:
        status: [404, 405]  # Accept either error code`;

export const timeoutExample = `collection:
  name: "Request with Timeout"
  requests:
    - name: "Quick request"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: GET
      timeout: 5000  # 5 seconds
      expect:
        status: 200
        
    - name: "Slow request"
      url: "https://httpbin.org/delay/3"
      method: GET
      timeout: 10000  # 10 seconds
      expect:
        status: 200`;

export const jsonBodyExample = `collection:
  name: "JSON Body Example"
  requests:
    - name: "Create user"
      url: "https://jsonplaceholder.typicode.com/users"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "name": "John Doe",
          "username": "johndoe",
          "email": "john@example.com",
          "address": {
            "street": "123 Main St",
            "city": "Anytown",
            "zipcode": "12345"
          },
          "phone": "555-123-4567",
          "website": "johndoe.com"
        }
      expect:
        status: 201
        body:
          name: "John Doe"
          username: "johndoe"
          email: "john@example.com"`;
