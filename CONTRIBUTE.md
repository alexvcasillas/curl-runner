# Contributing to curl-runner 🤝

Thank you for considering contributing to curl-runner! We welcome contributions from everyone, whether you're fixing bugs, adding features, improving documentation, or helping with issues.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Submitting Changes](#submitting-changes)
- [Releasing](#releasing)
- [Issue Guidelines](#issue-guidelines)
- [Community](#community)

## 🤝 Code of Conduct

This project and everyone participating in it is governed by our Code of Conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

**Our Standards:**

- Be respectful and inclusive
- Welcome newcomers and help them learn
- Focus on what is best for the community
- Show empathy towards other community members
- Give and receive constructive feedback gracefully

## 🚀 Getting Started

### Prerequisites

- **[Rust](https://rustup.rs)** (the toolchain is pinned in `rust-toolchain.toml`; needed for the CLI)
- **[Node](https://nodejs.org)** 24 and **[pnpm](https://pnpm.io)** (needed for the documentation site and repo tooling only; the CLI itself does not use them)
- **Git** for version control
- **Basic understanding** of HTTP and REST APIs

### Quick Setup

```bash
# Fork the repository on GitHub
# Clone your fork
git clone https://github.com/YOUR_USERNAME/curl-runner.git
cd curl-runner

# Add upstream remote
git remote add upstream https://github.com/yourusername/curl-runner.git

# Install dependencies (docs site and tooling)
pnpm install

# Verify everything works
cargo test --workspace
pnpm --filter @curl-runner/docs build
```

## 🛠️ Development Setup

### Workspace Overview

curl-runner is a monorepo: a Rust workspace for the CLI and a pnpm workspace for the npm wrapper and the documentation site:

```
crates/
├── curl-runner/         # CLI binary (args, commands, output, wizard)
├── curl-runner-core/    # Parser, executor, transport (libcurl), validation
└── testkit/             # Local test server and helpers
packages/
├── cli/                 # npm wrapper of the native binary (@curl-runner/cli)
└── docs/               # Documentation website (@curl-runner/docs)
parity/                  # TS vs Rust parity harness (see parity/README.md)
```

### Development Commands

```bash
# Install dependencies for all packages
pnpm install

# CLI (Rust)
cargo run -p curl-runner -- <args>   # or: pnpm cli <args>
cargo build --release -p curl-runner # or: pnpm build:cli
cargo fmt --all                      # Format
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace               # or: pnpm test

# Documentation site
pnpm --filter @curl-runner/docs dev         # Development server
pnpm --filter @curl-runner/docs build       # Build documentation site

# Code quality (TypeScript: docs site and tooling)
pnpm format           # Format with Biome
pnpm lint             # Lint with Biome
pnpm check            # Format and fix issues
pnpm check:ci         # CI-friendly check
```

## 📁 Project Structure

### CLI (`crates/`)

```
crates/curl-runner/          # binary: main.rs, args.rs, run.rs, commands, output
crates/curl-runner-core/     # library: parser, executor, transport, validate, snapshot, diff
crates/testkit/              # test server used by integration tests and parity
packages/cli/
├── bin/curl-runner          # npm launcher (POSIX sh)
├── examples/                # Example YAML files
└── README.md
```

### Documentation Package (`packages/docs/`)

```
packages/docs/
├── app/                # Next.js app directory
├── components/         # React components
│   ├── ui/            # Reusable UI components
│   └── docs/          # Documentation-specific components
├── lib/               # Utilities and helpers
├── styles/            # Global styles
└── package.json
```

## 🔄 Development Workflow

### 1. Choose What to Work On

- Check [Issues](https://github.com/yourusername/curl-runner/issues) for bugs and feature requests
- Look for issues labeled `good first issue` if you're new
- Check [Discussions](https://github.com/yourusername/curl-runner/discussions) for ideas
- Propose new features by opening an issue first

### 2. Create a Branch

```bash
# Create a feature branch
git checkout -b feature/amazing-new-feature

# Or a bug fix branch
git checkout -b fix/issue-123
```

### 3. Make Changes

- Write code following our [coding standards](#coding-standards)
- Add tests for new functionality
- **📚 Update documentation as needed** (see [Documentation Requirements](#documentation-requirements))
- Ensure all tests pass

#### 📚 Documentation Requirements

**⚠️ CRITICAL:** Any changes to the curl-runner source code MUST include corresponding documentation updates. This ensures users always have accurate, up-to-date information.

**When making code changes, you must:**

1. **Update CLI Documentation:**

   - Update `packages/cli/README.md` if CLI functionality changes
   - Update the main `README.md` with new features or options
   - Add/update YAML examples in `packages/cli/examples/`

2. **Update Website Documentation:**

   - Update relevant pages in `packages/docs/app/`
   - Update component documentation if UI changes
   - Ensure code examples match the current API

3. **Verify Documentation Accuracy:**

   - Test all code examples in documentation
   - Ensure YAML examples are valid and work with your changes
   - Check that CLI option descriptions match actual behavior
   - Validate that configuration references are complete

4. **Cross-Reference Check:**
   - CLI README ↔ Website docs consistency
   - Main README ↔ Package README alignment
   - Example files ↔ Documentation examples match

**Documentation Checklist for Code Changes:**

- [ ] CLI README updated for new features/options
- [ ] Main README reflects changes
- [ ] Website documentation pages updated
- [ ] YAML examples added/updated as needed
- [ ] All code examples tested and working
- [ ] Cross-references between docs are consistent
- [ ] API documentation (JSDoc) updated

### 4. Test Your Changes

```bash
# Run all tests
cargo test --workspace

# Test CLI functionality
cargo run -p curl-runner -- packages/cli/examples/

# Test documentation site
pnpm --filter @curl-runner/docs dev  # Visit http://localhost:3000

# Check code quality
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
pnpm check
```

### 5. Commit Changes

We use conventional commits:

```bash
git commit -m "feat: add parallel execution mode"
git commit -m "fix: handle empty YAML files gracefully"
git commit -m "docs: update API reference examples"
git commit -m "test: add integration tests for collections"
```

**Commit types:**

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `style:` Code style changes
- `refactor:` Code refactoring
- `test:` Adding or updating tests
- `chore:` Maintenance tasks

## 📏 Coding Standards

### Rust (CLI)

- `cargo fmt --all` before every commit (rustfmt defaults)
- `cargo clippy --workspace --all-targets -- -D warnings` must pass
- `cargo test --workspace` must pass; add unit tests next to the code and integration tests under `crates/*/tests`
- Behavior changes versus the former TypeScript CLI are logged in `crates/CHANGES-FROM-TS.md`

### TypeScript/JavaScript (docs site and tooling)

- **Indentation:** 2 spaces (configured in Biome)
- **Quotes:** Single quotes for strings
- **Semicolons:** No trailing semicolons
- **Line length:** 80 characters maximum
- **Naming conventions:**
  - `camelCase` for variables and functions
  - `PascalCase` for classes and types
  - `kebab-case` for file names

### Code Quality

```bash
# Rust
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings

# TypeScript (docs site)
pnpm format
pnpm check
pnpm lint
```

### File Organization

- Keep related code together
- Use clear, descriptive file names
- Export main functionality from index files
- Separate types into `.types.ts` files when needed

### Example Code Style

```typescript
// Good
interface RequestConfig {
  name: string;
  url: string;
  method: HttpMethod;
  headers?: Record<string, string>;
}

async function executeRequest(config: RequestConfig): Promise<Response> {
  const { url, method, headers } = config;

  try {
    const response = await fetch(url, {
      method,
      headers,
    });

    return response;
  } catch (error) {
    throw new Error(`Request failed: ${error.message}`);
  }
}
```

## 🧪 Testing

### Writing Tests

- Write unit tests for all new functions
- Add integration tests for CLI functionality
- Include edge cases and error scenarios
- Use descriptive test names

### Test Structure

Unit tests live in `#[cfg(test)]` modules next to the code; integration tests live in
`crates/*/tests` and use the `testkit` server.

```rust
#[test]
fn parses_a_single_request() {
    let doc = parse_yaml("request:\n  url: https://example.com\n  method: GET\n").unwrap();
    assert_eq!(doc.requests().len(), 1);
}

#[test]
fn rejects_invalid_yaml() {
    assert!(parse_yaml("invalid: yaml: content:").is_err());
}
```

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run one crate or test
cargo test -p curl-runner-core parser
cargo test -p curl-runner --test convert_cli
```

## 📚 Documentation

### Types of Documentation

1. **Code Documentation:** JSDoc comments for public APIs
2. **README Updates:** Keep package READMEs current
3. **Website Documentation:** Update the docs site for new features
4. **Example Files:** Add YAML examples for new functionality

### Writing Documentation

- Use clear, concise language
- Provide practical examples
- Include both simple and advanced use cases
- Test all code examples

### Documentation Structure

````typescript
/**
 * Executes HTTP requests based on YAML configuration
 *
 * @param configPath Path to YAML configuration file
 * @param options Execution options
 * @returns Promise resolving to execution results
 *
 * @example
 * ```typescript
 * const results = await executeRequests('config.yaml', {
 *   parallel: true,
 *   verbose: true
 * })
 * ```
 */
async function executeRequests(
  configPath: string,
  options: ExecutionOptions
): Promise<ExecutionResult[]>;
````

## 📤 Submitting Changes

### Pull Request Process

1. **Update Documentation:** Ensure docs reflect your changes
2. **Add Tests:** Include tests for new functionality
3. **Check Code Quality:** Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` (plus `pnpm check` for TypeScript changes)
4. **Update Examples:** Add/update YAML examples if needed
5. **Write Clear Description:** Explain what and why

### Pull Request Template

```markdown
## Description

Brief description of changes

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing

- [ ] All tests pass
- [ ] Added tests for new functionality
- [ ] Manual testing completed
- [ ] YAML examples tested and working

## Documentation (Required for Code Changes)

- [ ] CLI README updated if functionality changed
- [ ] Main README.md updated with new features
- [ ] Website documentation updated
- [ ] YAML examples added/updated as needed
- [ ] All code examples in docs tested
- [ ] Cross-references between docs verified
- [ ] JSDoc/API documentation updated

## Checklist

- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] **Documentation requirements fulfilled (mandatory for code changes)**
- [ ] No breaking changes (or marked as such)
```

### Review Process

1. **Automated Checks:** CI will run tests and linting
2. **Code Review:** Maintainers will review your code
3. **Feedback:** Address any requested changes
4. **Approval:** Once approved, we'll merge your PR

## 🚢 Releasing

Releases run from `.github/workflows/release.yml` (changesets). `@curl-runner/cli`
ships the native binary through five optional platform packages
(`@curl-runner/cli-{linux-x64,linux-arm64,darwin-x64,darwin-arm64,win32-x64}`,
sources in `npm/`), published in lockstep by `scripts/npm-publish.sh`. See
`packages/cli/README.md` ("How the npm package works").

### Before the first release

- [ ] **Bootstrap the five platform package names on npm.** CI publishes with
      npm trusted publishing (OIDC, no token), which can only be configured on a
      package that already exists. For each `@curl-runner/cli-<platform>`:
  1. Publish a placeholder once by hand, e.g. from `npm/cli-<platform>/` with
     version `0.0.0` (`npm publish --access public`), so the name exists.
  2. On npmjs.com → package → Settings → Trusted publishing, add GitHub
     Actions: repo `alexvcasillas/curl-runner`, workflow `release.yml`,
     environment `Production` (same as `@curl-runner/cli`).
- [ ] **Bootstrap the crates on crates.io.** CI publishes with crates.io
      trusted publishing, which can only be configured on a crate that
      already exists. Before merging the first version PR (2.0.0):
  1. On the version PR branch (Cargo.toml already at 2.0.0), with a token
     from `cargo login`: `cargo publish -p curl-runner-core && cargo publish -p curl-runner`.
  2. On crates.io → each crate → Settings → Trusted Publishing, add GitHub:
     repo `alexvcasillas/curl-runner`, workflow `release.yml`, environment
     `Production`.
  3. Merge the version PR; `scripts/crates-publish.sh` skips the versions
     already on crates.io.
- [ ] **Run the release smoke:** `gh workflow run release.yml --ref <branch> -f smoke=true`.
      It builds all binaries, smoke-tests the archives/installers, runs
      `scripts/npm-smoke.sh` (bun without node, npm without bun, pnpm) on
      linux-x64 and linux-arm64, and runs `cargo publish --workspace --dry-run`.
      It never publishes.
- [ ] **Know the gaps:** the `darwin-*` and `win32-x64` npm packages are not
      smoke-tested anywhere (CI or locally); only the linux packages are
      installed and run. Check `npm i -g @curl-runner/cli` on a Mac and on
      Windows after the first release. On Windows the npm/bun command needs `sh`
      on PATH; `install.ps1` is the supported route without it.

If a publish fails midway, re-run the failed jobs: versions already on npm or
crates.io are skipped, and `@curl-runner/cli` is only published after all platform
packages. The version PR bumps `packages/cli/package.json` and, through
`scripts/sync-cargo-version.sh`, the Cargo workspace version.

## 🐛 Issue Guidelines

### Bug Reports

Use the bug report template and include:

- **Environment:** OS, curl-runner version
- **Steps to Reproduce:** Clear, minimal steps
- **Expected Behavior:** What should happen
- **Actual Behavior:** What actually happens
- **YAML Config:** Include relevant configuration (sanitized)

### Feature Requests

Use the feature request template and include:

- **Problem:** What problem does this solve?
- **Solution:** Proposed solution or approach
- **Alternatives:** Other approaches considered
- **Use Case:** Real-world scenarios where this helps

### Questions

- Check existing issues and discussions first
- Use [Discussions](https://github.com/yourusername/curl-runner/discussions) for questions
- Provide context about what you're trying to achieve

## 🌟 Types of Contributions

### Code Contributions

- **Bug fixes:** Fix reported issues
- **New features:** Add CLI options, YAML features, etc.
- **Performance:** Optimize execution speed or memory usage
- **Developer experience:** Better error messages, debugging tools

### Documentation Contributions

- **API documentation:** Improve code documentation
- **User guides:** Write tutorials and how-tos
- **Examples:** Add real-world YAML configurations
- **Website:** Improve the documentation site

### Community Contributions

- **Issue triage:** Help categorize and reproduce issues
- **Code review:** Review pull requests from other contributors
- **Support:** Help users in issues and discussions
- **Testing:** Test beta features and provide feedback

## 🏆 Recognition

Contributors are recognized in:

- GitHub contributors list
- Release notes for significant contributions
- Documentation credits
- Special mentions in project announcements

## 📞 Community

### Communication Channels

- **GitHub Issues:** Bug reports and feature requests
- **GitHub Discussions:** Questions, ideas, and general chat
- **Pull Requests:** Code review and collaboration

### Getting Help

- **Documentation:** Check the website first
- **Discussions:** Ask questions in GitHub Discussions
- **Issues:** Report bugs or request features
- **Code Review:** Tag maintainers in PRs for review

## 📝 Development Tips

### Working with the CLI

```bash
# Test CLI changes quickly
cargo run -p curl-runner -- packages/cli/examples/simple.yaml

# Debug with verbose output
cargo run -p curl-runner -- -v packages/cli/examples/

# Test parallel execution
cargo run -p curl-runner -- -p packages/cli/examples/
```

### Working with Documentation

```bash
# Start docs development server
pnpm --filter @curl-runner/docs dev

# Build docs for production
pnpm --filter @curl-runner/docs build
```

### Debugging

- Use `dbg!` / `eprintln!` for quick debugging (CLI) and `console.log` (docs site)
- Add test cases to reproduce issues
- Run the parity harness (`parity/run.sh`, see `parity/README.md`) when touching runtime behavior

## 🎯 Areas Needing Help

Current areas where contributions are especially welcome:

- **Performance optimization:** Make the CLI faster
- **Error handling:** Better error messages and recovery
- **YAML features:** New configuration options
- **Testing:** More comprehensive test coverage
- **Documentation:** Real-world examples and tutorials
- **CLI options:** New command-line features
- **Integrations:** CI/CD plugins and integrations

---

Thank you for contributing to curl-runner! Every contribution, no matter how small, helps make the project better for everyone. 🎉
