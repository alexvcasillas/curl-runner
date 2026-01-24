---
"@curl-runner/cli": minor
---

Add SSL/TLS certificate configuration support

- New `ssl` configuration option for requests and global settings
- Support for custom CA certificates (`ssl.ca`)
- Support for mutual TLS (mTLS) with client certificates (`ssl.cert`, `ssl.key`)
- Support for disabling SSL verification (`ssl.verify: false`)
