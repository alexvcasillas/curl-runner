export const COMPARISON_CHECKED = '2026-11-03';

export interface ComparisonRow {
  label: string;
  curlRunner: string;
  postman: string;
  insomnia: string;
  curl: string;
  httpie: string;
}

export interface ComparisonCard {
  tool: string;
  what: string;
  pickCurlRunner: string;
  pickTool: string;
}

// Competitor values below were checked against official sources on COMPARISON_CHECKED.
// Where the official docs are silent we say "Not documented" rather than guess.
// curl-runner values reflect the repo (README.md, Cargo.toml license = "MIT", which
// crates/curl-runner-studio inherits) and the docs (features/studio, features/postman-import).
export const COMPARISON_ROWS: readonly ComparisonRow[] = [
  {
    label: 'Primary interface',
    curlRunner: 'CLI, plus a macOS app (preview)',
    // https://www.postman.com/downloads/ ("Postman on the web"); https://www.postman.com/pricing/ (Postman CLI); https://learning.postman.com/docs/postman-cli/postman-cli-overview/
    postman: 'Desktop and web app, plus the Postman CLI',
    // https://github.com/Kong/insomnia (cross-platform API client); https://developer.konghq.com/inso-cli/
    insomnia: 'Desktop app, plus the inso CLI',
    curl: 'CLI',
    // https://github.com/httpie/cli (links to HTTPie for Desktop and Web)
    httpie: 'CLI, plus desktop and web apps',
  },
  {
    label: 'Tests are written as',
    curlRunner: 'YAML files in your repo',
    // https://learning.postman.com/docs/collections/running-collections/intro-to-collection-runs/ (scripts, pm.execution)
    postman: 'Collections with JavaScript scripts',
    // https://developer.konghq.com/inso-cli/ (tests on design documents, request collections)
    insomnia: 'Collections and design documents',
    curl: 'Shell commands',
    httpie: 'Shell commands',
  },
  {
    label: 'Built-in assertions',
    curlRunner: 'Status, headers, body, timing',
    postman: 'Yes, in JavaScript test scripts',
    // https://developer.konghq.com/inso-cli/ ("Run tests")
    insomnia: 'Yes, as test suites',
    // https://curl.se/docs/manpage.html (--fail / --fail-with-body only act on HTTP error status)
    curl: 'Only HTTP error status (--fail)',
    // https://httpie.io/docs/cli/exit-status (--check-status exits non-zero on 3xx/4xx/5xx)
    httpie: 'Only HTTP error status (--check-status)',
  },
  {
    label: 'Headless / CI',
    curlRunner: 'Native, one binary',
    // https://learning.postman.com/docs/collections/running-collections/intro-to-collection-runs/ (Postman CLI for CI/CD); https://github.com/postmanlabs/newman
    postman: 'Via the Postman CLI or Newman',
    // https://developer.konghq.com/inso-cli/ ("from a terminal or CI/CD pipeline")
    insomnia: 'Via the inso CLI',
    curl: 'Yes',
    httpie: 'Yes',
  },
  {
    label: 'Parallel runs',
    curlRunner: 'Built in (--parallel)',
    // https://learning.postman.com/docs/collections/running-collections/intro-to-collection-runs/ (requests run in a chosen order, one after another)
    postman: 'Collection runs execute requests in order',
    // https://developer.konghq.com/inso-cli/ and https://github.com/Kong/insomnia (no parallel execution documented)
    insomnia: 'Not documented',
    // https://curl.se/docs/manpage.html (-Z, --parallel: "perform all transfers in parallel")
    curl: '--parallel for transfers',
    // https://httpie.io/docs/cli (no parallel option documented)
    httpie: 'Not documented',
  },
  {
    label: 'Import from Postman / OpenAPI / curl',
    curlRunner: 'Postman collections, OpenAPI/Swagger specs and curl (CLI and app)',
    // https://learning.postman.com/docs/getting-started/importing-and-exporting/importing-data/ (cURL, OpenAPI, Insomnia and other clients)
    postman: 'curl, OpenAPI, Insomnia and more',
    // https://developer.konghq.com/insomnia/import-export/ (Postman v2.0/v2.1, HAR, OpenAPI, cURL)
    insomnia: 'Postman collections, curl, OpenAPI and more',
    curl: 'Not applicable',
    // https://httpie.io/docs/desktop/postman-and-insomnia-import; https://httpie.io/docs/desktop/curl-import (desktop and web app docs; no CLI import documented)
    httpie: 'App: Postman, Insomnia, curl',
  },
  {
    label: 'Account needed',
    curlRunner: 'No (CLI or app)',
    // https://www.postman.com/pricing/ (Free plan); https://learning.postman.com/docs/getting-started/first-steps/sending-the-first-request/ (sign in to save); https://learning.postman.com/docs/postman-cli/postman-cli-overview/ (login, or init without an account)
    postman: 'Free plan available; sign-in needed to save work',
    // https://github.com/Kong/insomnia (local Scratch Pad without an account; most capabilities need one)
    insomnia: 'Local Scratch Pad needs none; most features do',
    curl: 'No',
    // https://httpie.io/product (use the app offline and without an account); https://httpie.io/docs/cli
    httpie: 'No (CLI); desktop app works without one',
  },
  {
    label: 'Open source',
    curlRunner: 'Yes, MIT (CLI and app)',
    // https://github.com/postmanlabs/newman (Apache-2.0); https://github.com/postmanlabs/postman-app-support (no app license stated)
    postman: 'Newman: Yes (Apache-2.0). App: no license stated',
    // https://github.com/Kong/insomnia (Apache-2.0)
    insomnia: 'Yes, Apache-2.0',
    curl: 'Yes',
    // https://github.com/httpie/cli (BSD-3-Clause); desktop/web app status not stated
    httpie: 'CLI: Yes (BSD-3-Clause)',
  },
];

export const COMPARISON_CARDS: readonly ComparisonCard[] = [
  {
    tool: 'Postman',
    what: 'GUI platform for designing, exploring and sharing APIs.',
    pickCurlRunner:
      'Suites reviewed in pull requests, CI without exports, one binary on any runner. Bring a collection over with convert postman, and write and debug in the macOS app (preview).',
    pickTool:
      'Exploring an API in the browser, shared workspaces, mock servers and published docs.',
  },
  {
    tool: 'curl',
    what: 'The universal command-line transfer tool, and the engine behind many scripts.',
    pickCurlRunner: 'Many requests with assertions, variables, retries and a report at the end.',
    pickTool:
      'One-off requests, scripts that only need the bytes, and anywhere you cannot install anything.',
  },
  {
    tool: 'Insomnia',
    what: 'Desktop API client with design and testing features.',
    pickCurlRunner:
      'Configuration as code, headless servers and terminal-first workflows, with a macOS app (preview) on the same files.',
    pickTool: 'A polished desktop client for GraphQL and gRPC exploration and design-first work.',
  },
  {
    tool: 'HTTPie',
    what: 'Human-friendly HTTP client for the terminal.',
    pickCurlRunner: 'Repeatable suites with validation, chaining and parallel runs.',
    pickTool: 'Quick, readable ad-hoc requests while you poke at an API.',
  },
];
