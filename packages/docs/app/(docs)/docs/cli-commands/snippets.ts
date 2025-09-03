// CLI commands page snippets
export const basicRunExample = `# Run a configuration file
curl-runner tests.yaml

# Run with verbose output
curl-runner --verbose tests.yaml

# Run in quiet mode
curl-runner --quiet tests.yaml`;

export const collectionSelectionExample = `# Run specific collection by name
curl-runner --collection "User Tests" tests.yaml

# Run multiple collections
curl-runner --collection "Users" --collection "Posts" tests.yaml

# List available collections
curl-runner --list-collections tests.yaml`;

export const outputFormatExample = `# JSON output (machine readable)
curl-runner --format json tests.yaml

# Pretty formatted output (human readable)  
curl-runner --format pretty tests.yaml

# Raw response content only
curl-runner --format raw tests.yaml`;

export const parallelExecutionExample = `# Run requests in parallel
curl-runner --parallel tests.yaml

# Force sequential execution
curl-runner --sequential tests.yaml

# Parallel with max concurrency
curl-runner --parallel --max-concurrent 5 tests.yaml`;

export const outputRedirectionExample = `# Save output to file
curl-runner --output results.json tests.yaml

# Save with timestamp
curl-runner --output "results-$(date +%Y%m%d).json" tests.yaml

# Append to existing file
curl-runner --output results.json --append tests.yaml`;

export const environmentExample = `# Load environment from file
curl-runner --env production.env tests.yaml

# Set environment variables inline
curl-runner --env API_KEY=abc123 --env BASE_URL=https://api.com tests.yaml

# Combine file and inline variables
curl-runner --env .env --env DEBUG=true tests.yaml`;

export const advancedOptionsExample = `# Disable SSL verification (development only)
curl-runner --insecure tests.yaml

# Set custom timeout for all requests
curl-runner --timeout 30000 tests.yaml

# Include response headers in output
curl-runner --include-headers tests.yaml

# Follow redirects (up to 5 by default)
curl-runner --follow-redirects tests.yaml

# Set max redirects
curl-runner --max-redirects 10 tests.yaml`;

export const debuggingExample = `# Enable debug mode
curl-runner --debug tests.yaml

# Verbose output with timing
curl-runner --verbose --timing tests.yaml

# Dry run (validate without executing)
curl-runner --dry-run tests.yaml

# Show configuration after variable substitution
curl-runner --show-config tests.yaml`;

export const helpExample = `# Show general help
curl-runner --help

# Show version information
curl-runner --version

# Show configuration file schema
curl-runner --schema

# Validate configuration file
curl-runner --validate tests.yaml`;

export const complexCommandExample = `# Complex production command
curl-runner \\
  --config production.yaml \\
  --collection "Health Checks" \\
  --collection "User API" \\
  --parallel \\
  --max-concurrent 10 \\
  --format json \\
  --output "results/$(date +%Y%m%d-%H%M%S)-test-results.json" \\
  --env production.env \\
  --env BUILD_NUMBER=\${BUILD_NUMBER} \\
  --timeout 30000 \\
  --include-headers \\
  --verbose`;

export const watchModeExample = `# Watch file for changes and re-run
curl-runner --watch tests.yaml

# Watch with specific collection
curl-runner --watch --collection "Development Tests" tests.yaml

# Watch mode with custom delay
curl-runner --watch --watch-delay 2000 tests.yaml`;

export const reportingExample = `# Generate detailed report
curl-runner --report --output-dir reports/ tests.yaml

# Generate HTML report
curl-runner --report --report-format html tests.yaml

# Generate both JSON and HTML reports
curl-runner --report --report-format json,html tests.yaml`;
