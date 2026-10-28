import { defineConfig, defineDocs } from 'fumadocs-mdx/config';
import { YAML_THEME } from './lib/shiki-theme';

export const docs = defineDocs({
  dir: 'content/docs',
  docs: {
    postprocess: {
      // Powers /docs.md, /docs/<page>.md and /llms.txt.
      includeProcessedMarkdown: true,
    },
  },
});

export default defineConfig({
  mdxOptions: {
    rehypeCodeOptions: {
      // Docs are dark only: one theme for both slots, matching the marketing YAML panes.
      themes: { light: YAML_THEME, dark: YAML_THEME },
    },
  },
});
