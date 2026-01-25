import { Apple, Download } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { BinaryDownload } from '@/lib/github-release';
import { formatFileSize, getPlatformLabel } from '@/lib/github-release';

interface DownloadButtonProps {
  binary: BinaryDownload;
  variant?: 'default' | 'outline' | 'secondary';
  showSize?: boolean;
}

function WindowsLogo({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="currentColor"
      className={className}
      xmlns="http://www.w3.org/2000/svg"
    >
      <path d="M0 3.45v7.298l9.792-.052V3.45zm10.104 0v7.247l13.79.052v-7.3zM0 13.404v7.3l9.792.051v-7.352zm10.104.052v7.247l13.79.052V13.455z" />
    </svg>
  );
}

function LinuxLogo({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="currentColor"
      className={className}
      xmlns="http://www.w3.org/2000/svg"
    >
      <path d="M12.504 0c-.155 0-.315.008-.48.021-4.226.333-3.105 4.807-3.17 6.298-.076 1.092-.3 1.953-1.05 3.02-.885 1.051-2.127 2.75-2.716 4.521-.278.88-.314 1.78-.078 2.53.362 1.16 1.05 2.11 1.967 2.95.53.483 1.11.925 1.72 1.26 1.035.57 2.157.96 3.36 1.15.36.06.73.1 1.1.12.165.01.33.015.495.015.165 0 .33-.005.495-.015.37-.02.74-.06 1.1-.12 1.203-.19 2.325-.58 3.36-1.15.61-.335 1.19-.777 1.72-1.26.917-.84 1.605-1.79 1.967-2.95.236-.75.2-1.65-.078-2.53-.589-1.771-1.831-3.47-2.716-4.521-.75-1.067-.974-1.928-1.05-3.02-.065-1.491 1.056-5.965-3.17-6.298-.165-.013-.325-.021-.48-.021zm-.555 2.61c.098 0 .195.005.293.013 1.082.087 1.61 1.12 1.61 1.12s.53-1.033 1.61-1.12c.098-.008.195-.013.293-.013.917 0 1.62.765 1.71 1.71.075.72.255 1.33.75 2.01.585.805 1.56 2.11 2.01 3.47.21.66.24 1.29.075 1.83-.225.735-.69 1.38-1.35 1.98-.48.44-.99.81-1.52 1.08-.88.485-1.85.81-2.88.96-.315.05-.64.08-.975.095-.16.01-.32.01-.48.01s-.32 0-.48-.01c-.335-.015-.66-.045-.975-.095-1.03-.15-2-.475-2.88-.96-.53-.27-1.04-.64-1.52-1.08-.66-.6-1.125-1.245-1.35-1.98-.165-.54-.135-1.17.075-1.83.45-1.36 1.425-2.665 2.01-3.47.495-.68.675-1.29.75-2.01.09-.945.793-1.71 1.71-1.71z" />
    </svg>
  );
}

function getPlatformIcon(os: string) {
  switch (os) {
    case 'darwin':
      return Apple;
    case 'windows':
      return WindowsLogo;
    case 'linux':
      return LinuxLogo;
    default:
      return LinuxLogo;
  }
}

export function DownloadButton({
  binary,
  variant = 'outline',
  showSize = true,
}: DownloadButtonProps) {
  const platformLabel = getPlatformLabel(binary.os, binary.arch);
  const fileSize = formatFileSize(binary.size);
  const PlatformIcon = getPlatformIcon(binary.os);

  return (
    <Button
      asChild
      variant={variant}
      className="w-full justify-between gap-3 px-4 py-6 h-auto active:scale-[0.98] transition-transform"
    >
      <a href={binary.downloadUrl} download={binary.fileName}>
        <div className="flex items-center gap-3 flex-1">
          <PlatformIcon className="h-5 w-5 flex-shrink-0" />
          <span className="flex-1 text-left">
            <span className="block font-medium">{platformLabel}</span>
            {showSize && <span className="block text-xs text-muted-foreground">{fileSize}</span>}
          </span>
        </div>
        <Download className="h-4 w-4 flex-shrink-0 text-muted-foreground" />
      </a>
    </Button>
  );
}
