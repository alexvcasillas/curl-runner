---
"@curl-runner/cli": minor
---

Add default value syntax for variables (`${VAR:default}`)

Variables can now specify fallback values that are used when the variable is not set:

- Basic defaults: `${API_TIMEOUT:5000}` - uses "5000" if API_TIMEOUT is not defined
- Nested defaults: `${DATABASE_HOST:${DB_HOST:localhost}}` - tries DATABASE_HOST, then DB_HOST, then falls back to "localhost"

This syntax does not conflict with existing dynamic variable patterns like `${DATE:YYYY-MM-DD}` or `${TIME:HH:mm:ss}`.
