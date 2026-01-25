import { ImageResponse } from 'next/og';

export const runtime = 'edge';

export const alt = 'Environment Variables - curl-runner Documentation';
export const size = {
  width: 1200,
  height: 630,
};

export const contentType = 'image/png';

export default async function Image() {
  return new ImageResponse(
    <div
      style={{
        fontSize: 48,
        background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        color: 'white',
        padding: '40px',
      }}
    >
      <div style={{ fontSize: 72, fontWeight: 'bold', marginBottom: 20 }}>curl-runner</div>
      <div style={{ fontSize: 36, opacity: 0.9, textAlign: 'center' }}>Environment Variables</div>
      <div style={{ fontSize: 24, opacity: 0.8, marginTop: 20, textAlign: 'center' }}>
        Configure curl-runner behavior using environment variables
      </div>
    </div>,
    { ...size },
  );
}
