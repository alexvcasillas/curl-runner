import { ImageResponse } from 'next/og';
import { getOgFonts } from '@/lib/og-fonts';

export const runtime = 'nodejs';
export const alt = 'Download curl-runner';
export const size = { width: 1200, height: 630 };
export const contentType = 'image/png';

const PLATFORMS = ['macOS', 'Linux', 'Windows'];

export default async function Image() {
  const fonts = await getOgFonts();

  return new ImageResponse(
    <div
      style={{
        background: '#0a0a0a',
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        padding: '0 96px',
        color: '#ededed',
        fontFamily: 'Geist',
      }}
    >
      <div style={{ display: 'flex', fontSize: 24, color: '#22d3ee', letterSpacing: '0.04em' }}>
        DOWNLOAD
      </div>
      <div
        style={{
          display: 'flex',
          fontSize: 88,
          fontWeight: 800,
          letterSpacing: '-0.045em',
          lineHeight: 1.02,
          marginTop: 20,
        }}
      >
        One binary. Every platform.
      </div>
      <div style={{ display: 'flex', fontSize: 30, color: '#a1a1a1', marginTop: 28 }}>
        A single static executable. No runtime, no dependencies.
      </div>
      <div style={{ display: 'flex', gap: 16, marginTop: 48 }}>
        {PLATFORMS.map((p) => (
          <div
            key={p}
            style={{
              display: 'flex',
              border: '1px solid #262626',
              borderRadius: 10,
              padding: '10px 20px',
              fontSize: 24,
              color: '#ededed',
            }}
          >
            {p}
          </div>
        ))}
      </div>
      <div
        style={{
          position: 'absolute',
          bottom: 40,
          right: 56,
          display: 'flex',
          fontSize: 20,
          color: '#8a8a8a',
        }}
      >
        curl-runner.com/downloads
      </div>
    </div>,
    { ...size, fonts },
  );
}
