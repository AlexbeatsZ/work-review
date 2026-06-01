<script>
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { ask, open as openDialog } from '@tauri-apps/plugin-dialog';
  import { cache } from '../../../lib/stores/cache.js';
  import { locale, t } from '$lib/i18n/index.js';
  import { showToast } from '$lib/stores/toast.js';
  
  export let config;
  export let storageStats = null;
  export let dataDir = '';
  export let databasePath = '';
  export let defaultDataDir = '';
  
  const dispatch = createEventDispatcher();
  $: currentLocale = $locale;
  let isClearing = false;
  let isMigrating = false;
  let isMigratingDatabase = false;
  let isCleaningPreviousDir = false;
  let cleanupCandidateDir = '';
  let localizedScreenshotModes = [];
  let screenshotIntervalLabel = '';
  let retentionDaysLabel = '';
  let storageRetentionLabel = '';
  let keepForever = false;
  const screenshotModes = [
    {
      value: 'active_window',
      labelKey: 'settingsStorage.modeActiveWindow',
      descriptionKey: 'settingsStorage.modeActiveWindowDesc',
    },
    {
      value: 'all',
      labelKey: 'settingsStorage.modeAll',
      descriptionKey: 'settingsStorage.modeAllDesc',
    },
  ];
  $: {
    currentLocale;
    localizedScreenshotModes = screenshotModes.map((mode) => ({
      ...mode,
      label: t(mode.labelKey),
      description: t(mode.descriptionKey),
    }));
  }

  function clearCache() {
    cache.clear();
    showToast(t('settingsStorage.clearCacheAction'));
    dispatch('clearCache');
  }

  async function clearOldData() {
    const confirmed = await ask(t('settingsStorage.clearHistoryConfirmMessage'), {
      title: t('settingsStorage.clearHistoryConfirmTitle'),
      kind: 'warning',
    });

    if (!confirmed) {
      return;
    }
    
    isClearing = true;
    try {
      await invoke('clear_old_activities');
      showToast(t('settingsStorage.clearDone'), 'success');
      cache.clear();
      dispatch('clearCache');
    } catch (e) {
      showToast(t('settingsStorage.clearFailed', { error: e }), 'error');
    } finally {
      isClearing = false;
    }
  }

  async function migrateToDataDir(targetDir) {
    const nextDir = targetDir?.trim();
    if (!nextDir) {
      return;
    }

    if (nextDir === dataDir) {
      showToast(t('settingsStorage.alreadyCurrentDir'));
      return;
    }

    const confirmed = await ask(
      t('settingsStorage.migrateConfirmMessage', { dir: nextDir }),
      {
        title: t('settingsStorage.migrateConfirmTitle'),
        kind: 'warning',
      },
    );

    if (!confirmed) {
      return;
    }

    isMigrating = true;
    try {
      const result = await invoke('change_data_dir', { targetDir: nextDir });
      cleanupCandidateDir = result?.oldDataDir || dataDir;
      showToast(t('settingsStorage.migrated'), 'success');
      dispatch('dataDirChanged', result);
    } catch (e) {
      showToast(t('settingsStorage.migrateFailed', { error: e }), 'error');
    } finally {
      isMigrating = false;
    }
  }

  async function pickDataDir() {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      defaultPath: dataDir || defaultDataDir || undefined,
    });

    if (!selected || Array.isArray(selected)) {
      return;
    }

    await migrateToDataDir(selected);
  }

  async function pickDatabaseDir() {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      defaultPath: databasePath || dataDir || defaultDataDir || undefined,
    });

    if (!selected || Array.isArray(selected)) {
      return;
    }

    const confirmed = await ask(
      `将把当前 SQLite 数据库迁移到：\n${selected}\\workreview.db\n\n截图、OCR、配置仍保留在当前数据目录。SQLite 运行时可能会在同目录生成 .db-wal / .db-shm 文件。是否继续？`,
      {
        title: '确认迁移数据库文件',
        kind: 'warning',
      },
    );

    if (!confirmed) {
      return;
    }

    isMigratingDatabase = true;
    try {
      const result = await invoke('change_database_path', { targetPath: selected });
      showToast('数据库位置已更新', 'success');
      dispatch('databasePathChanged', result);
    } catch (e) {
      showToast(`数据库迁移失败: ${e}`, 'error');
    } finally {
      isMigratingDatabase = false;
    }
  }

  async function restoreDefaultDataDir() {
    await migrateToDataDir(defaultDataDir);
  }

  async function openCurrentDataDir() {
    try {
      await invoke('open_data_dir');
    } catch (e) {
      showToast(t('settingsStorage.openDirFailed', { error: e }), 'error');
    }
  }

  async function cleanupPreviousDataDir() {
    const targetDir = cleanupCandidateDir?.trim();
    if (!targetDir || isCleaningPreviousDir) {
      return;
    }

    const confirmed = await ask(
      t('settingsStorage.cleanupOldConfirmMessage', { dir: targetDir }),
      {
        title: t('settingsStorage.cleanupOldConfirmTitle'),
        kind: 'warning',
      },
    );

    if (!confirmed) {
      return;
    }

    isCleaningPreviousDir = true;
    try {
      await invoke('cleanup_old_data_dir', { targetDir });
      cleanupCandidateDir = '';
      showToast(t('settingsStorage.oldDirCleaned'), 'success');
    } catch (e) {
      showToast(t('settingsStorage.cleanupOldFailed', { error: e }), 'error');
    } finally {
      isCleaningPreviousDir = false;
    }
  }

  function handleChange() {
    dispatch('change', config);
  }

  async function pickDailyReportExportDir() {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      defaultPath: config.daily_report_export_dir || dataDir || defaultDataDir || undefined,
    });

    if (!selected || Array.isArray(selected)) {
      return;
    }

    config.daily_report_export_dir = selected;
    handleChange();
  }

  function clearDailyReportExportDir() {
    config.daily_report_export_dir = null;
    handleChange();
  }

  // 计算存储使用百分比
  $: usagePercent = storageStats 
    ? Math.min(Math.round((storageStats.total_size_mb / storageStats.storage_limit_mb) * 100), 100) 
    : 0;

  // 使用量颜色
  $: usageColor = usagePercent > 80 ? 'bg-red-500' : usagePercent > 50 ? 'bg-amber-500' : 'bg-emerald-500';
  $: usingDefaultDataDir = dataDir && defaultDataDir && dataDir === defaultDataDir;
  $: {
    currentLocale;
    screenshotIntervalLabel = t('settingsStorage.secondsValue', { count: config?.screenshot_interval ?? 0 });
    retentionDaysLabel = t('settingsStorage.daysValue', { count: config?.storage?.screenshot_retention_days ?? 0 });
    keepForever = config?.storage?.screenshot_retention_days === 0;
    storageRetentionLabel = t('settingsStorage.daysValue', { count: storageStats?.retention_days ?? 0 });
  }
  $: if (cleanupCandidateDir && cleanupCandidateDir === dataDir) {
    cleanupCandidateDir = '';
  }
</script>

<!-- 截图与保留 -->
<div class="settings-card mb-5" data-locale={currentLocale}>
  <h3 class="settings-card-title">{t('settingsStorage.screenshotCardTitle')}</h3>
  
  <div class="settings-section">
    <div class="settings-row">
      <div>
        <span class="settings-text">{t('settingsStorage.screenshotsEnabled')}</span>
        <p class="settings-muted mt-0.5">{t('settingsStorage.screenshotsEnabledHint')}</p>
      </div>
      <button
        type="button"
        on:click={() => {
          config.storage.screenshots_enabled = !config.storage.screenshots_enabled;
          handleChange();
        }}
        class="switch-track {config.storage.screenshots_enabled ? 'bg-emerald-500' : 'bg-slate-300 dark:bg-slate-600'}"
        aria-pressed={config.storage.screenshots_enabled}
      >
        <span class="switch-thumb {config.storage.screenshots_enabled ? 'translate-x-5' : 'translate-x-0'}"></span>
      </button>
    </div>

    <!-- 轮询间隔 -->
    <div class="settings-block">
      <div class="flex items-center justify-between">
        <label for="screenshot-interval" class="settings-text">{t('settingsStorage.pollingInterval')}</label>
        <div class="flex items-center gap-2">
          <input
            type="number"
            min="5"
            max="600"
            step="5"
            bind:value={config.screenshot_interval}
            on:change={() => {
              config.screenshot_interval = Math.max(5, Math.min(600, Number(config.screenshot_interval) || 30));
              handleChange();
            }}
            class="w-16 rounded-md border border-slate-200 bg-white px-2 py-1 text-center text-sm dark:border-slate-600 dark:bg-slate-800"
          />
          <span class="text-xs settings-subtle">{t('settingsStorage.secondsUnit')}</span>
        </div>
      </div>
      <input
        id="screenshot-interval"
        type="range"
        bind:value={config.screenshot_interval}
        on:change={() => {
          config.screenshot_interval = Math.max(5, Math.min(600, Number(config.screenshot_interval) || 30));
          handleChange();
        }}
        min="10"
        max="120"
        step="5"
        class="range-input"
      />
      <div class="flex justify-between text-xs settings-subtle">
        <span>{t('settingsStorage.precise')}</span>
        <span>{t('settingsStorage.powerSave')}</span>
      </div>
    </div>

    <!-- 数据保留 -->
    <div class="settings-block">
      <div class="flex items-center justify-between">
        <label for="retention-days" class="settings-text">{t('settingsStorage.retentionDays')}</label>
        <div class="flex items-center gap-2">
          <label class="flex items-center gap-1.5 text-xs settings-subtle cursor-pointer select-none">
            <input
              type="checkbox"
              checked={keepForever}
              on:click={() => {
                if (!keepForever) {
                  config.storage.screenshot_retention_days = 0;
                } else {
                  config.storage.screenshot_retention_days = 7;
                }
                handleChange();
              }}
              class="rounded border-slate-300 dark:border-slate-600"
            />
            {t('settingsStorage.keepForever')}
          </label>
          {#if !keepForever}
            <input
              type="number"
              min="1"
              max="9999"
              step="1"
              bind:value={config.storage.screenshot_retention_days}
              on:change={() => {
                config.storage.screenshot_retention_days = Math.max(1, Number(config.storage.screenshot_retention_days) || 7);
                handleChange();
              }}
              class="w-16 rounded-md border border-slate-200 bg-white px-2 py-1 text-center text-sm dark:border-slate-600 dark:bg-slate-800"
            />
          {:else}
            <span class="settings-value">∞</span>
          {/if}
        </div>
      </div>
      {#if !keepForever}
        <input
          id="retention-days"
          type="range"
          bind:value={config.storage.screenshot_retention_days}
          on:change={() => {
            config.storage.screenshot_retention_days = Math.max(1, Number(config.storage.screenshot_retention_days) || 7);
            handleChange();
          }}
          min="1"
          max="90"
          step="1"
          class="range-input"
        />
        <div class="flex justify-between text-xs settings-subtle">
          <span>{t('settingsStorage.retentionMin')}</span>
          <span>{t('settingsStorage.retentionMax')}</span>
        </div>
      {/if}
    </div>

    <div class="settings-block">
      <p class="settings-text mb-2">{t('settingsStorage.screenshotMode')}</p>
      <div class="flex gap-2">
        {#each localizedScreenshotModes as mode}
          <button
            type="button"
            on:click={() => {
              config.storage.screenshot_display_mode = mode.value;
              handleChange();
            }}
            class="flex-1 min-h-16 px-3 py-2.5 rounded-lg text-sm font-medium leading-none transition-all duration-150
                   {config.storage.screenshot_display_mode === mode.value
                     ? 'settings-segment-active'
                     : 'settings-segment-base'}"
          >
            <div class="flex h-full flex-col items-center justify-center gap-1 text-center">
              <div class="leading-none">{mode.label}</div>
              <div class="text-[10px] leading-snug {config.storage.screenshot_display_mode === mode.value ? 'text-white/70' : 'settings-subtle'}">
                {mode.description}
              </div>
            </div>
          </button>
        {/each}
      </div>
    </div>
  </div>
</div>

<!-- 截图分辨率 -->
<div class="settings-card mb-5" data-locale={currentLocale}>
  <h3 class="settings-card-title">{t('settingsStorage.widthMode')}</h3>

  <div class="settings-section">
    <div class="settings-block">
      <div class="flex gap-2">
        <button
          type="button"
          on:click={() => {
            config.storage.screenshot_width_mode = 'auto';
            handleChange();
          }}
          class="flex-1 min-h-16 px-3 py-2.5 rounded-lg text-sm font-medium leading-none transition-all duration-150
                 {config.storage.screenshot_width_mode === 'auto'
                   ? 'settings-segment-active'
                   : 'settings-segment-base'}"
        >
          <div class="flex h-full flex-col items-center justify-center gap-1 text-center">
            <div class="leading-none">{t('settingsStorage.widthModeAuto')}</div>
            <div class="text-[10px] leading-snug {config.storage.screenshot_width_mode === 'auto' ? 'text-white/70' : 'settings-subtle'}">
              {t('settingsStorage.widthModeAutoDesc')}
            </div>
          </div>
        </button>
        <button
          type="button"
          on:click={() => {
            config.storage.screenshot_width_mode = 'fixed';
            handleChange();
          }}
          class="flex-1 min-h-16 px-3 py-2.5 rounded-lg text-sm font-medium leading-none transition-all duration-150
                 {config.storage.screenshot_width_mode === 'fixed'
                   ? 'settings-segment-active'
                   : 'settings-segment-base'}"
        >
          <div class="flex h-full flex-col items-center justify-center gap-1 text-center">
            <div class="leading-none">{t('settingsStorage.widthModeFixed')}</div>
            <div class="text-[10px] leading-snug {config.storage.screenshot_width_mode === 'fixed' ? 'text-white/70' : 'settings-subtle'}">
              {t('settingsStorage.widthModeFixedDesc')}
            </div>
          </div>
        </button>
      </div>
    </div>

    {#if config.storage.screenshot_width_mode === 'fixed'}
      <div class="settings-block">
        <div class="flex items-center justify-between">
          <label for="max-image-width" class="settings-text">{t('settingsStorage.maxWidth')}</label>
          <span class="settings-value">{config.storage.max_image_width}px</span>
        </div>
        <input
          id="max-image-width"
          type="range"
          bind:value={config.storage.max_image_width}
          on:change={handleChange}
          min="640"
          max="3840"
          step="64"
          class="range-input"
        />
        <div class="flex justify-between text-xs settings-subtle">
          <span>640px</span>
          <span>3840px</span>
        </div>
      </div>
    {/if}
  </div>
</div>

<!-- 日报导出 -->
<div class="settings-card mb-5" data-locale={currentLocale}>
  <h3 class="settings-card-title">{t('settingsStorage.exportTitle')}</h3>

  <div class="settings-block">
    <div class="rounded-2xl border border-slate-200/80 bg-slate-50/90 p-4 dark:border-slate-700/80 dark:bg-slate-800/40">
      <p class="settings-text">{t('settingsStorage.exportDir')}</p>
      <p class="settings-muted mt-1 break-all">
        {config.daily_report_export_dir || t('settingsStorage.notSet')}
      </p>
      <div class="mt-4 flex flex-wrap gap-3">
        <button
          type="button"
          on:click={pickDailyReportExportDir}
          class="settings-action-secondary"
        >
          {t('settingsStorage.chooseDir')}
        </button>
        {#if config.daily_report_export_dir}
          <button
            type="button"
            on:click={clearDailyReportExportDir}
            class="settings-action-secondary"
          >
            {t('settingsStorage.clearDir')}
          </button>
        {/if}
      </div>

      <div class="mt-4 flex items-center justify-between">
        <div>
          <p class="settings-text">{t('settingsStorage.autoExport')}</p>
          <p class="settings-muted mt-0.5">{t('settingsStorage.autoExportHint')}</p>
        </div>
        <button
          type="button"
          class="switch-track {config.daily_report_auto_export ? 'bg-emerald-500' : 'bg-slate-300 dark:bg-slate-600'} {!config.daily_report_export_dir ? 'opacity-60 cursor-not-allowed' : ''}"
          on:click={() => { if (config.daily_report_export_dir) config.daily_report_auto_export = !config.daily_report_auto_export; }}
          disabled={!config.daily_report_export_dir}
        >
          <span class="switch-thumb {config.daily_report_auto_export ? 'translate-x-5' : 'translate-x-0'}"></span>
        </button>
      </div>
    </div>
  </div>
</div>

<div class="settings-card mb-5" data-locale={currentLocale}>
  <h3 class="settings-card-title">{t('settingsStorage.dataDirTitle')}</h3>

  <div class="settings-section">
    <div class="settings-block">
      <div class="rounded-2xl border border-slate-200/80 bg-slate-50/90 p-4 dark:border-slate-700/80 dark:bg-slate-800/40">
        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <p class="settings-text">{t('settingsStorage.currentDir')}</p>
            <p class="settings-muted mt-1 break-all">{dataDir || t('common.loading')}</p>
          </div>
          <div>
            <p class="settings-text">{t('settingsStorage.defaultDir')}</p>
            <p class="settings-muted mt-1 break-all">{defaultDataDir || t('common.loading')}</p>
          </div>
        </div>

        <div class="mt-4 rounded-xl border border-slate-200/80 bg-white/60 p-3 dark:border-slate-700/80 dark:bg-slate-900/20">
          <p class="settings-text">当前数据库文件</p>
          <p class="settings-muted mt-1 break-all">{databasePath || t('common.loading')}</p>
          <p class="settings-muted mt-1">
            可单独迁移 SQLite DB。截图、OCR 和配置仍保留在上方数据目录。
          </p>
          <div class="mt-3 flex flex-wrap gap-3">
            <button
              type="button"
              on:click={pickDatabaseDir}
              disabled={isMigrating || isMigratingDatabase}
              class="settings-action-secondary"
            >
              {#if isMigratingDatabase}
                迁移数据库中...
              {:else}
                更改数据库位置
              {/if}
            </button>
          </div>
        </div>

        <div class="mt-4 flex flex-wrap gap-3">
          <button
            on:click={pickDataDir}
            disabled={isMigrating}
            class="settings-action-secondary"
          >
            {#if isMigrating}
              {t('settingsStorage.migrating')}
            {:else}
              {t('settingsStorage.changeLocation')}
            {/if}
          </button>

          <button
            on:click={openCurrentDataDir}
            disabled={isMigrating}
            class="settings-action-secondary"
          >
            {t('settingsStorage.openCurrentDir')}
          </button>

          {#if !usingDefaultDataDir && defaultDataDir}
            <button
              on:click={restoreDefaultDataDir}
              disabled={isMigrating}
              class="settings-action-secondary"
            >
              {t('settingsStorage.restoreDefaultDir')}
            </button>
          {/if}
        </div>

        {#if cleanupCandidateDir}
          <div class="mt-4 rounded-xl border border-amber-200/70 bg-amber-50/90 p-3 dark:border-amber-500/30 dark:bg-amber-950/20">
            <p class="settings-text">{t('settingsStorage.oldDirPending')}</p>
            <p class="settings-muted mt-1 break-all">{cleanupCandidateDir}</p>
            <div class="mt-3 flex flex-wrap gap-3">
              <button
                on:click={cleanupPreviousDataDir}
                disabled={isCleaningPreviousDir || isMigrating}
                class="settings-action-secondary"
              >
                {#if isCleaningPreviousDir}
                  {t('settingsStorage.cleaning')}
                {:else}
                  {t('settingsStorage.cleanOldDir')}
                {/if}
              </button>
              <button
                on:click={() => cleanupCandidateDir = ''}
                disabled={isCleaningPreviousDir}
                class="settings-action-secondary"
              >
                {t('settingsStorage.later')}
              </button>
            </div>
          </div>
        {/if}
      </div>
    </div>

    {#if storageStats}
      <div class="settings-block">
        <div class="rounded-2xl border border-slate-200/80 bg-slate-50/90 p-4 dark:border-slate-700/80 dark:bg-slate-800/40">
          <div class="mb-5">
            <div class="mb-2 flex items-end justify-between">
              <div>
                <span class="text-2xl font-bold text-slate-800 dark:text-white">{storageStats.total_size_mb}</span>
                <span class="settings-muted"> / {storageStats.storage_limit_mb} MB</span>
              </div>
              <span class="text-sm font-medium {usagePercent > 80 ? 'settings-text-danger' : 'settings-muted'}">{usagePercent}%</span>
            </div>
            <div class="h-2.5 w-full overflow-hidden rounded-full bg-slate-100 dark:bg-slate-700">
              <div
                class="h-full rounded-full transition-all duration-500 {usageColor}"
                style="width: {usagePercent}%"
              ></div>
            </div>
          </div>

          <div class="grid grid-cols-3 gap-3">
            <div class="rounded-xl bg-white/70 p-3 text-center ring-1 ring-slate-200/70 dark:bg-slate-900/20 dark:ring-slate-700/70">
              <p class="text-xl font-bold text-slate-800 dark:text-white">{storageStats.total_files}</p>
              <p class="settings-muted mt-0.5">{t('settingsStorage.screenshotsCount')}</p>
            </div>
            <div class="rounded-xl bg-white/70 p-3 text-center ring-1 ring-slate-200/70 dark:bg-slate-900/20 dark:ring-slate-700/70">
              <p class="text-xl font-bold text-slate-800 dark:text-white">{storageStats.total_size_mb} MB</p>
              <p class="settings-muted mt-0.5">{t('settingsStorage.usedSpace')}</p>
            </div>
            <div class="rounded-xl bg-white/70 p-3 text-center ring-1 ring-slate-200/70 dark:bg-slate-900/20 dark:ring-slate-700/70">
              <p class="text-xl font-bold text-slate-800 dark:text-white">{storageRetentionLabel}</p>
              <p class="settings-muted mt-0.5">{t('settingsStorage.retentionPeriod')}</p>
            </div>
          </div>
        </div>
      </div>
    {/if}

    <div class="space-y-2.5">
      <div class="settings-row">
        <span class="settings-text">{t('settingsStorage.clearCache')}</span>
        <button
          on:click={clearCache}
          class="settings-action-secondary"
        >
          {t('settingsStorage.clearCacheAction')}
        </button>
      </div>

      <div class="settings-panel-danger flex items-center justify-between">
        <span class="settings-text-danger text-sm font-medium">{t('settingsStorage.clearHistory')}</span>
        <button
          on:click={clearOldData}
          disabled={isClearing}
          class="settings-action-danger"
        >
          {#if isClearing}
            {t('settingsStorage.cleaning')}
          {:else}
            {t('settingsStorage.clearHistoryAction')}
          {/if}
        </button>
      </div>
    </div>
  </div>
</div>
