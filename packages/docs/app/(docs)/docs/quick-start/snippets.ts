// Quick start page snippets
export const firstTestExample = `collection:
  name: "My First Test"
  requests:
    - name: "Test JSONPlaceholder API"
      url: "https://jsonplaceholder.typicode.com/posts/1"
      method: GET
      expect:
        status: 200
        headers:
          content-type: "application/json; charset=utf-8"
        body:
          userId: 1
          id: 1
          title: "*"  # Any string value`;

export const runFirstTestExample = `curl-runner my-first-test.yaml`;

export const createUserTestExample = `collection:
  name: "User Management Tests"
  requests:
    - name: "Create new user"
      url: "https://jsonplaceholder.typicode.com/users"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "name": "John Doe",
          "username": "johndoe",
          "email": "john@example.com"
        }
      expect:
        status: 201
        body:
          name: "John Doe"
          username: "johndoe"
          email: "john@example.com"
          id: "*"  # Any ID value`;

export const withVariablesExample = `global:
  variables:
    BASE_URL: "https://jsonplaceholder.typicode.com"
    USER_ID: "1"

collection:
  name: "API Tests with Variables"
  requests:
    - name: "Get user by ID"
      url: "\${BASE_URL}/users/\${USER_ID}"
      method: GET
      expect:
        status: 200
        body:
          id: \${USER_ID}`;

export const authenticationExample = `global:
  variables:
    API_TOKEN: "\${API_TOKEN}"  # Load from environment
    BASE_URL: "https://api.example.com"

collection:
  name: "Authenticated API Tests"
  requests:
    - name: "Get protected resource"
      url: "\${BASE_URL}/protected"
      method: GET
      headers:
        Authorization: "Bearer \${API_TOKEN}"
      expect:
        status: 200`;

export const errorHandlingExample = `collection:
  name: "Error Handling Tests"
  requests:
    - name: "Test 404 error"
      url: "https://jsonplaceholder.typicode.com/posts/999999"
      method: GET
      expect:
        status: 404
        
    - name: "Test validation error"
      url: "https://jsonplaceholder.typicode.com/posts"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "invalidField": "value"
        }
      expect:
        status: [400, 422]  # Accept either error code`;

export const nextStepsExample = `# Run with different output formats
curl-runner --format json my-tests.yaml
curl-runner --format pretty my-tests.yaml

# Run in parallel for performance
curl-runner --parallel my-tests.yaml

# Save results to file
curl-runner --output results.json my-tests.yaml

# Run specific collection
curl-runner --collection "User Tests" my-tests.yaml`;
