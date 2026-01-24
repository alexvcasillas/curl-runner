# Request Snapshots - Implementation Plan

> Save response snapshots, compare future runs against them (like Jest snapshots). Perfect for regression testing.

## Overview

Save HTTP response snapshots and compare future runs against them. Enables regression testing for APIs.

---

## 1. Snapshot Configuration Schema

**Request-level config** (in YAML):
```yaml
request:
  name: Get User
  url: https://api.example.com/user/1
  snapshot:
    enabled: true
    name: "custom-name"          # Optional, defaults to request name
    include:                     # What to snapshot (default: body only)
      - body
      - status
      - headers
    exclude:                     # Paths to ignore (for dynamic values)
      - body.timestamp
      - body.createdAt
      - headers.date
      - headers.x-request-id
    match:                       # Flexible matching rules
      body.id: "*"               # Wildcard - any value
      body.version: "regex:^v\\d+"
```

**Global defaults** (`curl-runner.yaml` or collection global):
```yaml
global:
  snapshot:
    dir: "__snapshots__"         # Snapshot directory
    updateMode: "none"           # none | all | failing
    include: [body]              # Default includes
    exclude: []                  # Default excludes
```

---

## 2. CLI Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--snapshot` | `-s` | Enable snapshot mode |
| `--update-snapshots` | `-u` | Update all snapshots |
| `--update-failing` | | Update only failing snapshots |
| `--snapshot-dir <dir>` | | Custom snapshot directory |
| `--ci-snapshot` | | Fail if snapshots missing (CI mode) |

---

## 3. Snapshot File Format

**Location**: `__snapshots__/<yaml-filename>.snap.json`

**Structure**:
```json
{
  "version": 1,
  "snapshots": {
    "Get User": {
      "status": 200,
      "body": {
        "id": 1,
        "name": "John Doe",
        "email": "john@example.com"
      },
      "headers": {
        "content-type": "application/json"
      },
      "hash": "a1b2c3d4",
      "updatedAt": "2026-01-24T10:00:00Z"
    }
  }
}
```

---

## 4. File Structure (New Files)

```
packages/cli/src/
├── snapshot/
│   ├── snapshot-manager.ts      # Core: read/write/compare snapshots
│   ├── snapshot-differ.ts       # Deep diff with exclusions/wildcards
│   ├── snapshot-formatter.ts    # Pretty diff output
│   └── snapshot.test.ts         # Unit tests
├── types/
│   └── config.ts                # Add snapshot types (extend existing)
└── cli.ts                       # Add CLI flags
```

---

## 5. Core Implementation

### 5.1 `snapshot-manager.ts`

```typescript
interface SnapshotData {
  version: number;
  snapshots: Record<string, Snapshot>;
}

interface Snapshot {
  status?: number;
  headers?: Record<string, string>;
  body?: JsonValue;
  hash: string;
  updatedAt: string;
}

class SnapshotManager {
  constructor(snapshotDir: string);
  getSnapshotPath(yamlPath: string): string;
  load(yamlPath: string): Promise<SnapshotData | null>;
  save(yamlPath: string, data: SnapshotData): Promise<void>;
  get(yamlPath: string, requestName: string): Promise<Snapshot | null>;
  update(yamlPath: string, requestName: string, result: ExecutionResult, config: SnapshotConfig): Promise<void>;
  delete(yamlPath: string, requestName: string): Promise<void>;
  hash(content: unknown): string;
}
```

### 5.2 `snapshot-differ.ts`

```typescript
interface DiffResult {
  match: boolean;
  differences: Difference[];
}

interface Difference {
  path: string;
  expected: unknown;
  received: unknown;
  type: 'added' | 'removed' | 'changed' | 'type_mismatch';
}

class SnapshotDiffer {
  constructor(config: SnapshotConfig);
  compare(snapshot: Snapshot, result: ExecutionResult): DiffResult;
  deepCompare(expected: unknown, received: unknown, path: string): Difference[];
  isExcluded(path: string): boolean;
  matchesRule(path: string, value: unknown): boolean;
}
```

### 5.3 `snapshot-formatter.ts`

```typescript
class SnapshotFormatter {
  formatDiff(diff: DiffResult, requestName: string): string;
  formatSummary(stats: SnapshotStats): string;
  formatInlineDiff(expected: unknown, received: unknown): string;
}
```

**Example Output**:
```
Snapshot: Get User
   Mismatch found

   - Expected
   + Received

   body.email:
   - "john@example.com"
   + "john.doe@example.com"

   Run with --update-snapshots to update
```

---

## 6. Type Extensions (`types/config.ts`)

```typescript
interface SnapshotConfig {
  enabled?: boolean;
  name?: string;
  include?: ('body' | 'status' | 'headers')[];
  exclude?: string[];
  match?: Record<string, string>;
}

interface RequestConfig {
  snapshot?: SnapshotConfig | boolean;
}

interface GlobalSnapshotConfig extends SnapshotConfig {
  dir?: string;
  updateMode?: 'none' | 'all' | 'failing';
}
```

---

## 7. Execution Flow

```
1. Parse YAML with snapshot config
2. Execute request
3. If snapshot enabled:
   a. Load existing snapshot (if any)
   b. If no snapshot exists:
      - Create new (unless CI mode -> fail)
   c. If snapshot exists:
      - Compare response against snapshot
      - If mismatch:
        - updateMode=all -> update snapshot
        - updateMode=failing -> update snapshot
        - updateMode=none -> report failure
4. Continue with existing validation (expect)
5. Output snapshot status in results
```

---

## 8. Implementation Order

| Phase | Tasks | Files |
|-------|-------|-------|
| **1** | Types & interfaces | `types/config.ts` |
| **2** | Snapshot manager (read/write) | `snapshot/snapshot-manager.ts` |
| **3** | Differ (comparison logic) | `snapshot/snapshot-differ.ts` |
| **4** | Formatter (diff output) | `snapshot/snapshot-formatter.ts` |
| **5** | CLI flags | `cli.ts` |
| **6** | Executor integration | `executor/request-executor.ts` |
| **7** | Logger integration | `utils/logger.ts` |
| **8** | Unit tests | `snapshot/*.test.ts` |
| **9** | Integration tests | E2E test files |
| **10** | Documentation | `README.md`, examples |
| **11** | Changeset | `.changeset/` |

---

## 9. Example Usage

**Basic** (`examples/snapshot-basic.yaml`):
```yaml
request:
  name: Get Users
  url: https://jsonplaceholder.typicode.com/users/1
  snapshot: true
```

**Advanced** (`examples/snapshot-advanced.yaml`):
```yaml
global:
  snapshot:
    dir: "__snapshots__"
    exclude:
      - "*.timestamp"
      - "*.createdAt"

collection:
  name: User API Snapshots
  requests:
    - name: Get User
      url: https://api.example.com/users/1
      snapshot:
        include: [status, body]
        exclude:
          - body.lastLogin
        match:
          body.id: "*"
          body.version: "regex:^v\\d+\\.\\d+"
```

**CLI Commands**:
```bash
# Run with snapshots
curl-runner --snapshot api.yaml

# Update all snapshots
curl-runner --snapshot -u api.yaml

# Update only failing
curl-runner --snapshot --update-failing api.yaml

# CI mode (fail if no snapshot)
curl-runner --snapshot --ci-snapshot api.yaml
```

---

## 10. Edge Cases

| Case | Behavior |
|------|----------|
| Missing snapshot file | Create new (unless `--ci-snapshot`) |
| Request name changed | Treated as new snapshot |
| Array order differs | Fail (unless using `[*]` wildcard) |
| New field in response | Fail (shows as "added") |
| Removed field | Fail (shows as "removed") |
| Binary response | Hash comparison only |
| Empty response | Snapshot empty body |
| Non-JSON response | Store as string |

---

## 11. Testing Strategy

**Unit Tests**:
- `snapshot-manager.test.ts`: File I/O, hashing, CRUD ops
- `snapshot-differ.test.ts`: Deep comparison, exclusions, wildcards, regex
- `snapshot-formatter.test.ts`: Diff output formatting

**Integration Tests**:
- New snapshot creation
- Snapshot matching (pass)
- Snapshot mismatch (fail)
- Update modes (all, failing, none)
- CI mode behavior
- Sequential execution with snapshots
- Parallel execution with snapshots

---

## 12. Documentation Updates

| File | Changes |
|------|---------|
| `README.md` | Add snapshots section |
| `packages/cli/README.md` | CLI flags, config reference |
| `examples/README.md` | Snapshot examples guide |
| New: `examples/snapshot-*.yaml` | 3-4 example files |

---

## 13. GitHub Workflow Checks

```bash
bun run format        # Biome formatting
bun run lint          # Biome linting
bun run check:ci      # Biome check
bun run build:cli     # Build CLI
bun run build:docs    # Build docs
bun run test          # All tests
```

---

## 14. Design Decisions (Jest-inspired)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Snapshot storage | **Per-file** | Like Jest - smaller PR diffs, less merge conflicts |
| Binary responses | **Hash only** | Keeps snapshots readable; hash sufficient for change detection |
| Header normalization | **Lowercase + sorted** | HTTP headers case-insensitive; ensures deterministic output |
| Array wildcard syntax | **`body[*].id`** | JSONPath standard; `body.*.id` ambiguous with object keys |
| Watch mode | **No auto-update** | Would defeat purpose; show "press u to update" hint |
| Parallel writes | **Queue per file** | Simple mutex prevents corruption |
| Metrics in snapshot | **No** | Too volatile (network latency); causes false failures |
| Interactive mode | **TTY only** | Like Jest - prompt in terminal, silent fail in CI |
