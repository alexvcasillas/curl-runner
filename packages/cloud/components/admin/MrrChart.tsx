import { formatMoney } from '@/lib/admin';
import { shortDay } from '@/lib/admin-time';

const W = 600;
const H = 180;
const PAD = 6;

/**
 * MRR over the last 90 days as one line, in the same inline-SVG style as the
 * overview's charts. Each day has a hover column with a native tooltip, and the
 * figures are in a table for screen readers.
 */
export function MrrChart({ points }: { points: { day: string; mrr_cents: number }[] }) {
  if (points.length === 0) {
    return <p className="m-0 px-4 py-6 text-fg-muted">No revenue history yet.</p>;
  }
  const values = points.map((p) => p.mrr_cents);
  const max = Math.max(1, ...values);
  const min = Math.min(...values);
  const lo = min === max ? 0 : min;
  const step = points.length > 1 ? W / (points.length - 1) : 0;
  const y = (v: number) => H - PAD - ((v - lo) / Math.max(1, max - lo)) * (H - PAD * 2);
  const line = points
    .map((p, i) => `${Math.round(i * step)},${Math.round(y(p.mrr_cents))}`)
    .join(' ');
  const col = W / points.length;
  const first = points[0];
  const last = points[points.length - 1];
  const mid = points[Math.floor((points.length - 1) / 2)];
  const trend =
    last.mrr_cents > first.mrr_cents
      ? 'rising'
      : last.mrr_cents < first.mrr_cents
        ? 'falling'
        : 'flat';
  return (
    <div className="px-4 pt-4 pb-2">
      <svg
        viewBox={`0 0 ${W} ${H}`}
        width="100%"
        height={H}
        preserveAspectRatio="none"
        role="img"
        aria-label={`MRR ${trend} from ${formatMoney(first.mrr_cents)} to ${formatMoney(last.mrr_cents)} over ${points.length} days. The numbers are in the table below.`}
      >
        {[1, H / 2, H - 1].map((v) => (
          <line key={v} x1="0" y1={v} x2={W} y2={v} stroke="#1f1f1f" />
        ))}
        <polyline
          points={line}
          fill="none"
          stroke="#62c073"
          strokeWidth="2"
          vectorEffect="non-scaling-stroke"
        />
        {points.map((p, i) => (
          <rect
            key={p.day}
            x={i * col}
            y="0"
            width={col}
            height={H}
            fill="transparent"
            className="hover:fill-white/5"
          >
            <title>{`${shortDay(p.day)}: ${formatMoney(p.mrr_cents)}`}</title>
          </rect>
        ))}
      </svg>
      <div
        aria-hidden="true"
        className="flex justify-between pt-2 font-mono text-[11px] text-fg-subtle"
      >
        <span>{shortDay(first.day)}</span>
        <span>{shortDay(mid.day)}</span>
        <span>{shortDay(last.day)}</span>
      </div>
      <table className="sr-only">
        <caption>MRR per day</caption>
        <thead>
          <tr>
            <th scope="col">Day</th>
            <th scope="col">MRR</th>
          </tr>
        </thead>
        <tbody>
          {points.map((p) => (
            <tr key={p.day}>
              <th scope="row">{shortDay(p.day)}</th>
              <td>{formatMoney(p.mrr_cents)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
