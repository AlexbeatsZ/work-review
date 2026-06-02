import test from 'node:test';
import assert from 'node:assert/strict';
import { access, readFile } from 'node:fs/promises';

async function fileExists(url) {
  try {
    await access(url);
    return true;
  } catch {
    return false;
  }
}

test('lite 设置页不应继续提供 AI 设置组件', async () => {
  const aiComponentUrl = new URL('./components/SettingsAI.svelte', import.meta.url);
  const settingsSource = await readFile(new URL('./Settings.svelte', import.meta.url), 'utf8');

  assert.equal(await fileExists(aiComponentUrl), false);
  assert.doesNotMatch(settingsSource, /SettingsAI/);
  assert.doesNotMatch(settingsSource, /settings\.tabs\.ai/);
});

test('lite 设置页不应继续展示日报导出入口', async () => {
  const settingsSource = await readFile(new URL('./Settings.svelte', import.meta.url), 'utf8');
  const storageSource = await readFile(
    new URL('./components/SettingsStorage.svelte', import.meta.url),
    'utf8'
  );

  assert.doesNotMatch(settingsSource, /daily_report_export/);
  assert.doesNotMatch(settingsSource, /daily_report_auto_export/);
  assert.doesNotMatch(storageSource, /settingsStorage\.exportDir/);
  assert.doesNotMatch(storageSource, /pickDailyReportExportDir/);
});
