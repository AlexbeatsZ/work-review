import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
test.beforeEach(async ({ page }) => {
  page.on('pageerror', (error) => {
    throw error;
  });
});
async function login(page) {
  await page.goto('/');
  await page.getByLabel('查看密钥').fill('test-viewer-key-0000000000000000000');
  await page.getByRole('button', { name: '查看工作记录' }).click();
  await expect(page.getByRole('heading', { name: '一天的足迹' })).toBeVisible();
}
test('aggregate dashboard, device filters, search and note persistence', async ({
  page,
}) => {
  await login(page);
  await expect(page.locator('.activity-row')).toHaveCount(3);
  await expect(page.locator('.primary-metric strong')).toHaveText('35 分');
  await expect(page.locator('.metric').nth(1).locator('strong')).toHaveText(
    '45 分',
  );
  await mkdir('artifacts', { recursive: true });
  await page.screenshot({
    path: 'artifacts/dashboard-desktop.png',
    fullPage: true,
  });
  await page.getByRole('button', { name: '⌘ MacBook' }).click();
  await expect(page.locator('.activity-row')).toHaveCount(1);
  await page.locator('.activity-row').click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.getByAltText('该时段的屏幕截图')).toBeVisible();
  await page.getByLabel('备注').fill('验证 macOS 记录');
  await page.getByRole('button', { name: '保存备注' }).click();
  await expect(page.locator('.note-preview')).toContainText('验证 macOS 记录');
  await page.getByRole('button', { name: '关闭', exact: true }).click();
  // Let the old all-device response arrive during the search debounce.
  await page.route('**/api/activities?**', async (route) => {
    const params = new URL(route.request().url()).searchParams;
    if (!params.has('device') && !params.has('q'))
      await new Promise((resolve) => setTimeout(resolve, 150));
    await route.continue();
  });
  await page.getByRole('button', { name: '全部设备' }).click();
  await page.getByLabel('搜索记录').fill('后台采集');
  await expect(page.locator('.activity-row')).toHaveCount(1);
  await expect(page.locator('.activity-copy')).toContainText('Code');
  await page.getByLabel('搜索记录').fill('无此记录');
  await expect(page.getByText('没有匹配的记录')).toBeVisible();
  await page.getByLabel('搜索记录').fill('');
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: '导出记录' }).click();
  expect((await download).suggestedFilename()).toMatch(/工作记录.*\.md$/);
});
test('mobile layout and invalid credentials', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  await page.getByLabel('查看密钥').fill('wrong');
  await page.getByRole('button', { name: '查看工作记录' }).click();
  await expect(page.getByRole('alert')).toContainText('查看密钥无效');
  await login(page);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await expect(page.getByLabel('搜索记录')).toBeVisible();
  await mkdir('artifacts', { recursive: true });
  await page.screenshot({
    path: 'artifacts/dashboard-mobile.png',
    fullPage: true,
  });
});
