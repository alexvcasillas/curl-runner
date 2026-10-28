import { ImageResponse } from 'next/og';
import { getOgFonts } from '@/lib/og-fonts';

export const runtime = 'nodejs';
export const alt = 'curl-runner Documentation';
export const size = { width: 1200, height: 630 };
export const contentType = 'image/png';

// Monochrome tokens (app/globals.css).
const GROUND = '#0A0A0A';
const FG = '#EDEDED';
const FG_SECONDARY = '#A1A1A1';
const HAIRLINE = '#1F1F1F';
const BRAND = '#22D3EE';

export default async function Image() {
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
        fontFamily: 'Inter',
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
        <div style={{ fontSize: 24, letterSpacing: '0.06em', color: BRAND }}>DOCUMENTATION</div>
        <div
          style={{
            fontSize: 68,
            fontWeight: 800,
            lineHeight: 1.05,
            letterSpacing: '-0.04em',
            maxWidth: 1000,
          }}
        >
          HTTP requests as YAML. Run them anywhere.
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
        <span>Install, configure and run curl-runner</span>
        <span>curl-runner.com/docs</span>
      </div>
    </div>,
    { ...size, fonts },
  );
}
