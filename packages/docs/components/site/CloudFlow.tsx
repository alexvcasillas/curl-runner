'use client';

import { type ReactNode, useEffect, useRef, useState } from 'react';

const STEPS = 4;
/** Dot travel between two markers, and how long a card stays lit. */
const MOVE_MS = 700;
const HOLD_MS = 1100;
const FADE_MS = 400;

/**
 * How a collection travels in curl-runner Cloud: repo -> CI publish -> Cloud -> pull.
 * A vertical timeline: one spine with a marker per step, and a wide card per step. A cyan dot
 * stops on each marker in turn and lights that card, then moves on. Marker positions are
 * measured, so the dot lands on them at any width. Under prefers-reduced-motion nothing moves.
 */
export function CloudFlow({ children }: { children?: ReactNode }) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const [tops, setTops] = useState<number[]>([]);
  const [position, setPosition] = useState(0);
  const [active, setActive] = useState(-1);
  /** The marker the dot rests in, drawn by the marker itself so it is centred exactly. */
  const [resting, setResting] = useState(-1);
  /** Whether a resting dot fades (start and end of a loop) or swaps instantly (hand-over). */
  const [fade, setFade] = useState(true);
  /** The moving dot, shown only between two markers. */
  const [travelling, setTravelling] = useState(false);
  const [instant, setInstant] = useState(true);

  // Measure each marker's centre relative to the wrapper, again on every resize.
  useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap) {
      return;
    }
    const measure = () => {
      const base = wrap.getBoundingClientRect().top;
      const markers = wrap.querySelectorAll<HTMLElement>('[data-marker]');
      setTops(
        Array.from(markers, (m) => {
          const r = m.getBoundingClientRect();
          return r.top - base + r.height / 2;
        }),
      );
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(wrap);
    return () => observer.disconnect();
  }, []);

  // The loop: fade in on step 0, then light, hold, dim and move to the next step.
  useEffect(() => {
    if (tops.length !== STEPS) {
      return;
    }
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      return;
    }
    let cancelled = false;
    const timers: number[] = [];
    const wait = (ms: number) =>
      new Promise<void>((resolve) => {
        timers.push(window.setTimeout(resolve, ms));
      });
    const run = async () => {
      while (!cancelled) {
        setFade(true);
        setResting(0);
        await wait(FADE_MS);
        for (let step = 0; step < STEPS && !cancelled; step++) {
          if (step > 0) {
            // Hand over from the marker to the moving dot, travel, hand back.
            setFade(false);
            setInstant(true);
            setPosition(step - 1);
            setTravelling(true);
            setResting(-1);
            await wait(30);
            setInstant(false);
            setPosition(step);
            await wait(MOVE_MS);
            setTravelling(false);
            setResting(step);
          }
          setActive(step);
          await wait(HOLD_MS);
          setActive(-1);
          await wait(250);
        }
        setFade(true);
        setResting(-1);
        await wait(FADE_MS + 600);
      }
    };
    run();
    return () => {
      cancelled = true;
      for (const t of timers) {
        window.clearTimeout(t);
      }
    };
  }, [tops.length]);

  const top = tops[position] ?? 0;

  return (
    <figure className="not-prose my-8 min-w-0">
      <div className="rounded-[14px] border border-line bg-ground p-4 sm:p-6">
        <div ref={wrapRef} className="relative">
          {/* The spine, fading out at both ends. */}
          <div
            aria-hidden
            className="absolute top-0 bottom-0 left-[7px] w-px bg-[linear-gradient(to_bottom,transparent,var(--color-line)_12%,var(--color-line)_88%,transparent)]"
          />
          <span
            aria-hidden
            className="absolute left-[4px] z-10 block size-[7px] rounded-full bg-brand shadow-[0_0_10px_3px_rgba(34,211,238,0.45)] motion-reduce:hidden"
            style={{
              top: top - 3.5,
              opacity: travelling ? 1 : 0,
              transition: instant ? 'none' : `top ${MOVE_MS}ms cubic-bezier(.45,0,.55,1)`,
            }}
          />
          <ol className="m-0 list-none space-y-3 p-0">
            <Step
              index={0}
              active={active}
              resting={resting}
              fade={fade}
              title="Your repo"
              note="Git is the source of truth."
            >
              <div className="flex flex-wrap items-center gap-1.5 font-mono text-[11.5px]">
                <span className="text-fg">payments-api/</span>
                {['curl-runner.yaml', 'charges.yaml', 'refunds.yaml'].map((file) => (
                  <span
                    key={file}
                    className="rounded-md border border-line px-2 py-0.5 text-fg-secondary"
                  >
                    {file}
                  </span>
                ))}
              </div>
            </Step>

            <Step
              index={1}
              active={active}
              resting={resting}
              fade={fade}
              title="CI"
              note="Runs on every push to main."
            >
              <Command run="curl-runner publish" result="payments-api: published v3" />
            </Step>

            <Step
              index={2}
              active={active}
              resting={resting}
              fade={fade}
              title="curl-runner Cloud"
              note="Keeps every version."
            >
              <div className="flex flex-wrap items-center gap-x-4 gap-y-3">
                <div className="flex gap-1.5" role="list" aria-label="Versions">
                  {['v1', 'v2', 'v3'].map((version) => {
                    const latest = version === 'v3';
                    return (
                      <span
                        key={version}
                        role="listitem"
                        className={`rounded-md border px-2 py-0.5 font-mono text-[11.5px] ${
                          latest
                            ? 'border-brand/40 bg-brand/10 text-brand'
                            : 'border-line text-fg-tertiary'
                        }`}
                      >
                        {version}
                        {latest ? (
                          <span className="ml-1.5 font-sans text-[11px]">latest</span>
                        ) : null}
                      </span>
                    );
                  })}
                </div>
                <SourceSwitch />
              </div>
            </Step>

            <Step
              index={3}
              active={active}
              resting={resting}
              fade={fade}
              title="Your customers"
              note="Always on the latest."
            >
              <Command run="curl-runner pull payments-api" result="Updated Payments API to v3." />
            </Step>
          </ol>
        </div>
      </div>
      {children ? (
        <figcaption className="mt-3.5 text-[13px] leading-[1.6] text-fg-tertiary">
          {children}
        </figcaption>
      ) : null}
    </figure>
  );
}

function Step({
  index,
  active,
  resting,
  fade,
  title,
  note,
  children,
}: {
  index: number;
  active: number;
  resting: number;
  fade: boolean;
  title: string;
  note: string;
  children: ReactNode;
}) {
  const lit = active === index;
  return (
    <li className="relative m-0 p-0 pl-7">
      <span
        aria-hidden
        data-marker
        className={`absolute top-1/2 left-0 flex size-[15px] -translate-y-1/2 items-center justify-center rounded-full border bg-ground transition-[border-color,box-shadow] duration-500 ${
          lit ? 'border-brand shadow-[0_0_10px_rgba(34,211,238,0.5)]' : 'border-line'
        }`}
      >
        <span
          className="block size-[7px] rounded-full bg-brand shadow-[0_0_10px_3px_rgba(34,211,238,0.45)] motion-reduce:hidden"
          style={{
            opacity: resting === index ? 1 : 0,
            transition: fade ? 'opacity 400ms ease' : 'none',
          }}
        />
      </span>
      <section
        className={`flex flex-col gap-3 rounded-[10px] border bg-surface px-4 py-3.5 transition-[border-color,box-shadow] duration-500 sm:flex-row sm:items-center sm:gap-6 ${
          lit
            ? 'border-brand/50 shadow-[0_0_32px_-12px_rgba(34,211,238,0.45)]'
            : 'border-line shadow-[0_0_32px_-12px_rgba(34,211,238,0)]'
        }`}
      >
        <div className="sm:w-40 sm:shrink-0">
          <h3 className="m-0 text-[13px] leading-5 font-medium text-fg">{title}</h3>
          <p className="m-0 text-[12px] leading-[1.45] text-fg-tertiary">{note}</p>
        </div>
        <div className="min-w-0 flex-1">{children}</div>
      </section>
    </li>
  );
}

function SourceSwitch() {
  return (
    <div className="flex items-center gap-2">
      <span className="text-[11px] text-fg-tertiary">Source</span>
      <div
        className="flex rounded-md border border-line p-0.5 text-[11px]"
        role="img"
        aria-label="Source: git. Cloud as the source is coming."
      >
        <span className="rounded-[5px] bg-selected px-2 py-0.5 text-fg">git</span>
        <span className="px-2 py-0.5 text-fg-tertiary">Cloud</span>
      </div>
    </div>
  );
}

function Command({ run, result }: { run: string; result: string }) {
  return (
    <div className="min-w-0 font-mono text-[11.5px] leading-5">
      <p className="m-0 truncate text-fg">
        <span className="text-fg-tertiary">$ </span>
        {run}
      </p>
      <p className="m-0 truncate text-success">{result}</p>
    </div>
  );
}
