import { unstable_cache } from 'next/cache';

export interface ReleaseAsset {
  name: string;
  browser_download_url: string;
  size: number;
  content_type: string;
}

export interface BinaryDownload {
  platform: string;
  arch: string;
  os: 'linux' | 'darwin' | 'windows';
  fileName: string;
  downloadUrl: string;
  size: number;
}

export interface LatestRelease {
  version: string;
  tagName: string;
  publishedAt: string;
  htmlUrl: string;
  binaries: BinaryDownload[];
  checksumUrl?: string;
}

const GITHUB_REPO = 'alexvcasillas/curl-runner';
const GITHUB_API_URL = `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`;

function parseAssets(assets: ReleaseAsset[]): {
  binaries: BinaryDownload[];
  checksumUrl?: string;
} {
  const binaries: BinaryDownload[] = [];
  let checksumUrl: string | undefined;

  for (const asset of assets) {
    // Parse binary assets
    const binaryMatch = asset.name.match(
      /^curl-runner-cli-.+-(?<os>linux|darwin|windows)-(?<arch>x64|arm64)\.(tar\.gz|zip)$/,
    );

    if (binaryMatch?.groups) {
      const { os, arch } = binaryMatch.groups;
      const platform = `${os}-${arch}`;

      binaries.push({
        platform,
        arch,
        os: os as 'linux' | 'darwin' | 'windows',
        fileName: asset.name,
        downloadUrl: asset.browser_download_url,
        size: asset.size,
      });
    }

    // Find checksum file
    if (asset.name === 'SHA256SUMS.txt') {
      checksumUrl = asset.browser_download_url;
    }
  }

  return { binaries, checksumUrl };
}

async function fetchLatestReleaseUncached(): Promise<LatestRelease | null> {
  try {
    const headers: HeadersInit = {
      Accept: 'application/vnd.github.v3+json',
    };

    // Use GITHUB_TOKEN if available for higher rate limits
    if (process.env.GITHUB_TOKEN) {
      headers.Authorization = `token ${process.env.GITHUB_TOKEN}`;
    }

    const response = await fetch(GITHUB_API_URL, {
      headers,
      next: { revalidate: false }, // Cache handled by unstable_cache
    });

    if (!response.ok) {
      console.error(`GitHub API error: ${response.status}`);
      return null;
    }

    const data = await response.json();

    // Extract version from tag (e.g., @curl-runner/cli@1.10.0 -> 1.10.0)
    const versionMatch = data.tag_name.match(/@curl-runner\/cli@(.+)/);
    const version = versionMatch ? versionMatch[1] : data.tag_name;

    const { binaries, checksumUrl } = parseAssets(data.assets);

    return {
      version,
      tagName: data.tag_name,
      publishedAt: data.published_at,
      htmlUrl: data.html_url,
      binaries,
      checksumUrl,
    };
  } catch (error) {
    console.error('Failed to fetch latest release:', error);
    return null;
  }
}

export const getLatestRelease = unstable_cache(
  async () => fetchLatestReleaseUncached(),
  ['github-latest-release'],
  {
    revalidate: 3600, // 1 hour
    tags: ['github-release'],
  },
);

export function getPlatformLabel(os: string, arch: string): string {
  const labels: Record<string, string> = {
    'linux-x64': 'Linux x64',
    'linux-arm64': 'Linux ARM64',
    'darwin-x64': 'macOS Intel',
    'darwin-arm64': 'macOS Apple Silicon',
    'windows-x64': 'Windows x64',
  };
  return labels[`${os}-${arch}`] || `${os} ${arch}`;
}

export function formatFileSize(bytes: number): string {
  const mb = bytes / (1024 * 1024);
  return `${mb.toFixed(1)} MB`;
}
