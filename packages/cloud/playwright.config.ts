import { defineConfig } from '@playwright/test';

// The journey needs a running stack: Postgres, the API and `next start`. CI
// starts them in the e2e job. Locally, see the variables in `e2e/journey.spec.ts`.
export default defineConfig({
  testDir: './e2e',
  timeout: 90_000,
  expect: { timeout: 10_000 },
  retries: 0,
  workers: 1,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: {
    baseURL: process.env.E2E_APP_URL ?? 'http://localhost:3090',
    viewport: { width: 1280, height: 800 },
    trace: 'retain-on-failure',
  },
});
