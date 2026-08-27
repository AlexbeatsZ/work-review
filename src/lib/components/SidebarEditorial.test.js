import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('侧边栏应提供 NavigationView 式导航与底部状态区', async () => {
  const [source, appCssSource] = await Promise.all([
    readFile(new URL('./Sidebar.svelte', import.meta.url), 'utf8'),
    readFile(new URL('../../app.css', import.meta.url), 'utf8'),
  ]);

  assert.match(source, /sidebar-editorial-shell/);
  assert.match(source, /sidebar-nav-section/);
  assert.match(source, /sidebar-status-panel/);
  assert.match(source, /sidebar-toolbelt/);
  assert.match(source, /sidebar-recording-action/);
  assert.match(source, /locale-menu-item/);
  assert.doesNotMatch(source, /sidebar-brand-panel/);
  assert.match(appCssSource, /\.winui-shell \.sidebar-nav-section[\s\S]*background:\s*transparent;/);
  assert.match(appCssSource, /\.winui-shell \.sidebar-status-panel[\s\S]*border:\s*1px solid var\(--win-stroke\)/);
  assert.match(appCssSource, /\.winui-shell \.sidebar-toolbelt[\s\S]*justify-content:\s*flex-end/);
});

test('侧边栏激活态高亮条应位于图标区外侧，避免与导航图标重叠', async () => {
  const appCssSource = await readFile(new URL('../../app.css', import.meta.url), 'utf8');

  assert.match(appCssSource, /\.winui-shell \.sidebar-nav-rail[\s\S]*left:\s*-1px/);
});

test('侧边栏不应继续提供独立的设备节点入口，节点能力应收回设置页 Beta 标签', async () => {
  const source = await readFile(new URL('./Sidebar.svelte', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /path:\s*'\/node'/);
  assert.doesNotMatch(source, /labelKey:\s*'sidebar\.nav\.node'/);
  assert.doesNotMatch(source, /item\.icon === 'node'/);
});
