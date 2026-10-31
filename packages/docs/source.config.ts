import { defineConfig, defineDocs, frontmatterSchema } from 'fumadocs-mdx/config';
import { z } from 'zod';
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

// YAML reads an unquoted `2026-10-31` as a Date; keep both spellings as `YYYY-MM-DD`.
const isoDay = z
  .union([z.string().regex(/^\d{4}-\d{2}-\d{2}$/), z.date()])
  .transform((day) => (typeof day === 'string' ? day : day.toISOString().slice(0, 10)));

// Blog posts: /blog/<slug>, /blog/<slug>.md and the blog feed. One flat folder, newest first.
export const blog = defineDocs({
  dir: 'content/blog',
  docs: {
    schema: frontmatterSchema.extend({
      description: z.string(),
      date: isoDay,
      updated: isoDay.optional(),
      author: z.string(),
    }),
    postprocess: { includeProcessedMarkdown: true },
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
