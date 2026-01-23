// Response storage page snippets

export const basicStorageExample = `# Basic response storage
requests:
  # First request: Create a resource and store the ID
  - name: Create User
    url: https://api.example.com/users
    method: POST
    headers:
      Content-Type: application/json
    body:
      name: "John Doe"
      email: "john@example.com"
    store:
      userId: body.id        # Store the returned user ID

  # Second request: Use the stored ID
  - name: Get User Details
    url: https://api.example.com/users/\${store.userId}
    method: GET
    expect:
      status: 200`;

export const authFlowExample = `# Authentication flow with token storage
global:
  execution: sequential  # Required for response storage
  variables:
    API_URL: https://api.example.com

requests:
  # Step 1: Login and store the token
  - name: Login
    url: \${API_URL}/auth/login
    method: POST
    headers:
      Content-Type: application/json
    body:
      username: "admin"
      password: "secret123"
    expect:
      status: 200
    store:
      authToken: body.accessToken
      refreshToken: body.refreshToken
      tokenExpiry: body.expiresIn

  # Step 2: Use token for authenticated request
  - name: Get Protected Resource
    url: \${API_URL}/protected/data
    method: GET
    headers:
      Authorization: Bearer \${store.authToken}
    expect:
      status: 200

  # Step 3: Create a resource with authentication
  - name: Create Resource
    url: \${API_URL}/protected/resources
    method: POST
    headers:
      Authorization: Bearer \${store.authToken}
      Content-Type: application/json
    body:
      name: "New Resource"
    expect:
      status: 201`;

export const crudWorkflowExample = `# Complete CRUD workflow
requests:
  # CREATE: Make a new post
  - name: Create Post
    url: https://api.example.com/posts
    method: POST
    headers:
      Content-Type: application/json
    body:
      title: "My First Post"
      content: "Hello, World!"
      published: false
    expect:
      status: 201
    store:
      postId: body.id
      createdAt: body.createdAt

  # READ: Verify the post was created
  - name: Get Post
    url: https://api.example.com/posts/\${store.postId}
    method: GET
    expect:
      status: 200
      body:
        id: "\${store.postId}"

  # UPDATE: Modify the post
  - name: Update Post
    url: https://api.example.com/posts/\${store.postId}
    method: PUT
    headers:
      Content-Type: application/json
    body:
      title: "Updated Title"
      content: "Updated content"
      published: true
    expect:
      status: 200

  # DELETE: Remove the post
  - name: Delete Post
    url: https://api.example.com/posts/\${store.postId}
    method: DELETE
    expect:
      status: 204`;

export const nestedPathsExample = `# Extracting nested values
request:
  name: Get Complex Response
  url: https://api.example.com/data
  method: GET
  expect:
    status: 200
  store:
    # Top-level field
    requestId: body.id

    # Nested object field
    userName: body.user.name
    userEmail: body.user.email

    # Deeply nested field
    accessToken: body.auth.tokens.access

    # Array element by index
    firstItemId: body.items.0.id

    # Response headers
    contentType: headers.content-type
    requestId: headers.x-request-id

    # HTTP status code
    statusCode: status`;

export const pathSyntaxReference = `# Path syntax examples
store:
  # Body fields
  id: body.id                    # Top-level field
  name: body.user.name           # Nested field
  token: body.data.auth.token    # Deeply nested

  # Array access
  first: body.items.0            # First array element
  second: body.items.1           # Second element
  nested: body.items.0.id        # Field from first element

  # Headers (case-sensitive)
  contentType: headers.content-type
  authHeader: headers.authorization
  customHeader: headers.x-custom

  # Status code
  status: status                 # HTTP status code

  # Metrics
  duration: metrics.duration     # Request duration in ms
  size: metrics.size             # Response size in bytes`;

export const mixedVariablesExample = `# Combining store with other variables
global:
  variables:
    BASE_URL: https://api.example.com
    API_VERSION: v2

requests:
  - name: Create Session
    url: \${BASE_URL}/\${API_VERSION}/sessions
    method: POST
    headers:
      X-Request-ID: "\${UUID}"           # Dynamic variable
      X-Timestamp: "\${CURRENT_TIME}"    # Dynamic variable
    body:
      createdAt: "\${DATE:YYYY-MM-DD}"   # Date formatting
    store:
      sessionId: body.id

  - name: Use All Variable Types
    url: \${BASE_URL}/\${API_VERSION}/sessions/\${store.sessionId}
    method: GET
    headers:
      X-Request-ID: "\${UUID}"
      X-Session: "\${store.sessionId}"   # Stored value
    expect:
      status: 200`;

export const dataExtractionExample = `# Extracting data for chained requests
requests:
  # Get a list and extract specific items
  - name: List All Users
    url: https://api.example.com/users
    method: GET
    expect:
      status: 200
    store:
      firstUserId: body.users.0.id
      firstUserEmail: body.users.0.email
      totalCount: body.meta.total

  # Use extracted data for follow-up
  - name: Get First User Details
    url: https://api.example.com/users/\${store.firstUserId}
    method: GET
    expect:
      status: 200
      body:
        email: "\${store.firstUserEmail}"

  # Create related resource
  - name: Create User Profile
    url: https://api.example.com/profiles
    method: POST
    headers:
      Content-Type: application/json
    body:
      userId: "\${store.firstUserId}"
      email: "\${store.firstUserEmail}"
    expect:
      status: 201`;

export const errorHandlingExample = `# Handling missing values gracefully
global:
  continueOnError: true  # Continue even if store values are missing

requests:
  - name: Get Optional Data
    url: https://api.example.com/optional
    method: GET
    store:
      optionalField: body.maybeExists

  # If optionalField wasn't found, the variable remains as \${store.optionalField}
  # This can be used for conditional logic or validation
  - name: Use Optional Value
    url: https://api.example.com/action
    method: POST
    body:
      data: "\${store.optionalField}"  # Will be literal string if not found
    expect:
      status: [200, 400]  # Accept either success or error`;

export const realWorldExample = `# Real-world: E-commerce checkout flow
global:
  execution: sequential
  variables:
    API_URL: https://api.shop.example.com

requests:
  # 1. Add item to cart
  - name: Add to Cart
    url: \${API_URL}/cart/items
    method: POST
    headers:
      Content-Type: application/json
    body:
      productId: "PROD-123"
      quantity: 2
    expect:
      status: 201
    store:
      cartId: body.cartId
      cartItemId: body.itemId

  # 2. Apply discount code
  - name: Apply Discount
    url: \${API_URL}/cart/\${store.cartId}/discounts
    method: POST
    headers:
      Content-Type: application/json
    body:
      code: "SAVE20"
    expect:
      status: 200
    store:
      discountAmount: body.discount.amount
      finalPrice: body.totals.final

  # 3. Create order from cart
  - name: Create Order
    url: \${API_URL}/orders
    method: POST
    headers:
      Content-Type: application/json
    body:
      cartId: "\${store.cartId}"
      shippingAddress:
        street: "123 Main St"
        city: "Example City"
        zip: "12345"
    expect:
      status: 201
    store:
      orderId: body.orderId
      orderTotal: body.total

  # 4. Process payment
  - name: Process Payment
    url: \${API_URL}/orders/\${store.orderId}/payment
    method: POST
    headers:
      Content-Type: application/json
    body:
      method: "credit_card"
      amount: "\${store.orderTotal}"
    expect:
      status: 200
      body:
        status: "completed"`;
