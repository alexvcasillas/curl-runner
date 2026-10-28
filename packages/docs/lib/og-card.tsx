import { ImageResponse } from 'next/og';
import { getOgFonts } from '@/lib/og-fonts';

export const OG_SIZE = { width: 1200, height: 630 };

// Monochrome tokens (app/globals.css).
const GROUND = '#0A0A0A';
const FG = '#EDEDED';
const FG_SECONDARY = '#A1A1A1';
const HAIRLINE = '#1F1F1F';
const BRAND = '#22D3EE';

interface OgCardProps {
  eyebrow: string;
  headline: string;
  footerLeft: string;
  footerRight: string;
}

/** The dark Monochrome social card shared by the site and docs Open Graph images. */
export async function renderOgCard({ eyebrow, headline, footerLeft, footerRight }: OgCardProps) {
  const fonts = await getOgFonts();

  return new ImageResponse(
    <div
      style={{
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
        padding: '72px 80px',
        background: GROUND,
        color: FG,
        fontFamily: 'Geist',
      }}
    >
      <div
        style={{ display: 'flex', alignItems: 'center', gap: 14, fontSize: 30, fontWeight: 700 }}
      >
        <svg
          width="36"
          height="36"
          viewBox="0 0 24 24"
          fill="none"
          stroke={BRAND}
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <polyline points="4 17 10 11 4 5" />
          <line x1="12" y1="19" x2="20" y2="19" />
        </svg>
        curl-runner
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
        <div style={{ display: 'flex', fontSize: 24, letterSpacing: '0.06em', color: BRAND }}>
          {eyebrow.toUpperCase()}
        </div>
        <div
          style={{
            display: 'flex',
            fontSize: 68,
            fontWeight: 800,
            lineHeight: 1.05,
            letterSpacing: '-0.04em',
            maxWidth: 1000,
          }}
        >
          {headline}
        </div>
      </div>

      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          paddingTop: 28,
          borderTop: `1px solid ${HAIRLINE}`,
          fontSize: 22,
          color: FG_SECONDARY,
        }}
      >
        <span>{footerLeft}</span>
        <span>{footerRight}</span>
      </div>
    </div>,
    { ...OG_SIZE, fonts },
  );
}
