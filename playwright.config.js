import { defineConfig } from '@playwright/test';
import net from 'node:net';
if (!process.env.WORK_REVIEW_TEST_PORT) {
  const listener = net.createServer();
  await new Promise((resolve) => listener.listen(0, '127.0.0.1', resolve));
  process.env.WORK_REVIEW_TEST_PORT = String(listener.address().port);
  await new Promise((resolve) => listener.close(resolve));
}
const baseURL = `http://127.0.0.1:${process.env.WORK_REVIEW_TEST_PORT}`;
export default defineConfig({
  testDir: './tests',
  timeout: 30000,
  fullyParallel: false,
  use: {
    baseURL,
    timezoneId: 'Asia/Singapore',
    browserName: 'chromium',
    channel: process.platform === 'win32' ? 'msedge' : undefined,
  },
  webServer: {
    command: 'node scripts/test-hub.mjs',
    url: `${baseURL}/api/health`,
    reuseExistingServer: false,
    timeout: 30000,
  },
});
