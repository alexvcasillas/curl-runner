'use client';

import { useEffect, useRef, useState } from 'react';
import { AuthMessage } from '@/components/AuthMessage';
import { Spinner } from '@/components/Spinner';
import type { AuthProblem } from '@/lib/sign-in';

/**
 * Runs a sign-in step when the page opens, so a mail scanner that only fetches
 * the link never spends it. Shows the wait, then what went wrong. On success the
 * action redirects.
 */
export function AutoRun({
  run,
  title,
}: {
  run: () => Promise<AuthProblem | undefined>;
  title: string;
}) {
  const [problem, setProblem] = useState<AuthProblem | null>(null);
  // Strict Mode runs effects twice in development, and a link works once.
  const started = useRef(false);

  useEffect(() => {
    if (started.current) {
      return;
    }
    started.current = true;
    run().then((p) => p && setProblem(p));
  }, [run]);

  if (problem) {
    return <AuthMessage {...problem} />;
  }
  return (
    <div role="status" className="flex flex-col items-center gap-4 pt-12 text-center">
      <Spinner size={20} />
      <h1 className="m-0 text-[20px] leading-7 font-semibold">{title}</h1>
      <p className="m-0 text-fg-muted">This takes a second.</p>
    </div>
  );
}
