<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { locale, t } from '$lib/i18n/index.js';
  import { showToast } from '$lib/stores/toast.js';

  $: currentLocale = $locale;

  let runtimePlatform = '';
  let permissionStatus = null;
  let refreshing = false;
  let pendingPermissionItem = null;
  let detailsExpanded = true;

  function normalizePermissionStatus(rawStatus) {
    if (!rawStatus || typeof rawStatus !== 'object') {
      return null;
    }

    return {
      screenCapture: Boolean(rawStatus.screen_capture),
      accessibility: Boolean(rawStatus.accessibility),
      screenshotSupported: Boolean(rawStatus.screenshot_supported),
      allGranted: Boolean(rawStatus.all_granted),
    };
  }

  $: macPermissionItems = permissionStatus
    ? [
        {
          id: 'screen_capture',
          labelKey: 'settingsAppearance.avatarScreenCapturePermission',
          descriptionKey: 'settingsAppearance.avatarScreenCapturePermissionHint',
          granted: permissionStatus.screenCapture,
        },
        {
          id: 'accessibility',
          labelKey: 'settingsAppearance.avatarAccessibilityPermission',
          descriptionKey: 'settingsAppearance.avatarAccessibilityPermissionHint',
          granted: permissionStatus.accessibility,
        },
      ]
    : [];
  $: readyCount = runtimePlatform === 'macos'
    ? macPermissionItems.filter((item) => item.granted).length
    : Number(permissionStatus?.screenshotSupported ?? false);
  $: totalCount = runtimePlatform === 'macos' ? 2 : 1;
  $: needsAttention = permissionStatus ? readyCount < totalCount : false;

  async function refreshPlatformSupport(showNotice = false) {
    refreshing = true;

    try {
      runtimePlatform = await invoke('get_runtime_platform');
      permissionStatus = normalizePermissionStatus(await invoke('check_permissions'));
      if (showNotice && runtimePlatform === 'macos') {
        showToast(t('settingsGeneral.permissionsRefreshNotice'), 'info');
      }
    } catch (error) {
      console.error('读取系统权限状态失败:', error);
      showToast(t('settingsGeneral.permissionsLoadFailed', { error }), 'error');
      runtimePlatform = '';
      permissionStatus = null;
    } finally {
      refreshing = false;
    }
  }

  onMount(() => {
    refreshPlatformSupport();
  });

  function beginPermissionSetup(item) {
    pendingPermissionItem = item;
  }

  function closePermissionSetup() {
    pendingPermissionItem = null;
  }

  async function openPermissionSettings(permission) {
    try {
      await invoke('open_permission_settings', { permission });
    } catch (error) {
      console.error('打开系统权限设置失败:', error);
      showToast(t('settingsGeneral.permissionsOpenFailed', { error }), 'error');
    }
  }

  async function confirmPermissionSetup() {
    const permission = pendingPermissionItem?.id;
    closePermissionSetup();

    if (permission) {
      await openPermissionSettings(permission);
    }
  }

  function permissionSetupMessageKey(permissionId) {
    if (permissionId === 'accessibility') {
      return 'settingsGeneral.permissionsAccessibilityGuide';
    }

    return 'settingsGeneral.permissionsScreenCaptureGuide';
  }
</script>

<div class="settings-block permission-overview" data-locale={currentLocale}>
  <div class="permission-summary-strip">
    <div class="permission-summary-copy">
      <div class="permission-summary-title">{t('settingsGeneral.permissionsTitle')}</div>
      <div class="permission-summary-meta">
        {#if permissionStatus}
          <span class={`permission-summary-badge ${needsAttention ? 'permission-summary-badge-warn' : 'permission-summary-badge-ready'}`}>
            {needsAttention
              ? t('settingsGeneral.permissionsSummaryAttention', {
                  ready: readyCount,
                  total: totalCount,
                  pending: Math.max(0, totalCount - readyCount),
                })
              : t('settingsGeneral.permissionsSummaryReady', {
                  ready: readyCount,
                  total: totalCount,
                })}
          </span>
          <span class="permission-summary-platform">{runtimePlatform || '-'}</span>
        {:else}
          <span class="permission-summary-hint">{t('settingsGeneral.permissionsDescription')}</span>
        {/if}
      </div>
    </div>

    <div class="permission-summary-actions">
      <button type="button" class="permission-summary-toggle" on:click={() => (detailsExpanded = !detailsExpanded)}>
        {detailsExpanded
          ? t('settingsGeneral.permissionsHideDetails')
          : t('settingsGeneral.permissionsDetails')}
      </button>
      <button type="button" class="permission-refresh-button" on:click={() => refreshPlatformSupport(true)} disabled={refreshing}>
        {refreshing
          ? t('settingsGeneral.permissionsRefreshing')
          : t('settingsGeneral.permissionsRefresh')}
      </button>
    </div>
  </div>

  {#if detailsExpanded && permissionStatus}
    <div class="permission-details-panel">
      {#if runtimePlatform === 'macos'}
        {#each macPermissionItems as item}
          <div class={`permission-item-card ${item.granted ? 'permission-item-card-ready' : 'permission-item-card-action'}`}>
            <div class="permission-item-main">
              <div class="permission-item-leading">
                <span class={`permission-item-marker ${item.granted ? 'permission-item-marker-ready' : 'permission-item-marker-action'}`}></span>
                <div class="min-w-0 flex-1">
                  <div class="permission-item-title">{t(item.labelKey)}</div>
                  <div class="permission-item-copy">{t(item.descriptionKey)}</div>
                </div>
              </div>

              {#if item.granted}
                <div class="permission-status-pill permission-status-pill-ready">
                  {t('settingsGeneral.permissionsMacStatusAvailable')}
                </div>
              {:else}
                <button type="button" class="permission-status-pill permission-status-pill-action" on:click={() => beginPermissionSetup(item)}>
                  {t('settingsGeneral.permissionsOpen')}
                </button>
              {/if}
            </div>
          </div>
        {/each}
      {:else}
        <div class={`permission-item-card ${permissionStatus.screenshotSupported ? 'permission-item-card-ready' : 'permission-item-card-action'}`}>
          <div class="permission-item-main">
            <div class="permission-item-leading">
              <span class={`permission-item-marker ${permissionStatus.screenshotSupported ? 'permission-item-marker-ready' : 'permission-item-marker-action'}`}></span>
              <div class="min-w-0 flex-1">
                <div class="permission-item-title">{t('settingsAppearance.avatarScreenshotSupportTitle')}</div>
                <div class="permission-item-copy">
                  {runtimePlatform === 'linux'
                    ? t('settingsGeneral.permissionsLinuxScreenshotHint')
                    : t('settingsGeneral.permissionsWindowsScreenshotHint')}
                </div>
              </div>
            </div>
            <div class={`permission-status-pill ${permissionStatus.screenshotSupported ? 'permission-status-pill-ready' : 'permission-status-pill-warn'}`}>
              {permissionStatus.screenshotSupported
                ? t('settingsGeneral.permissionGranted')
                : t('settingsGeneral.permissionMissing')}
            </div>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

{#if pendingPermissionItem}
  <div class="fixed inset-0 z-[90] flex items-end justify-center bg-slate-950/42 px-4 pb-6 pt-10 backdrop-blur-sm sm:items-center">
    <div class="permission-setup-dialog w-full max-w-[440px] rounded-lg bg-white p-6 dark:bg-slate-900" role="dialog" aria-modal="true" aria-labelledby="permission-setup-title">
      <div class="permission-setup-accent"></div>
      <h3 id="permission-setup-title" class="permission-setup-title">{t(pendingPermissionItem.labelKey)}</h3>
      <p class="permission-setup-copy">{t(permissionSetupMessageKey(pendingPermissionItem.id))}</p>
      <div class="permission-setup-actions">
        <button type="button" class="permission-setup-button permission-setup-button-muted" on:click={closePermissionSetup}>
          {t('settingsGeneral.permissionsLater')}
        </button>
        <button type="button" class="permission-setup-button permission-setup-button-primary" on:click={confirmPermissionSetup}>
          {t('settingsGeneral.permissionsOpen')}
        </button>
      </div>
    </div>
  </div>
{/if}
