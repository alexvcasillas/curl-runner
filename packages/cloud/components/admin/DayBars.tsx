import type { Day } from '@/lib/admin';
import { formatInt } from '@/lib/admin';
import { shortDay } from '@/lib/admin-time';

const W = 280;
const H = 150;

/** One bar a day, today in the bright colour. Each bar has a native tooltip and a row in the table. */
export function DayBars({ days, label }: { days: Day[]; label: string }) {
  const max = Math.max(1, ...days.map((d) => d.n));
  const slot = W / Math.max(1, days.length);
  const gap = 6;
  const lo = Math.min(...days.map((d) => d.n));
  return (
    <div className="p-4">
      <svg
        viewBox={`0 0 ${W} ${H}`}
        width="100%"
        height={164}
        preserveAspectRatio="none"
        role="img"
        aria-label={`${label} over ${days.length} days, between ${formatInt(lo)} and ${formatInt(max)}. The numbers are in the table below.`}
      >
        <line x1="0" y1={H - 0.5} x2={W} y2={H - 0.5} stroke="#1f1f1f" />
        {days.map((d, i) => {
          const h = Math.max(d.n > 0 ? 1 : 0, Math.round((d.n / max) * (H - 4)));
          return (
            <rect
              key={d.day}
              data-bar=""
              x={i * slot + gap / 2}
              y={H - h}
              width={slot - gap}
              height={h}
              fill={i === days.length - 1 ? '#ededed' : '#2e2e2e'}
              className="hover:fill-fg-muted"
            >
              <title>{`${shortDay(d.day)}: ${formatInt(d.n)} publishes`}</title>
            </rect>
          );
        })}
      </svg>
      <table className="sr-only">
        <caption>{label} per day</caption>
        <thead>
          <tr>
            <th scope="col">Day</th>
            <th scope="col">Publishes</th>
          </tr>
        </thead>
        <tbody>
          {days.map((d) => (
            <tr key={d.day}>
              <th scope="row">{shortDay(d.day)}</th>
              <td>{d.n}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
