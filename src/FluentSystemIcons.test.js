import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, access } from 'node:fs/promises';
import { constants } from 'node:fs';

test('桌面端与移动端图标全面符合微软开源 Fluent System Icons 规范', async () => {
  const [
    appSource,
    sidebarSource,
    timelineSource,
    summarySource,
    settingsSource,
    toastSource,
    statsSource,
    appVisualsSource,
    androidLauncherSource,
  ] = await Promise.all([
    readFile(new URL('./App.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./lib/components/Sidebar.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./routes/timeline/Timeline.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./routes/timeline/Summary.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./routes/settings/Settings.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./lib/components/Toast.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./lib/components/StatsCard.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./lib/utils/appVisuals.js', import.meta.url), 'utf8'),
    readFile(new URL('../mobile-android/app/src/main/res/drawable/ic_launcher.xml', import.meta.url), 'utf8'),
  ]);

  // 1. App.svelte 窗口控制栏
  assert.match(appSource, /subtract_16_regular/);
  assert.match(appSource, /square_16_regular/);
  assert.match(appSource, /dismiss_16_regular/);

  // 2. Sidebar.svelte 侧边栏导航与语言切换
  assert.match(sidebarSource, /sidebar-nav-icon/);
  assert.match(sidebarSource, /M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm0 18a8 8 0 1 1 8-8/); // timeline_24_regular
  assert.match(sidebarSource, /M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8Zm-2.5 4a2.5 2.5 0 1 1 5 0/); // settings_24_regular
  assert.match(sidebarSource, /M3\.22 5\.47a\.75\.75 0 0 1 1\.06 0L8 9\.19l3\.72-3\.72/); // chevron_down_16_regular

  // 3. Timeline.svelte 时间线界面图标
  assert.match(timelineSource, /M10 2a8 8 0 1 0 7\.75 6\.02/); // arrow_clockwise_20_regular
  assert.match(timelineSource, /M3\.5 2a\.75\.75 0 0 1 \.75\.75V15h11\.25/); // chart_multiple_20_regular
  assert.match(timelineSource, /M5\.47 3\.22a\.75\.75 0 0 0 0 1\.06L9\.19 8/); // chevron_right_16_regular
  assert.match(timelineSource, /M4\.22 7\.47a\.75\.75 0 0 1 1\.06 0L10 12\.19/); // chevron_down_20_regular
  assert.match(timelineSource, /M5\.28 4\.22a\.75\.75 0 0 0-1\.06 1\.06L10\.94 12/); // dismiss_24_regular

  // 4. Summary.svelte 摘要页
  assert.match(summarySource, /M7\.78 3\.22a\.75\.75 0 0 1 0 1\.06L3\.81 8\.25/); // arrow_left_20_regular
  assert.match(summarySource, /M4 2\.75a\.75\.75 0 0 1 \.75\.75V18\.5/); // chart_multiple_24_regular

  // 5. Settings.svelte 设置页
  assert.match(settingsSource, /M17\.47 6\.47a\.75\.75 0 0 1 0 1\.06l-8\.5 8\.5/); // checkmark_20_regular
  assert.match(settingsSource, /M10 7a3 3 0 1 0 0 6 3 3 0 0 0 0-6Z/); // settings_20_regular
  assert.match(settingsSource, /M10 2a1\.75 1\.75 0 0 0-\.74\.16L3\.76 4\.91/); // shield_lock_20_regular
  assert.match(settingsSource, /M10 2c3\.87 0 7 1\.12 7 2\.5v11/); // database_20_regular

  // 6. Toast.svelte 与 ConfirmDialog.svelte
  assert.match(toastSource, /M10 2a8 8 0 1 1 0 16/); // checkmark_circle_20_regular
  assert.match(statsSource, /M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm7\.93 9/); // globe_24_regular
  assert.match(statsSource, /M4 4\.5A2\.5 2\.5 0 0 1 6\.5 2h2/); // apps_24_regular

  // 7. Android Launcher Icon
  assert.match(androidLauncherSource, /#0E1217/);
  assert.match(androidLauncherSource, /#7AA2F7/);
  assert.match(androidLauncherSource, /#43D6A2/);

  // 8. 图标文件存在性验证
  await Promise.all([
    access(new URL('../src-tauri/icons/icon.png', import.meta.url), constants.F_OK),
    access(new URL('../src-tauri/icons/windows-icon.png', import.meta.url), constants.F_OK),
    access(new URL('../src-tauri/icons/icon.ico', import.meta.url), constants.F_OK),
    access(new URL('../public/icon.png', import.meta.url), constants.F_OK),
    access(new URL('../public/favicon.png', import.meta.url), constants.F_OK),
    access(new URL('../public/icons/256x256.png', import.meta.url), constants.F_OK),
  ]);
});
