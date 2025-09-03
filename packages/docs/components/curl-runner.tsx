import { cn } from '@/lib/utils';

export const CurlRunner = ({ className }: { className?: string }) => {
  return (
    <span
      className={cn(
        'font-semibold font-mono bg-gradient-to-br from-cyan-400 to-cyan-600 bg-clip-text text-transparent',
        className,
      )}
    >
      curl-runner
    </span>
  );
};
