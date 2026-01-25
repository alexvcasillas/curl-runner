import { Download, Server } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { BinaryDownload } from '@/lib/github-release';
import { formatFileSize, getPlatformLabel } from '@/lib/github-release';

interface DownloadButtonProps {
  binary: BinaryDownload;
  variant?: 'default' | 'outline' | 'secondary';
  showSize?: boolean;
}

function AppleLogo({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      width="24"
      height="24"
      color="currentColor"
      fill="none"
      className={className}
    >
      <path
        d="M12 6.25C12 4.25 13.5 2.25 15.5 2.25C15.5 4.25 14 6.25 12 6.25Z"
        fill="currentColor"
      />
      <path
        d="M12.5 7.59001C11.9851 7.59001 11.5867 7.42646 11.1414 7.24368C10.5776 7.01225 9.93875 6.75 8.89334 6.75C7.02235 6.75 4 8.24945 4 12.2495C4 16.9016 7.10471 21.75 9.10471 21.75C9.77426 21.75 10.3775 21.4871 10.954 21.2359C11.4815 21.0059 11.9868 20.7857 12.5 20.7857C13.0132 20.7857 13.5185 21.0059 14.046 21.2359C14.6225 21.4871 15.2257 21.75 15.8953 21.75C17.2879 21.75 18.9573 19.3992 20 16.4008C18.3793 15.7202 17.338 14.118 17.338 12.25C17.338 10.621 18.2036 9.53982 19.5 8.75C18.5 7.25 17.0134 6.75 15.9447 6.75C14.8993 6.75 14.2604 7.01225 13.6966 7.24368C13.2514 7.42646 13.0149 7.59001 12.5 7.59001Z"
        fill="currentColor"
      />
    </svg>
  );
}

function WindowsLogo({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      width="24"
      height="24"
      color="currentColor"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
    >
      <path d="M14.0136 3.99538L8.01361 4.99036C5.61912 5.38744 4.42188 5.58597 3.71094 6.421C3 7.25602 3 8.46368 3 10.879L3 13.121C3 15.5363 3 16.744 3.71094 17.579C4.42188 18.414 5.61913 18.6126 8.01361 19.0096L14.0136 20.0046C17.2567 20.5424 18.8782 20.8113 19.9391 19.9171C21 19.023 21 17.3873 21 14.116V9.88402C21 6.6127 21 4.97704 19.9391 4.08286C18.8782 3.18868 17.2567 3.45758 14.0136 3.99538Z" />
      <path d="M11 4.5V19.5M3 12H21" />
    </svg>
  );
}

function getPlatformIcon(os: string) {
  switch (os) {
    case 'darwin':
      return AppleLogo;
    case 'windows':
      return WindowsLogo;
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
