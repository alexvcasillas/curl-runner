import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { expect, test } from '@playwright/test';

// The stack under test. CI and the local recipe start it before the run:
//   DATABASE_URL  Postgres the API uses. The test reads the sign-in code from it with `psql`.
//   E2E_API_URL   The API, started with SIGNUP=open, DEFAULT_PLAN=free and SMTP_HOST pointing at
//                 a port nothing listens on. With no SMTP the server logs a mail and blanks its
//                 stored body at once. A failed send keeps the body for 30 seconds, long enough to read.
//   E2E_APP_URL   The dashboard (the Playwright `baseURL`).
//   CURL_RUNNER_BIN  The CLI binary. Defaults to the release build.
const DATABASE_URL = process.env.DATABASE_URL ?? '';
const API_URL = process.env.E2E_API_URL ?? 'http://localhost:8090';
const CLI =
  process.env.CURL_RUNNER_BIN ?? path.resolve(__dirname, '../../../target/release/curl-runner');

/** The newest sign-in code mailed to `email`. The server queues mail there and, while sending fails, keeps the body. */
async function signInCode(email: string): Promise<string> {
  const sql = `SELECT text_body FROM outbox_emails WHERE to_addr = '${email}' ORDER BY next_attempt_at DESC LIMIT 1`;
  for (let i = 0; i < 40; i++) {
    const out = execFileSync('psql', [DATABASE_URL, '-t', '-A', '-c', sql], { encoding: 'utf8' });
    const code = out.match(/sign-in code is (\d{6})/)?.[1];
    if (code) {
      return code;
    }
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error(`No sign-in code for ${email} in outbox_emails.`);
}

function fixture(): string {
  const dir = mkdtempSync(path.join(tmpdir(), 'cloud-e2e-'));
  mkdirSync(path.join(dir, 'requests'));
  writeFileSync(
    path.join(dir, 'curl-runner.yaml'),
    [
      'cloud:',
      '  collection: payments-api',
      '  name: Payments API',
      '  description: Smoke tests for the payments service.',
      '  inputs:',
      '    - name: BASE_URL',
      '      description: Where the API runs.',
      '      default: https://api.example.com',
      '',
    ].join('\n'),
  );
  writeFileSync(
    path.join(dir, 'requests', 'create-charge.yaml'),
    [
      'request:',
      '  name: Create charge',
      '  method: POST',
      // biome-ignore lint/suspicious/noTemplateCurlyInString: a curl-runner variable, not a template.
      '  url: ${BASE_URL}/v1/charges',
      '  expect:',
      '    status: [200, 201]',
      '',
    ].join('\n'),
  );
  return dir;
}

test('sign in, create a team, publish, share, and open the link', async ({ page, browser }) => {
  expect(DATABASE_URL, 'Set DATABASE_URL to the API database.').not.toBe('');
  const email = `e2e-${Date.now()}@example.com`;

  // 1. Sign in by email.
  await page.goto('/sign-in');
  await page.getByLabel('Email').fill(email);
  await page.getByRole('button', { name: 'Email me a code' }).click();
  await expect(page.getByRole('heading', { name: 'Check your email' })).toBeVisible();
  const code = await signInCode(email);
  await page.getByLabel('Digit 1 of 6').fill(code);
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();

  // 2. Create team acme.
  await expect(page).toHaveURL(/\/new-team/);
  await page.getByLabel('Team name').fill('Acme');
  await page.getByLabel('Slug').fill('acme');
  await page.getByRole('button', { name: 'Create team' }).click();
  await expect(page).toHaveURL(/\/t\/acme/);

  // 3. Create a token and read its value.
  await page.goto('/t/acme/tokens');
  await page.getByRole('button', { name: 'Create token' }).first().click();
  await page.getByLabel('Name').fill('e2e');
  await page.getByRole('dialog').getByRole('button', { name: 'Create token' }).click();
  const created = page.getByRole('dialog', { name: 'Token created' });
  await expect(created).toBeVisible();
  const token = (await created.getByText(/^crt_\S+$/).textContent())?.trim() ?? '';
  expect(token).toMatch(/^crt_/);
  await created.getByRole('button', { name: 'Done' }).click();

  // 4. Publish with the CLI.
  const out = execFileSync(CLI, ['publish', fixture(), '--team', 'acme', '--server', API_URL], {
    encoding: 'utf8',
    env: { ...process.env, CURL_RUNNER_TOKEN: token },
  });
  expect(out).toContain('payments-api: published v1');

  // 5. Version 1 shows in the dashboard.
  await page.goto('/t/acme');
  await page
    .getByRole('link', { name: /Payments API/ })
    .first()
    .click();
  await expect(page).toHaveURL(/\/t\/acme\/c\/payments-api/);
  await expect(page.getByRole('heading', { name: 'Payments API' })).toBeVisible();
  await expect(page.getByRole('row', { name: /v?1\b.*Current/ })).toBeVisible();

  // 6. Set access to Public and read the link.
  await page.goto('/t/acme/c/payments-api/sharing');
  // The choice saves on the server first, so click the card and wait for it to stick.
  await page.locator('label', { hasText: /^Public/ }).click();
  const link = page.getByRole('textbox', { name: 'Share link' });
  await expect(link).toHaveValue(/\/s\/\S+/);
  const shareLink = await link.inputValue();
  await expect(page.getByRole('radio', { name: /Public/ })).toBeChecked();

  // 7. Open it in a fresh context.
  const visitor = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const open = await visitor.newPage();
  await open.goto(new URL(shareLink).pathname);
  await expect(open.getByText('Payments API').first()).toBeVisible();
  await visitor.close();

  // 8. The discovery file.
  const res = await page.request.get('/.well-known/curl-runner.json');
  expect(res.status()).toBe(200);
  expect(await res.json()).toEqual({ api: expect.stringMatching(/^https?:\/\//) });
});
