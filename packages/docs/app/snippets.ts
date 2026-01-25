// Landing page snippets
export const exampleYaml = `# api-tests.yaml
global:
  variables:
    BASE_URL: https://api.example.com
    API_TOKEN: your-token-here
  execution: parallel

requests:
  - name: Get Users
    url: \${BASE_URL}/users
    method: GET
    headers:
      Authorization: Bearer \${API_TOKEN}
      
  - name: Health Check
    url: \${BASE_URL}/health
    method: GET
    validation:
      status: 200
      body:
        status: ok`;

export const installCommand = `# Quick install (Linux/macOS)
curl -fsSL https://www.curl-runner.com/install.sh | bash

# Quick install (Windows PowerShell)
irm https://www.curl-runner.com/install.ps1 | iex

# Or with package managers
bun install -g @curl-runner/cli
npm install -g @curl-runner/cli`;

export const runCommand = `# Run your tests
curl-runner api-tests.yaml -v

# Run all YAML files in directory
curl-runner tests/ --parallel

# Save results to JSON
curl-runner api-tests.yaml --output results.json`;
