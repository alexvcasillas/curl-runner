import { notFound } from 'next/navigation';
import { getBlogMarkdown, getPost, getPosts } from '@/lib/blog';

export const revalidate = false;

// Served at /blog/<slug>.md via a rewrite in next.config.ts.
export async function GET(
  _req: Request,
  { params }: { params: Promise<{ slug: string }> },
): Promise<Response> {
  const post = getPost((await params).slug);
  if (!post) {
    notFound();
  }

  return new Response(await getBlogMarkdown(post), {
    headers: { 'Content-Type': 'text/markdown; charset=utf-8' },
  });
}

export function generateStaticParams() {
  return getPosts().map((post) => ({ slug: post.slug }));
}
