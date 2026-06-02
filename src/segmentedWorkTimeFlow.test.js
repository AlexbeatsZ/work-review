import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('统计仍通过有效工作时段接口，但配置层会统一返回全天', async () => {
  const commandsSource = await readFile(new URL('../src-tauri/src/commands.rs', import.meta.url), 'utf8');
  const configSource = await readFile(new URL('../crates/core/src/config.rs', import.meta.url), 'utf8');

  assert.match(commandsSource, /state\.config\.effective_work_segments\(\)/);
  assert.match(commandsSource, /get_daily_stats_with_segments/);
  assert.match(commandsSource, /is_work_time_in_segments/);
  assert.match(configSource, /start_hour:\s*0/);
  assert.match(configSource, /end_hour:\s*0/);
});

test('前端启动流程不应再依赖工作时间分段推导自动日报时间', async () => {
  const appSource = await readFile(new URL('./App.svelte', import.meta.url), 'utf8');

  assert.doesNotMatch(appSource, /work_time_segments/);
  assert.doesNotMatch(appSource, /resolveAutoReportWorkEnd/);
});
