# Documentation vs Implementation Audit

This document catalogs all gaps found between the curl-runner documentation and actual implementation.

**Audit Date:** 2026-01-24
**Auditor:** Claude (automated documentation audit)

---

## Summary

| # | Gap | Recommendation | Priority |
|---|-----|----------------|----------|
| 1 | Dynamic variable generators (UUID:short, RANDOM) | IMPLEMENT | High |
| 2 | Default value syntax for variables | IMPLEMENT | Medium |
| 3 | JavaScript expressions in variables | REMOVE from docs | N/A |
| 4 | String manipulation (upper/lower) | IMPLEMENT | Low |
| 5 | maxConcurrency for parallel execution | IMPLEMENT | High |
| 6 | Exponential backoff (retryBackoff) | IMPLEMENT | Medium |
| 7 | Advanced timeout settings | REMOVE from docs | N/A |
| 8 | SSL/TLS certificate configuration | IMPLEMENT | Medium |
| 9 | Rate limiting configuration | REMOVE from docs (future) | N/A |
| 10 | Tutorials page placeholder content | FIX | High |
| 11 | Use Cases page placeholder content | FIX | High |
| 12 | CLI flag inconsistencies | FIX docs | Medium |
| 13 | ENV object syntax | REMOVE from docs | N/A |
| 14 | Advanced output options | REMOVE from docs | N/A |

---

## Issue 1: Implement Additional Dynamic Variable Generators

### Title
`feat: Implement additional dynamic variable generators (UUID:short, RANDOM)`

### Labels
`enhancement`, `good first issue`

### Body

#### Summary

The documentation describes several dynamic variable generators that are not yet implemented in the CLI.

#### Documentation Claims (variables.md, request-object.md)

The docs show these dynamic variables:
```yaml
global:
  variables:
    SESSION_ID: "${UUID:short}"        # Short UUID variant
    RANDOM_NUM: "${RANDOM:1-1000}"     # Random number in range
    RANDOM_STR: "${RANDOM:string:10}"  # Random string of length 10
```

#### Current Implementation (parser/yaml.ts:94-117)

Only these dynamic variables are implemented:
- `${UUID}` - Full UUID
- `${TIMESTAMP}` / `${CURRENT_TIME}` - Unix timestamp
- `${DATE:format}` - Date with format (YYYY-MM-DD)
- `${TIME:format}` - Time with format (HH:mm:ss)

#### Recommendation: IMPLEMENT

These are valuable features for API testing:
- **`${UUID:short}`** - Useful for shorter IDs when full UUID is overkill
- **`${RANDOM:min-max}`** - Essential for testing with random numeric data
- **`${RANDOM:string:length}`** - Essential for generating test data

#### Implementation Notes

In `packages/cli/src/parser/yaml.ts`, extend `resolveDynamicVariable()`:

```typescript
// UUID:short
if (varName === 'UUID:short') {
  return crypto.randomUUID().split('-')[0]; // First segment (8 chars)
}

// RANDOM:min-max
const randomRangeMatch = varName.match(/^RANDOM:(\d+)-(\d+)$/);
if (randomRangeMatch) {
  const min = Number(randomRangeMatch[1]);
  const max = Number(randomRangeMatch[2]);
  return String(Math.floor(Math.random() * (max - min + 1)) + min);
}

// RANDOM:string:length
const randomStringMatch = varName.match(/^RANDOM:string:(\d+)$/);
if (randomStringMatch) {
  const length = Number(randomStringMatch[1]);
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  return Array.from({ length }, () => chars.charAt(Math.floor(Math.random() * chars.length))).join('');
}
```

#### Affected Documentation

- `packages/docs/public/docs/variables.md`
- `packages/docs/public/docs/api-reference/request-object.md`

---

## Issue 2: Implement Default Value Syntax for Variables

### Title
`feat: Implement default value syntax for variables (${VAR:default})`

### Labels
`enhancement`

### Body

#### Summary

The documentation shows a default value syntax for variables that is not implemented.

#### Documentation Claims (variables.md)

```yaml
global:
  variables:
    # Default value if environment variable not set
    API_TIMEOUT: "${API_TIMEOUT:5000}"

    # Multiple environment sources with fallback
    DB_HOST: "${DATABASE_HOST:${DB_HOST:localhost}}"
```

#### Current Implementation

The parser (`parser/yaml.ts`) does not support the `:default` syntax. Variables either resolve or remain as the original `${VAR}` string.

#### Recommendation: IMPLEMENT

This is a standard and expected feature for templating systems. It provides:
- Safe defaults when environment variables are missing
- More robust YAML configurations
- Better developer experience

#### Implementation Notes

In `packages/cli/src/parser/yaml.ts`, update `resolveVariable()`:

```typescript
static resolveVariable(
  varName: string,
  variables: Record<string, string>,
  storeContext?: ResponseStoreContext,
): string | null {
  // Check for default value syntax: ${VAR:default}
  const defaultMatch = varName.match(/^([^:]+):(.+)$/);
  if (defaultMatch) {
    const actualVarName = defaultMatch[1];
    const defaultValue = defaultMatch[2];

    // Don't confuse with DATE:, TIME:, UUID:short, RANDOM: patterns
    if (!['DATE', 'TIME', 'UUID', 'RANDOM'].includes(actualVarName)) {
      const resolved = YamlParser.resolveVariable(actualVarName, variables, storeContext);
      return resolved !== null ? resolved : defaultValue;
    }
  }

  // ... rest of existing logic
}
```

#### Affected Documentation

- `packages/docs/public/docs/variables.md`

---

## Issue 3: Remove JavaScript Expression Documentation from Variables

### Title
`docs: Remove undocumented JavaScript expression syntax from variables documentation`

### Labels
`documentation`

### Body

#### Summary

The documentation shows JavaScript expressions in variables that are not implemented and would add significant complexity.

#### Documentation Claims (variables.md, global-settings.md)

```yaml
global:
  variables:
    ENVIRONMENT: ${ENV.NODE_ENV || 'development'}
    BASE_URL: ${ENVIRONMENT === 'production'
      ? 'https://api.example.com'
      : 'https://api-staging.example.com'}
    DEBUG_MODE: ${ENVIRONMENT === 'development'}
```

#### Current Implementation

No JavaScript expression evaluation exists. Variables are simple string substitution only.

#### Recommendation: REMOVE FROM DOCS

**Reasoning:**
1. JavaScript evaluation in YAML configs is a security concern
2. Adds significant complexity to the parser
3. The simpler default value syntax (`${VAR:default}`) covers most use cases
4. Environment-specific configs are better handled via separate YAML files or environment variables

#### Action Items

Remove the following from documentation:
- JavaScript ternary expressions in variables
- `${ENV.NODE_ENV || 'default'}` syntax
- Any `===`, `?`, `:` conditional syntax in variable examples

Replace with simpler patterns:
- Use separate YAML files per environment
- Use environment variables directly
- Use the default value syntax once implemented

#### Affected Documentation

- `packages/docs/public/docs/variables.md` (lines 233-253, 299-304)
- `packages/docs/public/docs/global-settings.md` (lines 243-249)

---

## Issue 4: Implement String Manipulation for Variables

### Title
`feat: Implement string transformation for variables (upper/lower)`

### Labels
`enhancement`, `good first issue`

### Body

#### Summary

The documentation shows string manipulation transforms that are not implemented.

#### Documentation Claims (variables.md)

```yaml
global:
  variables:
    UPPER_ENV: "${ENV:upper}"
    LOWER_RESOURCE: "${RESOURCE:lower}"
```

#### Current Implementation

No string transformation is supported.

#### Recommendation: IMPLEMENT

Simple case transformations are useful for:
- Header values that need specific casing
- Normalizing user input
- Generating case-sensitive identifiers

#### Implementation Notes

In `packages/cli/src/parser/yaml.ts`, extend `resolveDynamicVariable()`:

```typescript
// String transforms: ${VAR:upper} or ${VAR:lower}
const transformMatch = varName.match(/^([^:]+):(upper|lower)$/);
if (transformMatch) {
  const baseVarName = transformMatch[1];
  const transform = transformMatch[2];
  const baseValue = variables[baseVarName] || process.env[baseVarName];

  if (baseValue) {
    return transform === 'upper' ? baseValue.toUpperCase() : baseValue.toLowerCase();
  }
}
```

#### Affected Documentation

- `packages/docs/public/docs/variables.md`

---

## Issue 5: Implement maxConcurrency for Parallel Execution

### Title
`feat: Implement maxConcurrency option for parallel execution`

### Labels
`enhancement`

### Body

#### Summary

The documentation describes a `maxConcurrency` option that limits simultaneous requests in parallel mode, but it's not implemented.

#### Documentation Claims (global-settings.md)

```yaml
global:
  execution: parallel
  maxConcurrency: 5  # Limit concurrent requests in parallel mode
```

#### Current Implementation (executor/request-executor.ts:539-546)

```typescript
async executeParallel(requests: RequestConfig[]): Promise<ExecutionSummary> {
  const startTime = performance.now();
  const promises = requests.map((request, index) => this.executeRequest(request, index + 1));
  const results = await Promise.all(promises);  // All requests fire at once
  return this.createSummary(results, performance.now() - startTime);
}
```

All requests are executed simultaneously with no concurrency limit.

#### Recommendation: IMPLEMENT

This is important for:
- Avoiding rate limiting from APIs
- Not overwhelming target servers
- Testing realistic concurrent load scenarios
- Respecting server resource constraints

#### Implementation Notes

Use a semaphore pattern or chunked execution:

```typescript
async executeParallel(requests: RequestConfig[]): Promise<ExecutionSummary> {
  const startTime = performance.now();
  const maxConcurrency = this.globalConfig.maxConcurrency || requests.length;
  const results: ExecutionResult[] = [];

  // Process in chunks
  for (let i = 0; i < requests.length; i += maxConcurrency) {
    const chunk = requests.slice(i, i + maxConcurrency);
    const chunkResults = await Promise.all(
      chunk.map((req, idx) => this.executeRequest(req, i + idx + 1))
    );
    results.push(...chunkResults);
  }

  return this.createSummary(results, performance.now() - startTime);
}
```

#### Affected Documentation

- `packages/docs/public/docs/global-settings.md`
- `packages/docs/public/docs/features/parallel-execution.md`

---

## Issue 6: Implement Exponential Backoff for Retries

### Title
`feat: Implement retryBackoff multiplier for exponential backoff`

### Labels
`enhancement`

### Body

#### Summary

The documentation describes exponential backoff for retries, but only fixed delay is implemented.

#### Documentation Claims (global-settings.md, retry-mechanism.md)

```yaml
global:
  retryDelay: 1000        # Initial delay between retries (ms)
  retryBackoff: 2.0       # Exponential backoff multiplier
```

This would produce delays: 1000ms, 2000ms, 4000ms, 8000ms...

#### Current Implementation (executor/request-executor.ts:100-105)

```typescript
if (attempt > 0) {
  requestLogger.logRetry(attempt, maxAttempts - 1);
  if (config.retry?.delay) {
    await Bun.sleep(config.retry.delay);  // Fixed delay, no backoff
  }
}
```

Only fixed delay is supported.

#### Recommendation: IMPLEMENT

Exponential backoff is:
- Industry standard for retry logic
- Essential for handling rate-limited APIs
- Better for server health during outages
- Already documented and expected by users

#### Implementation Notes

```typescript
if (attempt > 0) {
  requestLogger.logRetry(attempt, maxAttempts - 1);
  if (config.retry?.delay) {
    const backoff = config.retry.backoff || 1;  // Default: no backoff
    const delay = config.retry.delay * Math.pow(backoff, attempt - 1);
    await Bun.sleep(delay);
  }
}
```

Update `types/config.ts`:
```typescript
retry?: {
  count: number;
  delay?: number;
  backoff?: number;  // Add this
};
```

#### Affected Documentation

- `packages/docs/public/docs/global-settings.md`
- `packages/docs/public/docs/features/retry-mechanism.md`

---

## Issue 7: Remove Advanced Timeout Documentation

### Title
`docs: Remove undocumented connectionTimeout/readTimeout from docs`

### Labels
`documentation`

### Body

#### Summary

The documentation describes separate `connectionTimeout` and `readTimeout` settings that are not implemented.

#### Documentation Claims (global-settings.md)

```yaml
global:
  timeout: 10000
  connectionTimeout: 5000  # NOT IMPLEMENTED
  readTimeout: 15000       # NOT IMPLEMENTED
```

#### Current Implementation

Only a single `timeout` option exists, which maps to curl's `--max-time` flag.

#### Recommendation: REMOVE FROM DOCS

**Reasoning:**
1. curl doesn't have separate connection/read timeouts in the same way
2. The single `timeout` covers the total request time, which is sufficient for most use cases
3. Adding separate timeouts would require significant curl option complexity
4. Low user demand for this granularity

#### Action Items

Remove from `packages/docs/public/docs/global-settings.md`:
- `connectionTimeout` examples
- `readTimeout` examples

Keep only the standard `timeout` option.

---

## Issue 8: Implement SSL/TLS Certificate Configuration

### Title
`feat: Implement SSL/TLS certificate configuration options`

### Labels
`enhancement`

### Body

#### Summary

The documentation describes SSL/TLS certificate configuration that is not fully implemented.

#### Documentation Claims (global-settings.md, request-object.md)

```yaml
global:
  ssl:
    verify: true
    ca: "./certs/ca.pem"
    cert: "./certs/client.pem"
    key: "./certs/client-key.pem"
```

#### Current Implementation

Only `insecure: true` is implemented (maps to curl `-k` flag).

#### Recommendation: IMPLEMENT

This is important for:
- Enterprise environments with custom CA certificates
- Mutual TLS (mTLS) authentication
- Testing with self-signed certificates properly
- Security-conscious deployments

#### Implementation Notes

In `packages/cli/src/utils/curl-builder.ts`:

```typescript
if (config.ssl) {
  if (config.ssl.verify === false) {
    parts.push('-k');
  }
  if (config.ssl.ca) {
    parts.push('--cacert', config.ssl.ca);
  }
  if (config.ssl.cert) {
    parts.push('--cert', config.ssl.cert);
  }
  if (config.ssl.key) {
    parts.push('--key', config.ssl.key);
  }
}
```

Update `types/config.ts`:
```typescript
ssl?: {
  verify?: boolean;
  ca?: string;
  cert?: string;
  key?: string;
};
```

#### Affected Documentation

- `packages/docs/public/docs/global-settings.md`
- `packages/docs/public/docs/api-reference/request-object.md`

---

## Issue 9: Remove Rate Limiting Documentation

### Title
`docs: Remove undocumented rate limiting configuration from docs`

### Labels
`documentation`

### Body

#### Summary

The documentation describes rate limiting configuration that is not implemented.

#### Documentation Claims (global-settings.md)

```yaml
global:
  rateLimit:
    maxRequests: 100
    perSecond: 10
```

#### Current Implementation

No rate limiting functionality exists.

#### Recommendation: REMOVE FROM DOCS (for now)

**Reasoning:**
1. Rate limiting is complex to implement properly
2. `maxConcurrency` (Issue #5) provides partial solution
3. Can be revisited as a future feature
4. Users can implement client-side rate limiting via delays

#### Action Items

Remove from `packages/docs/public/docs/global-settings.md`:
- `rateLimit` configuration examples
- Any mentions of `maxRequests` or `perSecond`

Optionally, add a note that rate limiting is planned for future versions.

---

## Issue 10: Fix Tutorials Page Placeholder Content

### Title
`docs: Fix tutorials page showing template placeholders`

### Labels
`documentation`, `bug`

### Body

#### Summary

The tutorials page shows raw template placeholders instead of actual content.

#### Current State (tutorials.md)

```markdown
## Available Tutorials

{tutorial.description}

### {tutorial.title}

## Learning Paths

### {path.title}

{path.description}
```

These appear to be placeholders for a templating system that never populated them.

#### Recommendation: FIX

Options:
1. **Add actual tutorial content** - Write tutorials for common use cases
2. **Remove the page** - If no tutorials are planned
3. **Mark as "Coming Soon"** - With a clear message instead of broken placeholders

#### Suggested Tutorials (if option 1)

1. "Getting Started: Your First API Test"
2. "Authentication Workflows with Response Storage"
3. "CI/CD Integration with GitHub Actions"
4. "Performance Testing with Parallel Execution"
5. "Advanced Validation Patterns"

#### Affected Documentation

- `packages/docs/public/docs/tutorials.md`
- `packages/docs/public/docs/tutorials/coming-soon.md`

---

## Issue 11: Fix Use Cases Page Placeholder Content

### Title
`docs: Fix use cases page showing template placeholders`

### Labels
`documentation`, `bug`

### Body

#### Summary

The use cases page shows raw template placeholders instead of actual content.

#### Current State (use-cases.md)

```markdown
## Why Teams Choose curl-runner

{benefits[0].description}

{benefits[1].description}

## Popular Use Cases

### {useCase.title}

{useCase.description}

## Frequently Asked Questions

### {faq.question}

{faq.answer}
```

#### Recommendation: FIX

Same options as tutorials page:
1. Add actual use case content
2. Remove the page
3. Mark as "Coming Soon"

#### Affected Documentation

- `packages/docs/public/docs/use-cases.md`

---

## Issue 12: Fix CLI Flag Documentation Inconsistencies

### Title
`docs: Fix CLI flag naming inconsistencies between docs and implementation`

### Labels
`documentation`

### Body

#### Summary

Several CLI flags are documented differently than implemented.

#### Inconsistencies Found

| Documentation Says | Implementation Uses | Location |
|-------------------|---------------------|----------|
| `--retry 5` | `--retries 5` | cli-options.md, retry-mechanism.md |
| `--parallel` | `-p` or `--execution parallel` | parallel-execution.md |
| `--sequential` | `--execution sequential` | parallel-execution.md |

#### Current Implementation (cli.ts:334-428)

```typescript
// For retries
if (key === 'retries') {
  options.retries = Number.parseInt(nextArg, 10);
}

// For execution mode
case 'p':
  options.execution = 'parallel';
  break;
```

#### Recommendation: UPDATE DOCS

Update documentation to match implementation:
- Change `--retry` to `--retries`
- Change `--parallel` to `-p` or `--execution parallel`
- Change `--sequential` to `--execution sequential`

Alternatively, add the documented flags as aliases in the CLI.

#### Affected Documentation

- `packages/docs/public/docs/cli-options.md`
- `packages/docs/public/docs/features/parallel-execution.md`
- `packages/docs/public/docs/features/retry-mechanism.md`

---

## Issue 13: Remove ENV Object Syntax from Documentation

### Title
`docs: Remove undocumented ${ENV.VAR} syntax from documentation`

### Labels
`documentation`

### Body

#### Summary

The documentation shows an `ENV` object syntax that doesn't exist.

#### Documentation Claims (variables.md, global-settings.md)

```yaml
global:
  variables:
    ENVIRONMENT: ${ENV.NODE_ENV || 'development'}
```

#### Current Implementation

Environment variables are accessed directly via `${VAR_NAME}`. The parser checks `process.env` as a fallback for undefined static variables.

#### Recommendation: REMOVE FROM DOCS

**Reasoning:**
1. The `${VAR_NAME}` syntax already supports environment variables
2. Adding `ENV.` prefix adds no value
3. Simplifies documentation
4. Avoids confusion with non-existent syntax

#### Action Items

Replace `${ENV.VAR_NAME}` with `${VAR_NAME}` in all documentation.

#### Affected Documentation

- `packages/docs/public/docs/variables.md`
- `packages/docs/public/docs/global-settings.md`

---

## Issue 14: Remove Advanced Output Configuration Options

### Title
`docs: Remove undocumented output configuration options`

### Labels
`documentation`

### Body

#### Summary

Several output configuration options are documented but not implemented.

#### Documentation Claims (global-settings.md, output-formats.md)

```yaml
global:
  output:
    includeResponseBody: true    # NOT IMPLEMENTED (use showBody)
    includeRequestDetails: true  # NOT IMPLEMENTED
    showProgress: true           # NOT IMPLEMENTED
    timestampFormat: "ISO"       # NOT IMPLEMENTED
    colors: true                 # NOT IMPLEMENTED (hardcoded)
```

#### Current Implementation (types/config.ts:169-177)

```typescript
output?: {
  verbose?: boolean;
  showHeaders?: boolean;
  showBody?: boolean;
  showMetrics?: boolean;
  format?: 'json' | 'pretty' | 'raw';
  prettyLevel?: 'minimal' | 'standard' | 'detailed';
  saveToFile?: string;
};
```

#### Recommendation: REMOVE FROM DOCS

**Reasoning:**
1. `showBody` already provides the `includeResponseBody` functionality
2. `includeRequestDetails` is vague and not clearly needed
3. `showProgress` would require significant UI changes
4. `timestampFormat` is low priority
5. `colors` is always enabled in supported terminals

#### Action Items

Remove from documentation:
- `includeResponseBody` (use `showBody` instead)
- `includeRequestDetails`
- `showProgress`
- `timestampFormat`
- `colors`

#### Affected Documentation

- `packages/docs/public/docs/global-settings.md`
- `packages/docs/public/docs/features/output-formats.md`

---

## Priority Order for Implementation

1. **High Priority (Breaking UX/Docs)**
   - Issue 10: Fix Tutorials Page
   - Issue 11: Fix Use Cases Page
   - Issue 12: Fix CLI Flag Inconsistencies
   - Issue 5: Implement maxConcurrency

2. **Medium Priority (Feature Gaps)**
   - Issue 1: Dynamic Variable Generators
   - Issue 2: Default Value Syntax
   - Issue 6: Exponential Backoff
   - Issue 8: SSL/TLS Configuration

3. **Low Priority (Nice to Have)**
   - Issue 4: String Manipulation

4. **Documentation Updates (No Code)**
   - Issue 3: Remove JS Expressions
   - Issue 7: Remove Advanced Timeouts
   - Issue 9: Remove Rate Limiting
   - Issue 13: Remove ENV Object Syntax
   - Issue 14: Remove Advanced Output Options

---

## Quick Wins

These can be addressed quickly:

1. **CLI Flag Docs** (Issue 12) - Just update text
2. **Remove Undocumented Features** (Issues 3, 7, 9, 13, 14) - Delete text
3. **Placeholder Pages** (Issues 10, 11) - Either add content or "Coming Soon"
