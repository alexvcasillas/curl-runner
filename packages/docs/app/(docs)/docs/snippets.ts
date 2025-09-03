// Root docs page snippets
export const basicUsageExample = `# Basic API test configuration
collection:
  name: "My API Tests"
  requests:
    - name: "Get users"
      url: "https://jsonplaceholder.typicode.com/users"
      method: GET
      expect:
        status: 200
        
    - name: "Create user"
      url: "https://jsonplaceholder.typicode.com/users"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "name": "John Doe",
          "email": "john@example.com"
        }
      expect:
        status: 201`;

export const quickInstallExample = `# Install curl-runner globally
npm install -g curl-runner

# Or using yarn
yarn global add curl-runner

# Or using pnpm
pnpm install -g curl-runner`;

export const quickRunExample = `# Run a simple test
curl-runner my-api-tests.yaml

# Run with verbose output
curl-runner --verbose my-api-tests.yaml

# Run specific collection
curl-runner --collection "User API" my-tests.yaml`;
