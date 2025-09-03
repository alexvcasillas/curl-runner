import { AlertTriangle, CheckCircle, Info } from 'lucide-react';
import { CodeBlockServer } from '@/components/code-block-server';
import { DocsPageHeader } from '@/components/docs-page-header';
import { H2, H3 } from '@/components/mdx-heading';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

const globalVariablesExample = `# Global variables available to all requests
global:
  variables:
    BASE_URL: https://api.example.com
    API_VERSION: v1
    API_TOKEN: your-secret-token
    TIMEOUT: 5000

requests:
  - name: Get Users
    url: \${BASE_URL}/\${API_VERSION}/users
    method: GET
    headers:
      Authorization: Bearer \${API_TOKEN}
    timeout: \${TIMEOUT}`;

const collectionVariablesExample = `# Collection-level variables
global:
  variables:
    BASE_URL: https://api.example.com
    
collection:
  name: User Management Tests
  variables:
    USER_ID: 123
    ADMIN_TOKEN: admin-secret-token
    
  requests:
    - name: Get User Profile
      url: \${BASE_URL}/users/\${USER_ID}
      headers:
        Authorization: Bearer \${ADMIN_TOKEN}`;

const envVariablesExample = `# Using environment variables
global:
  variables:
    # Environment variables are automatically available
    BASE_URL: \${ENV.API_BASE_URL}
    API_KEY: \${ENV.API_KEY}
    DEBUG: \${ENV.DEBUG}

requests:
  - name: API Call with Env Variables
    url: \${BASE_URL}/endpoint
    headers:
      X-API-Key: \${API_KEY}
    # Conditional behavior based on environment
    timeout: \${DEBUG ? 30000 : 5000}`;

const dynamicVariablesExample = `# Dynamic variables and computed values
global:
  variables:
    BASE_URL: https://api.example.com
    TIMESTAMP: \${Date.now()}
    UUID: \${crypto.randomUUID()}
    CURRENT_DATE: \${new Date().toISOString().split('T')[0]}

requests:
  - name: Create Resource with Dynamic Data
    url: \${BASE_URL}/resources
    method: POST
    body:
      id: \${UUID}
      timestamp: \${TIMESTAMP}
      created_date: \${CURRENT_DATE}
      name: "Resource-\${TIMESTAMP}"`;

const variableScopingExample = `# Variable scoping and precedence
global:
  variables:
    SHARED_VAR: "global-value"
    BASE_URL: https://api.example.com
    
collection:
  variables:
    SHARED_VAR: "collection-value"  # Overrides global
    COLLECTION_VAR: "collection-only"
    
  defaults:
    headers:
      X-Shared: \${SHARED_VAR}  # Will be "collection-value"
      X-Collection: \${COLLECTION_VAR}
      
  requests:
    - name: Test Variable Precedence
      url: \${BASE_URL}/test
      # SHARED_VAR = "collection-value" (collection overrides global)
      # COLLECTION_VAR = "collection-only"
      # BASE_URL = "https://api.example.com" (from global)`;

const conditionalVariablesExample = `# Conditional variables and expressions
global:
  variables:
    ENV: production
    DEBUG_MODE: false
    API_TIMEOUT: \${ENV === 'development' ? 30000 : 5000}
    LOG_LEVEL: \${DEBUG_MODE ? 'debug' : 'info'}
    
collection:
  variables:
    # Conditional API endpoints
    BASE_URL: \${ENV === 'production' ? 'https://api.example.com' : 'https://api-staging.example.com'}
    
  requests:
    - name: Environment-Aware Request
      url: \${BASE_URL}/data
      timeout: \${API_TIMEOUT}
      headers:
        X-Log-Level: \${LOG_LEVEL}
        X-Debug: \${DEBUG_MODE ? 'true' : 'false'}`;

const variableInterpolationExample = `# Complex variable interpolation
global:
  variables:
    API_HOST: api.example.com
    API_PORT: 443
    API_PROTOCOL: https
    API_PATH: /v1/api
    
    # Computed base URL
    BASE_URL: "\${API_PROTOCOL}://\${API_HOST}:\${API_PORT}\${API_PATH}"
    
    # User data
    USER_NAME: john.doe
    USER_DOMAIN: example.com
    USER_EMAIL: "\${USER_NAME}@\${USER_DOMAIN}"

requests:
  - name: Complex Interpolation Example
    url: \${BASE_URL}/users/search
    method: POST
    body:
      query: "email:\${USER_EMAIL}"
      filters:
        domain: \${USER_DOMAIN}
        username: \${USER_NAME}`;

export default function VariablesPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Variables"
          text="Use variables and templating to create reusable, dynamic HTTP request configurations."
        />

        <div className="space-y-12">
          {/* Introduction */}
          <section>
            <p className="text-lg text-muted-foreground mb-6">
              Variables in curl-runner allow you to create flexible, reusable configurations. You
              can define variables at different scopes and use them throughout your request
              definitions with template interpolation.
            </p>

            <div className="rounded-lg border bg-card p-4">
              <div className="flex items-start gap-3">
                <div className="rounded-full bg-blue-500/10 p-2">
                  <Info className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                </div>
                <div className="flex-1 pt-1">
                  <h4 className="font-medium mb-2">Variable Interpolation Syntax</h4>
                  <p className="text-sm text-muted-foreground">
                    Variables are interpolated using the{' '}
                    <code className="bg-primary/20 text-primary px-2 py-1 rounded text-sm font-mono">
                      ${`{VARIABLE_NAME}`}
                    </code>{' '}
                    syntax, which supports JavaScript expressions and complex logic.
                  </p>
                </div>
              </div>
            </div>
          </section>

          {/* Variable Scopes */}
          <section>
            <H2>Variable Scopes</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Variables can be defined at different levels, each with its own scope and precedence
              rules.
            </p>

            <div className="space-y-8">
              <div>
                <H3>Global Variables</H3>
                <p className="text-muted-foreground mb-4">
                  Global variables are available to all requests in the file and have the lowest
                  precedence.
                </p>
                <CodeBlockServer language="yaml" filename="global-variables.yaml">
                  {globalVariablesExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3>Collection Variables</H3>
                <p className="text-muted-foreground mb-4">
                  Collection variables are scoped to a specific collection and override global
                  variables.
                </p>
                <CodeBlockServer language="yaml" filename="collection-variables.yaml">
                  {collectionVariablesExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3>Environment Variables</H3>
                <p className="text-muted-foreground mb-4">
                  Access system environment variables using the <code>ENV</code> object.
                </p>
                <CodeBlockServer language="yaml" filename="env-variables.yaml">
                  {envVariablesExample}
                </CodeBlockServer>
              </div>
            </div>
          </section>

          {/* Variable Precedence */}
          <section>
            <H2>Variable Precedence</H2>
            <p className="text-muted-foreground text-lg mb-6">
              When variables are defined at multiple levels, curl-runner follows a specific
              precedence order.
            </p>

            <div className="rounded-lg border bg-card p-4 mb-6">
              <div className="flex items-start gap-3">
                <div className="rounded-full bg-purple-500/10 p-2">
                  <AlertTriangle className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                </div>
                <div className="flex-1 pt-1">
                  <h4 className="font-medium mb-4">Precedence Order (Highest to Lowest)</h4>
                  <div className="space-y-3">
                    <div className="flex items-center space-x-3">
                      <Badge variant="default" className="bg-primary">
                        1
                      </Badge>
                      <span className="font-medium">Environment Variables</span>
                      <code className="text-sm bg-primary/20 text-primary px-2 py-1 rounded font-mono">
                        ${`{ENV.VAR_NAME}`}
                      </code>
                    </div>
                    <div className="flex items-center space-x-3">
                      <Badge variant="secondary">2</Badge>
                      <span className="font-medium">Collection Variables</span>
                      <code className="text-sm bg-muted px-2 py-1 rounded font-mono">
                        collection.variables
                      </code>
                    </div>
                    <div className="flex items-center space-x-3">
                      <Badge variant="outline">3</Badge>
                      <span className="font-medium">Global Variables</span>
                      <code className="text-sm bg-muted px-2 py-1 rounded font-mono">
                        global.variables
                      </code>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <CodeBlockServer language="yaml" filename="variable-precedence.yaml">
              {variableScopingExample}
            </CodeBlockServer>
          </section>

          {/* Dynamic Variables */}
          <section>
            <H2>Dynamic Variables</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Create dynamic values using JavaScript expressions and built-in functions.
            </p>

            <div className="space-y-6">
              <H3>Built-in Functions</H3>
              <div className="grid gap-4 sm:grid-cols-2 mb-6">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-orange-500/10 p-2">
                      <Info className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                    </div>
                    <div className="flex-1 pt-1">
                      <h4 className="font-medium mb-3">Date & Time</h4>
                      <div className="space-y-2 text-sm">
                        <div>
                          <code className="bg-primary/10 text-primary px-2 py-1 rounded font-mono">
                            Date.now()
                          </code>{' '}
                          - Current timestamp
                        </div>
                        <div>
                          <code className="bg-primary/10 text-primary px-2 py-1 rounded font-mono">
                            new Date().toISOString()
                          </code>{' '}
                          - ISO date
                        </div>
                        <div>
                          <code className="bg-primary/10 text-primary px-2 py-1 rounded font-mono">
                            new Date().getTime()
                          </code>{' '}
                          - Unix timestamp
                        </div>
                      </div>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-cyan-500/10 p-2">
                      <Info className="h-4 w-4 text-cyan-600 dark:text-cyan-400" />
                    </div>
                    <div className="flex-1 pt-1">
                      <h4 className="font-medium mb-3">Random Values</h4>
                      <div className="space-y-2 text-sm">
                        <div>
                          <code className="bg-primary/10 text-primary px-2 py-1 rounded font-mono">
                            crypto.randomUUID()
                          </code>{' '}
                          - UUID v4
                        </div>
                        <div>
                          <code className="bg-primary/10 text-primary px-2 py-1 rounded font-mono">
                            Math.random()
                          </code>{' '}
                          - Random number
                        </div>
                        <div>
                          <code className="bg-primary/10 text-primary px-2 py-1 rounded font-mono">
                            Math.floor(Math.random() * 1000)
                          </code>{' '}
                          - Random int
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              <CodeBlockServer language="yaml" filename="dynamic-variables.yaml">
                {dynamicVariablesExample}
              </CodeBlockServer>
            </div>
          </section>

          {/* Conditional Variables */}
          <section>
            <H2>Conditional Logic</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Use JavaScript expressions to create conditional variables and environment-specific
              configurations.
            </p>

            <CodeBlockServer language="yaml" filename="conditional-variables.yaml">
              {conditionalVariablesExample}
            </CodeBlockServer>
          </section>

          {/* Complex Interpolation */}
          <section>
            <H2>Complex Interpolation</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Combine multiple variables and expressions to create complex, computed values.
            </p>

            <CodeBlockServer language="yaml" filename="complex-interpolation.yaml">
              {variableInterpolationExample}
            </CodeBlockServer>
          </section>

          {/* Best Practices */}
          <section>
            <H2>Best Practices</H2>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1 pt-1">
                    <h4 className="font-medium mb-3">Best Practices</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <p>• Use descriptive variable names</p>
                      <p>• Define common values as variables</p>
                      <p>• Use environment variables for secrets</p>
                      <p>• Group related variables logically</p>
                      <p>• Document complex expressions</p>
                    </div>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-amber-500/10 p-2">
                    <AlertTriangle className="h-4 w-4 text-amber-600 dark:text-amber-400" />
                  </div>
                  <div className="flex-1 pt-1">
                    <h4 className="font-medium mb-3">Avoid These Mistakes</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <p>• Hard-coding sensitive information</p>
                      <p>• Using overly complex expressions</p>
                      <p>• Creating circular references</p>
                      <p>• Overusing dynamic variables</p>
                      <p>• Ignoring variable naming conventions</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Common Patterns */}
          <section>
            <H2>Common Patterns</H2>

            <div className="space-y-6">
              <H3>API Authentication</H3>
              <CodeBlockServer language="yaml" title="API Authentication Pattern">
                {`global:
  variables:
    API_KEY: \${ENV.API_KEY}
    AUTH_HEADER: "Bearer \${API_KEY}"
    
  defaults:
    headers:
      Authorization: \${AUTH_HEADER}`}
              </CodeBlockServer>

              <H3>Environment-Specific URLs</H3>
              <CodeBlockServer language="yaml" title="Environment-Specific URLs">
                {`global:
  variables:
    ENVIRONMENT: \${ENV.NODE_ENV || 'development'}
    BASE_URL: \${ENVIRONMENT === 'production' 
      ? 'https://api.example.com' 
      : 'https://api-staging.example.com'}`}
              </CodeBlockServer>

              <H3>Request Correlation IDs</H3>
              <CodeBlockServer language="yaml" title="Request Correlation IDs">
                {`global:
  variables:
    CORRELATION_ID: \${crypto.randomUUID()}
    
  defaults:
    headers:
      X-Correlation-ID: \${CORRELATION_ID}
      X-Request-Time: \${Date.now()}`}
              </CodeBlockServer>
            </div>
          </section>
        </div>
      </div>

      {/* Table of Contents */}
      <div className="hidden text-sm xl:block">
        <div className="sticky top-16 -mt-10 pt-4">
          <TableOfContents />
        </div>
      </div>
    </main>
  );
}
