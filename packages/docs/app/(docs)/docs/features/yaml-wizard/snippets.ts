// YAML Wizard page snippets

export const quickInitExample = `# Quick init - prompts for URL and method
curl-runner init

# Quick init with URL
curl-runner init https://api.example.com/users

# The wizard will prompt for method and filename`;

export const fullWizardExample = `# Full interactive wizard
curl-runner init --wizard

# Or short form
curl-runner init -w

# With custom output file
curl-runner init --wizard -o api-test.yaml`;

export const editModeExample = `# Edit an existing YAML file
curl-runner edit api-test.yaml

# Edit and save to a new file
curl-runner edit api-test.yaml -o api-test-v2.yaml`;

export const wizardFlowExample = `┌─ curl-runner wizard v1.x.x ──────────────────┐
│                                              │
│  ? Start from a template?                    │
│    ○ Blank - Start from scratch              │
│    ● Basic GET Request                       │
│    ○ Basic POST Request                      │
│    ○ API Test                                │
│    ○ File Upload                             │
│    ○ Auth Flow                               │
│                                              │
│  ? Request URL                               │
│    https://api.example.com/users             │
│                                              │
│  ? HTTP method                               │
│    GET                                       │
│                                              │
│  ? Request name (optional)                   │
│    Get All Users                             │
│                                              │
│  ? Add headers?                              │
│    Yes                                       │
│                                              │
│  ... (continues through all options)         │
│                                              │
└──────────────────────────────────────────────┘`;

export const wizardOutputExample = `request:
  name: Get All Users
  url: "https://api.example.com/users"
  headers:
    Accept: application/json
    Authorization: Bearer my-token
  expect:
    status: 200
    responseTime: "< 2000"`;

export const templatesExample = `# Available templates:

# 1. Basic GET Request
# - Simple GET with default settings
# - Follow redirects enabled

# 2. Basic POST Request
# - POST method with JSON content-type
# - Prompts for JSON body

# 3. API Test
# - Includes response validation
# - Response time assertions
# - Verbose output enabled

# 4. File Upload
# - Multipart form data
# - File attachment support

# 5. Auth Flow
# - Bearer token authentication
# - Pre-configured headers`;

export const advancedOptionsExample = `# The wizard prompts for these advanced options:

# Timeout
? Timeout (ms): 30000

# Redirects
? Follow redirects? Yes

# SSL
? Skip SSL verification? No

# HTTP/2
? Use HTTP/2? Yes

# Retry configuration
? Configure retry strategy? Yes
? Max retry attempts: 3
? Retry delay (ms): 1000
? Backoff multiplier: 2

# Response validation
? Add response validation? Yes
? Expected status code(s): 200
? Response time constraint: < 2000`;

export const runAfterCreateExample = `# After creating the file, the wizard asks:

┌─ Preview ──────────────────────────────────────┐
│                                                │
│  request:                                      │
│    url: "https://api.example.com/users"        │
│    headers:                                    │
│      Accept: application/json                  │
│    expect:                                     │
│      status: 200                               │
│                                                │
└────────────────────────────────────────────────┘

? Save to: api-test.yaml
? Run request after creating? Yes

# If you say yes, the request executes immediately
# so you can verify your configuration works`;
