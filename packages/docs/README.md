# @curl-runner/docs

[![Next.js](https://img.shields.io/badge/Next-black?style=flat-square&logo=next.js&logoColor=white)](https://nextjs.org/)
[![React](https://img.shields.io/badge/react-%2320232a.svg?style=flat-square&logo=react&logoColor=%2361DAFB)](https://reactjs.org/)
[![TypeScript](https://img.shields.io/badge/typescript-%23007ACC.svg?style=flat-square&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Tailwind CSS](https://img.shields.io/badge/tailwindcss-%2338B2AC.svg?style=flat-square&logo=tailwind-css&logoColor=white)](https://tailwindcss.com/)

The official documentation website for curl-runner, built with Next.js 15, React 19, and modern web technologies. Features comprehensive documentation, interactive examples, and a beautiful, responsive design with dark mode support.

## 🌟 Features

### 📖 Documentation

- **Comprehensive Guides**: Complete documentation for all curl-runner features
- **Interactive Examples**: Live code examples with syntax highlighting
- **API Reference**: Detailed TypeScript interface documentation
- **Search Functionality**: Fast, client-side search across all content

### 🎨 Modern Design

- **Responsive Layout**: Optimized for desktop, tablet, and mobile
- **Dark/Light Mode**: Automatic theme switching with system preference detection
- **Accessible UI**: Built with Radix UI primitives for maximum accessibility
- **Beautiful Typography**: Carefully crafted reading experience

### ⚡ Performance

- **Next.js App Router**: Latest Next.js architecture for optimal performance
- **Static Generation**: Pre-rendered pages for fast loading
- **Code Splitting**: Automatic code splitting for optimal bundle sizes
- **Image Optimization**: Automatic image optimization and lazy loading

### 🛠️ Developer Experience

- **TypeScript**: Full type safety throughout the application
- **Component Library**: Reusable UI components with shadcn/ui
- **MDX Support**: Rich content with React component integration
- **Hot Reload**: Fast development with instant updates

## 📦 Installation & Setup

### Prerequisites

- **Node.js** 18+ or **Bun** 1.2.21+
- **Git** for version control

### Development Setup

```bash
# Clone the monorepo
git clone https://github.com/alexvcasillas/curl-runner.git
cd curl-runner

# Install dependencies
bun install

# Start the development server
bun run dev:docs

# Or run all packages in development mode
bun run dev
```

The documentation site will be available at [http://localhost:3000](http://localhost:3000).

### Building for Production

```bash
# Build the documentation site
bun run build:docs

# Start the production server
bun run start:docs
```

## 🏗️ Architecture

### Tech Stack

- **[Next.js 15](https://nextjs.org/)** - React framework with App Router
- **[React 19](https://reactjs.org/)** - Latest React with concurrent features
- **[TypeScript](https://www.typescriptlang.org/)** - Type-safe development
- **[Tailwind CSS](https://tailwindcss.com/)** - Utility-first styling
- **[shadcn/ui](https://ui.shadcn.com/)** - Accessible component library
- **[Radix UI](https://www.radix-ui.com/)** - Low-level UI primitives
- **[Lucide Icons](https://lucide.dev/)** - Beautiful icon library
- **[Shiki](https://shiki.style/)** - Syntax highlighting
- **[GSAP](https://gsap.com/)** - Advanced animations

### Project Structure

```
packages/docs/
├── app/                          # Next.js App Router
│   ├── (docs)/                  # Documentation routes
│   │   └── docs/               # Main docs pages
│   │       ├── page.tsx        # Introduction page
│   │       ├── installation/   # Installation guide
│   │       ├── quick-start/    # Quick start guide
│   │       ├── yaml-structure/ # YAML configuration
│   │       ├── variables/      # Variable interpolation
│   │       ├── features/       # Feature-specific docs
│   │       ├── examples/       # Usage examples
│   │       └── api-reference/  # API documentation
│   ├── globals.css             # Global styles
│   ├── layout.tsx              # Root layout
│   ├── page.tsx                # Homepage
│   └── sitemap.ts              # SEO sitemap
├── components/                  # React components
│   ├── ui/                     # shadcn/ui components
│   ├── icons/                  # Custom icons
│   ├── animate-ui/             # Animation components
│   ├── code-block.tsx          # Syntax highlighting
│   ├── docs-sidebar.tsx        # Documentation sidebar
│   ├── search-dialog.tsx       # Search functionality
│   ├── theme-toggle.tsx        # Dark mode toggle
│   └── mdx-components.tsx      # MDX component overrides
├── lib/                        # Utilities and configuration
│   ├── docs-config.ts          # Navigation configuration
│   └── utils.ts                # Utility functions
├── scripts/                    # Build scripts
│   └── jsx-to-markdown-with-frontmatter.ts
├── next.config.ts              # Next.js configuration
├── tailwind.config.js          # Tailwind CSS configuration
├── components.json             # shadcn/ui configuration
└── package.json               # Package configuration
```

## 📝 Content Management

### Adding New Documentation Pages

1. **Create the page component**:

```tsx
// app/(docs)/docs/new-feature/page.tsx
import { CodeBlockServer } from "@/components/code-block-server";
import { DocsPageHeader } from "@/components/docs-page-header";

export default function NewFeaturePage() {
  return (
    <div className="container max-w-4xl py-6 lg:py-10">
      <DocsPageHeader
        title="New Feature"
        description="Learn about this amazing new feature"
      />

      <div className="prose prose-neutral dark:prose-invert max-w-none">
        <p>Your content here...</p>

        <CodeBlockServer
          language="yaml"
          title="example.yaml"
          code={`
request:
  name: Example Request
  url: https://api.example.com
  method: GET
          `}
        />
      </div>
    </div>
  );
}
```

2. **Add to navigation**:

```tsx
// lib/docs-config.ts
export const docsConfig: SidebarConfig = {
  sidebarNav: [
    {
      title: "Features",
      items: [
        // ... existing items
        {
          title: "New Feature",
          href: "/docs/new-feature",
          description: "Description of the new feature",
        },
      ],
    },
  ],
};
```

3. **Create snippets file** (optional):

```tsx
// app/(docs)/docs/new-feature/snippets.ts
export const exampleYaml = `
request:
  name: Example Request
  url: https://api.example.com
  method: GET
`;
```

### Code Examples

Use the `CodeBlockServer` component for syntax-highlighted code:

```tsx
import { CodeBlockServer } from "@/components/code-block-server";

<CodeBlockServer
  language="yaml"
  title="config.yaml"
  code={yamlExample}
  showLineNumbers={true}
/>;
```

## 🎨 Styling & Theming

### Tailwind CSS Configuration

The project uses Tailwind CSS with custom design tokens:

```js
// tailwind.config.js
module.exports = {
  darkMode: ["class"],
  content: [
    "./pages/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
    "./app/**/*.{ts,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Custom color palette
      },
      typography: {
        // Custom prose styles
      },
    },
  },
};
```

### Dark Mode Support

Dark mode is handled automatically with `next-themes`:

```tsx
import { ThemeProvider } from "next-themes";

function MyApp({ children }) {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      disableTransitionOnChange
    >
      {children}
    </ThemeProvider>
  );
}
```

### Component Styling

Components use the shadcn/ui design system:

```tsx
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

<Button variant="default" size="lg" className={cn("custom-styles")}>
  Click me
</Button>;
```

## 🔍 Search Functionality

The documentation includes a fast, client-side search:

```tsx
// components/search-dialog.tsx
import { CommandDialog } from "@/components/ui/command";

export function SearchDialog() {
  return <CommandDialog>{/* Search implementation */}</CommandDialog>;
}
```

### Adding Searchable Content

Content is automatically indexed from:

- Page titles and descriptions
- Headings and text content
- Code examples and snippets

## 📱 Responsive Design

The site is fully responsive with breakpoints:

- **Mobile**: `<640px`
- **Tablet**: `640px - 1024px`
- **Desktop**: `>1024px`

```tsx
// Example responsive component
<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
  {/* Content */}
</div>
```

## 🧪 Development Workflow

### Available Scripts

```bash
# Development
bun run dev                    # Start development server
bun run dev:docs              # Start docs development server only

# Building
bun run build                 # Build for production
bun run build:docs           # Build docs only
bun run start                 # Start production server

# Content Generation
bun run generate-markdown     # Generate markdown from JSX components

# Code Quality
bun run format               # Format code with Biome
bun run lint                 # Lint code
bun run check               # Format and fix issues
```

### Development Best Practices

1. **Component Organization**: Keep components focused and reusable
2. **Type Safety**: Use TypeScript interfaces for all props
3. **Accessibility**: Test with screen readers and keyboard navigation
4. **Performance**: Optimize images and use Next.js features
5. **SEO**: Include proper meta tags and structured data

### Testing

```bash
# Run component tests
bun test

# Visual regression testing (manual)
bun run dev:docs
# Navigate and test in browser
```

## 🚀 Deployment

### Vercel Deployment (Recommended)

1. **Connect your repository** to Vercel
2. **Configure build settings**:
   ```
   Framework Preset: Next.js
   Root Directory: packages/docs
   Build Command: bun run build
   Output Directory: .next
   ```
3. **Environment variables**: Configure any needed environment variables
4. **Deploy**: Automatic deployments on git push

### Manual Deployment

```bash
# Build the static site
bun run build:docs

# The output will be in .next/ directory
# Upload to your hosting provider
```

### Static Export (Optional)

For static hosting:

```js
// next.config.ts
const nextConfig = {
  output: "export",
  images: {
    unoptimized: true,
  },
};
```

## 🎯 Performance Optimization

### Core Web Vitals

The site is optimized for:

- **LCP (Largest Contentful Paint)**: <2.5s
- **FID (First Input Delay)**: <100ms
- **CLS (Cumulative Layout Shift)**: <0.1

### Optimization Techniques

1. **Image Optimization**: Using Next.js Image component
2. **Code Splitting**: Automatic with Next.js App Router
3. **Static Generation**: Pre-rendered pages for speed
4. **Font Optimization**: System fonts with fallbacks
5. **Bundle Analysis**: Regular bundle size monitoring

### Monitoring

```bash
# Analyze bundle size
npx @next/bundle-analyzer
```

## 🔧 Customization

### Adding New UI Components

1. **Install shadcn/ui component**:

```bash
bunx shadcn-ui@latest add button
```

2. **Customize in components/ui/**:

```tsx
// components/ui/custom-button.tsx
import { cn } from "@/lib/utils";
import { Button, ButtonProps } from "./button";

interface CustomButtonProps extends ButtonProps {
  special?: boolean;
}

export function CustomButton({
  special,
  className,
  ...props
}: CustomButtonProps) {
  return (
    <Button
      className={cn(
        special && "bg-gradient-to-r from-blue-500 to-purple-500",
        className
      )}
      {...props}
    />
  );
}
```

### Custom Animations

Using GSAP for advanced animations:

```tsx
import { useGSAP } from "@gsap/react";
import gsap from "gsap";

export function AnimatedComponent() {
  const containerRef = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      gsap.from(containerRef.current, {
        y: 50,
        opacity: 0,
        duration: 0.6,
        ease: "power2.out",
      });
    },
    { scope: containerRef }
  );

  return <div ref={containerRef}>Content</div>;
}
```

## 📊 Analytics & SEO

### Vercel Analytics

```tsx
// app/layout.tsx
import { Analytics } from "@vercel/analytics/react";

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html>
      <body>
        {children}
        <Analytics />
      </body>
    </html>
  );
}
```

### SEO Configuration

```tsx
// app/layout.tsx
export const metadata: Metadata = {
  title: {
    default: "curl-runner",
    template: "%s | curl-runner",
  },
  description:
    "Powerful CLI tool for HTTP request management using YAML configuration files",
  keywords: ["curl", "http", "api", "testing", "yaml", "cli"],
  openGraph: {
    type: "website",
    locale: "en_US",
    url: "https://curl-runner.dev",
    siteName: "curl-runner",
  },
  twitter: {
    card: "summary_large_image",
    creator: "@yourhandle",
  },
};
```

## 🤝 Contributing

### Documentation Contributions

1. **Content Updates**: Improve existing documentation
2. **New Guides**: Add tutorials and examples
3. **UI Improvements**: Enhance the user experience
4. **Bug Fixes**: Fix broken links or incorrect information

### Contribution Workflow

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b docs/new-feature`
3. **Make your changes**: Update content, components, or styles
4. **Test locally**: `bun run dev:docs`
5. **Commit changes**: Follow conventional commit format
6. **Open a pull request**: Describe your changes clearly

### Content Guidelines

- **Clear and Concise**: Write for clarity and brevity
- **Examples**: Include practical code examples
- **Screenshots**: Add visuals where helpful
- **Accessibility**: Ensure content is accessible to all users
- **SEO**: Use proper headings and meta descriptions

## 📄 License

MIT License - see [LICENSE.md](../../LICENSE.md) for details.

## 🔗 Related

- **[CLI Package](../cli/)**: The curl-runner CLI tool
- **[Main Repository](../../@README.md)**: Project overview and setup
- **[Contributing Guide](../../CONTRIBUTE.md)**: How to contribute to the project

---

<div align="center">
  <strong>@curl-runner/docs</strong> - Beautiful documentation for curl-runner
  <br><br>
  Built with ❤️ using Next.js, React, and modern web technologies
</div>
