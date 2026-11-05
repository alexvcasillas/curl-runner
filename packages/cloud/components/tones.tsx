import { CheckIcon, ErrorIcon, InfoIcon, WarningIcon } from './icons';

/** A state colour. Always paired with an icon or a word. */
export type Tone = 'info' | 'ok' | 'warn' | 'err';

export const TONE_BOX: Record<Tone, string> = {
  info: 'border-info-border bg-info-bg',
  ok: 'border-success-border bg-success-bg',
  warn: 'border-warning-border bg-warning-bg',
  err: 'border-danger-border bg-danger-bg',
};

export function ToneIcon({ tone, size = 14 }: { tone: Tone; size?: number }) {
  switch (tone) {
    case 'info':
      return <InfoIcon size={size} className="shrink-0 text-info" />;
    case 'ok':
      return <CheckIcon size={size} className="shrink-0 text-success" />;
    case 'warn':
      return <WarningIcon size={size} className="shrink-0 text-warning" />;
    case 'err':
      return <ErrorIcon size={size} className="shrink-0 text-danger" />;
  }
}
