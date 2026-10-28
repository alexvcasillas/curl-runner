import { notFound } from 'next/navigation';
import { renderOgCard } from '@/lib/og-card';
import { source } from '@/lib/source';

export const runtime = 'nodejs';
export const dynamicParams = false;

/** The bundled Geist subset has no ⇄, and Satori would fetch a font for it. */
function ogText(text: string): string {
  return text.replace('⇄', '/');
}

// `/og/docs/<page slugs>/image.png`; the docs index is `/og/docs/image.png`.
export function generateStaticParams() {
  return source.getPages().map((page) => ({ slug: [...page.slugs, 'image.png'] }));
}

export async function GET(_request: Request, { params }: { params: Promise<{ slug: string[] }> }) {
  const { slug } = await params;
  const page = source.getPage(slug.slice(0, -1));
  if (!page) {
    notFound();
  }

  const isIndex = page.slugs.length === 0;

  return renderOgCard({
    eyebrow: 'Documentation',
    headline: isIndex ? 'HTTP requests as YAML. Run them anywhere.' : ogText(page.data.title),
    footerLeft: 'Install, configure and run curl-runner',
    footerRight: `curl-runner.com${page.url}`,
  });
}
