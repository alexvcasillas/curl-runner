// Output formats page snippets
export const jsonFormatExample = `# JSON format output
global:
  output:
    format: json
    verbose: true

collection:
  name: "JSON Output Example"
  requests:
    - name: "Get user"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: GET

# Output will be structured JSON like:
{
  "summary": {
    "total": 1,
    "passed": 1,
    "failed": 0,
    "duration": 1234
  },
  "results": [
    {
      "name": "Get user",
      "status": "passed",
      "request": {
        "method": "GET",
        "url": "https://jsonplaceholder.typicode.com/users/1"
      },
      "response": {
        "status": 200,
        "headers": {...},
        "body": {...},
        "time": 234
      }
    }
  ]
}`;

export const prettyFormatExample = `# Pretty format output (default)
global:
  output:
    format: pretty
    verbose: true
    showHeaders: true
    showBody: true

collection:
  name: "Pretty Output Example"
  requests:
    - name: "Get user"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: GET

# Output will be human-readable:
# ✓ Get user
#   GET https://jsonplaceholder.typicode.com/users/1
#   Status: 200 OK (234ms)
#   Headers:
#     content-type: application/json; charset=utf-8
#     content-length: 509
#   Body:
#     {
#       "id": 1,
#       "name": "Leanne Graham",
#       ...
#     }`;

export const rawFormatExample = `# Raw format output
global:
  output:
    format: raw
    showBody: true

collection:
  name: "Raw Output Example"
  requests:
    - name: "Get user data"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: GET

# Output will be just the raw response body:
{
  "id": 1,
  "name": "Leanne Graham",
  "username": "Bret",
  "email": "Sincere@april.biz",
  ...
}`;

export const outputConfigurationExample = `# Comprehensive output configuration
global:
  output:
    # Format options: json, pretty, raw
    format: pretty
    
    # Verbosity control
    verbose: true          # Show detailed information
    quiet: false           # Suppress non-essential output
    
    # Content control
    showHeaders: true      # Include response headers
    showBody: true         # Include response body
    showMetrics: true      # Show timing and performance data
    
    # File output
    saveToFile: "results/test-results.json"
    append: false          # Overwrite file (default) vs append
    
    # Formatting options
    colors: true           # Enable colored output
    indent: 2              # JSON indentation spaces
    
collection:
  name: "Output Configuration Example"
  requests:
    - name: "Test request"
      url: "https://httpbin.org/json"
      method: GET`;

export const minimalOutputExample = `# Minimal output configuration
global:
  output:
    format: json
    verbose: false
    showHeaders: false
    showBody: false
    showMetrics: false
    quiet: true

collection:
  name: "Minimal Output"
  requests:
    - name: "Quick check"
      url: "https://httpbin.org/status/200"
      method: GET

# Produces minimal output:
{"summary":{"total":1,"passed":1,"failed":0}}`;

export const detailedOutputExample = `# Detailed output configuration
global:
  output:
    format: pretty
    verbose: true
    showHeaders: true
    showBody: true
    showMetrics: true
    colors: true

collection:
  name: "Detailed Output"
  requests:
    - name: "Comprehensive test"
      url: "https://httpbin.org/json"
      method: GET

# Produces detailed output with:
# - Request details
# - Response status and timing
# - Complete headers
# - Formatted response body
# - Performance metrics
# - Color coding for status`;

export const cicdOutputExample = `# CI/CD optimized output
global:
  output:
    format: json
    verbose: false
    showHeaders: false
    showBody: false
    showMetrics: true
    saveToFile: "test-results.json"
    colors: false          # Disable colors for CI logs

collection:
  name: "CI/CD Pipeline Tests"
  requests:
    - name: "Health check"
      url: "https://api.production.com/health"
      method: GET
      
    - name: "Authentication test"
      url: "https://api.production.com/auth/verify"
      method: GET

# Suitable for:
# - Automated testing pipelines
# - Test result archival
# - Machine processing
# - Integration with test reporting tools`;

export const debugOutputExample = `# Debug output configuration
global:
  output:
    format: pretty
    verbose: true
    showHeaders: true
    showBody: true
    showMetrics: true
    debug: true            # Enable debug information

collection:
  name: "Debug Output"
  requests:
    - name: "Debug request"
      url: "https://httpbin.org/json"
      method: GET

# Debug output includes:
# - Variable resolution details
# - Request construction steps
# - Validation process details
# - Retry attempt information
# - Detailed error messages`;

export const customOutputExample = `# Custom output with selective content
global:
  output:
    format: json
    
collection:
  name: "Custom Output"
  requests:
    - name: "API response extraction"
      url: "https://jsonplaceholder.typicode.com/users/1"
      method: GET
      output:
        # Request-level output overrides
        format: raw
        showBody: true
        showHeaders: false
        
    - name: "Headers inspection"
      url: "https://httpbin.org/headers"
      method: GET
      output:
        format: pretty
        showHeaders: true
        showBody: false
        
# Each request can have different output formatting`;

export const fileOutputExample = `# File output options
global:
  output:
    format: json
    saveToFile: "results/api-tests-{DATE}.json"
    
collection:
  name: "File Output Example"
  requests:
    - name: "Test 1"
      url: "https://httpbin.org/status/200"
      method: GET
      
    - name: "Test 2"
      url: "https://httpbin.org/json"
      method: GET

# File naming variables:
# {DATE} - Current date (YYYY-MM-DD)
# {TIME} - Current time (HH-MM-SS)
# {TIMESTAMP} - Unix timestamp
# {UUID} - Random UUID

# Examples:
# - "results/test-{DATE}.json"
# - "output/api-tests-{TIMESTAMP}.json"
# - "reports/{UUID}-results.json"`;

export const multiFormatOutputExample = `# Multiple output formats
global:
  output:
    format: pretty      # Console output
    saveToFile: "results.json"  # Also save as JSON

collection:
  name: "Multi-format Output"
  requests:
    - name: "Test endpoint"
      url: "https://httpbin.org/json"
      method: GET

# Command line options for multiple outputs:
# curl-runner --format pretty --output results.json tests.yaml
# curl-runner --format json --output console.json --report-html report.html tests.yaml`;

export const streamingOutputExample = `# Streaming output for long-running tests
global:
  output:
    format: pretty
    streaming: true     # Output results as they complete
    
collection:
  name: "Streaming Output"
  requests:
    - name: "Quick request"
      url: "https://httpbin.org/delay/1"
      method: GET
      
    - name: "Medium request"
      url: "https://httpbin.org/delay/3"
      method: GET
      
    - name: "Slow request"
      url: "https://httpbin.org/delay/5"
      method: GET

# Results appear immediately as each request completes
# instead of waiting for all requests to finish`;

export const templateOutputExample = `# Custom output templates
global:
  output:
    format: template
    template: |
      Test: {{name}}
      URL: {{request.url}}
      Status: {{response.status}}
      Time: {{response.time}}ms
      {% if response.status >= 400 %}
      ❌ FAILED
      {% else %}
      ✅ PASSED
      {% endif %}
      
collection:
  name: "Template Output"
  requests:
    - name: "Custom formatted test"
      url: "https://httpbin.org/status/200"
      method: GET

# Template variables available:
# - name: Request name
# - request: Full request object
# - response: Full response object
# - duration: Total request time
# - status: Pass/fail status`;
