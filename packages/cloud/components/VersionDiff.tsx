import { cx } from '@/lib/cx';
import { type FileDiff, type Hunk, hunkHeader, lineCounts } from '@/lib/diff';
import { Pill, type PillTone } from './Pill';

const STATUS: Record<
  FileDiff['status'],
  { word: string; mark: string; tone: PillTone; color: string }
> = {
  added: { word: 'Added', mark: 'A', tone: 'ok', color: 'text-success' },
  changed: { word: 'Changed', mark: 'M', tone: 'warn', color: 'text-warning' },
  removed: { word: 'Removed', mark: 'D', tone: 'err', color: 'text-danger' },
};

type Row =
  | { kind: 'hunk'; text: string }
  | { kind: 'add' | 'del' | 'ctx'; oldNo: number | null; newNo: number | null; text: string }
  | { kind: 'note'; text: string };

/** The hunk lines with their old and new line numbers. */
function rowsOf(hunks: Hunk[]): Row[] {
  const rows: Row[] = [];
  for (const h of hunks) {
    rows.push({ kind: 'hunk', text: hunkHeader(h) });
    let oldNo = h.oldStart;
    let newNo = h.newStart;
    for (const line of h.lines) {
      const text = line.slice(1);
      if (line.startsWith('+')) {
        rows.push({ kind: 'add', oldNo: null, newNo: newNo++, text });
      } else if (line.startsWith('-')) {
        rows.push({ kind: 'del', oldNo: oldNo++, newNo: null, text });
      } else if (line.startsWith('\\')) {
        rows.push({ kind: 'note', text: line.slice(2) });
      } else {
        rows.push({ kind: 'ctx', oldNo: oldNo++, newNo: newNo++, text });
      }
    }
  }
  return rows;
}

const LINE = 'flex font-mono text-[12px] leading-5 whitespace-pre';
const NO = 'box-border w-11 shrink-0 pr-2.5 text-right text-fg-subtle select-none';

function Line({ row }: { row: Row }) {
  if (row.kind === 'hunk') {
    return <div className={cx(LINE, 'bg-info-bg px-3.5 text-fg-subtle')}>{row.text}</div>;
  }
  if (row.kind === 'note') {
    return <div className={cx(LINE, 'pl-[84px] text-fg-subtle italic')}>{row.text}</div>;
  }
  const mark = row.kind === 'add' ? '+' : row.kind === 'del' ? '-' : '';
  return (
    <div
      className={cx(
        LINE,
        row.kind === 'add' && 'bg-success-bg',
        row.kind === 'del' && 'bg-danger-bg',
      )}
    >
      <span className={NO}>{row.oldNo ?? ''}</span>
      <span className={NO}>{row.newNo ?? ''}</span>
      <span
        aria-hidden="true"
        className={cx(
          'w-5 shrink-0 text-center',
          row.kind === 'add' && 'text-success',
          row.kind === 'del' && 'text-danger',
        )}
      >
        {mark}
      </span>
      <span className="pr-4">
        {row.kind === 'add' ? <span className="sr-only">Added: </span> : null}
        {row.kind === 'del' ? <span className="sr-only">Removed: </span> : null}
        {row.text}
      </span>
    </div>
  );
}

/** Changed files as a list on the left and, for each, its hunks with line numbers. */
export function VersionDiff({
  diffs,
  version,
  downloadHref,
}: {
  diffs: FileDiff[];
  /** The version being viewed, for "Download v14 to see it". */
  version: number;
  downloadHref: string;
}) {
  return (
    <div className="flex flex-wrap items-start gap-5">
      <nav
        aria-label="Changed files"
        className="flex min-w-0 max-w-[280px] flex-[1_1_240px] flex-col gap-0.5 max-md:max-w-none"
      >
        <span className="px-2 pb-1.5 text-[12px] text-fg-subtle">Files</span>
        {diffs.map((d, i) => {
          const s = STATUS[d.status];
          return (
            <a
              key={d.path}
              href={`#file-${i}`}
              className="flex h-[30px] items-center gap-2 rounded-[6px] px-2 text-[12px] text-fg-muted hover:bg-surface-200 hover:text-fg"
            >
              <span
                className={cx('w-3.5 shrink-0 text-center font-mono text-[11px]', s.color)}
                role="img"
                aria-label={s.word}
              >
                {s.mark}
              </span>
              <span className="min-w-0 truncate font-mono">{d.path}</span>
            </a>
          );
        })}
      </nav>

      <div className="flex min-w-0 flex-[999_1_560px] flex-col gap-4">
        {diffs.map((d, i) => {
          const s = STATUS[d.status];
          const { plus, minus } = lineCounts(d);
          return (
            <section
              key={d.path}
              id={`file-${i}`}
              aria-label={d.path}
              className="min-w-0 scroll-mt-4 overflow-hidden rounded-[8px] border border-hairline"
            >
              <div className="box-border flex min-h-11 flex-wrap items-center gap-2.5 border-b border-hairline bg-raised px-3.5 py-1.5">
                <Pill tone={s.tone}>{s.word}</Pill>
                <span className="min-w-0 flex-1 font-mono text-[12px] [overflow-wrap:anywhere]">
                  {d.path}
                </span>
                {d.tooLarge ? null : (
                  <span className="font-mono text-[12px]">
                    <span className="text-success">+{plus}</span>{' '}
                    <span className="text-danger">-{minus}</span>
                  </span>
                )}
              </div>
              <div className="overflow-x-auto py-1">
                <div className="w-max min-w-full">
                  {d.tooLarge ? (
                    <div className="px-3.5 py-2 text-fg-muted">
                      This file is too large to show here.{' '}
                      <a href={downloadHref}>Download v{version}</a> to see it.
                    </div>
                  ) : d.hunks.length === 0 ? (
                    <div className="px-3.5 py-2 text-fg-subtle">An empty file.</div>
                  ) : (
                    rowsOf(d.hunks).map((row, j) => <Line key={j} row={row} />)
                  )}
                </div>
              </div>
            </section>
          );
        })}
      </div>
    </div>
  );
}
