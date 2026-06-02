<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { locale, t } from '$lib/i18n/index.js';

  export let config;

  const dispatch = createEventDispatcher();
  $: currentLocale = $locale;
  let autoStartEnabled = false;

  onMount(async () => {
    try {
      autoStartEnabled = await invoke('is_autostart_enabled');
      if (config.auto_start !== autoStartEnabled) {
        config.auto_start = autoStartEnabled;
        try {
          await invoke('save_config', { config });
        } catch (e) {
          console.error('对齐注册表自启状态时写盘失败:', e);
        }
        dispatch('change', config);
      }
    } catch (e) {
      console.error('查询自启动状态失败:', e);
    }
  });

  function handleChange() {
    dispatch('change', config);
  }

  async function toggleAutoStart() {
    const targetState = !autoStartEnabled;
    try {
      if (targetState) {
        await invoke('enable_autostart', { silent: !!config.auto_start_silent });
      } else {
        await invoke('disable_autostart');
      }
    } catch (e) {
      console.warn(`切换系统自启失败/警告 (目标状态: ${targetState}):`, e);
    }
    try {
      autoStartEnabled = await invoke('is_autostart_enabled');
      config.auto_start = autoStartEnabled;
      try {
        await invoke('save_config', { config });
      } catch (e) {
        console.error('保存开机自启状态失败:', e);
      }
      dispatch('change', config);
    } catch (e) {
      console.error('重新校验开机自启状态失败:', e);
    }
  }

  async function toggleDockIcon() {
    config.hide_dock_icon = !config.hide_dock_icon;
    try {
      await invoke('set_dock_visibility', { visible: !config.hide_dock_icon });
    } catch (e) {
      console.error('设置 Dock 图标失败:', e);
    }
    dispatch('change', config);
  }

  async function updateAutoStartLaunchMode(silentMode) {
    config.auto_start_silent = silentMode;
    try {
      await invoke('save_config', { config });
    } catch (e) {
      console.error('保存启动模式失败:', e);
    }
    if (autoStartEnabled) {
      try {
        await invoke('enable_autostart', { silent: silentMode });
      } catch (e) {
        console.error('更新自启动参数失败:', e);
      }
    }
    dispatch('change', config);
  }
</script>

<div class="settings-card" data-locale={currentLocale}>
  <h3 class="settings-card-title">{t('settingsGeneral.title')}</h3>

  <div class="settings-section">
    <div class="settings-block">
      <div class="flex items-center justify-between mt-3">
        <div>
          <span class="settings-text text-sm">{t('settingsGeneral.idleThreshold')}</span>
          <p class="settings-muted mt-0.5">{t('settingsGeneral.idleThresholdHint')}</p>
        </div>
        <div class="flex items-center gap-2">
          <input
            type="number"
            min="1"
            max="60"
            step="1"
            bind:value={config.idle_threshold_minutes}
            on:change={() => {
              config.idle_threshold_minutes = Math.max(1, Math.min(60, Number(config.idle_threshold_minutes) || 5));
              handleChange();
            }}
            class="w-16 rounded-md border border-slate-200 bg-white px-2 py-1 text-center text-sm dark:border-slate-600 dark:bg-slate-800"
          />
          <span class="text-xs settings-subtle">{t('settingsGeneral.minutesUnit')}</span>
        </div>
      </div>
    </div>

    <!-- 日报设置 -->
    <div class="settings-block pt-4 border-t border-slate-200 dark:border-slate-700">
      <div class="flex flex-wrap items-center gap-3">
        <span class="settings-text">{t('settingsGeneral.reportAutoGenerateTime')}</span>
        <div class="control-inline">
          <input
            type="time"
            value={config.daily_report_auto_generate_time ?? ''}
            on:change={(e) => {
              config.daily_report_auto_generate_time = e.target.value || null;
              dispatch('change', config);
            }}
            class="w-20 bg-transparent text-sm font-mono text-slate-800 dark:text-white focus:outline-none"
          />
        </div>
        {#if config.daily_report_auto_generate_time}
          <button
            type="button"
            class="inline-flex items-center gap-1 px-2.5 py-1.5 text-xs rounded-lg text-rose-500 hover:bg-rose-50 dark:text-rose-400 dark:hover:bg-rose-950/30 transition-colors"
            on:click={() => {
              config.daily_report_auto_generate_time = null;
              dispatch('change', config);
            }}
          >
            {t('settingsGeneral.reportAutoGenerateReset')}
          </button>
        {/if}
      </div>
      <p class="settings-note">{t('settingsGeneral.reportAutoGenerateTimeHint')}</p>
    </div>

    <!-- 系统行为 -->
    <div class="settings-block pt-4 border-t border-slate-200 dark:border-slate-700">
      <div class="space-y-2.5">
        <div class="settings-row">
          <span class="settings-text">{t('settingsGeneral.autoStart')}</span>
          <button
            on:click={toggleAutoStart}
            class="switch-track {autoStartEnabled ? 'bg-primary-500' : 'bg-slate-300 dark:bg-slate-600'}"
          >
            <span class="switch-thumb {autoStartEnabled ? 'translate-x-5' : 'translate-x-0'}"></span>
          </button>
        </div>

        {#if autoStartEnabled}
          <div class="ml-3 pl-3 border-l-2 border-primary-200/60 dark:border-primary-800/40">
            <span class="settings-label">{t('settingsGeneral.autoStartLaunchMode')}</span>
            <div class="mt-2 flex gap-2">
              <button
                type="button"
                on:click={() => updateAutoStartLaunchMode(false)}
                class="segment-btn {config.auto_start_silent ? 'settings-segment-base' : 'settings-segment-active'}"
              >
                {t('settingsGeneral.autoStartLaunchShow')}
              </button>
              <button
                type="button"
                on:click={() => updateAutoStartLaunchMode(true)}
                class="segment-btn {config.auto_start_silent ? 'settings-segment-active' : 'settings-segment-base'}"
              >
                {t('settingsGeneral.autoStartLaunchSilent')}
              </button>
            </div>
          </div>
        {/if}

        <div class="settings-row">
          <span class="settings-text">{t('settingsGeneral.hideDockIcon')}</span>
          <button
            on:click={toggleDockIcon}
            class="switch-track {config.hide_dock_icon ? 'bg-primary-500' : 'bg-slate-300 dark:bg-slate-600'}"
          >
            <span class="switch-thumb {config.hide_dock_icon ? 'translate-x-5' : 'translate-x-0'}"></span>
          </button>
        </div>

      </div>
    </div>
  </div>
</div>
