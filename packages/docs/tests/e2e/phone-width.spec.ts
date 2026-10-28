import { expect, test } from '@playwright/test';

const PAGES = [
  '/',
  '/features/retry-mechanism',
  '/alternatives',
  '/downloads',
  '/tools-database',
  '/changelog',
  '/docs/quick-start',
];

test.use({ viewport: { width: 390, height: 844 } });

for (const path of PAGES) {
  test(`${path} has no horizontal scroll at 390px`, async ({ page }) => {
    await page.goto(path);
    const { scrollWidth, innerWidth } = await page.evaluate(() => ({
      scrollWidth: document.documentElement.scrollWidth,
      innerWidth: window.innerWidth,
    }));
    expect(scrollWidth).toBeLessThanOrEqual(innerWidth);
  });
}

for (const path of ['/', '/docs/quick-start']) {
  test(`${path} marketing header stays compact at 390px`, async ({ page }) => {
    await page.goto(path);
    const height = await page.evaluate(
      () => document.querySelector('header')?.getBoundingClientRect().height ?? 0,
    );
    expect(height).toBeLessThanOrEqual(130);
  });
}
