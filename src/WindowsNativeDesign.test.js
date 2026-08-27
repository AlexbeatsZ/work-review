import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('桌面壳层采用 WinUI 令牌、原生 DWM Mica 与 Fluent 导航结构', async () => {
  const [appSource, sidebarSource, timelineSource, cssSource, rustSource, tauriConfigSource, designSource, mobileSource] = await Promise.all([
    readFile(new URL('./App.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./lib/components/Sidebar.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./routes/timeline/Timeline.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./app.css', import.meta.url), 'utf8'),
    readFile(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8'),
    readFile(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
    readFile(new URL('../docs/design/cross-platform-interface.md', import.meta.url), 'utf8'),
    readFile(new URL('../mobile-android/app/src/main/java/com/metacodex/workreview/ui/WorkReviewMobileApp.kt', import.meta.url), 'utf8'),
  ]);

  assert.match(appSource, /app-shell winui-shell/);
  assert.match(appSource, /app-shell-window-identity/);
  assert.doesNotMatch(appSource, /work-ledger-shell/);
  assert.match(sidebarSource, /sidebar-status-panel[\s\S]*sidebar-recording-action/);
  assert.doesNotMatch(sidebarSource, /sidebar-brand-panel/);

  assert.match(cssSource, /--win-mica-fallback:\s*#202020/);
  assert.match(cssSource, /--win-accent:\s*#60cdff/);
  assert.match(cssSource, /--win-radius-control:\s*4px/);
  assert.match(cssSource, /\.winui-shell \.settings-tab-rail[\s\S]*flex-direction:\s*row/);
  assert.match(cssSource, /\.winui-shell \.timeline-rail[\s\S]*var\(--win-accent\)/);
  assert.match(cssSource, /@media \(prefers-reduced-motion: reduce\)/);

  const declaredWinTokens = new Set([...cssSource.matchAll(/(--win-[\w-]+)\s*:/g)].map((match) => match[1]));
  const timelineWinTokens = new Set([...timelineSource.matchAll(/var\((--win-[\w-]+)/g)].map((match) => match[1]));
  assert.deepEqual([...timelineWinTokens].filter((token) => !declaredWinTokens.has(token)), []);

  assert.match(rustSource, /DwmSetWindowAttribute/);
  assert.match(rustSource, /DWMWA_SYSTEMBACKDROP_TYPE/);
  assert.match(rustSource, /DWMSBT_MAINWINDOW/);
  assert.equal(JSON.parse(tauriConfigSource).app.windows[0].transparent, true);

  assert.match(designSource, /Windows 11 native Fluent/);
  assert.match(designSource, /DWM Mica/);

  assert.match(mobileSource, /private val Void = Color\(0xFF080A0D\)/);
  assert.match(mobileSource, /private val Signal = Color\(0xFF7AA2F7\)/);
  assert.match(mobileSource, /colorScheme = DarkColors/);
});
