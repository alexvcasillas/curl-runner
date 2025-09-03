// Global settings page snippets
export const executionModesExample = `# Sequential execution (default)
global:
  execution: sequential  # One request at a time
  continueOnError: false # Stop on first failure

---

# Parallel execution
global:
  execution: parallel    # All requests simultaneously  
  continueOnError: true  # Continue despite failures`;

export const variablesExample = `global:
  variables:
    # Environment configuration
    BASE_URL: "https://api.example.com"
    API_VERSION: "v2"
    
    # From environment variables
    API_KEY: "\${API_KEY}"
    DATABASE_URL: "\${DATABASE_URL}"
    
    # Common values
    DEFAULT_TIMEOUT: "5000"
    USER_AGENT: "curl-runner/1.0"`;

export const defaultsExample = `global:
  defaults:
    # Apply to all requests
    timeout: 10000
    headers:
      User-Agent: "curl-runner/1.0"
      Accept: "application/json"
    retry:
      count: 3
      delay: 1000
    expect:
      headers:
        content-type: "application/json"`;

export const outputConfigExample = `global:
  output:
    # Control verbosity
    verbose: true
    
    # Include response details
    showHeaders: true
    showBody: true
    showMetrics: true
    
    # Format options: json, pretty, raw
    format: pretty
    
    # Save to file
    saveToFile: "results/test-output.json"`;

export const errorHandlingExample = `# Stop on first error (strict)
global:
  continueOnError: false
  execution: sequential

---

# Continue on errors (resilient)
global:
  continueOnError: true
  execution: sequential`;

export const completeExample = `# Complete global configuration
global:
  # Execution control
  execution: sequential
  continueOnError: false
  
  # Global variables
  variables:
    BASE_URL: "https://api.example.com"
    API_VERSION: "v1"
    TIMEOUT_MS: "5000"
    API_TOKEN: "\${API_TOKEN}"
    
  # Output settings
  output:
    verbose: true
    showHeaders: true
    showBody: true
    showMetrics: true
    format: pretty
    saveToFile: "test-results.json"
    
  # Default request settings
  defaults:
    timeout: \${TIMEOUT_MS}
    headers:
      User-Agent: "curl-runner/1.0"
      Authorization: "Bearer \${API_TOKEN}"
      Accept: "application/json"
    followRedirects: true
    retry:
      count: 3
      delay: 1000
    expect:
      headers:
        content-type: "application/json"`;
