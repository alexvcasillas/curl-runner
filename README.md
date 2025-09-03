# 🚀 curl-runner Monorepo

A powerful CLI tool and documentation for HTTP request management using YAML configuration files. Built with [Bun](https://bun.sh) for blazing-fast performance.

[![Bun](https://img.shields.io/badge/Bun-%23000000.svg?style=for-the-badge&logo=bun&logoColor=white)](https://bun.sh)
[![TypeScript](https://img.shields.io/badge/typescript-%23007ACC.svg?style=for-the-badge&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Next.js](https://img.shields.io/badge/Next-black?style=for-the-badge&logo=next.js&logoColor=white)](https://nextjs.org/)
[![YAML](https://img.shields.io/badge/yaml-%23ffffff.svg?style=for-the-badge&logo=yaml&logoColor=151515)](https://yaml.org/)

## 📦 Monorepo Structure

This monorepo contains two main packages:

```
curl-runner/
├── packages/
│   ├── cli/               # CLI tool package
│   │   ├── src/           # Source code
│   │   ├── examples/      # Example YAML configurations
│   │   └── package.json
│   └── web/               # Documentation website
│       ├── src/           # Next.js source
│       └── package.json
├── package.json           # Root workspace configuration
├── tsconfig.base.json     # Shared TypeScript config
├── biome.json            # Code formatting and linting
└── README.md             # This file
```

## 🚀 Quick Start

### Prerequisites

- [Bun](https://bun.sh) v1.2.21 or higher

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/curl-runner.git
cd curl-runner

# Install dependencies for all packages
bun install

# Build the CLI tool
bun run build:cli
```

### Running the CLI

```bash
# Run the CLI directly
bun run cli examples/simple.yaml

# Or from anywhere in the monorepo
bun run packages/cli/src/cli.ts examples/

# Run with options
bun run cli -pv packages/cli/examples/
```

### Running the Documentation Website

```bash
# Start the development server
bun run dev:web

# Or run both CLI and web in development mode
bun run dev
```

Visit [http://localhost:3000](http://localhost:3000) to see the documentation.

## 📚 Package Details

### [@curl-runner/cli](./packages/cli)

The core CLI tool for running HTTP requests from YAML files.

**Features:**
- 📝 YAML configuration for HTTP requests
- 📁 Directory support for bulk execution
- 🔄 Sequential and parallel execution modes
- 🔧 Variable interpolation
- ✅ Response validation
- 📊 Beautiful console output

[View CLI Documentation →](./packages/cli/README.md)

### [@curl-runner/web](./packages/web)

Next.js-based documentation website for curl-runner.

**Features:**
- 📖 Comprehensive documentation
- 🎨 Modern, responsive design
- 🌙 Dark mode support
- 🔍 Searchable content
- 📱 Mobile-friendly

## 🛠️ Development

### Workspace Commands

```bash
# Install dependencies for all packages
bun install

# Run development mode for all packages
bun run dev

# Run development for specific package
bun run dev:cli    # CLI only
bun run dev:web    # Web only

# Build all packages
bun run build

# Build specific package
bun run build:cli  # Build CLI
bun run build:web  # Build website

# Format and lint code
bun run format     # Format with Biome
bun run lint       # Lint with Biome
bun run check      # Format and fix all issues

# Run tests
bun test          # Run all tests
bun test:cli      # Run CLI tests only
```

### Project Scripts

The root `package.json` manages workspace-wide operations:

| Script | Description |
|--------|-------------|
| `dev` | Run all packages in development mode |
| `dev:cli` | Run CLI in watch mode |
| `dev:web` | Run documentation site in dev mode |
| `build` | Build all packages |
| `build:cli` | Compile CLI to binary |
| `build:web` | Build documentation site |
| `format` | Format code with Biome |
| `lint` | Lint code with Biome |
| `check` | Format and fix issues |
| `test` | Run all tests |

### Adding New Features

1. **CLI Features**: Add code in `packages/cli/src/`
2. **Documentation**: Update both `packages/cli/README.md` and website in `packages/web/`
3. **Examples**: Add example YAML files in `packages/cli/examples/`
4. **Tests**: Add tests alongside the code

### Code Quality

- **Formatter**: Biome with 2-space indentation
- **Type Safety**: Strict TypeScript configuration
- **Monorepo**: Bun workspaces for package management

## 🧪 Testing

### CLI Testing

```bash
# Run CLI with example files
bun run cli packages/cli/examples/simple.yaml
bun run cli packages/cli/examples/collection.yaml
bun run cli packages/cli/examples/parallel.yaml

# Test directory execution
bun run cli packages/cli/examples/

# Test with various options
bun run cli -pv packages/cli/examples/  # Parallel + Verbose
bun run cli -c --output results.json packages/cli/examples/  # Continue on error + Save results
```

## 🤝 Contributing

We welcome contributions! Please follow these steps:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes in the appropriate package
4. Update documentation in both README and website
5. Run `bun run check` to ensure code quality
6. Commit your changes (`git commit -m 'feat: add amazing feature'`)
7. Push to your branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

### Development Guidelines

- **Code Style**: Run `bun run format` before committing
- **Type Safety**: Avoid `any` types
- **Documentation**: Update both CLI and web docs
- **Examples**: Add examples for new features
- **Tests**: Write tests for new functionality

## 📄 License

MIT License - see [LICENSE](./LICENSE) file for details

## 🙏 Acknowledgments

- Built with [Bun](https://bun.sh) - The fast all-in-one JavaScript runtime
- Documentation powered by [Next.js](https://nextjs.org/)
- Formatted with [Biome](https://biomejs.dev/)
- Styled with [Tailwind CSS](https://tailwindcss.com/)

## 📞 Support

- 🐛 [Report bugs](https://github.com/yourusername/curl-runner/issues)
- 💡 [Request features](https://github.com/yourusername/curl-runner/issues)
- 📖 [View documentation](https://yourusername.github.io/curl-runner)
- 💬 [Discussions](https://github.com/yourusername/curl-runner/discussions)

---

<div align="center">
  <b>curl-runner</b> - Making HTTP request management simple and powerful
  <br>
  Made with ❤️ using Bun
</div>