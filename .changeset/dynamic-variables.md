---
"@curl-runner/cli": minor
---

Add support for additional dynamic variable generators:

- `${UUID:short}` - Returns the first 8 characters of a UUID
- `${RANDOM:min-max}` - Generates a random integer in the specified range
- `${RANDOM:string:length}` - Generates a random alphanumeric string of specified length

These variables are useful for API testing scenarios where shorter IDs or random test data are needed.
