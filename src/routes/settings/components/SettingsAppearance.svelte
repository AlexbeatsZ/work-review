<script>
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { showToast } from '$lib/stores/toast.js';
  import { locale, t } from '$lib/i18n/index.js';

  export let config;
  export let mode = 'background-only';

  const dispatch = createEventDispatcher();

  $: currentLocale = $locale;
  $: showBackgroundSettings = mode === 'full' || mode === 'background-only';

  let blurLabels = [];
  let bgPreview = null;
  let bgUploading = false;
  let appearanceDestroyed = false;

  $: {
    currentLocale;
    blurLabels = [
      t('settingsAppearance.blurClear'),
      t('settingsAppearance.blurLight'),
      t('settingsAppearance.blurMedium'),
    ];
  }

  onMount(async () => {
    if (!showBackgroundSettings) {
      return;
    }

    try {
      const b64 = await invoke('get_background_image');
      if (b64) {
        bgPreview = `data:image/jpeg;base64,${b64}`;
      }
    } catch (error) {
      console.warn('读取背景图失败:', error);
    }
  });

  onDestroy(() => {
    appearanceDestroyed = true;
  });

  function handleBgFileSelect(event) {
    const file = event.target.files?.[0];
    if (!file) return;
    if (!file.type.startsWith('image/')) return;
    if (file.size > 10 * 1024 * 1024) {
      showToast(t('settingsAppearance.imageTooLarge'), 'warning');
      return;
    }

    bgUploading = true;
    const reader = new FileReader();
    reader.onload = async () => {
      if (appearanceDestroyed) return;

      try {
        const b64Data = typeof reader.result === 'string' ? reader.result.split(',')[1] : null;
        if (!b64Data) {
          throw new Error(t('settingsAppearance.imageReadFailed'));
        }
        await invoke('save_background_image', { data: b64Data });
        if (appearanceDestroyed) return;
        config.background_image = 'background.jpg';
        await invoke('save_config', { config });
        const freshB64 = await invoke('get_background_image');
        if (appearanceDestroyed) return;
        const imageUrl = freshB64 ? `data:image/jpeg;base64,${freshB64}` : null;
        bgPreview = imageUrl;
        dispatchBgEvent(imageUrl);
      } catch (error) {
        if (appearanceDestroyed) return;
        console.error('上传背景图失败:', error);
        showToast(t('settingsAppearance.uploadFailed', { error }), 'error');
      } finally {
        if (!appearanceDestroyed) {
          bgUploading = false;
        }
      }
    };
    reader.readAsDataURL(file);
  }

  async function clearBg() {
    try {
      await invoke('clear_background_image');
      bgPreview = null;
      config.background_image = null;
      dispatchBgEvent(null);
      await invoke('save_config', { config });
    } catch (error) {
      console.error('清除背景图失败:', error);
      showToast(t('settingsAppearance.clearFailed', { error }), 'error');
    }
  }

  function updateBgOpacity(value) {
    config.background_opacity = parseFloat(value);
    dispatch('change', config);
    dispatchBgEvent(bgPreview);
    saveConfigQuietly();
  }

  function updateBgBlur(value) {
    config.background_blur = parseInt(value);
    dispatch('change', config);
    dispatchBgEvent(bgPreview);
    saveConfigQuietly();
  }

  function dispatchBgEvent(image) {
    window.dispatchEvent(new CustomEvent('background-changed', {
      detail: {
        image,
        opacity: config.background_opacity ?? 0.25,
        blur: config.background_blur ?? 1,
      },
    }));
  }

  async function saveConfigQuietly() {
    try {
      await invoke('save_config', { config });
    } catch (error) {
      console.error('自动保存配置失败:', error);
    }
  }
</script>

{#if showBackgroundSettings}
  <div class="settings-card" data-locale={currentLocale}>
    <h3 class="settings-card-title">{t('settingsAppearance.backgroundImage')}</h3>

    <div class="settings-section">
      <div class="flex items-start gap-4">
        {#if bgPreview}
          <div class="h-20 w-32 flex-shrink-0 overflow-hidden rounded-lg border border-slate-200 dark:border-slate-700">
            <img src={bgPreview} alt={t('settingsAppearance.bgPreviewAlt')} class="h-full w-full object-cover" />
          </div>
        {:else}
          <div class="flex h-20 w-32 flex-shrink-0 items-center justify-center rounded-lg border-2 border-dashed border-slate-200 dark:border-slate-700">
            <span class="settings-subtle">{t('settingsAppearance.noBackground')}</span>
          </div>
        {/if}

        <div class="settings-field flex-1">
          <label class="settings-action-secondary cursor-pointer">
            {#if bgUploading}
              <div class="h-3 w-3 animate-spin rounded-full border-2 border-slate-500 border-t-transparent"></div>
              {t('common.processing')}
            {:else}
              <svg class="h-4 w-4" fill="currentColor" viewBox="0 0 20 20"><path d="M4 3a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2H4Zm0 1.5h12a.5.5 0 0 1 .5.5v5.88l-3.3-3.3a1.5 1.5 0 0 0-2.12 0l-4.59 4.6-1.5-1.5a1.5 1.5 0 0 0-2.12 0L3.5 11.06V5a.5.5 0 0 1 .5-.5Zm12 11H4a.5.5 0 0 1-.5-.5v-1.88l1.44-1.44a.5.5 0 0 1 .7 0l1.86 1.85a.75.75 0 0 0 1.06 0l4.94-4.94a.5.5 0 0 1 .71 0l3.3 3.3v1.11a.5.5 0 0 1-.5.5ZM6.5 6a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3Z" /></svg>
              {t('settingsAppearance.chooseImage')}
            {/if}
            <input type="file" accept="image/*" class="hidden" on:change={handleBgFileSelect} disabled={bgUploading} />
          </label>
          {#if bgPreview}
            <button type="button" on:click={clearBg} class="settings-link-danger">
              {t('settingsAppearance.clearBackground')}
            </button>
          {/if}
          <p class="settings-muted">{t('settingsAppearance.bgSupport')}</p>
        </div>
      </div>

      {#if bgPreview || config.background_image}
        <hr class="border-slate-200 dark:border-slate-700" />

        <div class="settings-block">
          <div class="flex items-center justify-between">
            <span class="settings-text">{t('settingsAppearance.bgStrength')}</span>
            <span class="settings-value">{Math.round((config.background_opacity ?? 0.25) * 100)}%</span>
          </div>
          <input
            type="range"
            min="0.05"
            max="0.60"
            step="0.01"
            value={config.background_opacity ?? 0.25}
            on:input={(event) => updateBgOpacity(event.target.value)}
            class="range-input"
          />
          <div class="settings-subtle flex justify-between text-[10px]">
            <span>{t('settingsAppearance.bgLight')}</span>
            <span>{t('settingsAppearance.bgStrong')}</span>
          </div>
        </div>

        <div class="settings-block">
          <div class="flex items-center justify-between">
            <span class="settings-text">{t('settingsAppearance.bgBlur')}</span>
            <span class="settings-muted">{blurLabels[config.background_blur ?? 1]}</span>
          </div>
          <div class="flex gap-2">
            {#each [0, 1, 2] as level}
              <button
                type="button"
                on:click={() => updateBgBlur(level)}
                class="segment-btn {(config.background_blur ?? 1) === level ? 'settings-segment-active' : 'settings-segment-base'}"
              >
                {blurLabels[level]}
              </button>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}
