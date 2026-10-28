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

const PLATFORM_ORDER = ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64', 'windows-x64'];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

/**
 * Pure parser for a GitHub release payload. Returns null unless the tag looks
 * like `@curl-runner/cli@X.Y.Z`.
 */
export function parseRelease(json: unknown): LatestRelease | null {
  if (!isRecord(json) || typeof json.tag_name !== 'string') {
    return null;
  }
  const versionMatch = json.tag_name.match(/^@curl-runner\/cli@(\d+\.\d+\.\d+\S*)$/);
  if (!versionMatch) {
    return null;
  }

  const binaries: BinaryDownload[] = [];
  let checksumUrl: string | undefined;
  const assets = Array.isArray(json.assets) ? json.assets : [];

  for (const asset of assets) {
    if (
      !isRecord(asset) ||
      typeof asset.name !== 'string' ||
      typeof asset.browser_download_url !== 'string'
    ) {
      continue;
    }
    if (asset.name === 'SHA256SUMS.txt') {
      checksumUrl = asset.browser_download_url;
      continue;
    }
    const match = asset.name.match(
      /^curl-runner-cli-.+-(linux|darwin|windows)-(x64|arm64)\.(tar\.gz|zip)$/,
    );
    if (match) {
      binaries.push({
        platform: `${match[1]}-${match[2]}`,
        arch: match[2],
        os: match[1] as BinaryDownload['os'],
        fileName: asset.name,
        downloadUrl: asset.browser_download_url,
        size: typeof asset.size === 'number' ? asset.size : 0,
      });
    }
  }

  binaries.sort((x, y) => PLATFORM_ORDER.indexOf(x.platform) - PLATFORM_ORDER.indexOf(y.platform));

  return {
    version: versionMatch[1],
    tagName: json.tag_name,
    publishedAt: typeof json.published_at === 'string' ? json.published_at : '',
    htmlUrl:
      typeof json.html_url === 'string'
        ? json.html_url
        : `https://github.com/${GITHUB_REPO}/releases`,
    binaries,
    checksumUrl,
  };
}

/** Throws on any failure so that `unstable_cache` never stores it. */
async function fetchLatestReleaseOrThrow(): Promise<LatestRelease | null> {
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
    throw new Error(`GitHub API error: ${response.status}`);
  }

  return parseRelease(await response.json());
}

const getCachedLatestRelease = unstable_cache(
  fetchLatestReleaseOrThrow,
  ['github-latest-release'],
  {
    revalidate: 3600, // 1 hour
    tags: ['github-release'],
  },
);

/**
 * The latest CLI release, cached for an hour. A failed fetch resolves null but
 * is not cached, so the next request retries instead of waiting out the hour.
 */
export async function getLatestRelease(): Promise<LatestRelease | null> {
  try {
    return await getCachedLatestRelease();
  } catch (error) {
    console.error('Failed to fetch latest release:', error);
    return null;
  }
}

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
