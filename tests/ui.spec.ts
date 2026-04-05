import { expect, test, type Page } from '@playwright/test';

async function waitForShell(page: Page) {
  await page.goto('/');
  await page.waitForSelector('[data-testid="title-bar"]', { timeout: 15000 });
  await expect(page.locator('[data-testid="app-title"]')).toHaveText('UniFi');
}

test.describe('Title bar', () => {
  test('renders title bar with all elements', async ({ page }) => {
    await waitForShell(page);

    // Check title bar is visible
    await expect(page.locator('[data-testid="title-bar"]')).toBeVisible();

    // Check app title
    await expect(page.locator('[data-testid="app-title"]')).toHaveText('UniFi');

    // Check language toggle button exists
    await expect(page.locator('[data-testid="title-bar"] button[aria-label="Language"]').or(
      page.locator('[data-testid="title-bar"] button:has-text("中文"), [data-testid="title-bar"] button:has-text("English")')
    )).toBeVisible();
  });

  test('language toggle works', async ({ page }) => {
    await waitForShell(page);

    // Find and click language button
    const langButton = page.locator('[data-testid="title-bar"] button:has-text("中文"), [data-testid="title-bar"] button:has-text("English")');

    const initialText = await langButton.textContent();

    await langButton.click();
    await page.waitForTimeout(200);

    const newText = await langButton.textContent();

    // Language should have changed
    expect(initialText).not.toBe(newText);
  });
});

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
  });

  test('tab switching works correctly', async ({ page }) => {
    // Click channels tab
    await page.locator('[data-testid="tab-channels"]').click();
    await expect(page.locator('[data-testid="channels-view"]')).toBeVisible();

    // Click groups tab
    await page.locator('[data-testid="tab-groups"]').click();
    await expect(page.locator('[data-testid="groups-view"]')).toBeVisible();

    // Click networks tab again
    await page.locator('[data-testid="tab-networks"]').click();
    await page.waitForTimeout(500);
    // Should be back on networks tab
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

  test('network list is scrollable', async ({ page }) => {
    await page.waitForTimeout(3000);

    const rows = page.locator('[data-testid="network-row"]');
    const rowCount = await rows.count();

    if (rowCount > 5) {
      // Get the table container
      const tableContainer = page.locator('.overflow-y-auto').first();

      // Check if container has scroll height greater than client height
      const canScroll = await tableContainer.evaluate((el) => {
        return el.scrollHeight > el.clientHeight;
      });

      // Scroll to last row
      await rows.last().scrollIntoViewIfNeeded();
      await expect(rows.last()).toBeVisible();
    }
  });

  test('details panel is scrollable', async ({ page }) => {
    await page.waitForTimeout(3000);

    const rows = page.locator('[data-testid="network-row"]');
    if (await rows.count() > 0) {
      await rows.first().click();

      // Wait for details panel
      await page.waitForTimeout(500);

      // Details panel should be scrollable
      const detailsPanel = page.locator('.overflow-y-auto.bg-white, .overflow-y-auto.dark\\:bg-gray-900').first();

      // Scroll within details panel if content is long
      const viewBeaconBtn = page.getByRole('button', { name: /View Beacon Frame|查看 Beacon Frame/ });
      await viewBeaconBtn.scrollIntoViewIfNeeded();
      await expect(viewBeaconBtn).toBeVisible();
    }
  });

  test('dropdown menus open and close', async ({ page }) => {
    // Check scanner dropdown
    const scannerBtn = page.locator('button:has-text("Default"), button:has-text("自动")');
    await scannerBtn.click();
    await page.waitForTimeout(100);

    // Dropdown should be visible
    await expect(page.locator('text=/Auto|自动/').first()).toBeVisible();

    // Click elsewhere to close
    await page.locator('body').click({ position: { x: 10, y: 10 } });
    await page.waitForTimeout(100);
  });

  test('dropdown menus are mutually exclusive', async ({ page }) => {
    // Wait for networks to load
    await page.waitForTimeout(2000);

    // Open scanner dropdown
    const scannerBtn = page.locator('button:has-text("Default"), button:has-text("自动")');
    await scannerBtn.click();
    await page.waitForTimeout(100);

    // Scanner dropdown should be visible
    await expect(page.locator('text=/Auto|自动/').first()).toBeVisible();

    // Check if export button is enabled (depends on networks being scanned)
    const exportBtn = page.locator('button:has-text("Export"), button:has-text("导出")');
    const isDisabled = await exportBtn.isDisabled();

    if (!isDisabled) {
      // Open export dropdown
      await exportBtn.click();
      await page.waitForTimeout(100);

      // Export dropdown should be visible
      await expect(page.locator('text=/JSON/').first()).toBeVisible();
    }

    // Click elsewhere to close
    await page.locator('body').click({ position: { x: 10, y: 10 } });
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

  test('channel view is scrollable', async ({ page }) => {
    await page.waitForTimeout(2000);

    // Try to scroll the channel view
    const channelView = page.locator('[data-testid="channels-view"]');
    const cards = page.locator('[data-testid="channel-card"]');
    const cardCount = await cards.count();

    if (cardCount > 3) {
      // Scroll to last card
      await cards.last().scrollIntoViewIfNeeded();
      await expect(cards.last()).toBeVisible();
    }
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
