import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { ImageResponse } from 'next/og';

// Use Node.js runtime to access file system
export const runtime = 'nodejs';

export const alt = 'curl-runner Documentation';
export const size = {
  width: 1200,
  height: 630,
};

export const contentType = 'image/png';

export default async function Image() {
  // Load your actual icon from the local file system
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
          backgroundImage:
            'radial-gradient(circle at 25% 25%, #3b82f6 0%, transparent 50%), radial-gradient(circle at 75% 75%, #8b5cf6 0%, transparent 50%)',
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
        {/* Logo/Icon using your actual brand icon loaded locally */}
        <div
          style={{
            width: 140,
            height: 140,
            background: 'rgba(255, 255, 255, 0.95)',
            borderRadius: 28,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: 40,
            padding: 20,
            boxShadow: '0 20px 40px rgba(0,0,0,0.3)',
          }}
        >
          {/* Your actual curl-runner icon loaded from local file */}
          <img
            src={iconSrc}
            alt="curl-runner"
            width="100"
            height="100"
            style={{
              borderRadius: 16,
            }}
          />
        </div>

        {/* Title */}
        <h1
          style={{
            fontSize: 72,
            fontWeight: 800,
            color: 'white',
            margin: 0,
            marginBottom: 20,
            letterSpacing: '-0.02em',
          }}
        >
          curl-runner
        </h1>

        {/* Subtitle */}
        <p
          style={{
            fontSize: 32,
            color: '#94a3b8',
            margin: 0,
            marginBottom: 40,
            maxWidth: 800,
            lineHeight: 1.3,
          }}
        >
          A powerful CLI tool for HTTP request management using YAML configuration files
        </p>

        {/* Features */}
        <div
          style={{
            display: 'flex',
            gap: 40,
            alignItems: 'center',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <div style={{ fontSize: 24 }}>📄</div>
            <span style={{ color: '#e2e8f0', fontSize: 20 }}>YAML Config</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <div style={{ fontSize: 24 }}>⚡</div>
            <span style={{ color: '#e2e8f0', fontSize: 20 }}>Fast Execution</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <div style={{ fontSize: 24 }}>✅</div>
            <span style={{ color: '#e2e8f0', fontSize: 20 }}>Validation</span>
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
        }}
      >
        curl-runner.com
      </div>
    </div>,
    { ...size },
  );
}
