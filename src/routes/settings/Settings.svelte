<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { cache } from '../../lib/stores/cache.js';
  import { locale, t } from '$lib/i18n/index.js';
  import { showToast } from '../../lib/stores/toast.js';

  import SettingsGeneral from './components/SettingsGeneral.svelte';
  import SettingsSystem from './components/SettingsSystem.svelte';
  import SettingsPrivacy from './components/SettingsPrivacy.svelte';
  import SettingsStorage from './components/SettingsStorage.svelte';
  let config = null;
  let loading = true;
  let saving = false;
  let dirty = false;
  let error = null;
  let success = false;
  let runningApps = [];
  let recentApps = [];
  let storageStats = null;
  let dataDir = '';
  let databasePath = '';
  let defaultDataDir = '';
  let settingsRuntimePlatform = '';
  let successTimer = null;
  $: currentLocale = $locale;

  // 当前激活的标签
  let activeTab = 'general';

  const tabs = [
    { id: 'general', labelKey: 'settings.tabs.general', icon: 'general' },
    { id: 'privacy', labelKey: 'settings.tabs.privacy', icon: 'privacy' },
    { id: 'storage', labelKey: 'settings.tabs.storage', icon: 'storage' },
  ];

  // 加载配置
  async function loadConfig() {
    loading = true;
    error = null;
    try {
      const [loadedConfig, loadedStorageStats, loadedDataDir, loadedDatabasePath, loadedDefaultDataDir, loadedRuntimePlatform] = await Promise.all([
        invoke('get_config'),
        invoke('get_storage_stats'),
        invoke('get_data_dir'),
        invoke('get_database_path'),
        invoke('get_default_data_dir'),
        invoke('get_runtime_platform'),
      ]);

      config = loadedConfig;
      cache.setConfig(config);
      storageStats = loadedStorageStats;
      dataDir = loadedDataDir;
      databasePath = loadedDatabasePath;
      defaultDataDir = loadedDefaultDataDir;
      settingsRuntimePlatform = loadedRuntimePlatform;

      // 确保对象存在
      if (!config.ai_provider) {
        config.ai_provider = { provider: 'ollama', endpoint: 'http://localhost:11434', api_key: null, model: 'llava', vision_model: 'llava' };
      }
      if (!config.text_model) {
        config.text_model = { provider: 'ollama', endpoint: 'http://localhost:11434', api_key: null, model: 'qwen2.5' };
      }
      if (!config.text_model_profiles) {
        config.text_model_profiles = [];
      }
      if (typeof config.daily_report_custom_prompt !== 'string') {
        config.daily_report_custom_prompt = '';
      }
      if (typeof config.daily_report_export_dir !== 'string' && config.daily_report_export_dir !== null) {
        config.daily_report_export_dir = null;
      }
      if (typeof config.daily_report_auto_export !== 'boolean') {
        config.daily_report_auto_export = false;
      }
      if (!Array.isArray(config.daily_report_prompt_presets)) {
        config.daily_report_prompt_presets = [];
      }
      if (typeof config.localhost_api_enabled !== 'boolean') {
        config.localhost_api_enabled = false;
      }
      if (!Number.isInteger(config.localhost_api_port) || config.localhost_api_port <= 0) {
        config.localhost_api_port = 47831;
      }
      if (typeof config.localhost_api_host !== 'string' && config.localhost_api_host !== null) {
        config.localhost_api_host = null;
      }
      if (typeof config.telegram_bot_enabled !== 'boolean') {
        config.telegram_bot_enabled = false;
      }
      if (typeof config.telegram_bot_token !== 'string' && config.telegram_bot_token !== null) {
        config.telegram_bot_token = null;
      }
      if (typeof config.telegram_bot_proxy !== 'string' && config.telegram_bot_proxy !== null) {
        config.telegram_bot_proxy = null;
      }
      if (!Array.isArray(config.node_devices)) {
        config.node_devices = [];
      }
      if (typeof config.feishu_bot_enabled !== 'boolean') {
        config.feishu_bot_enabled = false;
      }
      if (typeof config.feishu_app_id !== 'string' && config.feishu_app_id !== null) {
        config.feishu_app_id = null;
      }
      if (typeof config.feishu_app_secret !== 'string' && config.feishu_app_secret !== null) {
        config.feishu_app_secret = null;
      }
      if (typeof config.feishu_verification_token !== 'string' && config.feishu_verification_token !== null) {
        config.feishu_verification_token = null;
      }
      if (!config.node_gateway || typeof config.node_gateway !== 'object') {
        config.node_gateway = {
          device_name: null,
        };
      }
      if (
        typeof config.node_gateway.device_name !== 'string' &&
        config.node_gateway.device_name !== null
      ) {
        config.node_gateway.device_name = null;
      }
      if (!config.vision_model) {
        config.vision_model = { provider: 'ollama', endpoint: 'http://localhost:11434', api_key: null, model: 'llava' };
      }
      if (typeof config.lightweight_mode !== 'boolean') {
        config.lightweight_mode = true;
      }
      config.lightweight_mode = true;
      if (typeof config.auto_start_silent !== 'boolean') {
        config.auto_start_silent = false;
      }
      if (!config.storage) {
        config.storage = {
          screenshot_retention_days: 7,
          metadata_retention_days: 30,
          storage_limit_mb: 2048,
          jpeg_quality: 85,
          max_image_width: 1280,
          screenshots_enabled: true,
          screenshot_display_mode: 'active_window',
          screenshot_width_mode: 'auto',
        };
      }
      if (typeof config.storage.screenshots_enabled !== 'boolean') {
        config.storage.screenshots_enabled = true;
      }
      if (!config.storage.screenshot_display_mode) {
        config.storage.screenshot_display_mode = 'active_window';
      }
      if (!['auto', 'fixed'].includes(config.storage.screenshot_width_mode)) {
        config.storage.screenshot_width_mode = 'auto';
      }
      if (!config.app_category_rules) config.app_category_rules = [];
      if (!config.privacy) config.privacy = {};
      if (!config.privacy.app_rules) config.privacy.app_rules = [];
      if (!config.privacy.excluded_keywords) config.privacy.excluded_keywords = [];
      delete config.privacy.sensitive_keywords;
    } catch (e) {
      error = e.toString();
      console.error('加载配置失败:', e);
      settingsRuntimePlatform = '';
    } finally {
      loading = false;
    }
  }

  // 加载运行中的应用
  async function loadRunningApps() {
    try {
      runningApps = await invoke('get_running_apps');
    } catch (e) {
      console.error('获取运行应用失败:', e);
      runningApps = [];
    }
  }

  // 加载历史应用列表
  async function loadRecentApps() {
    try {
      recentApps = await invoke('get_recent_apps');
    } catch (e) {
      console.error('获取历史应用失败:', e);
      recentApps = [];
    }
  }

  // 保存配置
  async function saveConfig() {
    saving = true;
    error = null;
    success = false;

    try {
      delete config.privacy?.sensitive_keywords;
      await invoke('save_config', { config });
      success = true;
      dirty = false;
      cache.setConfig(config);
      showToast(t('settings.saveSuccessToast'), 'success');
      
      clearTimeout(successTimer);
      successTimer = setTimeout(() => {
        success = false;
        successTimer = null;
      }, 3000);
    } catch (e) {
      error = e.toString();
    } finally {
      saving = false;
    }
  }

  // 清理缓存回调
  async function handleClearCache() {
    try {
      const [latestStats, latestDataDir, latestDatabasePath] = await Promise.all([
        invoke('get_storage_stats'),
        invoke('get_data_dir'),
        invoke('get_database_path'),
      ]);
      storageStats = latestStats;
      dataDir = latestDataDir;
      databasePath = latestDatabasePath;
    } catch (e) {
      console.error('刷新存储状态失败:', e);
    }
  }

  async function handleDataDirChanged() {
    try {
      const [latestStats, latestDataDir, latestDatabasePath] = await Promise.all([
        invoke('get_storage_stats'),
        invoke('get_data_dir'),
        invoke('get_database_path'),
      ]);
      storageStats = latestStats;
      dataDir = latestDataDir;
      databasePath = latestDatabasePath;
      cache.clear();
    } catch (e) {
      console.error('切换数据目录后刷新状态失败:', e);
    }
  }

  onMount(() => {
    const unsubscribeCache = cache.subscribe((state) => {
      if (!state.config) return;
      // 保存中或用户已编辑配置时，不覆盖（避免丢弃未保存的修改）
      if (saving) return;
      if (config && dirty) return;
      config = state.config;
    });

    loadConfig();
    loadRunningApps();
    loadRecentApps();

    return () => {
      unsubscribeCache();
      clearTimeout(successTimer);
    };
  });
</script>

<div class="page-shell settings-editorial-shell" data-locale={currentLocale}>
  <div class="page-header">
    <div class="page-title-group">
      <div class="page-title-badge">
        <svg fill="currentColor" viewBox="0 0 24 24">
          <path d="M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8Zm-2.5 4a2.5 2.5 0 1 1 5 0 2.5 2.5 0 0 1-5 0Z" />
          <path d="M10.23 2.5a1.75 1.75 0 0 0-1.7 1.34l-.32 1.35a7.99 7.99 0 0 0-1.63.94l-1.3-.5a1.75 1.75 0 0 0-2.07.67l-1.73 3a1.75 1.75 0 0 0 .37 2.15l1.04.9a8.16 8.16 0 0 0 0 1.9l-1.04.9a1.75 1.75 0 0 0-.37 2.15l1.73 3a1.75 1.75 0 0 0 2.07.67l1.3-.5c.5.38 1.05.7 1.63.94l.32 1.35a1.75 1.75 0 0 0 1.7 1.34h3.46a1.75 1.75 0 0 0 1.7-1.34l.32-1.35c.58-.24 1.13-.56 1.63-.94l1.3.5a1.75 1.75 0 0 0 2.07-.67l1.73-3a1.75 1.75 0 0 0-.37-2.15l-1.04-.9a8.16 8.16 0 0 0 0-1.9l1.04-.9a1.75 1.75 0 0 0 .37-2.15l-1.73-3a1.75 1.75 0 0 0-2.07-.67l-1.3.5a7.99 7.99 0 0 0-1.63-.94l-.32-1.35a1.75 1.75 0 0 0-1.7-1.34h-3.46Zm-.23 2.84c.08-.06.2-.09.33-.09h3.46c.13 0 .25.03.33.09l.3 1.25a1 1 0 0 0 .8.75c.6.2 1.16.5 1.67.89a1 1 0 0 0 1.07.13l1.2-.47c.12-.05.25-.03.32.02l1.73 3c.06.1.05.23-.03.3l-.97.84a1 1 0 0 0-.33 1.05c.08.62.08 1.24 0 1.86a1 1 0 0 0 .33 1.05l.97.84c.08.07.1.2.03.3l-1.73 3a.35.35 0 0 1-.32.02l-1.2-.47a1 1 0 0 0-1.07.13c-.51.39-1.07.69-1.67.89a1 1 0 0 0-.8.75l-.3 1.25a.4.4 0 0 1-.33.09h-3.46a.4.4 0 0 1-.33-.09l-.3-1.25a1 1 0 0 0-.8-.75 6.7 6.7 0 0 1-1.67-.89 1 1 0 0 0-1.07-.13l-1.2.47a.35.35 0 0 1-.32-.02l-1.73-3a.35.35 0 0 1 .03-.3l.97-.84a1 1 0 0 0 .33-1.05 6.8 6.8 0 0 1 0-1.86 1 1 0 0 0-.33-1.05l-.97-.84a.35.35 0 0 1-.03-.3l1.73-3c.07-.1.2-.12.32-.02l1.2.47a1 1 0 0 0 1.07-.13c.51-.39 1.07-.69 1.67-.89a1 1 0 0 0 .8-.75l.3-1.25Z" />
        </svg>
      </div>
      <div class="page-title-copy">
        <h2>{t('settings.title')}</h2>
        <p>{t('settings.subtitle')}</p>
      </div>
    </div>

    <!-- 保存按钮 -->
    <div class="settings-save-dock">
      <button
        on:click={saveConfig}
        disabled={loading || saving}
        class="settings-action-primary px-4 rounded-xl"
      >
        {#if saving}
          <div class="animate-spin rounded-full h-4 w-4 border-2 border-white border-t-transparent"></div>
          {t('settings.saving')}
        {:else if success}
          <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
            <path d="M17.47 6.47a.75.75 0 0 1 0 1.06l-8.5 8.5a.75.75 0 0 1-1.06 0l-4.25-4.25a.75.75 0 1 1 1.06-1.06L8.44 14.94l7.97-7.97a.75.75 0 0 1 1.06 0Z" />
          </svg>
          {t('settings.saved')}
        {:else}
          {t('settings.save')}
        {/if}
      </button>
    </div>
  </div>

  {#if loading}
    <div class="flex justify-center py-12">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
    </div>
  {:else if error}
    <div class="page-banner-error mb-6">
      <div>
        <p class="font-semibold">{t('settings.loadError')}</p>
        <p class="text-sm mt-1">{error}</p>
      </div>
      <button on:click={loadConfig} class="page-action-brand">{t('settings.retry')}</button>
    </div>
  {:else if config}
    <div class="w-full settings-editorial-board">
      {#if settingsRuntimePlatform === 'macos'}
        <div class="settings-card settings-top-status-zone">
          <SettingsSystem />
        </div>
      {/if}

      <div class="settings-stage-layout">
        <div class="settings-tab-rail">
          {#each tabs as tab}
            <button
              on:click={() => activeTab = tab.id}
              class="settings-tab-rail-item {activeTab === tab.id ? 'settings-tab-rail-item-active' : ''}"
            >
              <span class="settings-tab-rail-icon">
                {#if tab.icon === 'general'}
                  <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 20 20">
                    <path d="M10 7a3 3 0 1 0 0 6 3 3 0 0 0 0-6Zm-1.5 3a1.5 1.5 0 1 1 3 0 1.5 1.5 0 0 1-3 0Z" />
                    <path d="M8.58 2.25a1.5 1.5 0 0 0-1.46 1.15l-.26 1.13c-.45.18-.87.42-1.25.72l-1.08-.41a1.5 1.5 0 0 0-1.78.58l-1.5 2.6a1.5 1.5 0 0 0 .32 1.84l.87.75a6.9 6.9 0 0 0 0 1.58l-.87.75a1.5 1.5 0 0 0-.32 1.84l1.5 2.6a1.5 1.5 0 0 0 1.78.58l1.08-.41c.38.3.8.54 1.25.72l.26 1.13a1.5 1.5 0 0 0 1.46 1.15h3a1.5 1.5 0 0 0 1.46-1.15l.26-1.13c.45-.18.87-.42 1.25-.72l1.08.41a1.5 1.5 0 0 0 1.78-.58l1.5-2.6a1.5 1.5 0 0 0-.32-1.84l-.87-.75a6.9 6.9 0 0 0 0-1.58l.87-.75a1.5 1.5 0 0 0 .32-1.84l-1.5-2.6a1.5 1.5 0 0 0-1.78-.58l-1.08.41a5.6 5.6 0 0 0-1.25-.72l-.26-1.13a1.5 1.5 0 0 0-1.46-1.15h-3Zm-.26 2.38a.25.25 0 0 1 .24-.13h3c.1 0 .2.05.24.13l.25 1.05a.75.75 0 0 0 .6.56c.49.16.94.41 1.34.72a.75.75 0 0 0 .8.1l1-.38a.25.25 0 0 1 .29.1l1.5 2.6a.25.25 0 0 1-.05.3l-.8.7a.75.75 0 0 0-.25.77c.07.5.07 1.02 0 1.52a.75.75 0 0 0 .25.77l.8.7a.25.25 0 0 1 .05.3l-1.5 2.6a.25.25 0 0 1-.29.1l-1-.38a.75.75 0 0 0-.8.1c-.4.31-.85.56-1.34.72a.75.75 0 0 0-.6.56l-.25 1.05a.25.25 0 0 1-.24.13h-3a.25.25 0 0 1-.24-.13l-.25-1.05a.75.75 0 0 0-.6-.56 4.4 4.4 0 0 1-1.34-.72.75.75 0 0 0-.8-.1l-1 .38a.25.25 0 0 1-.29-.1l-1.5-2.6a.25.25 0 0 1 .05-.3l.8-.7a.75.75 0 0 0 .25-.77 5.5 5.5 0 0 1 0-1.52.75.75 0 0 0-.25-.77l-.8-.7a.25.25 0 0 1-.05-.3l1.5-2.6a.25.25 0 0 1 .29-.1l1 .38a.75.75 0 0 0 .8-.1c.4-.31.85-.56 1.34-.72a.75.75 0 0 0 .6-.56l.25-1.05Z" />
                  </svg>
                {:else if tab.icon === 'privacy'}
                  <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 20 20">
                    <path d="M10 2a1.75 1.75 0 0 0-.74.16L3.76 4.91A1.75 1.75 0 0 0 2.75 6.47v4.28c0 4.19 2.66 7.42 6.8 8.94.29.1.61.1.9 0 4.14-1.52 6.8-4.75 6.8-8.94V6.47a1.75 1.75 0 0 0-1.01-1.56l-5.5-2.75A1.75 1.75 0 0 0 10 2Zm0 1.63 5.5 2.75c.12.06.2.19.2.33v4.04c0 3.52-2.18 6.27-5.7 7.6a.75.75 0 0 1-.5 0C5.98 17.02 3.8 14.27 3.8 10.75V6.71c0-.14.08-.27.2-.33L9.5 3.63a.75.75 0 0 1 .5 0Zm-.75 4.62a1.75 1.75 0 0 1 2.25 1.68V11h.25a1 1 0 0 1 1 1v2.5a1 1 0 0 1-1 1h-3.5a1 1 0 0 1-1-1V12a1 1 0 0 1 1-1h.25V9.93a1.75 1.75 0 0 1 .75-1.68Zm1.25 1.68a.5.5 0 0 0-1 0V11h1V9.93Z" />
                  </svg>
                {:else if tab.icon === 'storage'}
                  <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 20 20">
                    <path d="M10 2c3.87 0 7 1.12 7 2.5v11c0 1.38-3.13 2.5-7 2.5s-7-1.12-7-2.5v-11C3 3.12 6.13 2 10 2Zm0 1.5C6.96 3.5 4.5 4.3 4.5 4.5S6.96 5.5 10 5.5s5.5-.8 5.5-1S13.04 3.5 10 3.5ZM4.5 7.15c.87.6 2.82 1.1 5.5 1.1s4.63-.5 5.5-1.1v2.1c-.87.6-2.82 1.1-5.5 1.1s-4.63-.5-5.5-1.1V7.15Zm0 4.25c.87.6 2.82 1.1 5.5 1.1s4.63-.5 5.5-1.1v2.1c-.87.6-2.82 1.1-5.5 1.1s-4.63-.5-5.5-1.1V11.4Z" />
                  </svg>
                {/if}
              </span>
              <span class="inline-flex items-center gap-1 whitespace-nowrap">
                <span>{t(tab.labelKey)}</span>
                {#if tab.beta}
                  <span class="inline-flex items-center rounded-full border border-amber-200 bg-amber-50 px-1 py-px text-[8px] font-semibold uppercase tracking-[0.06em] text-amber-700 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-200">
                    Beta
                  </span>
                {/if}
              </span>
            </button>
          {/each}
        </div>

        <div class="settings-stage-shell">
        {#if activeTab === 'general'}
          <SettingsGeneral bind:config on:change={() => dirty = true} />
        {:else if activeTab === 'privacy'}
          <SettingsPrivacy
            bind:config
            {runningApps}
            {recentApps}
            on:change={() => dirty = true}
          />
        {:else if activeTab === 'storage'}
          <SettingsStorage
            bind:config
            {storageStats}
            {dataDir}
            {databasePath}
            {defaultDataDir}
            on:change={() => dirty = true}
            on:clearCache={handleClearCache}
            on:dataDirChanged={handleDataDirChanged}
            on:databasePathChanged={handleDataDirChanged}
          />
        {/if}
        </div>
      </div>
    </div>
  {/if}
</div>
