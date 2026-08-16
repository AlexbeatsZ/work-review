import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('桌面壳层使用冷调暗黑 Dark Current 令牌与信号轨道', async () => {
  const [appSource, timelineSource, cssSource, tailwindSource, designSource, mobileSource] = await Promise.all([
    readFile(new URL('./App.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./routes/timeline/Timeline.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./app.css', import.meta.url), 'utf8'),
    readFile(new URL('../tailwind.config.js', import.meta.url), 'utf8'),
    readFile(new URL('../docs/design/cross-platform-interface.md', import.meta.url), 'utf8'),
    readFile(new URL('../mobile-android/app/src/main/java/com/metacodex/workreview/ui/WorkReviewMobileApp.kt', import.meta.url), 'utf8'),
  ]);

  assert.match(appSource, /work-ledger-shell/);
  assert.match(timelineSource, /timeline-signal-kicker/);
  assert.match(cssSource, /--current-void:\s*#080a0d/);
  assert.match(cssSource, /--current-signal:\s*#7aa2f7/);
  assert.match(cssSource, /\.work-ledger-shell \.timeline-rail[\s\S]*var\(--current-signal\)/);
  assert.match(cssSource, /@media \(prefers-reduced-motion: reduce\)/);
  assert.doesNotMatch(cssSource, /--ledger-paper:\s*#f7f3ec/);
  assert.match(tailwindSource, /400:\s*'#7aa2f7'/);
  assert.match(designSource, /dark-only, cool-neutral/);
  assert.match(designSource, /Brown, copper, cream, warm paper, and serif typography are prohibited/);
  assert.match(mobileSource, /private val Void = Color\(0xFF080A0D\)/);
  assert.match(mobileSource, /private val Signal = Color\(0xFF7AA2F7\)/);
  assert.match(mobileSource, /colorScheme = DarkColors/);
  assert.doesNotMatch(mobileSource, /isSystemInDarkTheme|lightColorScheme|FontFamily\.Serif/);
});
