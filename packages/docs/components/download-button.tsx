import { Apple, Download, Monitor, Server } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { BinaryDownload } from '@/lib/github-release';
import { formatFileSize, getPlatformLabel } from '@/lib/github-release';

interface DownloadButtonProps {
  binary: BinaryDownload;
  variant?: 'default' | 'outline' | 'secondary';
  showSize?: boolean;
}

function getPlatformIcon(os: string) {
  switch (os) {
    case 'darwin':
      return Apple;
    case 'windows':
      return Monitor;
    case 'linux':
      return Server;
    default:
      return Server;
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
