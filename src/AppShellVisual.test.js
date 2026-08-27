import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('应用壳层应使用原生窗口底板与 NavigationView 式左右结构', async () => {
  const [appSource, appCssSource] = await Promise.all([
    readFile(new URL('./App.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./app.css', import.meta.url), 'utf8'),
  ]);

  assert.match(appSource, /app-shell/);
  assert.match(appSource, /app-shell-stage/);
  assert.match(appSource, /app-shell-sidebar-frame/);
  assert.match(appSource, /app-shell-main-frame/);
  assert.match(appSource, /app-shell-windowbar/);

  assert.match(appCssSource, /\.app-shell\b/);
  assert.match(appCssSource, /\.app-shell-stage\b/);
  assert.match(appCssSource, /\.app-shell-sidebar-frame\b/);
  assert.match(appCssSource, /\.app-shell-main-frame\b/);
  assert.match(appCssSource, /\.app-shell-windowbar\b/);
  assert.match(appSource, /winui-shell/);
  assert.match(appCssSource, /\.winui-shell \.app-shell-stage/);
  assert.match(appCssSource, /grid-template-columns:\s*15rem minmax\(0, 1fr\)/);
  assert.match(appCssSource, /\.winui-shell \.app-shell-sidebar-frame[\s\S]*border-right:/);
});

test('主导航与设置 Pivot 使用 WinUI 紧凑字号层级', async () => {
  const appCssSource = await readFile(new URL('./app.css', import.meta.url), 'utf8');

  assert.match(appCssSource, /\.winui-shell \.sidebar-nav-label[\s\S]*font-size:\s*0\.875rem;/);
  assert.match(appCssSource, /\.winui-shell \.settings-tab-rail-item[\s\S]*font-size:\s*0\.8125rem;/);
});

test('统一底板结构下不应继续保留旧的主内容外壳伪元素修补逻辑', async () => {
  const appCssSource = await readFile(new URL('./app.css', import.meta.url), 'utf8');

  assert.doesNotMatch(appCssSource, /\.app-shell-main::before/);
  assert.doesNotMatch(appCssSource, /\.dark\s+\.app-shell-main::before/);
  assert.doesNotMatch(appCssSource, /\.app-shell-windowbar::before/);
});

test('自定义窗口栏存在时，统一底板本身应整体下移，侧栏和主内容不再分别补偿顶部偏移', async () => {
  const appSource = await readFile(new URL('./App.svelte', import.meta.url), 'utf8');

  assert.match(
    appSource,
    /app-shell-stage[\s\S]*\{platform !== 'macos' \? 'pt-8' : 'pt-2'\}/
  );
  assert.doesNotMatch(
    appSource,
    /app-shell-sidebar-frame[\s\S]*\{platform !== 'macos' \? 'pt-7' : 'pt-2'\}/
  );
  assert.doesNotMatch(
    appSource,
    /app-shell-main-frame[\s\S]*\{platform !== 'macos' \? 'pt-7' : ''\}/
  );
});

test('原生底板只在侧栏与内容层建立分区，不叠加玻璃卡片', async () => {
  const appCssSource = await readFile(new URL('./app.css', import.meta.url), 'utf8');

  assert.match(appCssSource, /\.winui-shell \.app-shell-sidebar-frame[\s\S]*background:\s*var\(--win-pane\)/);
  assert.match(appCssSource, /\.winui-shell \.app-shell-main-frame[\s\S]*background:\s*rgba\(32, 32, 32, 0\.74\)/);
  assert.match(appCssSource, /\.winui-shell \.app-shell-sidebar,[\s\S]*background:\s*transparent;/);
  assert.match(appCssSource, /\.winui-shell \.app-shell-sidebar,[\s\S]*box-shadow:\s*none;/);
  assert.doesNotMatch(appCssSource, /\.sidebar-editorial-shell::before/);
});
