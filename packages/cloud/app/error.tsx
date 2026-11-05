'use client';

import { Button } from '@/components/Button';
import { EmptyState } from '@/components/EmptyState';

export default function ErrorPage({
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <main className="mx-auto box-border flex min-h-dvh w-full max-w-[560px] flex-col justify-center px-4 py-16">
      <EmptyState
        as="h1"
        code="503 Service Unavailable"
        title="Cloud isn’t answering right now"
        actions={
          <Button variant="primary" onClick={reset}>
            Try again
          </Button>
        }
      >
        Your collections are safe. Try again in a minute.
      </EmptyState>
    </main>
  );
}
