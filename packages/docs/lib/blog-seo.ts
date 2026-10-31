import type { BlogPost } from '@/lib/blog';
import { SITE_URL } from '@/lib/site';

type PostRef = Pick<
  BlogPost,
  'slug' | 'url' | 'title' | 'description' | 'date' | 'updated' | 'author'
>;

/** The generated Open Graph image of a post (app/og/blog/[slug]/image.png/route.tsx). */
export function getBlogOgUrl(post: Pick<BlogPost, 'slug'>): string {
  return `/og/blog/${post.slug}/image.png`;
}

/**
 * schema.org BlogPosting for a post's `<script type="application/ld+json">`. `<` is escaped
 * so a title can never close the script tag; JSON.parse reads `<` back as `<`.
 */
export function blogPostingJsonLd(post: PostRef): string {
  return JSON.stringify({
    '@context': 'https://schema.org',
    '@type': 'BlogPosting',
    headline: post.title,
    description: post.description,
    datePublished: post.date,
    dateModified: post.updated ?? post.date,
    author: { '@type': 'Person', name: post.author },
    image: `${SITE_URL}${getBlogOgUrl(post)}`,
    mainEntityOfPage: `${SITE_URL}${post.url}`,
  }).replace(/</g, '\\u003c');
}
