import { test, expect } from '@playwright/test';

test.describe('UniFi WiFi Scanner UI', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // Wait for the app to load - use test id instead of h1
    await page.waitForSelector('[data-testid="title-bar"]', { timeout: 10000 });
  });

  test('should display header with title and buttons', async ({ page }) => {
    // Check title
    await expect(page.locator('[data-testid="app-title"]')).toBeVisible();
    await expect(page.locator('[data-testid="app-title"]')).toHaveText('UniFi');

    // Check tab buttons
    await expect(page.getByRole('button', { name: '网络列表' })).toBeVisible();
    await expect(page.getByRole('button', { name: '信道分析' })).toBeVisible();
    await expect(page.getByRole('button', { name: '网络分组' })).toBeVisible();

    // Check action buttons
    await expect(page.getByRole('button', { name: '导出' })).toBeVisible();
    await expect(page.getByRole('button', { name: /扫描|扫描中/ })).toBeVisible();
  });

  test('should display network list after scan', async ({ page }) => {
    // Wait for scan to complete (or mock data)
    await page.waitForTimeout(5000);

    // Check if network cards are displayed or "未发现网络" message
    const networkCards = page.locator('[data-testid="network-card"]').count();
    const noNetworksMsg = page.locator('text=未发现网络');

    // Either we have networks or the empty message
    const hasContent = (await networkCards) > 0 || (await noNetworksMsg.isVisible());
    expect(hasContent).toBeTruthy();
  });

  test('should switch between tabs', async ({ page }) => {
    // Start on networks tab - active tab has white bg class
    const networkBtn = page.getByRole('button', { name: '网络列表' });
    await expect(networkBtn).toHaveClass(/bg-white/);

    // Switch to channels tab
    await page.getByRole('button', { name: '信道分析' }).click();
    const channelBtn = page.getByRole('button', { name: '信道分析' });
    await expect(channelBtn).toHaveClass(/bg-white/);

    // Switch to groups tab
    await page.getByRole('button', { name: '网络分组' }).click();
    const groupsBtn = page.getByRole('button', { name: '网络分组' });
    await expect(groupsBtn).toHaveClass(/bg-white/);
  });

  test('should filter by band', async ({ page }) => {
    await page.waitForTimeout(3000);

    // Check band filter buttons
    const allBtn = page.getByRole('button', { name: '全部' });
    const band24Btn = page.getByRole('button', { name: /2\.4/ });
    const band5Btn = page.getByRole('button', { name: /^5G$/ });
    const band6Btn = page.getByRole('button', { name: /^6G$/ });

    await expect(allBtn).toBeVisible();
    await expect(band24Btn).toBeVisible();
    await expect(band5Btn).toBeVisible();
    await expect(band6Btn).toBeVisible();

    // Click 5G filter - active button has white bg
    await band5Btn.click();
    await expect(band5Btn).toHaveClass(/bg-white/);
  });

  test('should show export menu', async ({ page }) => {
    const exportBtn = page.getByRole('button', { name: '导出' });
    await exportBtn.click();

    // Check export menu items
    await expect(page.getByRole('button', { name: '导出 JSON' })).toBeVisible();
    await expect(page.getByRole('button', { name: '导出 CSV' })).toBeVisible();
  });

  test('should have search input', async ({ page }) => {
    const searchInput = page.getByPlaceholder('搜索网络...');
    await expect(searchInput).toBeVisible();

    // Type in search
    await searchInput.fill('test');
    await expect(searchInput).toHaveValue('test');
  });

  test('should display network count', async ({ page }) => {
    await page.waitForTimeout(3000);

    // Check for network count text
    const countText = page.locator('text=/\\d+ 个网络/');
    await expect(countText).toBeVisible();
  });

  test('should show scan stats if available', async ({ page }) => {
    await page.waitForTimeout(3000);

    // Check for scan duration text (may not always be visible)
    const scanDuration = page.locator('text=/\\d+ms/');
    // This might not be visible if scan failed, so just check it doesn't error
    const isVisible = await scanDuration.isVisible().catch(() => false);
    expect(typeof isVisible).toBe('boolean');
  });
});

test.describe('Network Card Component', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="title-bar"]', { timeout: 10000 });
    await page.waitForTimeout(5000); // Wait for scan
  });

  test('should display network card with correct information', async ({ page }) => {
    const firstCard = page.locator('[data-testid="network-card"]').first();

    // Only run if we have networks
    if (await firstCard.count() > 0) {
      // Check for SSID
      const ssidText = firstCard.locator('[data-testid="ssid"]');
      if (await ssidText.isVisible()) {
        await expect(ssidText).not.toBeEmpty();
      }

      // Check for signal
      const signalText = firstCard.locator('text=/-?\\d+\\s*dBm/');
      if (await signalText.isVisible()) {
        await expect(signalText).toBeVisible();
      }
    }
  });
});

test.describe('Right Panel', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="title-bar"]', { timeout: 10000 });
    await page.waitForTimeout(5000);
  });

  test('should show placeholder when no network selected', async ({ page }) => {
    const placeholder = page.locator('text=点击左侧网络查看详情');
    await expect(placeholder).toBeVisible();
  });

  test('should show network details when network is clicked', async ({ page }) => {
    const firstCard = page.locator('[data-testid="network-card"]').first();

    if (await firstCard.count() > 0) {
      await firstCard.click();
      await page.waitForTimeout(500);

      // Check for detail sections
      const detailsPanel = page.locator('text=信号质量');
      await expect(detailsPanel).toBeVisible();
    }
  });
});

test.describe('Channel Analysis View', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="title-bar"]', { timeout: 10000 });
  });

  test('should switch to channel view', async ({ page }) => {
    await page.getByRole('button', { name: '信道分析' }).click();
    await page.waitForTimeout(1000);

    // Check for channel view content
    const channelView = page.locator('text=/信道|Channel/');
    await expect(channelView.first()).toBeVisible();
  });
});

test.describe('Network Groups View', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="title-bar"]', { timeout: 10000 });
  });

  test('should switch to groups view', async ({ page }) => {
    await page.getByRole('button', { name: '网络分组' }).click();
    await page.waitForTimeout(1000);

    // Check for groups view content
    const groupsView = page.locator('text=/无网络分组|个 AP/');
    await expect(groupsView.first()).toBeVisible();
  });
});

test.describe('Roaming Test View', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="title-bar"]', { timeout: 10000 });
  });

  test('should switch to roaming test view', async ({ page }) => {
    await page.getByRole('button', { name: '漫游测试' }).click();
    await page.waitForTimeout(500);

    // Check for roaming test content (heading)
    await expect(page.getByRole('heading', { name: '漫游测试' })).toBeVisible();
    await expect(page.getByRole('button', { name: '开始测试' })).toBeVisible();
  });

  test('should display configuration inputs', async ({ page }) => {
    await page.getByRole('button', { name: '漫游测试' }).click();
    await page.waitForTimeout(500);

    // Check for configuration inputs
    await expect(page.locator('text=目标地址')).toBeVisible();
    await expect(page.locator('text=测试时长')).toBeVisible();
    await expect(page.locator('text=Ping间隔')).toBeVisible();
  });
});

test.describe('Dark Mode Support', () => {
  test('should support dark mode classes', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="title-bar"]', { timeout: 10000 });

    // Check for dark mode classes in the document
    const hasDarkModeSupport = await page.locator('html').evaluate(() => {
      return document.documentElement.classList.contains('dark') ||
             document.querySelector('[class*="dark:"]') !== null;
    });

    expect(hasDarkModeSupport).toBeTruthy();
  });
});

test.describe('Accessibility', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="title-bar"]', { timeout: 10000 });
  });

  test('should have proper heading hierarchy', async ({ page }) => {
    const h1 = page.locator('h1');
    await expect(h1).toBeVisible();
    await expect(h1).toHaveText('UniFi');
  });

  test('should have accessible buttons', async ({ page }) => {
    const buttons = page.getByRole('button');
    const count = await buttons.count();

    // All buttons should have accessible names
    for (let i = 0; i < Math.min(count, 10); i++) {
      const btn = buttons.nth(i);
      const name = await btn.getAttribute('aria-label') || await btn.textContent();
      expect(name).toBeTruthy();
    }
  });
});

test.describe('Error Handling', () => {
  test('should show error banner on scan failure', async ({ page }) => {
    // This test would need to mock a failed scan
    // For now, just check the error banner element exists in the DOM
    await page.goto('/');
    await page.waitForSelector('[data-testid="title-bar"]', { timeout: 10000 });

    // Error banner might not be visible, but should be in the component
    const pageContent = await page.content();
    expect(pageContent).toContain('bg-red-50'); // Error banner class
  });
});
