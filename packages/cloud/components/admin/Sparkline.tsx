import type { Day } from '@/lib/admin';
import { formatInt } from '@/lib/admin';
import { shortDay } from '@/lib/admin-time';

const W = 600;
const H = 180;
const PAD = 6;

function points(days: Day[], max: number): string {
  const step = days.length > 1 ? W / (days.length - 1) : 0;
  return days
    .map((d, i) => {
      const y = H - PAD - (d.n / max) * (H - PAD * 2);
      return `${Math.round(i * step)},${Math.round(y)}`;
    })
    .join(' ');
}

/**
 * Two daily series on one shared scale: sign-ups as the accent line, new teams as
 * a dashed neutral one. Each day has a hover column with a native tooltip, and the
 * numbers are in a table for screen readers.
 */
export function Sparkline({ signups, teams }: { signups: Day[]; teams: Day[] }) {
  const max = Math.max(1, ...signups.map((d) => d.n), ...teams.map((d) => d.n));
  const totalSignups = signups.reduce((n, d) => n + d.n, 0);
  const totalTeams = teams.reduce((n, d) => n + d.n, 0);
  const col = W / Math.max(1, signups.length);
  const first = signups[0]?.day;
  const last = signups[signups.length - 1]?.day;
  const mid = signups[Math.floor((signups.length - 1) / 2)]?.day;
  return (
    <div className="px-4 pt-4 pb-2">
      <div className="flex flex-wrap gap-x-4 gap-y-1 pb-2.5 text-[12px] text-fg-muted">
        <span className="inline-flex items-center gap-1.5">
          <span aria-hidden="true" className="h-0.5 w-2.5 bg-blue" />
          Sign-ups · {formatInt(totalSignups)}
        </span>
        <span className="inline-flex items-center gap-1.5">
          <span aria-hidden="true" className="w-2.5 border-t-2 border-dashed border-fg" />
          New teams · {formatInt(totalTeams)}
        </span>
      </div>
      <svg
        viewBox={`0 0 ${W} ${H}`}
        width="100%"
        height={H}
        preserveAspectRatio="none"
        role="img"
        aria-label={`Sign-ups ${formatInt(totalSignups)} and new teams ${formatInt(totalTeams)} in the last ${signups.length} days, peaking at ${formatInt(max)} a day. The numbers are in the table below.`}
      >
        {[1, H / 2, H - 1].map((y) => (
          <line key={y} x1="0" y1={y} x2={W} y2={y} stroke="#1f1f1f" />
        ))}
        <polyline
          points={points(signups, max)}
          fill="none"
          stroke="#52a8ff"
          strokeWidth="2"
          vectorEffect="non-scaling-stroke"
        />
        <polyline
          points={points(teams, max)}
          fill="none"
          stroke="#ededed"
          strokeWidth="2"
          strokeDasharray="4 4"
          vectorEffect="non-scaling-stroke"
        />
        {signups.map((d, i) => (
          <rect
            key={d.day}
            x={i * col}
            y="0"
            width={col}
            height={H}
            fill="transparent"
            className="hover:fill-white/5"
          >
            <title>{`${shortDay(d.day)}: ${d.n} sign-ups, ${teams[i]?.n ?? 0} new teams`}</title>
          </rect>
        ))}
      </svg>
      <div
        aria-hidden="true"
        className="flex justify-between pt-2 font-mono text-[11px] text-fg-subtle"
      >
        <span>{first ? shortDay(first) : ''}</span>
        <span>{mid ? shortDay(mid) : ''}</span>
        <span>{last ? shortDay(last) : ''}</span>
      </div>
      <table className="sr-only">
        <caption>Sign-ups and new teams per day</caption>
        <thead>
          <tr>
            <th scope="col">Day</th>
            <th scope="col">Sign-ups</th>
            <th scope="col">New teams</th>
          </tr>
        </thead>
        <tbody>
          {signups.map((d, i) => (
            <tr key={d.day}>
              <th scope="row">{shortDay(d.day)}</th>
              <td>{d.n}</td>
              <td>{teams[i]?.n ?? 0}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
