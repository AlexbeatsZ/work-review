import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('跨端桌面壳层使用 Work Ledger 设计令牌与时间脊线', async () => {
  const [appSource, timelineSource, cssSource, tailwindSource] = await Promise.all([
    readFile(new URL('./App.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./routes/timeline/Timeline.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./app.css', import.meta.url), 'utf8'),
    readFile(new URL('../tailwind.config.js', import.meta.url), 'utf8'),
  ]);

  assert.match(appSource, /work-ledger-shell/);
  assert.match(appSource, /work-ledger-atmosphere/);
  assert.match(timelineSource, /timeline-ledger-kicker/);
  assert.match(cssSource, /--ledger-paper:\s*#f7f3ec/);
  assert.match(cssSource, /--ledger-copper:\s*#c5663d/);
  assert.match(cssSource, /\.work-ledger-shell \.timeline-rail[\s\S]*var\(--ledger-copper\)/);
  assert.match(cssSource, /@media \(prefers-reduced-motion: reduce\)/);
  assert.match(tailwindSource, /500:\s*'#c5663d'/);
});
