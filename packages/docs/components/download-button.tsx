import { Download } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { BinaryDownload } from '@/lib/github-release';
import { formatFileSize, getPlatformLabel } from '@/lib/github-release';

interface DownloadButtonProps {
  binary: BinaryDownload;
  variant?: 'default' | 'outline' | 'secondary';
  showSize?: boolean;
}

export function DownloadButton({
  binary,
  variant = 'outline',
  showSize = true,
}: DownloadButtonProps) {
  const platformLabel = getPlatformLabel(binary.os, binary.arch);
  const fileSize = formatFileSize(binary.size);

  return (
    <Button asChild variant={variant} className="w-full justify-start gap-2">
      <a href={binary.downloadUrl} download={binary.fileName}>
        <Download className="h-4 w-4" />
        <span className="flex-1 text-left">
          <span className="block font-medium">{platformLabel}</span>
          {showSize && <span className="block text-xs text-muted-foreground">{fileSize}</span>}
        </span>
      </a>
    </Button>
  );
}
