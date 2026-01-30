---
"@curl-runner/cli": minor
---

Add TCP connection pooling with HTTP/2 multiplexing

- New `connectionPool` global configuration for request batching
- Groups requests by host and executes in single curl process
- Uses `curl -Z --parallel --http2` for HTTP/2 stream multiplexing
- Dramatically reduces TCP/TLS handshake overhead for parallel requests
- Configuration options: `enabled`, `maxStreamsPerHost`, `keepaliveTime`, `connectTimeout`
