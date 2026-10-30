// curl-runner studio: a macOS desktop app, released as a preview in 2.2.0. It is not
// distributed yet (no signed build, Homebrew formula or download), so every page that
// shows it says so and points at the build from source.

import { SITE_URL } from '@/lib/site';

export const STUDIO_REPO_URL = 'https://github.com/alexvcasillas/curl-runner';
export const STUDIO_SOURCE_URL = `${STUDIO_REPO_URL}/tree/main/crates/curl-runner-studio`;
export const STUDIO_BUILD_COMMAND = 'cargo run -p curl-runner-studio';
export const STUDIO_CHANGELOG_HREF = '/changelog#2.2.0';
export const STUDIO_PAGE_HREF = '/features/studio';
export const STUDIO_DOCS_HREF = '/docs/features/studio';
export const STUDIO_PAGE_URL = `${SITE_URL}${STUDIO_PAGE_HREF}`;

export interface Screenshot {
  /** Path under public/. */
  src: string;
  width: number;
  height: number;
  alt: string;
  title: string;
  description: string;
}

/** Animated collection run on the home page, and its still for reduced motion. */
export const STUDIO_HERO = {
  src: '/studio/studio-collection-run.webp',
  still: '/studio/studio-collection-run-still.webp',
  width: 1440,
  height: 900,
  alt: 'curl-runner studio running the Payments API collection: fourteen requests in four folders pass one after another, the progress bar fills green and the summary reads 14 passed, 0 failed.',
} as const;

const SCREENSHOTS = {
  'collection-run': {
    src: '/studio/collection-run.png',
    width: 2400,
    height: 1500,
    alt: 'A finished collection run of the Payments API: 14 of 15 requests passed, one failed after 2 retries. The charges folder is expanded and shows each request with its status code, duration bar, checks and attempts.',
    title: 'Run a whole collection',
    description:
      'Every request in the folder, grouped as on disk, with passed, failed and retried counts and a duration bar per request. Re-run only the failures, or export the report.',
  },
  inspector: {
    src: '/studio/inspector.png',
    width: 2400,
    height: 1500,
    alt: 'The get-charge.yaml request open in the editor next to the inspector: 200 OK in 200 ms, the JSON response body, seven passing checks for status, content-type, body fields and response time, and a timing bar.',
    title: 'Read the response',
    description:
      'The body, headers and timing of the last run, and each check from the expect block next to the YAML that asked for it.',
  },
  'failed-request': {
    src: '/studio/failed-request.png',
    width: 2400,
    height: 1667,
    alt: 'A failed Capture charge request: the editor marks the two failing expect lines, the Checks tab shows expected "succeeded" against actual "pending" and expected true against actual false, a diff of the response body, and a retry timeline with two 503 attempts followed by a 200.',
    title: 'See why a check failed',
    description:
      'Expected against actual for each failed check, a diff of the response body, and every retry attempt with its status, duration and wait.',
  },
  'editor-quick-fix': {
    src: '/studio/editor-quick-fix.png',
    width: 2400,
    height: 1500,
    alt: 'The YAML editor with the key backof underlined as a warning. A popover reads: Unknown key backof (did you mean backoff?) and offers the quick fix Rename to "backoff". The Problems panel lists the same warning at line 13.',
    title: 'Catch mistakes as you type',
    description:
      'Everything curl-runner validate looks for is checked in the editor: problems are underlined as you type, and many come with a one-key quick fix.',
  },
  environments: {
    src: '/studio/environments.png',
    width: 2400,
    height: 1500,
    alt: 'The Environments screen of the Payments API collection on the Staging tab: BASE_URL, ACCOUNT_ID and API_VERSION come from .env.staging, and SECRET_API_KEY is marked secret with its value masked.',
    title: 'Environments from your .env files',
    description:
      'Each environment is a .env file in the collection. SECRET_ values stay masked, and the ones you enter in the studio are kept in the macOS Keychain, not in the file.',
  },
  history: {
    src: '/studio/history.png',
    width: 2400,
    height: 1500,
    alt: 'Run history of the Payments API: two collection runs, 14 of 15 and 15 of 15 passed. The latest is compared with the previous run: Capture charge is listed as fixed and the other 14 requests are unchanged.',
    title: 'Compare with the last run',
    description:
      'Run history stays on your machine. Pick a run to see what it fixed, what it broke and what got slower than the run before it.',
  },
} as const satisfies Record<string, Screenshot>;

export type StudioScreenshotName = keyof typeof SCREENSHOTS;

/** Feature page order. */
export const STUDIO_SCREENSHOT_NAMES: readonly StudioScreenshotName[] = [
  'collection-run',
  'inspector',
  'failed-request',
  'editor-quick-fix',
  'environments',
  'history',
];

export function studioScreenshot(name: StudioScreenshotName): Screenshot {
  return SCREENSHOTS[name];
}

export function studioScreenshots(): Screenshot[] {
  return STUDIO_SCREENSHOT_NAMES.map((name) => SCREENSHOTS[name]);
}
