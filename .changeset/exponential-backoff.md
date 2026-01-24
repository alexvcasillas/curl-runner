---
"@curl-runner/cli": minor
---

Add exponential backoff support for retry mechanism

The retry configuration now supports a `backoff` multiplier for exponential backoff between retries. When set, the delay increases exponentially with each retry attempt using the formula: `delay * backoff^(attempt-1)`.

Example usage:
```yaml
retry:
  count: 3
  delay: 1000      # Initial delay: 1 second
  backoff: 2.0     # Multiplier
# Delays: 1000ms, 2000ms, 4000ms
```

The backoff defaults to 1 (no backoff) to maintain backward compatibility.
