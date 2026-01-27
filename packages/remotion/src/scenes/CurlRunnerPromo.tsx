import {
  AbsoluteFill,
  interpolate,
  Sequence,
  spring,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from 'remotion';

const palette = {
  bg: '#0b1224',
  panel: '#101a32',
  edge: '#1f2b40',
  primary: '#5ba8ff',
  primarySoft: 'rgba(91, 168, 255, 0.16)',
  purple: '#8b5cf6',
  text: '#e9eeff',
  muted: '#96a6c7',
  success: '#4ade80',
  accent: '#c7d9ff',
  glow: '#3b82f6',
};

const logo = staticFile('icon-light-192.png');

const fontSans = '"Inter", "SF Pro Display", "Soehne", system-ui, -apple-system, sans-serif';
const fontMono =
  '"JetBrains Mono", "SFMono-Regular", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace';

const softShadow = '0 20px 80px rgba(0, 0, 0, 0.45)';

export const CurlRunnerPromo = () => {
  return (
    <AbsoluteFill style={{ background: palette.bg, color: palette.text, fontFamily: fontSans }}>
      <GradientBars />
      <Sequence from={0} durationInFrames={110}>
        <HeroScene />
      </Sequence>
      <Sequence from={90} durationInFrames={110}>
        <YamlScene />
      </Sequence>
      <Sequence from={180} durationInFrames={100}>
        <ParallelScene />
      </Sequence>
      <Sequence from={260} durationInFrames={100}>
        <ValidationScene />
      </Sequence>
      <Sequence from={340} durationInFrames={110}>
        <CtaScene />
      </Sequence>
    </AbsoluteFill>
  );
};

const HeroScene = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const scale = spring({ frame, fps, config: { damping: 200, mass: 0.7 } });
  const subtitleOpacity = interpolate(frame, [0, 25], [0, 1], { extrapolateRight: 'clamp' });
  const float = interpolate(frame, [0, 40], [22, 0], { extrapolateRight: 'clamp' });

  return (
    <AbsoluteFill style={{ alignItems: 'center', justifyContent: 'center' }}>
      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 18 }}>
        <LogoTile float={float} />
        <div
          style={{
            textTransform: 'uppercase',
            letterSpacing: 6,
            color: palette.muted,
            fontSize: 18,
            opacity: subtitleOpacity,
          }}
        >
          Bun-native HTTP orchestration
        </div>
        <div
          style={{
            fontSize: 82,
            fontWeight: 800,
            lineHeight: 1.05,
            textAlign: 'center',
            transform: `scale(${scale})`,
          }}
        >
          curl-runner
          <span style={{ color: palette.primary }}> promo</span>
        </div>
        <div
          style={{
            marginTop: 12,
            color: palette.accent,
            fontSize: 24,
            fontWeight: 500,
            textAlign: 'center',
          }}
        >
          YAML-first flows. Parallel where it counts. CI-ready output.
        </div>
        <HeroBadges />
      </div>
    </AbsoluteFill>
  );
};

const HeroBadges = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const translate = interpolate(frame, [10, 40], [30, 0], { extrapolateRight: 'clamp' });
  const opacity = interpolate(frame, [10, 40], [0, 1], { extrapolateRight: 'clamp' });

  return (
    <div
      style={{
        display: 'flex',
        gap: 12,
        marginTop: 36,
        transform: `translateY(${translate}px)`,
        opacity,
      }}
    >
      {[
        { label: 'Parallel + pacing', color: palette.primary },
        { label: 'Watch + retries', color: palette.success },
        { label: 'Snapshots + diffs', color: palette.accent },
      ].map((badge) => (
        <div
          key={badge.label}
          style={{
            border: `1px solid ${palette.edge}`,
            background: palette.panel,
            padding: '10px 16px',
            borderRadius: 999,
            boxShadow: softShadow,
            color: badge.color,
            fontSize: 16,
            fontWeight: 600,
          }}
        >
          {badge.label}
        </div>
      ))}
    </div>
  );
};

const YamlScene = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const slide = interpolate(frame, [0, 18], [80, 0], { extrapolateRight: 'clamp' });
  const opacity = spring({ frame, fps, config: { damping: 200, mass: 0.7 } });

  return (
    <AbsoluteFill style={{ padding: '96px 120px', justifyContent: 'center' }}>
      <div style={{ display: 'grid', gridTemplateColumns: '1.2fr 1fr', gap: 36 }}>
        <div
          style={{
            borderRadius: 24,
            border: `1px solid ${palette.edge}`,
            background: palette.panel,
            boxShadow: softShadow,
            padding: 28,
            transform: `translateY(${slide}px)`,
            opacity,
          }}
        >
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              marginBottom: 18,
            }}
          >
            <div style={{ fontWeight: 700, color: palette.text, fontSize: 22 }}>
              YAML-first flows
            </div>
            <Pill>watch · env · validate</Pill>
          </div>
          <CodeBlock>
            {`global:
  execution: parallel
  watch:
    paths: ./flows
  variables:
    BASE_URL: https://api.curl.run
    TOKEN: ${'${env:API_TOKEN}'}

collection:
  name: Release checks
  outputs:
    format: json
    save: ./out/results.json
  requests:
    - name: List users
      url: ${'${BASE_URL}'}/users
      method: GET
      expect:
        status: 200
        body.contains: users
    - name: Create user
      url: ${'${BASE_URL}'}/users
      method: POST
      headers:
        Authorization: Bearer ${'${TOKEN}'}
      body:
        name: Nova Ops
      retry:
        attempts: 3
        strategy: exponential
    - name: Snapshot profile
      url: ${'${BASE_URL}'}/me
      method: GET
      snapshot: prod-profile`}
          </CodeBlock>
        </div>
        <div style={{ display: 'grid', gap: 16, alignContent: 'center' }}>
          <FeatureCard
            title="Variables + secrets"
            copy="Drop env vars inline, share collections safely."
          />
          <FeatureCard
            title="Snapshots + diffs"
            copy="Track responses over time for quick regressions."
          />
          <FeatureCard title="CI-ready" copy="Quiet JSON output + fail-fast control." />
        </div>
      </div>
    </AbsoluteFill>
  );
};

const ParallelScene = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const progress = spring({ frame, fps, config: { damping: 180, mass: 0.9 } });

  return (
    <AbsoluteFill style={{ padding: '90px 120px' }}>
      <SectionTitle
        title="Parallel where it counts"
        kicker="Threads, pacing, retries — all tuned by YAML"
        progress={progress}
      />
      <div
        style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 16, marginTop: 28 }}
      >
        <StatCard
          label="Throughput"
          value="18.4k req/min"
          accent={palette.primary}
          progress={progress}
        />
        <StatCard label="P95 latency" value="142 ms" accent={palette.accent} progress={progress} />
        <StatCard label="Success" value="99.3%" accent={palette.success} progress={progress} />
      </div>
      <CliPanel />
    </AbsoluteFill>
  );
};

const CliPanel = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const opacity = interpolate(frame, [10, 30], [0, 1], { extrapolateRight: 'clamp' });
  const translate = interpolate(frame, [10, 30], [30, 0], { extrapolateRight: 'clamp' });

  const lines = [
    '$ bun run cli flows/release.yaml --parallel --watch --show-metrics',
    '↳ warming up 12 workers · pacing 50 req/s',
    '✓ list users    200 | 48ms | 2.1kb',
    '✓ create user   201 | 92ms | 1.4kb · retry x1',
    '✓ snapshot me   200 | 55ms | stored: prod-profile',
    'summary 12 req | p95 142ms | max 301ms | 0 errors',
  ];

  return (
    <div
      style={{
        marginTop: 34,
        borderRadius: 20,
        border: `1px solid ${palette.edge}`,
        background: 'linear-gradient(135deg, rgba(13,19,33,0.95), rgba(13,19,33,0.75))',
        padding: 22,
        fontFamily: fontMono,
        color: palette.accent,
        boxShadow: softShadow,
        opacity,
        transform: `translateY(${translate}px)`,
      }}
    >
      {lines.map((line, idx) => {
        const rowOpacity = interpolate(frame, [20 + idx * 6, 30 + idx * 6], [0, 1], {
          extrapolateRight: 'clamp',
        });
        const hue =
          idx === 0 ? palette.primary : idx === lines.length - 1 ? palette.success : palette.accent;

        return (
          <div
            key={line}
            style={{ opacity: rowOpacity, marginBottom: 8, color: hue, fontSize: 17 }}
          >
            {line}
          </div>
        );
      })}
    </div>
  );
};

const ValidationScene = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const progress = spring({ frame, fps, config: { damping: 200, mass: 0.9 } });
  const rotate = interpolate(frame, [0, 90], [-6, 0], { extrapolateRight: 'clamp' });

  return (
    <AbsoluteFill style={{ padding: '90px 120px' }}>
      <SectionTitle
        title="Confident shipping"
        kicker="Assertions, snapshots, CI-ready output"
        progress={progress}
      />
      <div style={{ display: 'grid', gridTemplateColumns: '1.1fr 1fr', gap: 28, marginTop: 22 }}>
        <div
          style={{
            borderRadius: 24,
            border: `1px solid ${palette.edge}`,
            background: palette.panel,
            padding: 26,
            boxShadow: softShadow,
            transform: `rotate(${rotate}deg)`,
          }}
        >
          <div style={{ fontWeight: 700, fontSize: 22, marginBottom: 12 }}>Validations</div>
          <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'grid', gap: 10 }}>
            {[
              'Status · headers · body assertions',
              'Schema-aware diffs + snapshots',
              'Retries with jitter/backoff',
              'JSON output for CI pipelines',
            ].map((item, idx) => (
              <li
                key={item}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  color: idx === 0 ? palette.primary : palette.accent,
                  fontSize: 18,
                }}
              >
                <span
                  style={{
                    display: 'inline-flex',
                    width: 28,
                    height: 28,
                    alignItems: 'center',
                    justifyContent: 'center',
                    borderRadius: 999,
                    background: palette.primarySoft,
                    color: palette.primary,
                    fontWeight: 700,
                  }}
                >
                  ✓
                </span>
                {item}
              </li>
            ))}
          </ul>
        </div>
        <div style={{ display: 'grid', gap: 14, alignContent: 'center' }}>
          <MetricRow label="diff match" value="100%" accent={palette.success} progress={progress} />
          <MetricRow
            label="retries solved"
            value="42"
            accent={palette.primary}
            progress={progress}
          />
          <MetricRow label="p99 drift" value="-18%" accent={palette.accent} progress={progress} />
        </div>
      </div>
    </AbsoluteFill>
  );
};

const CtaScene = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const scale = spring({ frame, fps, config: { damping: 180, mass: 0.8 } });
  const glow = interpolate(frame, [0, 40], [0, 1], { extrapolateRight: 'clamp' });

  return (
    <AbsoluteFill style={{ alignItems: 'center', justifyContent: 'center', gap: 20 }}>
      <div
        style={{
          fontSize: 58,
          fontWeight: 800,
          textAlign: 'center',
          transform: `scale(${scale})`,
        }}
      >
        Ship faster with curl-runner
      </div>
      <div style={{ fontSize: 24, color: palette.accent, textAlign: 'center' }}>
        Parallel runs, strict validation, beautiful metrics.
      </div>
      <div style={{ display: 'flex', gap: 12, marginTop: 12 }}>
        <CTAButton primary label="bunx @remotion/cli preview" />
        <CTAButton label="curl-runner.com/docs" />
      </div>
      <div
        style={{
          width: 320,
          height: 320,
          borderRadius: 32,
          background:
            `radial-gradient(120% 120% at 50% 50%, rgba(91,168,255,${0.28 * glow}), transparent)` +
            `, linear-gradient(135deg, ${palette.panel}, ${palette.edge})`,
          border: `1px solid ${palette.edge}`,
          boxShadow: softShadow,
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
          }}
        >
          <LogoTile size={160} />
        </div>
      </div>
    </AbsoluteFill>
  );
};

const CTAButton = ({ label, primary }: { label: string; primary?: boolean }) => {
  return (
    <div
      style={{
        padding: '14px 18px',
        borderRadius: 14,
        fontWeight: 700,
        border: `1px solid ${primary ? palette.primary : palette.edge}`,
        color: primary ? '#04060c' : palette.accent,
        background: primary ? palette.primary : palette.panel,
        boxShadow: softShadow,
        fontFamily: fontMono,
        fontSize: 16,
      }}
    >
      {label}
    </div>
  );
};

const LogoTile = ({ float = 0, size = 120 }: { float?: number; size?: number }) => {
  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius: 24,
        border: `1px solid ${palette.edge}`,
        background: 'linear-gradient(145deg, rgba(91,168,255,0.16), rgba(16,26,50,0.9))',
        boxShadow: softShadow,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        transform: `translateY(-${float}px)`,
        overflow: 'hidden',
      }}
    >
      <img src={logo} alt="curl-runner" width={size - 26} height={size - 26} />
    </div>
  );
};

const SectionTitle = ({
  title,
  kicker,
  progress,
}: {
  title: string;
  kicker: string;
  progress: number;
}) => {
  const translate = interpolate(progress, [0, 1], [16, 0]);
  const opacity = progress;

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: 12,
        opacity,
        transform: `translateY(${translate}px)`,
      }}
    >
      <div>
        <div
          style={{
            color: palette.muted,
            letterSpacing: 4,
            textTransform: 'uppercase',
            fontSize: 14,
          }}
        >
          {kicker}
        </div>
        <div style={{ fontSize: 36, fontWeight: 800, marginTop: 6 }}>{title}</div>
      </div>
      <Pill>curl-runner · metrics</Pill>
    </div>
  );
};

const StatCard = ({
  label,
  value,
  accent,
  progress,
}: {
  label: string;
  value: string;
  accent: string;
  progress: number;
}) => {
  const lift = interpolate(progress, [0, 1], [18, 0]);
  const opacity = progress;

  return (
    <div
      style={{
        padding: 18,
        borderRadius: 18,
        border: `1px solid ${palette.edge}`,
        background: palette.panel,
        boxShadow: softShadow,
        transform: `translateY(${lift}px)`,
        opacity,
      }}
    >
      <div style={{ color: palette.muted, fontSize: 14 }}>{label}</div>
      <div style={{ fontSize: 26, fontWeight: 800, color: accent, marginTop: 6 }}>{value}</div>
    </div>
  );
};

const MetricRow = ({
  label,
  value,
  accent,
  progress,
}: {
  label: string;
  value: string;
  accent: string;
  progress: number;
}) => {
  const width = interpolate(progress, [0, 1], [40, 100]);
  return (
    <div
      style={{
        borderRadius: 14,
        border: `1px solid ${palette.edge}`,
        background: palette.panel,
        padding: 14,
        boxShadow: softShadow,
      }}
    >
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          color: palette.muted,
          fontSize: 15,
        }}
      >
        <span>{label}</span>
        <span style={{ color: accent, fontWeight: 700 }}>{value}</span>
      </div>
      <div style={{ marginTop: 8, height: 6, borderRadius: 999, background: palette.edge }}>
        <div
          style={{ width: `${width}%`, height: '100%', borderRadius: 999, background: accent }}
        />
      </div>
    </div>
  );
};

const FeatureCard = ({ title, copy }: { title: string; copy: string }) => {
  return (
    <div
      style={{
        borderRadius: 18,
        border: `1px solid ${palette.edge}`,
        background: palette.panel,
        padding: 18,
        boxShadow: softShadow,
      }}
    >
      <div style={{ fontWeight: 700, color: palette.accent, marginBottom: 6 }}>{title}</div>
      <div style={{ color: palette.muted, fontSize: 16 }}>{copy}</div>
    </div>
  );
};

const CodeBlock = ({ children }: { children: string }) => {
  return (
    <div
      style={{
        fontFamily: fontMono,
        fontSize: 16,
        lineHeight: 1.5,
        whiteSpace: 'pre-wrap',
        color: palette.accent,
        background: 'linear-gradient(180deg, rgba(17,27,46,0.9), rgba(17,27,46,0.8))',
        borderRadius: 16,
        padding: 16,
        border: `1px solid ${palette.edge}`,
      }}
    >
      {children}
    </div>
  );
};

const Pill = ({ children }: { children: string }) => {
  return (
    <div
      style={{
        padding: '8px 12px',
        borderRadius: 999,
        border: `1px solid ${palette.edge}`,
        background: palette.primarySoft,
        color: palette.primary,
        fontWeight: 700,
        letterSpacing: 1,
        textTransform: 'uppercase',
        fontSize: 12,
      }}
    >
      {children}
    </div>
  );
};

const GradientBars = () => {
  return (
    <AbsoluteFill style={{ pointerEvents: 'none' }}>
      <div
        style={{
          position: 'absolute',
          inset: 0,
          background:
            'radial-gradient(60% 60% at 50% 30%, rgba(59,130,246,0.16), transparent),' +
            'radial-gradient(40% 40% at 80% 70%, rgba(139,92,246,0.14), transparent)',
        }}
      />
      <div
        style={{
          position: 'absolute',
          inset: 0,
          background:
            'linear-gradient(135deg, rgba(91,168,255,0.08) 0%, rgba(11,18,36,0) 40%),' +
            'linear-gradient(45deg, rgba(139,92,246,0.08) 10%, rgba(11,18,36,0) 55%),' +
            'linear-gradient(180deg, rgba(16,26,50,0.7), rgba(11,18,36,0.85))',
        }}
      />
    </AbsoluteFill>
  );
};

export const CurlRunnerThumb = () => {
  return (
    <AbsoluteFill
      style={{
        background: palette.bg,
        color: palette.text,
        fontFamily: fontSans,
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      <div
        style={{
          borderRadius: 28,
          padding: 40,
          border: `1px solid ${palette.edge}`,
          background: palette.panel,
          boxShadow: softShadow,
          textAlign: 'center',
          width: 760,
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'center', marginBottom: 18 }}>
          <LogoTile size={96} />
        </div>
        <div
          style={{
            textTransform: 'uppercase',
            color: palette.muted,
            letterSpacing: 6,
            fontSize: 14,
            marginBottom: 10,
          }}
        >
          curl-runner
        </div>
        <div style={{ fontSize: 54, fontWeight: 800, lineHeight: 1.05 }}>YAML to reality.</div>
        <div style={{ marginTop: 12, color: palette.accent, fontSize: 20 }}>
          Parallel requests, assertions, metrics — instantly rendered.
        </div>
      </div>
    </AbsoluteFill>
  );
};
