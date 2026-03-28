import { expect, test, type Page } from '@playwright/test';

async function waitForShell(page: Page) {
  await page.goto('/');
  await page.waitForSelector('[data-testid="title-bar"]', { timeout: 15000 });
  await expect(page.locator('[data-testid="app-title"]')).toHaveText('UniFi');
}

test.describe('UniFi shell', () => {
  test.beforeEach(async ({ page }) => {
    await waitForShell(page);
  });

  test('renders shell actions and tabs', async ({ page }) => {
    await expect(page.locator('[data-testid="tab-networks"]')).toBeVisible();
    await expect(page.locator('[data-testid="tab-channels"]')).toBeVisible();
    await expect(page.locator('[data-testid="tab-groups"]')).toBeVisible();
    await expect(page.locator('[data-testid="tab-roaming"]')).toBeVisible();
    await expect(page.getByRole('button', { name: /Scan|扫描/ })).toBeVisible();
    await expect(page.getByRole('button', { name: /Export|导出/ })).toBeVisible();
  });

  test('keeps networks table interactive', async ({ page }) => {
    await page.waitForTimeout(2500);

    const rows = page.locator('[data-testid="network-row"]');
    const rowCount = await rows.count();
    if (rowCount === 0) {
      await expect(page.locator('text=/No networks found|未发现网络|Scanning|扫描中/').first()).toBeVisible();
      return;
    }

    await rows.first().click();
    await expect(page.getByRole('button', { name: /View Beacon Frame|查看 Beacon Frame/ })).toBeVisible();
  });
});

test.describe('Channel analysis', () => {
  test.beforeEach(async ({ page }) => {
    await waitForShell(page);
    await page.locator('[data-testid="tab-channels"]').click();
  });

  test('renders channel summary and channel cards', async ({ page }) => {
    await expect(page.locator('[data-testid="channels-view"]')).toBeVisible();
    await expect(page.locator('[data-testid="channels-summary-scanned"]')).toBeVisible();
    await expect(page.locator('[data-testid="channels-summary-best"]')).toBeVisible();
    await expect(page.locator('[data-testid="channel-card"]').first()).toBeVisible();
  });

  test('switches between 2.4 GHz and 5 GHz bands', async ({ page }) => {
    const band24 = page.locator('[data-testid="channel-band-2.4"]');
    const band5 = page.locator('[data-testid="channel-band-5"]');

    await expect(band24).toBeVisible();
    await expect(band5).toBeVisible();
    await band5.click();
    await expect(band5).toHaveClass(/bg-blue-500/);
    await expect(page.locator('[data-testid="channel-card"]')).toHaveCount(25);
  });
});

test.describe('Network groups', () => {
  test.beforeEach(async ({ page }) => {
    await waitForShell(page);
    await page.locator('[data-testid="tab-groups"]').click();
    await page.waitForTimeout(1000);
  });

  test('renders groups summary or empty state cleanly', async ({ page }) => {
    await expect(page.locator('[data-testid="groups-view"]')).toBeVisible();

    const cards = page.locator('[data-testid="group-card"]');
    if (await cards.count()) {
      await expect(page.locator('[data-testid="groups-summary"]')).toBeVisible();
      await expect(cards.first()).toBeVisible();
    } else {
      await expect(page.locator('[data-testid="groups-empty"]')).toBeVisible();
    }
  });
});
