// Conditional execution page snippets

export const basicWhenExample = `# Basic conditional execution
requests:
  # First request: Get data and store values
  - name: Get User
    url: https://api.example.com/users/1
    method: GET
    expect:
      status: 200
    store:
      userId: body.id
      userRole: body.role

  # Only runs if userId exists
  - name: Get User Details
    url: https://api.example.com/users/\${store.userId}/details
    method: GET
    when:
      left: store.userId
      operator: exists
    expect:
      status: 200`;

export const stringShorthandExample = `# String shorthand syntax
requests:
  - name: Check Status
    url: https://api.example.com/status
    store:
      code: status
      type: body.type

  # Equality check
  - name: Handle Success
    url: https://api.example.com/success
    when: "store.code == 200"

  # Comparison
  - name: Handle Error
    url: https://api.example.com/error
    when: "store.code >= 400"

  # Exists check
  - name: Process Type
    url: https://api.example.com/process
    when: "store.type exists"`;

export const allConditionsExample = `# AND conditions - all must be true
requests:
  - name: Get Feature Flags
    url: https://api.example.com/flags
    store:
      enabled: body.enabled
      version: body.version
      region: body.region

  # Only runs if ALL conditions pass
  - name: Use New Feature
    url: https://api.example.com/new-feature
    when:
      all:
        - left: store.enabled
          operator: "=="
          right: true
        - left: store.version
          operator: ">="
          right: 2
        - left: store.region
          operator: "=="
          right: "us-east"`;

export const anyConditionsExample = `# OR conditions - any must be true
requests:
  - name: Get User
    url: https://api.example.com/user
    store:
      role: body.role

  # Runs if user is admin OR superuser
  - name: Admin Dashboard
    url: https://api.example.com/admin
    when:
      any:
        - left: store.role
          operator: "=="
          right: "admin"
        - left: store.role
          operator: "=="
          right: "superuser"
        - left: store.role
          operator: "=="
          right: "moderator"`;

export const operatorsReference = `# Operator reference
requests:
  - name: Get Data
    url: https://api.example.com/data
    store:
      status: status
      count: body.count
      name: body.name
      error: body.error

  # Equality operators
  - name: Check Equal
    url: https://api.example.com/eq
    when:
      left: store.status
      operator: "=="
      right: 200

  - name: Check Not Equal
    url: https://api.example.com/neq
    when:
      left: store.status
      operator: "!="
      right: 404

  # Comparison operators
  - name: Check Greater
    url: https://api.example.com/gt
    when:
      left: store.count
      operator: ">"
      right: 0

  - name: Check Less Or Equal
    url: https://api.example.com/lte
    when:
      left: store.count
      operator: "<="
      right: 100

  # String operators
  - name: Check Contains
    url: https://api.example.com/contains
    when:
      left: store.name
      operator: contains
      right: "admin"

  - name: Check Matches (regex)
    url: https://api.example.com/matches
    when:
      left: store.name
      operator: matches
      right: "^user_\\\\d+"

  # Existence operators
  - name: Check Exists
    url: https://api.example.com/exists
    when:
      left: store.count
      operator: exists

  - name: Check Not Exists
    url: https://api.example.com/not-exists
    when:
      left: store.error
      operator: not-exists`;

export const caseSensitiveExample = `# Case sensitivity control
requests:
  - name: Get User
    url: https://api.example.com/user
    store:
      name: body.name  # "John"

  # Case-insensitive (default) - matches "john", "John", "JOHN"
  - name: Check Name Insensitive
    url: https://api.example.com/check
    when:
      left: store.name
      operator: "=="
      right: "john"

  # Case-sensitive - only matches exact "John"
  - name: Check Name Sensitive
    url: https://api.example.com/check
    when:
      left: store.name
      operator: "=="
      right: "John"
      caseSensitive: true`;

export const authFlowExample = `# Authentication flow with conditions
global:
  execution: sequential
  continueOnError: true

requests:
  # Step 1: Try to authenticate
  - name: Login
    url: https://api.example.com/auth/login
    method: POST
    body:
      username: "admin"
      password: "secret"
    store:
      status: status
      token: body.token
      error: body.error

  # Step 2: Only proceed if login succeeded
  - name: Get Protected Data
    url: https://api.example.com/protected
    headers:
      Authorization: Bearer \${store.token}
    when:
      all:
        - left: store.status
          operator: "=="
          right: 200
        - left: store.token
          operator: exists
    expect:
      status: 200

  # Step 3: Handle login failure
  - name: Refresh Token
    url: https://api.example.com/auth/refresh
    method: POST
    when:
      left: store.status
      operator: "!="
      right: 200`;

export const errorHandlingExample = `# Error handling with conditions
global:
  continueOnError: true

requests:
  - name: Create Resource
    url: https://api.example.com/resources
    method: POST
    body:
      name: "Test"
    store:
      status: status
      resourceId: body.id
      errorCode: body.error.code

  # Success path
  - name: Verify Creation
    url: https://api.example.com/resources/\${store.resourceId}
    when: "store.status == 201"
    expect:
      status: 200

  # Error path - duplicate
  - name: Handle Duplicate
    url: https://api.example.com/resources/existing
    when:
      all:
        - left: store.status
          operator: "=="
          right: 409
        - left: store.errorCode
          operator: "=="
          right: "DUPLICATE"

  # Error path - validation error
  - name: Handle Validation Error
    url: https://api.example.com/validation-errors
    method: POST
    when: "store.status == 400"`;

export const workflowExample = `# Complete workflow with branching
global:
  execution: sequential
  continueOnError: true

requests:
  # 1. Check feature flag
  - name: Check Feature
    url: https://api.example.com/features/new-checkout
    store:
      featureEnabled: body.enabled
      rolloutPercent: body.rollout

  # 2a. New checkout flow (if enabled and rollout >= 50%)
  - name: New Checkout
    url: https://api.example.com/checkout/v2
    method: POST
    when:
      all:
        - left: store.featureEnabled
          operator: "=="
          right: true
        - left: store.rolloutPercent
          operator: ">="
          right: 50
    body:
      items: ["item-1", "item-2"]

  # 2b. Legacy checkout (if feature disabled)
  - name: Legacy Checkout
    url: https://api.example.com/checkout/v1
    method: POST
    when:
      left: store.featureEnabled
      operator: "!="
      right: true
    body:
      items: ["item-1", "item-2"]`;
