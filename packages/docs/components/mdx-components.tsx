import type { MDXComponents } from 'mdx/types';
import { CodeBlockServer } from '@/components/code-block-server';
import { H1, H2, H3, H4, H5, H6 } from '@/components/mdx-heading';

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    h1: H1,
    h2: H2,
    h3: H3,
    h4: H4,
    h5: H5,
    h6: H6,
    pre: ({ children, ...props }) => {
      // Extract code content and language from children
      const childrenAsElement = children as React.ReactElement;
      const code =
        typeof children === 'string' ? children : childrenAsElement?.props?.children || '';
      const language = childrenAsElement?.props?.className?.replace('language-', '') || 'text';

      return (
        <CodeBlockServer language={language} {...props}>
          {code}
        </CodeBlockServer>
      );
    },
    code: ({ className, children, ...props }) => {
      const language = className?.replace('language-', '') || 'text';

      // Inline code
      if (!className) {
        return (
          <code
            className="bg-primary/10 text-primary px-2 py-1 rounded text-sm font-mono"
            {...props}
          >
            {children}
          </code>
        );
      }

      // Block code
      return <CodeBlockServer language={language}>{String(children)}</CodeBlockServer>;
    },
    ...components,
  };
}
