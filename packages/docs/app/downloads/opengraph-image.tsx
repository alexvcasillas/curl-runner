import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { ImageResponse } from 'next/og';

export const runtime = 'nodejs';
export const alt = 'Download curl-runner';
export const size = {
  width: 1200,
  height: 630,
};
export const contentType = 'image/png';

export default async function Image() {
  const iconData = await readFile(join(process.cwd(), 'public', 'icon-light-192.png'));
  const iconSrc = `data:image/png;base64,${iconData.toString('base64')}`;

  return new ImageResponse(
    <div
      style={{
        background: 'linear-gradient(135deg, #0f172a 0%, #1e293b 100%)',
        width: '100%',
        height: '100%',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        flexDirection: 'column',
      }}
    >
      {/* Background Pattern */}
      <div
        style={{
          position: 'absolute',
          width: '100%',
          height: '100%',
          opacity: 0.1,
          display: 'flex',
          backgroundImage:
            'radial-gradient(circle at 25% 25%, #10b981 0%, transparent 50%), radial-gradient(circle at 75% 75%, #3b82f6 0%, transparent 50%)',
        }}
      />

      {/* Main Content */}
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          textAlign: 'center',
          zIndex: 1,
        }}
      >
        {/* Logo */}
        <div
          style={{
            width: 120,
            height: 120,
            background: 'rgba(255, 255, 255, 0.95)',
            borderRadius: 24,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: 32,
            padding: 16,
            boxShadow: '0 20px 40px rgba(0,0,0,0.3)',
          }}
        >
          <img
            src={iconSrc}
            alt="curl-runner"
            width="88"
            height="88"
            style={{ borderRadius: 12 }}
          />
        </div>

        {/* Badge */}
        <div
          style={{
            display: 'flex',
            background: 'rgba(16, 185, 129, 0.2)',
            border: '1px solid rgba(16, 185, 129, 0.4)',
            borderRadius: 20,
            padding: '8px 20px',
            marginBottom: 24,
          }}
        >
          <span style={{ color: '#10b981', fontSize: 18, fontWeight: 600 }}>
            No Dependencies Required
          </span>
        </div>

        {/* Title */}
        <h1
          style={{
            fontSize: 64,
            fontWeight: 800,
            color: 'white',
            margin: 0,
            marginBottom: 16,
            letterSpacing: '-0.02em',
          }}
        >
          Download curl-runner
        </h1>

        {/* Subtitle */}
        <p
          style={{
            fontSize: 28,
            color: '#94a3b8',
            margin: 0,
            marginBottom: 40,
            maxWidth: 700,
            lineHeight: 1.3,
          }}
        >
          Pre-compiled binaries for all platforms
        </p>

        {/* Platform Icons */}
        <div style={{ display: 'flex', gap: 48, alignItems: 'center' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <div style={{ fontSize: 28 }}>🍎</div>
            <span style={{ color: '#e2e8f0', fontSize: 20 }}>macOS</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <div style={{ fontSize: 28 }}>🐧</div>
            <span style={{ color: '#e2e8f0', fontSize: 20 }}>Linux</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <div style={{ fontSize: 28 }}>🪟</div>
            <span style={{ color: '#e2e8f0', fontSize: 20 }}>Windows</span>
          </div>
        </div>
      </div>

      {/* URL */}
      <div
        style={{
          position: 'absolute',
          bottom: 40,
          right: 40,
          fontSize: 18,
          color: '#64748b',
          display: 'flex',
        }}
      >
        curl-runner.com/downloads
      </div>
    </div>,
    { ...size },
  );
}
