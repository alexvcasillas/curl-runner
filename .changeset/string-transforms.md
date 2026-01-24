---
"@curl-runner/cli": minor
---

Add string transformation support for variables

New `:upper` and `:lower` transforms for case manipulation:

- `${VAR:upper}` converts variable value to uppercase
- `${VAR:lower}` converts variable value to lowercase
- Works with both static variables and environment variables

Example usage:

```yaml
global:
  variables:
    ENV: "production"
    RESOURCE: "Users"

collection:
  requests:
    - name: API Request
      url: "https://api.example.com/${RESOURCE:lower}"
      headers:
        X-Environment: "${ENV:upper}"
```

This resolves to `https://api.example.com/users` with header `X-Environment: PRODUCTION`.
