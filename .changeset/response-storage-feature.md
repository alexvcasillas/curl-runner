---
"@curl-runner/cli": minor
---

feat: Add response storage for sequential request chaining

Store response values from one request and use them in subsequent requests using the new `store` property and `${store.variableName}` syntax.

**New Features:**
- Add `store` property to request configuration for extracting response values
- Support JSON path syntax for extracting nested values (e.g., `body.data.token`, `headers.content-type`)
- Reference stored values in subsequent requests using `${store.variableName}`
- Works seamlessly with existing static and dynamic variables

**Example:**
```yaml
requests:
  - name: Login
    url: https://api.example.com/auth/login
    method: POST
    body:
      username: "admin"
      password: "secret"
    store:
      authToken: body.accessToken
      userId: body.user.id

  - name: Get Profile
    url: https://api.example.com/users/${store.userId}
    headers:
      Authorization: Bearer ${store.authToken}
```

**Supported paths:**
- `body.field` - Response body fields
- `body.nested.field` - Nested fields
- `body.array.0.field` - Array elements by index
- `headers.name` - Response headers
- `status` - HTTP status code
- `metrics.duration` - Response metrics

**Note:** Response storage requires sequential execution mode (the default).
