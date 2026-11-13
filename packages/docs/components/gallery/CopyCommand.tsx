'use client';

import { CopyButton } from '@/components/site/CopyButton';

/** The pull command in a mono box with a copy button. */
export function CopyCommand({ command }: { command: string }) {
  return (
    <div className="flex items-start gap-1 rounded-[10px] border border-line bg-surface pl-3">
      <code className="min-w-0 grow py-3 font-mono text-[12.5px] leading-[1.6] break-all text-fg">
        <span aria-hidden="true" className="text-fg-tertiary">
          ${' '}
        </span>
        {command}
      </code>
      <CopyButton text={command} label="Copy the pull command" />
    </div>
  );
}
