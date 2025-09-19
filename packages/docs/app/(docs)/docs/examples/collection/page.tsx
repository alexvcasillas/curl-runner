import {
  Activity,
  CheckCircle,
  Code,
  FileText,
  FolderOpen,
  GitBranch,
  MessageCircle,
  Settings,
  TestTube,
} from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'Collection Examples',
  description:
    'Organize related HTTP requests into collections with curl-runner. Learn request collections, shared variables, and test suite management.',
  keywords: [
    'curl-runner collection examples',
    'request collections',
    'test suites',
    'organized API testing',
    'collection structure',
    'grouped requests',
    'shared variables in collections',
    'API test organization',
    'request grouping',
    'collection configuration',
    'test collection patterns',
    'structured testing',
  ],
  openGraph: {
    title: 'Collection Examples | curl-runner Documentation',
    description:
      'Organize related HTTP requests into collections with curl-runner. Learn about request collections, shared variables, and managing test suites.',
    url: 'https://www.curl-runner.com/docs/examples/collection',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Collection Examples | curl-runner Documentation',
    description: 'Learn how to organize HTTP requests into collections with curl-runner.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/examples/collection',
  },
};

const basicCollection = `# Basic collection structure
collection:
  name: User Management API Tests
  description: Testing user CRUD operations
  
  requests:
    - name: List All Users
      url: https://api.example.com/users
      method: GET
      
    - name: Get Specific User
      url: https://api.example.com/users/1
      method: GET
      
    - name: Create New User
      url: https://api.example.com/users
      method: POST
      headers:
        Content-Type: application/json
      body:
        name: "John Doe"
        email: "john@example.com"`;

const collectionWithVariables = `# Collection with shared variables
collection:
  name: E-Commerce API Tests
  variables:
    USER_ID: 12345
    PRODUCT_ID: 98765
    ORDER_ID: 555
    
  defaults:
    headers:
      Authorization: Bearer \${API_TOKEN}
      Content-Type: application/json
  
  requests:
    - name: Get User Profile
      url: \${BASE_URL}/users/\${USER_ID}
      method: GET
      
    - name: Get User Orders
      url: \${BASE_URL}/users/\${USER_ID}/orders
      method: GET
      
    - name: Get Product Details
      url: \${BASE_URL}/products/\${PRODUCT_ID}
      method: GET
      
    - name: Add Product to Cart
      url: \${BASE_URL}/users/\${USER_ID}/cart
      method: POST
      body:
        productId: \${PRODUCT_ID}
        quantity: 2`;

const fullCollectionExample = `# Complete collection with global settings
global:
  execution: sequential
  continueOnError: true
  variables:
    BASE_URL: https://jsonplaceholder.typicode.com
    API_VERSION: v1
  output:
    verbose: true
    showMetrics: true
    saveToFile: test-results.json

collection:
  name: JSONPlaceholder Complete Test Suite
  description: Comprehensive API testing example
  
  variables:
    TEST_USER_ID: 1
    TEST_POST_ID: 1
  
  defaults:
    timeout: 5000
    retry:
      count: 2
      delay: 1000
    headers:
      Accept: application/json
      X-API-Version: \${API_VERSION}
  
  requests:
    # User Operations
    - name: Get All Users
      url: \${BASE_URL}/users
      method: GET
      expect:
        status: 200
        
    - name: Get Test User
      url: \${BASE_URL}/users/\${TEST_USER_ID}
      method: GET
      expect:
        status: 200
        body:
          id: 1
          
    # Post Operations
    - name: Get User Posts
      url: \${BASE_URL}/users/\${TEST_USER_ID}/posts
      method: GET
      expect:
        status: 200
        
    - name: Create New Post
      url: \${BASE_URL}/posts
      method: POST
      headers:
        Content-Type: application/json
      body:
        title: "Test Post"
        body: "This is a test post created by curl-runner"
        userId: \${TEST_USER_ID}
      expect:
        status: 201
        body:
          userId: 1
          
    - name: Update Post
      url: \${BASE_URL}/posts/\${TEST_POST_ID}
      method: PUT
      body:
        id: \${TEST_POST_ID}
        title: "Updated Post"
        body: "This post has been updated"
        userId: \${TEST_USER_ID}
      expect:
        status: 200
        
    - name: Delete Post
      url: \${BASE_URL}/posts/\${TEST_POST_ID}
      method: DELETE
      expect:
        status: 200
        
    # Comment Operations
    - name: Get Post Comments
      url: \${BASE_URL}/posts/\${TEST_POST_ID}/comments
      method: GET
      expect:
        status: 200
        
    # Album and Photo Operations
    - name: Get User Albums
      url: \${BASE_URL}/users/\${TEST_USER_ID}/albums
      method: GET
      expect:
        status: 200
        
    # Todo Operations
    - name: Get User Todos
      url: \${BASE_URL}/users/\${TEST_USER_ID}/todos
      method: GET
      expect:
        status: 200`;

const nestedCollections = `# Multiple collections in one file
global:
  variables:
    API_URL: https://api.example.com
    AUTH_TOKEN: secret-token

# First Collection - Authentication Tests
collection:
  name: Authentication Suite
  requests:
    - name: Login
      url: \${API_URL}/auth/login
      method: POST
      body:
        username: "testuser"
        password: "testpass"
      expect:
        status: 200
        body:
          token: "*"
          
    - name: Refresh Token
      url: \${API_URL}/auth/refresh
      method: POST
      headers:
        Authorization: Bearer \${AUTH_TOKEN}
      expect:
        status: 200

---
# Second Collection - User Management
collection:
  name: User Management Suite
  defaults:
    headers:
      Authorization: Bearer \${AUTH_TOKEN}
      
  requests:
    - name: Get Profile
      url: \${API_URL}/profile
      method: GET
      expect:
        status: 200
        
    - name: Update Profile
      url: \${API_URL}/profile
      method: PATCH
      body:
        displayName: "Updated Name"
      expect:
        status: 200`;

const runCommand = `# Run collection tests
curl-runner collection.yaml

# Run with specific output format
curl-runner collection.yaml --output-format pretty

# Run in parallel mode
curl-runner collection.yaml --parallel

# Save results to file
curl-runner collection.yaml --output results.json

# Run with verbose output
curl-runner collection.yaml --verbose`;

export default function CollectionExamplePage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Collection Example"
          text="Learn how to organize multiple requests into collections for comprehensive API testing."
        />

        <div className="space-y-8">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground mb-6">
              Collections allow you to group related HTTP requests together, share variables and
              defaults, and run comprehensive test suites. They're perfect for testing complete API
              workflows or organizing tests by feature.
            </p>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <FolderOpen className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <h4 className="font-medium">Group Related Tests</h4>
                      <Badge>Organization</Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Organize requests by feature, endpoint, or workflow
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Settings className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <h4 className="font-medium">Share Configuration</h4>
                      <Badge variant="secondary">Reusability</Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Define variables and defaults once, use everywhere
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Basic Collection */}
          <section>
            <H2 id="basic-collection">Basic Collection</H2>
            <p className="text-muted-foreground mb-6">
              A simple collection groups related requests under a name and description.
            </p>

            <CodeBlockServer language="yaml" filename="basic-collection.yaml">
              {basicCollection}
            </CodeBlockServer>
          </section>

          {/* Collection with Variables */}
          <section>
            <H2 id="collection-variables">Collection Variables</H2>
            <p className="text-muted-foreground mb-6">
              Define variables at the collection level to share across all requests.
            </p>

            <CodeBlockServer language="yaml" filename="collection-variables.yaml">
              {collectionWithVariables}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-blue-500/5 dark:bg-blue-500/10 border-blue-500/20 p-4">
              <p className="text-sm">
                <strong className="text-blue-600 dark:text-blue-400">Variable Precedence:</strong>{' '}
                Collection variables override global variables but can be overridden by
                request-level variables.
              </p>
            </div>
          </section>

          {/* Complete Example */}
          <section>
            <H2 id="complete-example">Complete Collection Example</H2>
            <p className="text-muted-foreground mb-6">
              A comprehensive example showing all collection features including global settings,
              defaults, variables, and validation.
            </p>

            <CodeBlockServer language="yaml" filename="complete-collection.yaml">
              {fullCollectionExample}
            </CodeBlockServer>
          </section>

          {/* Collection Structure */}
          <section>
            <H2 id="collection-structure">Collection Structure</H2>
            <p className="text-muted-foreground mb-6">
              Understanding the collection configuration options.
            </p>

            <div className="border rounded-lg overflow-hidden">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Field</th>
                    <th className="text-left p-3 font-medium">Type</th>
                    <th className="text-left p-3 font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">name</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">string</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Collection name (required)
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">description</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">string</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Collection description (optional)
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">variables</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">object</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Collection-level variables
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">defaults</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">object</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Default request configuration
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">requests</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">array</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      List of requests (required)
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Multiple Collections */}
          <section>
            <H2 id="multiple-collections">Multiple Collections</H2>
            <p className="text-muted-foreground mb-6">
              You can define multiple collections in separate files or use YAML document separators.
            </p>

            <CodeBlockServer language="yaml" filename="multiple-collections.yaml">
              {nestedCollections}
            </CodeBlockServer>
          </section>

          {/* Running Collections */}
          <section>
            <H2 id="running-collections">Running Collections</H2>
            <p className="text-muted-foreground mb-6">
              Execute collection tests using the curl-runner CLI.
            </p>

            <CodeBlockServer language="bash" filename="terminal">
              {runCommand}
            </CodeBlockServer>
          </section>

          {/* Best Practices - Bento Grid */}
          <section>
            <H2 id="best-practices">Best Practices</H2>
            <div className="mt-8 grid gap-4 lg:grid-cols-3 lg:grid-rows-2">
              {/* Organize by Feature - Large Left Card */}
              <div className="relative lg:row-span-2">
                <div className="absolute inset-px rounded-lg bg-card lg:rounded-l-2xl" />
                <div className="relative flex h-full flex-col overflow-hidden rounded-lg lg:rounded-l-2xl border">
                  <div className="px-6 pt-6 pb-3 sm:px-8 sm:pt-8 sm:pb-0">
                    <div className="flex items-center gap-3 mb-4">
                      <div className="rounded-full bg-blue-500/10 p-2">
                        <FolderOpen className="h-5 w-5 text-blue-600 dark:text-blue-400" />
                      </div>
                      <Badge className="bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20">
                        Organization
                      </Badge>
                    </div>
                    <h4 className="text-lg font-semibold mb-3">Organize by Feature</h4>
                    <p className="text-sm text-muted-foreground mb-4">
                      Group related endpoints together for better organization and maintainability.
                    </p>
                  </div>
                  <div className="flex-1 px-6 pb-6 sm:px-8 sm:pb-8">
                    <div className="mt-4 space-y-3">
                      <div className="flex items-start gap-3">
                        <div className="w-2 h-2 rounded-full bg-blue-500 mt-2 flex-shrink-0" />
                        <div className="text-sm text-muted-foreground">
                          User management operations (CRUD)
                        </div>
                      </div>
                      <div className="flex items-start gap-3">
                        <div className="w-2 h-2 rounded-full bg-blue-500 mt-2 flex-shrink-0" />
                        <div className="text-sm text-muted-foreground">
                          Authentication & authorization flows
                        </div>
                      </div>
                      <div className="flex items-start gap-3">
                        <div className="w-2 h-2 rounded-full bg-blue-500 mt-2 flex-shrink-0" />
                        <div className="text-sm text-muted-foreground">
                          Product catalog operations
                        </div>
                      </div>
                      <div className="flex items-start gap-3">
                        <div className="w-2 h-2 rounded-full bg-blue-500 mt-2 flex-shrink-0" />
                        <div className="text-sm text-muted-foreground">
                          Payment & order processing
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              {/* Use Descriptive Names - Top Middle Card */}
              <div className="relative max-lg:row-start-1">
                <div className="absolute inset-px rounded-lg bg-card max-lg:rounded-t-2xl" />
                <div className="relative flex h-full flex-col overflow-hidden rounded-lg max-lg:rounded-t-2xl border">
                  <div className="px-6 pt-6 sm:px-8 sm:pt-8">
                    <div className="flex items-center gap-3 mb-4">
                      <div className="rounded-full bg-green-500/10 p-2">
                        <FileText className="h-5 w-5 text-green-600 dark:text-green-400" />
                      </div>
                      <Badge className="bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20">
                        Naming
                      </Badge>
                    </div>
                    <h4 className="text-lg font-semibold mb-3">Use Descriptive Names</h4>
                    <p className="text-sm text-muted-foreground">
                      Clear, meaningful names make collections and requests self-documenting.
                    </p>
                  </div>
                  <div className="flex-1 px-6 pb-6 sm:px-8 sm:pb-8 flex items-end">
                    <div className="w-full rounded-lg bg-slate-100 dark:bg-zinc-950/70 backdrop-blur-sm border border-slate-200 dark:border-zinc-800 p-3 font-mono text-xs">
                      <div className="text-green-600 dark:text-green-400">
                        ✓ User Registration Flow
                      </div>
                      <div className="text-green-600 dark:text-green-400">
                        ✓ Payment Processing Suite
                      </div>
                      <div className="text-red-400">✗ test1, test2, temp</div>
                    </div>
                  </div>
                </div>
              </div>

              {/* Leverage Defaults - Bottom Middle Card */}
              <div className="relative max-lg:row-start-3 lg:col-start-2 lg:row-start-2">
                <div className="absolute inset-px rounded-lg bg-card" />
                <div className="relative flex h-full flex-col overflow-hidden rounded-lg border">
                  <div className="px-6 pt-6 sm:px-8 sm:pt-8">
                    <div className="flex items-center gap-3 mb-4">
                      <div className="rounded-full bg-purple-500/10 p-2">
                        <Settings className="h-5 w-5 text-purple-600 dark:text-purple-400" />
                      </div>
                      <Badge className="bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-500/20">
                        Configuration
                      </Badge>
                    </div>
                    <h4 className="text-lg font-semibold mb-3">Leverage Defaults</h4>
                    <p className="text-sm text-muted-foreground">
                      Set common configuration once at the collection level.
                    </p>
                  </div>
                </div>
              </div>

              {/* Add Validation & Document Variables - Large Right Card */}
              <div className="relative lg:row-span-2">
                <div className="absolute inset-px rounded-lg bg-card max-lg:rounded-b-2xl lg:rounded-r-2xl" />
                <div className="relative flex h-full flex-col overflow-hidden rounded-lg max-lg:rounded-b-2xl lg:rounded-r-2xl border">
                  <div className="px-6 pt-6 pb-3 sm:px-8 sm:pt-8 sm:pb-0">
                    <div className="flex items-center gap-3 mb-4">
                      <div className="rounded-full bg-cyan-500/10 p-2">
                        <CheckCircle className="h-5 w-5 text-cyan-600 dark:text-cyan-400" />
                      </div>
                      <Badge className="bg-cyan-500/10 text-cyan-600 dark:text-cyan-400 border-cyan-500/20">
                        Quality
                      </Badge>
                    </div>
                    <h4 className="text-lg font-semibold mb-3">Validation & Documentation</h4>
                    <p className="text-sm text-muted-foreground mb-4">
                      Ensure reliability with proper validation and clear documentation.
                    </p>
                  </div>
                  <div className="flex-1 px-6 pb-6 sm:px-8 sm:pb-8">
                    <div className="mt-4 space-y-4">
                      <div className="flex items-center gap-3">
                        <div className="h-8 w-8 rounded-lg bg-cyan-500/10 flex items-center justify-center">
                          <CheckCircle className="h-4 w-4 text-cyan-500" />
                        </div>
                        <div className="flex-1">
                          <div className="text-sm font-medium">Response Validation</div>
                          <div className="text-xs text-muted-foreground">
                            Validate status codes and body content
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-3">
                        <div className="h-8 w-8 rounded-lg bg-yellow-500/10 flex items-center justify-center">
                          <MessageCircle className="h-4 w-4 text-yellow-500" />
                        </div>
                        <div className="flex-1">
                          <div className="text-sm font-medium">Document Variables</div>
                          <div className="text-xs text-muted-foreground">
                            Comment variable purposes and values
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-3">
                        <div className="h-8 w-8 rounded-lg bg-green-500/10 flex items-center justify-center">
                          <Settings className="h-4 w-4 text-green-500" />
                        </div>
                        <div className="flex-1">
                          <div className="text-sm font-medium">Error Handling</div>
                          <div className="text-xs text-muted-foreground">
                            Set retry policies and timeouts
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Use Cases */}
          <section>
            <H2 id="use-cases">Common Use Cases</H2>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <TestTube className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">API Test Suites</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Comprehensive testing of all API endpoints with organized test cases and
                      validation rules.
                    </p>
                    <Badge className="bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20">
                      Testing
                    </Badge>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <GitBranch className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">CI/CD Pipelines</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Automated API testing in deployment workflows for continuous integration and
                      delivery.
                    </p>
                    <Badge className="bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20">
                      Integration
                    </Badge>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Activity className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Health Checks</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Regular API availability and performance monitoring with automated alerts.
                    </p>
                    <Badge className="bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-500/20">
                      Monitoring
                    </Badge>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-yellow-500/10 p-2">
                    <Code className="h-4 w-4 text-yellow-600 dark:text-yellow-500" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">API Documentation</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Living documentation of API usage patterns and examples for development teams.
                    </p>
                    <Badge className="bg-yellow-500/10 text-yellow-600 dark:text-yellow-400 border-yellow-500/20">
                      Development
                    </Badge>
                  </div>
                </div>
              </div>
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
