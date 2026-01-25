import { ExternalLink, ShieldCheck } from 'lucide-react';
import Link from 'next/link';
import { DownloadButton } from '@/components/download-button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { getLatestRelease } from '@/lib/github-release';

interface DownloadsSectionProps {
  showHeader?: boolean;
  compact?: boolean;
}

export async function DownloadsSection({
  showHeader = true,
  compact = false,
}: DownloadsSectionProps) {
  const release = await getLatestRelease();

  if (!release || release.binaries.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Pre-compiled Binaries</CardTitle>
          <CardDescription>
            Unable to fetch latest release. Visit{' '}
            <Link
              href="https://github.com/alexvcasillas/curl-runner/releases/latest"
              className="underline"
              target="_blank"
              rel="noopener noreferrer"
            >
              GitHub Releases
            </Link>{' '}
            to download.
          </CardDescription>
        </CardHeader>
      </Card>
    );
  }

  const sortedBinaries = [...release.binaries].sort((a, b) => {
    const order = ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64', 'windows-x64'];
    return order.indexOf(a.platform) - order.indexOf(b.platform);
  });

  return (
    <Card>
      {showHeader && (
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Pre-compiled Binaries</CardTitle>
              <CardDescription>Download standalone executables for your platform</CardDescription>
            </div>
            <Badge variant="secondary">v{release.version}</Badge>
          </div>
        </CardHeader>
      )}
      <CardContent className={showHeader ? '' : 'pt-6'}>
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {sortedBinaries.map((binary) => (
            <DownloadButton
              key={binary.platform}
              binary={binary}
              variant="outline"
              showSize={!compact}
            />
          ))}
        </div>

        <div className="mt-6 flex flex-col gap-3 border-t pt-4 text-sm text-muted-foreground">
          {release.checksumUrl && (
            <div className="flex items-start gap-2">
              <ShieldCheck className="mt-0.5 h-4 w-4 flex-shrink-0" />
              <span>
                Verify downloads with{' '}
                <Link
                  href={release.checksumUrl}
                  className="underline"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  SHA256SUMS.txt
                </Link>
              </span>
            </div>
          )}
          <div className="flex items-start gap-2">
            <ExternalLink className="mt-0.5 h-4 w-4 flex-shrink-0" />
            <span>
              View{' '}
              <Link
                href={release.htmlUrl}
                className="underline"
                target="_blank"
                rel="noopener noreferrer"
              >
                full release notes
              </Link>{' '}
              on GitHub
            </span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
