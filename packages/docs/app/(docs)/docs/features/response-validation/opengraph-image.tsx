import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { ImageResponse } from 'next/og';

// Use Node.js runtime to access file system
export const runtime = 'nodejs';




export const alt = 'Response Validation | curl-runner Documentation';
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
        fontFamily: 'system-ui, sans-serif',
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
            'radial-gradient(circle at 25% 25%, #10b981 0%, transparent 50%), radial-gradient(circle at 75% 75%, #059669 0%, transparent 50%)',
        }}
      />

      {/* Main Content */}
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          textAlign: 'center',
        }}
      >
        {/* Logo/Icon using your actual brand icon loaded locally */}
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
            style={{
              borderRadius: 12,
            }}
          />
        </div>

        {/* Page Badge */}
        <div
          style={{
            display: 'flex',
            background: 'rgba(16, 185, 129, 0.2)',
            border: '2px solid rgba(16, 185, 129, 0.3)',
            borderRadius: 16,
            padding: '8px 20px',
            marginBottom: 24,
          }}
        >
          <span
            style={{
              fontSize: 16,
              fontWeight: 600,
              color: '#10b981',
              textTransform: 'uppercase',
              letterSpacing: '0.1em',
            }}
          >
            Response Validation
          </span>
        </div>

        {/* Title */}
        <h1
          style={{
            fontSize: 64,
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
            fontSize: 28,
            color: '#94a3b8',
            margin: 0,
            marginBottom: 32,
            maxWidth: 700,
            lineHeight: 1.3,
          }}
        >
          Validate status, headers, and body content
        </p>

        {/* Validation Types */}
        <div
          style={{
            display: 'flex',
            gap: 32,
            alignItems: 'center',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <div style={{ fontSize: 20 }}>✅</div>
            <span style={{ color: '#e2e8f0', fontSize: 18 }}>Status</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <div style={{ fontSize: 20 }}>📋</div>
            <span style={{ color: '#e2e8f0', fontSize: 18 }}>Headers</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <div style={{ fontSize: 20 }}>🔍</div>
            <span style={{ color: '#e2e8f0', fontSize: 18 }}>Body</span>
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
        https://www.curl-runner.com/docs/features/response-validation
      </div>
    </div>,
    { ...size },
  );
}
