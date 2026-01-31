# refactor: modularize curl-runner core for better testability and maintainability

## Summary

Refactor curl-runner core to improve modularization, testability, and maintainability. The current architecture has grown organically with tightly coupled components that make it difficult to add new features and deeply test existing functionality.

## Current State Analysis

### Key Issues

| File | Lines | Problem |
|------|-------|---------|
| `cli.ts` | 1462 | God Object - handles config, parsing, routing, execution |
| `request-executor.ts` | 765 | Mixed concerns - strategies, validation, snapshots, logging |
| `CurlBuilder` + `PooledCurlExecutor` | - | Duplicated curl building logic (~80% overlap) |
| Config handling | - | 50+ env vars mixed with CLI parsing, scattered merge logic |

### Tight Coupling

- `RequestExecutor` ↔ `SnapshotManager` (direct instantiation)
- `RequestExecutor` ↔ `Logger` (created per-request)
- `CurlBuilder` ↔ `PooledCurlExecutor` (code duplication)
- CLI ↔ Config (env vars, file loading, CLI args all in one place)

### What Works Well

- `diff/` module - good separation (4 classes, clear responsibilities)
- `watcher/` module - independent, cleanly integrated
- `condition-evaluator.ts` - well-isolated utility
- Types in `types/config.ts` - comprehensive, well-documented

---

## Modularization Plan

### Phase 1: Config Layer Extraction

**Goal:** Extract config handling from CLI into dedicated modules

**New Structure:**
```
packages/cli/src/
├── core/
│   ├── config/
│   │   ├── index.ts              # Re-exports
│   │   ├── defaults.ts           # Default values centralized (~50 lines)
│   │   ├── env-loader.ts         # Environment variable parsing (~120 lines)
│   │   ├── file-loader.ts        # Config file discovery & loading (~40 lines)
│   │   ├── cli-parser.ts         # CLI argument parsing, typed (~200 lines)
│   │   └── resolver.ts           # Merges env + file + CLI (~100 lines)
│   └── index.ts
├── cli.ts                        # Thin wrapper (~300 lines after extraction)
└── ...
```

**Key Changes:**

1. **`defaults.ts`** - Centralize all default values
   ```ts
   export const CONFIG_DEFAULTS = {
     execution: 'sequential',
     maxConcurrency: undefined,
     continueOnError: false,
     timeout: 30000,
     watchDebounce: 300,
     profileIterations: 10,
     profileWarmup: 1,
     snapshotDir: '__snapshots__',
     baselineDir: '__baselines__',
     // ...
   } as const;
   ```

2. **`cli-parser.ts`** - Type-safe CLI options (manual parsing, no external deps)
   ```ts
   export interface CLIOptions {
     files: string[];
     all?: boolean;

     // Execution
     execution?: 'sequential' | 'parallel';
     maxConcurrency?: number;
     dryRun?: boolean;
     http2?: boolean;
     connectionPool?: boolean;

     // Output
     verbose?: boolean;
     quiet?: boolean;
     outputFormat?: 'json' | 'pretty' | 'raw';
     prettyLevel?: 'minimal' | 'standard' | 'detailed';

     // Watch
     watch?: boolean;
     watchDebounce?: number;
     watchClear?: boolean;

     // Profile
     profile?: number;
     profileWarmup?: number;
     profileConcurrency?: number;
     profileHistogram?: boolean;
     profileExport?: string;

     // Snapshot
     snapshot?: boolean;
     snapshotUpdate?: 'none' | 'all' | 'failing';
     snapshotDir?: string;
     snapshotCi?: boolean;

     // Diff
     diff?: boolean;
     diffSave?: boolean;
     diffLabel?: string;
     diffCompare?: string;
     diffDir?: string;
     diffOutput?: 'terminal' | 'json' | 'markdown';

     // CI
     strictExit?: boolean;
     failOn?: number;
     failOnPercentage?: number;

     // Retry
     timeout?: number;
     retries?: number;
     retryDelay?: number;
     noRetry?: boolean;

     // Connection Pool
     maxStreams?: number;
     keepaliveTime?: number;
     connectTimeout?: number;

     // Meta
     help?: boolean;
     version?: boolean;
   }

   export function parseCliArgs(args: string[]): CLIOptions;
   ```

3. **`env-loader.ts`** - Extract from `loadEnvironmentVariables()`
   ```ts
   export function loadEnvironmentConfig(): Partial<GlobalConfig>;
   ```

4. **`file-loader.ts`** - Config file discovery
   ```ts
   export const CONFIG_PATHS = [
     'curl-runner.config.yaml',
     'curl-runner.config.yml',
     '.curl-runner.yaml',
     '.curl-runner.yml',
   ];

   export async function loadConfigFile(): Promise<Partial<GlobalConfig> | null>;
   ```

5. **`resolver.ts`** - Unified config resolution
   ```ts
   export interface ResolvedConfig {
     config: GlobalConfig;
     files: string[];
     mode: 'normal' | 'watch' | 'profile' | 'diff-subcommand' | 'upgrade' | 'help' | 'version';
   }

   export class ConfigResolver {
     static async resolve(args: string[]): Promise<ResolvedConfig>;
   }
   ```

6. **`cli.ts`** - Becomes thin orchestrator
   ```ts
   class CurlRunnerCLI {
     async run(args: string[]) {
       const { config, files, mode } = await ConfigResolver.resolve(args);

       switch (mode) {
         case 'help': return this.showHelp();
         case 'version': return this.showVersion();
         case 'upgrade': return this.handleUpgrade(args);
         case 'diff-subcommand': return this.handleDiffSubcommand(config, files);
         case 'watch': return this.handleWatch(config, files);
         case 'profile': return this.handleProfile(config, files);
         default: return this.handleNormal(config, files);
       }
     }
   }
   ```

**Types:** Keep in `types/config.ts` (already well-organized, 694 lines)

---

### Phase 2: Curl Command Building DRY-up

**Goal:** Eliminate duplication between `CurlBuilder` and `PooledCurlExecutor`

**New Structure:**
```
packages/cli/src/
├── core/
│   ├── curl/
│   │   ├── index.ts
│   │   ├── args-builder.ts       # Shared arg building logic
│   │   ├── response-parser.ts    # Extract response/metrics parsing
│   │   └── types.ts              # Curl-specific types
```

**Key Changes:**

1. **`args-builder.ts`** - Shared curl argument building
   ```ts
   export interface CurlArgs {
     args: string[];
     writeOutFormat: string;
   }

   export function buildCurlArgs(request: RequestConfig, options?: CurlOptions): CurlArgs;
   ```

2. **`response-parser.ts`** - Extract parsing logic
   ```ts
   export function parseResponse(output: string): ParsedResponse;
   export function parseMetrics(metricsLine: string): RequestMetrics;
   ```

3. **Refactor existing files:**
   - `utils/curl-builder.ts` → uses `buildCurlArgs()`
   - `executor/pooled-curl-executor.ts` → uses `buildCurlArgs()` + batching logic

---

### Phase 3: Interpolator Extraction

**Goal:** Separate variable interpolation from YAML parsing

**New Structure:**
```
packages/cli/src/
├── core/
│   ├── interpolator/
│   │   ├── index.ts
│   │   ├── static.ts             # ${VAR}, ${VAR:default}
│   │   ├── dynamic.ts            # ${UUID}, ${TIMESTAMP}, ${RANDOM:1-100}
│   │   ├── store.ts              # ${store.key}
│   │   └── transforms.ts         # ${VAR|upper}, ${VAR|lower}
```

**Benefits:**
- Testable without YAML parsing
- Reusable for non-YAML sources
- Clear separation of concerns

---

### Phase 4: Execution Strategies

**Goal:** Extract execution modes into strategy pattern

**New Structure:**
```
packages/cli/src/
├── core/
│   ├── executor/
│   │   ├── index.ts
│   │   ├── strategy.ts           # ExecutionStrategy interface
│   │   ├── sequential.ts         # SequentialStrategy
│   │   ├── parallel.ts           # ParallelStrategy
│   │   ├── pooled.ts             # PooledStrategy
│   │   ├── runner.ts             # Single request execution
│   │   └── coordinator.ts        # Strategy coordinator
```

**Key Changes:**

1. **`strategy.ts`** - Strategy interface
   ```ts
   export interface ExecutionStrategy {
     execute(requests: RequestConfig[], context: ExecutionContext): Promise<ExecutionResult[]>;
   }
   ```

2. **`runner.ts`** - Single request execution (~300 lines)
   - Execute single request
   - Validation
   - Retry logic
   - Store value extraction

3. **`coordinator.ts`** - Orchestrates strategies (~200 lines)
   - Selects strategy based on config
   - Manages store context
   - Coordinates with snapshot/diff services

**Current `request-executor.ts` (765 lines) splits into:**
- `coordinator.ts` (~200 lines)
- `runner.ts` (~300 lines)
- `sequential.ts` (~100 lines)
- `parallel.ts` (~80 lines)
- `pooled.ts` (~80 lines)

---

### Phase 5: Validation Layer

**Goal:** Extract validation into pluggable validators

**New Structure:**
```
packages/cli/src/
├── core/
│   ├── validation/
│   │   ├── index.ts
│   │   ├── validator.ts          # Validator interface
│   │   ├── status.ts             # Status code validation
│   │   ├── headers.ts            # Header validation
│   │   ├── body.ts               # Body validation (JSON path, regex)
│   │   ├── timing.ts             # Response time validation
│   │   └── composite.ts          # Combines validators
```

**Key Changes:**

```ts
export interface Validator {
  validate(response: Response, expect: ExpectConfig): ValidationResult;
}

export interface ValidationResult {
  passed: boolean;
  errors: ValidationError[];
}
```

---

### Phase 6: Snapshot Service Interface

**Goal:** Decouple snapshot from RequestExecutor

**Changes:**
- Create `SnapshotService` interface
- Inject into `RequestRunner` instead of direct instantiation
- Enables mocking in tests

```ts
export interface SnapshotService {
  compare(name: string, response: Response, config: SnapshotConfig): Promise<SnapshotCompareResult>;
  update(name: string, response: Response, config: SnapshotConfig): Promise<void>;
}
```

---

### Phase 7: Output Formatters

**Goal:** Decouple output formatting from execution

**New Structure:**
```
packages/cli/src/
├── core/
│   ├── output/
│   │   ├── index.ts
│   │   ├── formatter.ts          # OutputFormatter interface
│   │   ├── console.ts            # Terminal output (current Logger logic)
│   │   ├── json.ts               # JSON output
│   │   └── quiet.ts              # Minimal output
```

---

## Final Directory Structure

```
packages/cli/src/
├── core/
│   ├── config/
│   │   ├── index.ts
│   │   ├── defaults.ts
│   │   ├── env-loader.ts
│   │   ├── file-loader.ts
│   │   ├── cli-parser.ts
│   │   └── resolver.ts
│   ├── curl/
│   │   ├── index.ts
│   │   ├── args-builder.ts
│   │   ├── response-parser.ts
│   │   └── types.ts
│   ├── interpolator/
│   │   ├── index.ts
│   │   ├── static.ts
│   │   ├── dynamic.ts
│   │   ├── store.ts
│   │   └── transforms.ts
│   ├── executor/
│   │   ├── index.ts
│   │   ├── strategy.ts
│   │   ├── sequential.ts
│   │   ├── parallel.ts
│   │   ├── pooled.ts
│   │   ├── runner.ts
│   │   └── coordinator.ts
│   ├── validation/
│   │   ├── index.ts
│   │   ├── validator.ts
│   │   ├── status.ts
│   │   ├── headers.ts
│   │   ├── body.ts
│   │   ├── timing.ts
│   │   └── composite.ts
│   ├── output/
│   │   ├── index.ts
│   │   ├── formatter.ts
│   │   ├── console.ts
│   │   ├── json.ts
│   │   └── quiet.ts
│   └── index.ts
├── cli.ts                        # Thin orchestrator (~300 lines)
├── types/config.ts               # Keep existing (well-organized)
├── parser/yaml.ts                # Simplified (interpolation extracted)
├── executor/                     # Legacy, gradually migrate
├── snapshot/                     # Keep, add interface
├── diff/                         # Keep (already well-structured)
├── watcher/                      # Keep (already well-isolated)
├── utils/                        # Gradually migrate to core/
└── commands/                     # Keep (upgrade, etc.)
```

---

## Testing Strategy

| Module | Test Type | Benefit |
|--------|-----------|---------|
| `cli-parser.ts` | Unit | Test arg parsing without execution |
| `env-loader.ts` | Unit | Test env var parsing with mocked env |
| `resolver.ts` | Unit | Test merge logic with fixtures |
| `args-builder.ts` | Unit | Test curl command generation |
| `response-parser.ts` | Unit | Test parsing with fixtures |
| `interpolator/*` | Unit | Test each interpolation type |
| `validator/*` | Unit | Test validators without HTTP |
| `sequential.ts` | Unit | Test strategy with mocked runner |
| `parallel.ts` | Unit | Test concurrency logic |
| `runner.ts` | Integration | Test single request execution |

**Existing tests:** Update import paths, keep integration tests

---

## Implementation Order

| Phase | Scope | Risk | Estimated Files |
|-------|-------|------|-----------------|
| 1 | Config extraction | Low | 6 new + 1 refactor |
| 2 | Curl DRY-up | Low | 3 new + 2 refactor |
| 3 | Interpolator extraction | Low | 5 new + 1 refactor |
| 4 | Execution strategies | Medium | 7 new + 1 refactor |
| 5 | Validation layer | Medium | 7 new + 1 refactor |
| 6 | Snapshot interface | Low | 1 new + 2 refactor |
| 7 | Output formatters | Low | 5 new + 1 refactor |

**After each phase batch:** Run `bun run typecheck && bun run lint`

---

## Decisions Made

| Topic | Decision | Rationale |
|-------|----------|-----------|
| Types location | Keep in `types/config.ts` | Already comprehensive, avoid churn |
| Arg parser | Manual (no external deps) | Works in compiled Bun binary |
| Subcommands | Route in cli.ts | Simple routing, handlers in commands/ |
| New directory | `core/` | Clear separation from legacy code |
| Backward compat exports | Not needed | No breaking changes for end users |
| Test batching | After all phases | Efficient, batch typecheck + lint |
| DI container | Manual injection | Codebase not large enough for framework |

---

## Non-Goals

- Full rewrite of working modules (diff/, watcher/)
- External dependencies for arg parsing
- DI framework
- Breaking changes for end users

---

## Success Criteria

1. `cli.ts` reduced from ~1460 to ~300 lines
2. `request-executor.ts` split into focused modules
3. No curl building duplication
4. All existing tests pass
5. Type checking passes
6. Linting passes
7. Each new module has corresponding unit tests
