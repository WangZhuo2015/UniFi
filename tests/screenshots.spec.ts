import { test, type Page } from '@playwright/test';

async function openApp(page: Page) {
  await page.goto('/');
  await page.waitForSelector('[data-testid="title-bar"]', { timeout: 15000 });
}

test.describe('UI screenshots', () => {
  test('networks view', async ({ page }) => {
    await openApp(page);
    await page.waitForTimeout(2500);
    await page.screenshot({ path: 'docs/screenshots/01-main-view.png', fullPage: true });
  });

  test('channels view', async ({ page }) => {
    await openApp(page);
    await page.locator('[data-testid="tab-channels"]').click();
    await page.waitForTimeout(500);
    await page.screenshot({ path: 'docs/screenshots/03-channel-view.png', fullPage: true });
  });

  test('groups view', async ({ page }) => {
    await openApp(page);
    await page.locator('[data-testid="tab-groups"]').click();
    await page.waitForTimeout(500);
    await page.screenshot({ path: 'docs/screenshots/04-groups-view.png', fullPage: true });
  });
});
