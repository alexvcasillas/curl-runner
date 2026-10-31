import { describe, expect, it } from 'vitest';
import { getBlogMarkdown, getPost, getPosts, readingMinutes, sortPosts } from '@/lib/blog';

const SLUG = 'testing-in-the-agent-era';

describe('blog posts', () => {
  it('loads the first post with its URL and frontmatter', () => {
    const post = getPosts().find((p) => p.slug === SLUG);
    expect(post).toBeDefined();
    expect(post?.url).toBe(`/blog/${SLUG}`);
    expect(post?.title).toBe('The importance of testing in the Agent era');
    expect(post?.date).toBe('2026-10-31');
    expect(post?.author).toBe('Alex Casillas');
    expect(post?.description.length).toBeGreaterThan(0);
  });

  it('keeps every description short enough for search results', () => {
    for (const post of getPosts()) {
      expect(post.description.length, post.slug).toBeLessThanOrEqual(160);
    }
  });

  it('returns undefined for an unknown slug', () => {
    expect(getPost('nope')).toBeUndefined();
  });

  it('orders posts newest first, then by title', () => {
    const order = sortPosts([
      { date: '2026-01-01', title: 'B' },
      { date: '2026-02-01', title: 'Z' },
      { date: '2026-01-01', title: 'A' },
    ]).map((p) => p.title);
    expect(order).toEqual(['Z', 'A', 'B']);
  });

  it('computes reading time at 230 words a minute, at least one', () => {
    expect(readingMinutes('word '.repeat(460))).toBe(2);
    expect(readingMinutes('')).toBe(1);
  });

  it('renders a post as Markdown for LLMs', async () => {
    const post = getPost(SLUG);
    if (!post) {
      throw new Error('missing post');
    }
    const md = await getBlogMarkdown(post);
    expect(
      md.startsWith(
        `# The importance of testing in the Agent era\n\nURL: https://www.curl-runner.com/blog/${SLUG}`,
      ),
    ).toBe(true);
    expect(md).toContain(post.description);
  });
});
