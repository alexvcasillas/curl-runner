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

- **[Bun](https://bun.sh)** v1.2.21 or higher
- **Git** for version control
- **Node.js** knowledge (TypeScript/JavaScript)
- **Basic understanding** of HTTP and REST APIs

### Quick Setup

```bash
# Fork the repository on GitHub
# Clone your fork
git clone https://github.com/YOUR_USERNAME/curl-runner.git
cd curl-runner

# Add upstream remote
git remote add upstream https://github.com/yourusername/curl-runner.git

# Install dependencies
bun install

# Verify everything works
bun run build
bun run test
```

## 🛠️ Development Setup

### Workspace Overview

curl-runner is a monorepo with two main packages:

```
packages/
├── cli/                 # CLI tool (@curl-runner/cli)
└── docs/               # Documentation website (@curl-runner/docs)
```

### Development Commands

```bash
# Install dependencies for all packages
bun install

# Development mode
bun run dev              # All packages in dev mode
bun run dev:cli          # CLI only (watch mode)
bun run dev:docs         # Documentation site only

# Building
bun run build            # Build all packages
bun run build:cli        # Build CLI binary
bun run build:docs       # Build documentation site

# Code quality
bun run format           # Format with Biome
bun run lint             # Lint with Biome
bun run check            # Format and fix issues
bun run check:ci         # CI-friendly check

# Testing
bun test                 # Run all tests
bun test:cli             # Run CLI tests only
```

## 📁 Project Structure

### CLI Package (`packages/cli/`)

```
packages/cli/
├── src/
│   ├── cli.ts          # Main CLI entry point
│   ├── core/           # Core functionality
│   ├── parsers/        # YAML parsing logic
│   ├── http/           # HTTP client and utilities
│   ├── validators/     # Response validation
│   └── utils/          # Shared utilities
├── examples/           # Example YAML files
├── tests/              # Test files
└── package.json
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
bun test

# Test CLI functionality
bun run cli packages/cli/examples/

# Test documentation site
bun run dev:docs  # Visit http://localhost:3000

# Check code quality
bun run check
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

### TypeScript/JavaScript

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
# Format code
bun run format

# Check and fix issues
bun run check

# Lint code
bun run lint
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

```typescript
import { test, expect } from "bun:test";
import { parseYamlConfig } from "../src/parsers/yaml";

test("parseYamlConfig - should parse valid single request", () => {
  const yaml = `
    request:
      name: Test Request
      url: https://example.com
      method: GET
  `;

  const result = parseYamlConfig(yaml);

  expect(result.type).toBe("single");
  expect(result.request.name).toBe("Test Request");
  expect(result.request.url).toBe("https://example.com");
});

test("parseYamlConfig - should throw on invalid YAML", () => {
  const invalidYaml = "invalid: yaml: content:";

  expect(() => parseYamlConfig(invalidYaml)).toThrow();
});
```

### Running Tests

```bash
# Run all tests
bun test

# Run specific test file
bun test src/parsers/yaml.test.ts

# Run tests with coverage
bun test --coverage

# Run CLI integration tests
bun test:cli
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
3. **Check Code Quality:** Run `bun run check`
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

## 🐛 Issue Guidelines

### Bug Reports

Use the bug report template and include:

- **Environment:** OS, Bun version, curl-runner version
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
bun run packages/cli/src/cli.ts examples/simple.yaml

# Debug with verbose output
bun run packages/cli/src/cli.ts -v examples/

# Test parallel execution
bun run packages/cli/src/cli.ts -p examples/
```

### Working with Documentation

```bash
# Start docs development server
bun run dev:docs

# Generate documentation from JSX components
bun run --filter '@curl-runner/docs' generate-markdown

# Build docs for production
bun run build:docs
```

### Debugging

- Use `console.log` for quick debugging
- Use Bun's built-in debugger: `bun --inspect src/file.ts`
- Add test cases to reproduce issues
- Use TypeScript strict mode to catch errors early

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
