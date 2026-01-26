'use client';

import { useEvent } from '@pulsora/react';
import { Check, Copy } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

interface CopyButtonProps {
  value: string;
  className?: string;
  trackAs?: string;
}

export function CopyButton({ value, className, trackAs }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);
  const track = useEvent();

  const onCopy = async () => {
    await navigator.clipboard.writeText(value);
    setCopied(true);
    if (trackAs) {
      track('copy_command', { type: trackAs });
    }
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button size="icon" variant="ghost" className={`h-6 w-6 ${className}`} onClick={onCopy}>
            {copied ? <Check className="h-3 w-3 text-green-500" /> : <Copy className="h-3 w-3" />}
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          <p>{copied ? 'Copied!' : 'Copy to clipboard'}</p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
