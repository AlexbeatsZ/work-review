import fs from 'node:fs/promises';
import path from 'node:path';
import { chromium } from '@playwright/test';

// Read the private hub config from disk so keys never appear in command arguments.
const [url, configPath, output = 'artifacts/deployment'] =
  process.argv.slice(2);
if (!url || !configPath)
  throw new Error(
    'Usage: node scripts/check-deployment.mjs <hub-url> <private-hub-config> [output-directory]',
  );
const { view_token: key } = JSON.parse(await fs.readFile(configPath, 'utf8'));
const browser = await chromium.launch({
  channel: process.platform === 'win32' ? 'msedge' : undefined,
  args: ['--no-proxy-server'],
});
try {
  const page = await browser.newPage({
    viewport: { width: 1440, height: 1050 },
    timezoneId: 'Asia/Singapore',
  });
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto(url);
  await page.getByLabel('查看密钥').fill(key);
  await page.getByRole('button', { name: /查看工作记录/ }).click();
  await page.getByRole('button', { name: /OMEN/ }).first().waitFor();
  await page
    .getByRole('button', { name: /MacBook Pro/ })
    .first()
    .waitFor();
  await fs.mkdir(output, { recursive: true });
  await page.screenshot({
    path: path.join(output, 'dashboard.png'),
    fullPage: true,
  });
  if (errors.length) throw new Error(errors.join('\n'));
  const response = await page.request.get(`${url}/api/devices`, {
    headers: { Authorization: `Bearer ${key}` },
  });
  if (!response.ok())
    throw new Error(`Device API returned ${response.status()}`);
  const devices = await response.json();
  const now = Math.floor(Date.now() / 1000);
  const records = [];
  for (const device of devices) {
    const query = new URLSearchParams({
      from: String(now - 86400),
      to: String(now + 1),
      device: device.id,
      limit: '1',
    });
    const activityResponse = await page.request.get(
      `${url}/api/activities?${query}`,
      { headers: { Authorization: `Bearer ${key}` } },
    );
    if (!activityResponse.ok())
      throw new Error(`Activity API returned ${activityResponse.status()}`);
    const { items } = await activityResponse.json();
    if (!items.length)
      throw new Error(
        `${device.name} has no uploaded activity in the past day`,
      );
    records.push({
      device: device.name,
      latest_record_at: items[0].timestamp,
    });
  }
  console.log(
    JSON.stringify(
      {
        url,
        devices: devices.map(
          ({ name, platform, state, pending, last_error }) => ({
            name,
            platform,
            state,
            pending,
            last_error,
          }),
        ),
        records,
        page_errors: errors,
      },
      null,
      2,
    ),
  );
} finally {
  await browser.close();
}
