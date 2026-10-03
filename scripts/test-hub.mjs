import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
const root = process.platform === 'win32' ? path.join(tmpdir(), '.agents') : '/tmp/.agents';
await mkdir(root, { recursive: true });
const directory = await mkdtemp(path.join(root, 'work-review-e2e-'));
const agentToken = 'test-collector-key-00000000000000000';
const viewToken = 'test-viewer-key-0000000000000000000';
const address = `127.0.0.1:${process.env.WORK_REVIEW_TEST_PORT || 47839}`;
await writeFile(
  path.join(directory, 'config.json'),
  JSON.stringify({
    bind: address,
    agent_token: agentToken,
    view_token: viewToken,
  }),
);
const executable = path.resolve(
  'target/debug',
  process.platform === 'win32'
    ? 'work-review-server.exe'
    : 'work-review-server',
);
const server = spawn(executable, ['--data-dir', directory, 'run'], {
  stdio: 'inherit',
  windowsHide: true,
});
let stopped = false;
async function stop() {
  if (stopped) return;
  stopped = true;
  server.kill();
}
process.once('SIGINT', stop);
process.once('SIGTERM', stop);
server.once('exit', async (code) => {
  await rm(directory, { recursive: true, force: true });
  process.exit(code || 0);
});
const base = `http://${address}`;
for (let attempt = 0; attempt < 100; attempt++) {
  try {
    if ((await fetch(`${base}/api/health`)).ok) break;
  } catch {}
  await new Promise((resolve) => setTimeout(resolve, 100));
}
const today = new Intl.DateTimeFormat('en-CA', {
  timeZone: 'Asia/Singapore',
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
}).format(new Date());
const start = new Date(`${today}T09:00:00+08:00`).getTime() / 1000;
const windows = {
  id: '11111111-1111-4111-8111-111111111111',
  name: 'OMEN',
  platform: 'windows',
};
const mac = {
  id: '22222222-2222-4222-8222-222222222222',
  name: 'MacBook',
  platform: 'macos',
};
const image =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a2ioAAAAASUVORK5CYII=';
for (const [device, activities] of [
  [
    windows,
    [
      { app: 'Code', title: 'work-review · 后台采集', offset: 0 },
      { app: 'Edge', title: '同步协议资料', offset: 3600 },
    ],
  ],
  [mac, [{ app: 'Safari', title: 'macOS 开发文档', offset: 300, image }]],
]) {
  const response = await fetch(`${base}/api/ingest`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${agentToken}`,
    },
    body: JSON.stringify({
      device,
      status: {
        state: 'recording',
        pending: 0,
        last_error: null,
        version: '0.2.0',
      },
      activities: activities.map((item) => ({
        activity: {
          id: randomUUID(),
          device_id: device.id,
          timestamp: start + item.offset,
          duration: 900,
          app_name: item.app,
          window_title: item.title,
          category: 'development',
          browser_url:
            item.app === 'Code' ? null : 'https://developer.apple.com',
          ocr_text: item.image ? '文档内容' : null,
          screenshot: !!item.image,
          note: '',
        },
        screenshot_base64: item.image || null,
      })),
    }),
  });
  if (!response.ok) throw new Error(`seed failed: ${response.status}`);
}
await new Promise(() => {});
