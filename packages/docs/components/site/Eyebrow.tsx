import type { ReactNode } from 'react';

export function Eyebrow({ children }: { children: ReactNode }) {
  return (
    <p className="m-0 font-mono text-[13px] tracking-[0.04em] text-brand uppercase">{children}</p>
  );
}
