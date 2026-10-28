import { FileText } from 'lucide-react';
import type { ReactNode } from 'react';
import { CopyButton } from '@/components/site/CopyButton';

interface CodePaneProps {
  filename: string;
  copyText?: string;
  children: ReactNode;
}

export function CodePane({ filename, copyText, children }: CodePaneProps) {
  return (
    <div className="min-w-0 overflow-hidden rounded-[14px] border border-line bg-surface">
      <div className="flex h-11 items-center gap-2 border-b border-hairline pr-0 pl-4 font-mono text-[13px] text-fg-secondary">
        <FileText aria-hidden="true" className="size-3.5 shrink-0" />
        <span className="grow truncate">{filename}</span>
        {copyText !== undefined ? <CopyButton text={copyText} label={`Copy ${filename}`} /> : null}
      </div>
      <div className="min-w-0">{children}</div>
    </div>
  );
}
