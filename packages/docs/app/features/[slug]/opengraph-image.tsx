import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import type { ImageResponseOptions } from 'next/dist/compiled/@vercel/og/types';
import { ImageResponse } from 'next/og';
import { featuresData } from '@/lib/features-data';

export const runtime = 'nodejs';
export const alt = 'curl-runner Feature';
export const size = {
  width: 1200,
  height: 630,
};
export const contentType = 'image/png';

const colorMap: Record<string, { bg: string; border: string; text: string; glow: string }> = {
  cyan: {
    bg: 'rgba(6, 182, 212, 0.2)',
    border: 'rgba(6, 182, 212, 0.4)',
    text: '#06b6d4',
    glow: '#06b6d4',
  },
  green: {
    bg: 'rgba(34, 197, 94, 0.2)',
    border: 'rgba(34, 197, 94, 0.4)',
    text: '#22c55e',
    glow: '#22c55e',
  },
  purple: {
    bg: 'rgba(168, 85, 247, 0.2)',
    border: 'rgba(168, 85, 247, 0.4)',
    text: '#a855f7',
    glow: '#a855f7',
  },
  orange: {
    bg: 'rgba(249, 115, 22, 0.2)',
    border: 'rgba(249, 115, 22, 0.4)',
    text: '#f97316',
    glow: '#f97316',
  },
  blue: {
    bg: 'rgba(59, 130, 246, 0.2)',
    border: 'rgba(59, 130, 246, 0.4)',
    text: '#3b82f6',
    glow: '#3b82f6',
  },
  pink: {
    bg: 'rgba(236, 72, 153, 0.2)',
    border: 'rgba(236, 72, 153, 0.4)',
    text: '#ec4899',
    glow: '#ec4899',
  },
  yellow: {
    bg: 'rgba(234, 179, 8, 0.2)',
    border: 'rgba(234, 179, 8, 0.4)',
    text: '#eab308',
    glow: '#eab308',
  },
  indigo: {
    bg: 'rgba(99, 102, 241, 0.2)',
    border: 'rgba(99, 102, 241, 0.4)',
    text: '#6366f1',
    glow: '#6366f1',
  },
  violet: {
    bg: 'rgba(139, 92, 246, 0.2)',
    border: 'rgba(139, 92, 246, 0.4)',
    text: '#8b5cf6',
    glow: '#8b5cf6',
  },
  emerald: {
    bg: 'rgba(16, 185, 129, 0.2)',
    border: 'rgba(16, 185, 129, 0.4)',
    text: '#10b981',
    glow: '#10b981',
  },
  teal: {
    bg: 'rgba(20, 184, 166, 0.2)',
    border: 'rgba(20, 184, 166, 0.4)',
    text: '#14b8a6',
    glow: '#14b8a6',
  },
  rose: {
    bg: 'rgba(244, 63, 94, 0.2)',
    border: 'rgba(244, 63, 94, 0.4)',
    text: '#f43f5e',
    glow: '#f43f5e',
  },
};

export function generateImageMetadata() {
  return Object.keys(featuresData).map((slug) => ({
    id: slug,
    alt: `curl-runner - ${featuresData[slug].title}`,
    size,
    contentType,
  }));
}

export default async function Image({ params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params;
  const feature = featuresData[slug];

  if (!feature) {
    return new ImageResponse(
      <div
        style={{
          background: '#0f172a',
          width: '100%',
          height: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'white',
          fontSize: 48,
        }}
      >
        Feature not found
      </div>,
      { ...size } as ImageResponseOptions,
    );
  }

  const iconData = await readFile(join(process.cwd(), 'public', 'icon-light-192.png'));
  const iconSrc = `data:image/png;base64,${iconData.toString('base64')}`;
  const colors = colorMap[feature.color] || colorMap.blue;

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
          opacity: 0.15,
          display: 'flex',
          backgroundImage: `radial-gradient(circle at 25% 25%, ${colors.glow} 0%, transparent 50%), radial-gradient(circle at 75% 75%, ${colors.glow} 0%, transparent 50%)`,
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
            width: 100,
            height: 100,
            background: 'rgba(255, 255, 255, 0.95)',
            borderRadius: 20,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: 28,
            padding: 14,
            boxShadow: '0 20px 40px rgba(0,0,0,0.3)',
          }}
        >
          <img
            src={iconSrc}
            alt="curl-runner"
            width="72"
            height="72"
            style={{ borderRadius: 10 }}
          />
        </div>

        {/* Badge */}
        <div
          style={{
            display: 'flex',
            background: colors.bg,
            border: `1px solid ${colors.border}`,
            borderRadius: 20,
            padding: '8px 20px',
            marginBottom: 20,
          }}
        >
          <span style={{ color: colors.text, fontSize: 18, fontWeight: 600 }}>
            curl-runner Feature
          </span>
        </div>

        {/* Title */}
        <h1
          style={{
            fontSize: 56,
            fontWeight: 800,
            color: 'white',
            margin: 0,
            marginBottom: 16,
            letterSpacing: '-0.02em',
          }}
        >
          {feature.title}
        </h1>

        {/* Subtitle */}
        <p
          style={{
            fontSize: 26,
            color: '#94a3b8',
            margin: 0,
            maxWidth: 800,
            lineHeight: 1.4,
          }}
        >
          {feature.shortDescription}
        </p>
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
        {`curl-runner.com/features/${slug}`}
      </div>
    </div>,
    { ...size } as ImageResponseOptions,
  );
}
