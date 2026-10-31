import { notFound } from 'next/navigation';
import { getPost, getPosts } from '@/lib/blog';
import { renderOgCard } from '@/lib/og-card';

export const runtime = 'nodejs';
export const dynamicParams = false;

export function generateStaticParams() {
  return getPosts().map((post) => ({ slug: post.slug }));
}

export async function GET(_request: Request, { params }: { params: Promise<{ slug: string }> }) {
  const post = getPost((await params).slug);
  if (!post) {
    notFound();
  }

  return renderOgCard({
    eyebrow: 'Blog',
    headline: post.title,
    footerLeft: post.author,
    footerRight: `curl-runner.com${post.url}`,
  });
}
