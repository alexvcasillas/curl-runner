import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { loader } from 'fumadocs-core/source';
import { blog } from '@/.source/server';
import { SITE_URL } from '@/lib/site';

export const BLOG_CONTENT_DIR = 'content/blog';

const WORDS_PER_MINUTE = 230;

const blogSource = loader({
  baseUrl: '/blog',
  source: blog.toFumadocsSource(),
});

type BlogPage = ReturnType<typeof blogSource.getPages>[number];

export interface BlogPost {
  slug: string;
  title: string;
  description: string;
  /** `YYYY-MM-DD`. */
  date: string;
  updated?: string;
  author: string;
  /** `/blog/<slug>`. */
  url: string;
  /** The source file, relative to this package: dates the sitemap entry. */
  file: string;
  readingMinutes: number;
  body: BlogPage['data']['body'];
  getText: BlogPage['data']['getText'];
}

/** Whole minutes at 230 words a minute, never less than one. */
export function readingMinutes(text: string): number {
  const words = text.split(/\s+/).filter(Boolean).length;
  return Math.max(1, Math.round(words / WORDS_PER_MINUTE));
}

/** Newest first; posts from the same day by title, so the order never depends on the filesystem. */
export function sortPosts<T extends { date: string; title: string }>(posts: T[]): T[] {
  return [...posts].sort((a, b) =>
    a.date === b.date ? a.title.localeCompare(b.title) : b.date.localeCompare(a.date),
  );
}

function toPost(page: BlogPage): BlogPost {
  const file = `${BLOG_CONTENT_DIR}/${page.path}`;
  const raw = readFileSync(join(process.cwd(), file), 'utf8').replace(/^---[\s\S]*?\n---\n/, '');
  const { title, description, date, updated, author, body, getText } = page.data;
  return {
    slug: page.slugs[0],
    title,
    description,
    date,
    updated,
    author,
    url: page.url,
    file,
    readingMinutes: readingMinutes(raw),
    body,
    getText,
  };
}

export function getPosts(): BlogPost[] {
  return sortPosts(blogSource.getPages().map(toPost));
}

export function getPost(slug: string): BlogPost | undefined {
  const page = blogSource.getPage([slug]);
  return page ? toPost(page) : undefined;
}

/** A post as Markdown for LLMs, in the same shape as the docs pages' `.md` copies. */
export async function getBlogMarkdown(post: BlogPost): Promise<string> {
  const body = await post.getText('processed');
  return `${[`# ${post.title}`, `URL: ${SITE_URL}${post.url}`, post.description, body.trim()].join('\n\n')}\n`;
}
