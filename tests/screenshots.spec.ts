import { test, expect } from '@playwright/test';

test.describe('UI Screenshots', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('h1:has-text("UniFi")', { timeout: 10000 });
  });

  test('main view screenshot', async ({ page }) => {
    await page.waitForTimeout(5000);
    await page.screenshot({ path: 'docs/screenshots/01-main-view.png', fullPage: true });
  });

  test('network details screenshot', async ({ page }) => {
    await page.waitForTimeout(5000);
    const firstCard = page.locator('[data-testid="network-card"]').first();
    if (await firstCard.count() > 0) {
      await firstCard.click();
      await page.waitForTimeout(500);
      await page.screenshot({ path: 'docs/screenshots/02-network-details.png', fullPage: true });
    }
  });

  test('channel view screenshot', async ({ page }) => {
    await page.getByRole('button', { name: '信道分析' }).click();
    await page.waitForTimeout(2000);
    await page.screenshot({ path: 'docs/screenshots/03-channel-view.png', fullPage: true });
  });

  test('groups view screenshot', async ({ page }) => {
    await page.getByRole('button', { name: '网络分组' }).click();
    await page.waitForTimeout(2000);
    await page.screenshot({ path: 'docs/screenshots/04-groups-view.png', fullPage: true });
  });
});
