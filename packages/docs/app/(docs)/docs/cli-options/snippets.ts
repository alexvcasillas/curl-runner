// CLI options page snippets
export const helpUsageExample = `curl-runner --help
curl-runner -h`;

export const versionExample = `curl-runner --version
curl-runner -v`;

export const verboseExample = `# Enable verbose output
curl-runner --verbose tests.yaml
curl-runner -V tests.yaml

# Quiet mode (minimal output)
curl-runner --quiet tests.yaml
curl-runner -q tests.yaml`;

export const formatOptionsExample = `# JSON format
curl-runner --format json tests.yaml
curl-runner -f json tests.yaml

# Pretty format (default)
curl-runner --format pretty tests.yaml

# Raw format
curl-runner --format raw tests.yaml`;

export const outputOptionsExample = `# Save to file
curl-runner --output results.json tests.yaml
curl-runner -o results.json tests.yaml

# Save to directory
curl-runner --output-dir ./results tests.yaml

# Append to existing file
curl-runner --append --output results.json tests.yaml`;

export const collectionOptionsExample = `# Run specific collection
curl-runner --collection "User Tests" tests.yaml
curl-runner -c "User Tests" tests.yaml

# Run multiple collections
curl-runner -c "Users" -c "Posts" tests.yaml

# List available collections
curl-runner --list-collections tests.yaml
curl-runner -l tests.yaml`;

export const executionOptionsExample = `# Parallel execution
curl-runner --parallel tests.yaml
curl-runner -p tests.yaml

# Sequential execution (default)
curl-runner --sequential tests.yaml
curl-runner -s tests.yaml

# Max concurrent requests
curl-runner --parallel --max-concurrent 5 tests.yaml
curl-runner -p --max-concurrent 5 tests.yaml`;

export const environmentOptionsExample = `# Load environment file
curl-runner --env production.env tests.yaml
curl-runner -e production.env tests.yaml

# Set environment variables
curl-runner --env API_KEY=abc123 tests.yaml
curl-runner -e API_KEY=abc123 -e DEBUG=true tests.yaml`;

export const timeoutOptionsExample = `# Set global timeout (milliseconds)
curl-runner --timeout 30000 tests.yaml
curl-runner -t 30000 tests.yaml

# Disable timeout
curl-runner --timeout 0 tests.yaml`;

export const retryOptionsExample = `# Set retry count
curl-runner --retry 5 tests.yaml
curl-runner -r 5 tests.yaml

# Set retry delay (milliseconds)
curl-runner --retry-delay 2000 tests.yaml

# Disable retries
curl-runner --retry 0 tests.yaml`;

export const headerOptionsExample = `# Include response headers
curl-runner --include-headers tests.yaml
curl-runner -i tests.yaml

# Add custom headers to all requests
curl-runner --header "X-API-Version: v2" tests.yaml
curl-runner -H "Authorization: Bearer token" tests.yaml`;

export const redirectOptionsExample = `# Follow redirects
curl-runner --follow-redirects tests.yaml
curl-runner -L tests.yaml

# Set max redirects
curl-runner --max-redirects 10 tests.yaml

# Don't follow redirects
curl-runner --no-follow-redirects tests.yaml`;

export const sslOptionsExample = `# Disable SSL verification (development only)
curl-runner --insecure tests.yaml
curl-runner -k tests.yaml

# Specify CA certificate
curl-runner --cacert /path/to/ca.pem tests.yaml

# Use client certificate
curl-runner --cert /path/to/client.pem tests.yaml`;

export const debugOptionsExample = `# Enable debug mode
curl-runner --debug tests.yaml
curl-runner -d tests.yaml

# Show timing information
curl-runner --timing tests.yaml

# Dry run (validate only)
curl-runner --dry-run tests.yaml

# Show resolved configuration
curl-runner --show-config tests.yaml`;

export const validateOptionsExample = `# Validate configuration file
curl-runner --validate tests.yaml

# Show configuration schema
curl-runner --schema

# Check syntax only
curl-runner --syntax-check tests.yaml`;

export const watchOptionsExample = `# Watch file for changes
curl-runner --watch tests.yaml
curl-runner -w tests.yaml

# Set watch delay
curl-runner --watch --watch-delay 1000 tests.yaml`;

export const configOptionsExample = `# Specify config file explicitly
curl-runner --config tests.yaml tests.yaml
curl-runner -C tests.yaml

# Use configuration from stdin
echo "collection: ..." | curl-runner --config -

# Multiple configuration files
curl-runner --config base.yaml --config overrides.yaml`;

export const reportOptionsExample = `# Generate report
curl-runner --report tests.yaml

# Specify report format
curl-runner --report --report-format html tests.yaml

# Save report to directory
curl-runner --report --report-dir ./reports tests.yaml`;

export const complexExample = `# Advanced command with multiple options
curl-runner \\
  --config production.yaml \\
  --collection "Critical Tests" \\
  --parallel \\
  --max-concurrent 8 \\
  --format json \\
  --output results.json \\
  --env production.env \\
  --env BUILD_ID=\${CI_BUILD_ID} \\
  --timeout 45000 \\
  --retry 3 \\
  --retry-delay 2000 \\
  --include-headers \\
  --follow-redirects \\
  --max-redirects 5 \\
  --verbose \\
  --report \\
  --report-format html,json \\
  --report-dir ./test-reports`;

export const shortFormExample = `# Using short form options
curl-runner -c "API Tests" -p -f json -o results.json -V -e .env tests.yaml

# Equivalent to:
curl-runner \\
  --collection "API Tests" \\
  --parallel \\
  --format json \\
  --output results.json \\
  --verbose \\
  --env .env \\
  tests.yaml`;
