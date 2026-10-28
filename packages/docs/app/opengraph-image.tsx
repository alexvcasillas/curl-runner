import { OG_SIZE, renderOgCard } from '@/lib/og-card';

export const runtime = 'nodejs';
export const alt = 'curl-runner: API tests as YAML, run anywhere';
export const size = OG_SIZE;
export const contentType = 'image/png';

export default function Image() {
  return renderOgCard({
    eyebrow: 'curl-runner',
    headline: 'API tests as YAML. Run them anywhere.',
    footerLeft: 'One native binary, no runtime',
    footerRight: 'curl-runner.com',
  });
}
