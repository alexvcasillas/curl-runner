import defaultMdxComponents from 'fumadocs-ui/mdx';
import type { MDXComponents } from 'mdx/types';
import { StudioScreenshot } from '@/components/site/Screenshot';

export function getMDXComponents(components?: MDXComponents) {
  return {
    ...defaultMdxComponents,
    StudioScreenshot,
    ...components,
  } satisfies MDXComponents;
}

export const useMDXComponents = getMDXComponents;

declare global {
  type MDXProvidedComponents = ReturnType<typeof getMDXComponents>;
}
