export interface HomeFeature {
  key: string;
  title: string;
  body: string;
}

export const HOME_FEATURES: readonly HomeFeature[] = [
  {
    key: 'yaml',
    title: 'Plain YAML suites',
    body: 'Requests, headers, bodies and uploads in files your team can review in a pull request.',
  },
  {
    key: 'expect',
    title: 'Response validation',
    body: 'Assert status, headers, body fields and response time. Failures say exactly what differed.',
  },
  {
    key: 'parallel',
    title: 'Parallel or sequential',
    body: 'Run a whole directory at once, or in order when requests depend on each other.',
  },
  {
    key: 'store / when',
    title: 'Chained requests',
    body: 'Store values from one response, reuse them in the next, and run steps conditionally.',
  },
  {
    key: 'retry',
    title: 'Retries and timeouts',
    body: 'Per-request retries with delays, and real millisecond timeouts for slow endpoints.',
  },
  {
    key: 'snapshot',
    title: 'Snapshots and diffs',
    body: 'Save a baseline, then see what changed between runs or environments.',
  },
  {
    key: '--profile',
    title: 'Performance profiling',
    body: 'Repeat requests, measure latency percentiles and export the results.',
  },
  {
    key: '--watch',
    title: 'Watch mode',
    body: 'Re-run the suite every time a YAML file changes while you build the API.',
  },
  {
    key: 'convert',
    title: 'From curl to YAML',
    body: 'Paste a curl command or a shell script and get a ready-to-run suite back.',
  },
];
