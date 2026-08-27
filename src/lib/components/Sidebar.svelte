<script>
  import { link, location } from 'svelte-spa-router';
  import { invoke } from '@tauri-apps/api/core';
  import { getLocaleShortLabel, locale, setLocale, t } from '$lib/i18n/index.js';

  export let isRecording = true;
  export let isPaused = false;
  
  let localeMenuOpen = false;
  let localeMenuContainer;

  const navItems = [
    { path: '/timeline', labelKey: 'sidebar.nav.timeline', icon: 'timeline' },
    { path: '/settings', labelKey: 'sidebar.nav.settings', icon: 'settings' },
  ];

  $: currentLocale = $locale;
  $: translate = (key, params = {}) => {
    currentLocale;
    return t(key, params);
  };
  const localeOptionsBase = [
    { value: 'zh-CN', label: 'ZH', fullLabelKey: 'sidebar.localeNames.zhCN' },
    { value: 'zh-TW', label: 'TW', fullLabelKey: 'sidebar.localeNames.zhTW' },
    { value: 'en', label: 'EN', fullLabelKey: 'sidebar.localeNames.en' },
  ];
  $: localeOptions = localeOptionsBase.map((option) => ({
    ...option,
    fullLabel: translate(option.fullLabelKey),
  }));
  $: currentLocaleLabel = getLocaleShortLabel(currentLocale);

  function toggleLocaleMenu() {
    localeMenuOpen = !localeMenuOpen;
  }

  function selectLocale(nextLocale) {
    setLocale(nextLocale);
    localeMenuOpen = false;
  }

  function handleWindowClick(event) {
    if (!localeMenuOpen || localeMenuContainer?.contains(event.target)) {
      return;
    }

    localeMenuOpen = false;
  }

  function handleWindowKeydown(event) {
    if (event.key === 'Escape') {
      localeMenuOpen = false;
    }
  }

  async function toggleRecording() {
    try {
      if (isPaused) {
        await invoke('resume_recording');
      } else {
        await invoke('pause_recording');
      }
    } catch (e) {
      console.error('切换录制状态失败:', e);
    }
  }

  $: activeStates = navItems.reduce((acc, item) => {
    const loc = $location || '/';
    if (item.path === '/timeline') {
      acc[item.path] = loc === '/' || loc === '/timeline' || loc.startsWith('/timeline/');
    } else {
      acc[item.path] = loc === item.path || loc.startsWith(item.path + '/');
    }
    return acc;
  }, {});
</script>

<svelte:window on:click={handleWindowClick} on:keydown={handleWindowKeydown} />

<div class="sidebar-editorial-shell h-full flex flex-col overflow-hidden">
  <div class="sidebar-main">
    <nav class="sidebar-nav sidebar-nav-section">
      <ul class="sidebar-nav-list">
        {#each navItems as item}
          <li>
            <a href={item.path} use:link
              class="group sidebar-nav-item
                {activeStates[item.path]
                  ? 'sidebar-nav-item-active'
                  : 'sidebar-nav-item-idle'}">

              {#if activeStates[item.path]}
                <div class="sidebar-nav-rail"></div>
              {/if}
              <div class="sidebar-nav-main">
                <!-- SVG 图标 (Microsoft Fluent System Icons) -->
                <div class="sidebar-nav-icon {activeStates[item.path] ? 'text-indigo-600 dark:text-indigo-400' : 'text-slate-400 group-hover:text-slate-500 dark:group-hover:text-slate-300'}">
                  {#if item.icon === 'timeline'}
                    <svg fill="currentColor" viewBox="0 0 24 24">
                      <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8Zm.5-13a.75.75 0 0 0-.75.75v4.5c0 .2.08.39.22.53l3 3a.75.75 0 0 0 1.06-1.06L13.25 11.94V7.75A.75.75 0 0 0 12.5 7Z" />
                    </svg>
                  {:else if item.icon === 'settings'}
                    <svg fill="currentColor" viewBox="0 0 24 24">
                      <path d="M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8Zm-2.5 4a2.5 2.5 0 1 1 5 0 2.5 2.5 0 0 1-5 0Z" />
                      <path d="M10.23 2.5a1.75 1.75 0 0 0-1.7 1.34l-.32 1.35a7.99 7.99 0 0 0-1.63.94l-1.3-.5a1.75 1.75 0 0 0-2.07.67l-1.73 3a1.75 1.75 0 0 0 .37 2.15l1.04.9a8.16 8.16 0 0 0 0 1.9l-1.04.9a1.75 1.75 0 0 0-.37 2.15l1.73 3a1.75 1.75 0 0 0 2.07.67l1.3-.5c.5.38 1.05.7 1.63.94l.32 1.35a1.75 1.75 0 0 0 1.7 1.34h3.46a1.75 1.75 0 0 0 1.7-1.34l.32-1.35c.58-.24 1.13-.56 1.63-.94l1.3.5a1.75 1.75 0 0 0 2.07-.67l1.73-3a1.75 1.75 0 0 0-.37-2.15l-1.04-.9a8.16 8.16 0 0 0 0-1.9l1.04-.9a1.75 1.75 0 0 0 .37-2.15l-1.73-3a1.75 1.75 0 0 0-2.07-.67l-1.3.5a7.99 7.99 0 0 0-1.63-.94l-.32-1.35a1.75 1.75 0 0 0-1.7-1.34h-3.46Zm-.23 2.84c.08-.06.2-.09.33-.09h3.46c.13 0 .25.03.33.09l.3 1.25a1 1 0 0 0 .8.75c.6.2 1.16.5 1.67.89a1 1 0 0 0 1.07.13l1.2-.47c.12-.05.25-.03.32.02l1.73 3c.06.1.05.23-.03.3l-.97.84a1 1 0 0 0-.33 1.05c.08.62.08 1.24 0 1.86a1 1 0 0 0 .33 1.05l.97.84c.08.07.1.2.03.3l-1.73 3a.35.35 0 0 1-.32.02l-1.2-.47a1 1 0 0 0-1.07.13c-.51.39-1.07.69-1.67.89a1 1 0 0 0-.8.75l-.3 1.25a.4.4 0 0 1-.33.09h-3.46a.4.4 0 0 1-.33-.09l-.3-1.25a1 1 0 0 0-.8-.75 6.7 6.7 0 0 1-1.67-.89 1 1 0 0 0-1.07-.13l-1.2.47a.35.35 0 0 1-.32-.02l-1.73-3a.35.35 0 0 1 .03-.3l.97-.84a1 1 0 0 0 .33-1.05 6.8 6.8 0 0 1 0-1.86 1 1 0 0 0-.33-1.05l-.97-.84a.35.35 0 0 1-.03-.3l1.73-3c.07-.1.2-.12.32-.02l1.2.47a1 1 0 0 0 1.07-.13c.51-.39 1.07-.69 1.67-.89a1 1 0 0 0 .8-.75l.3-1.25Z" />
                    </svg>
                  {/if}
                </div>

                <span class="sidebar-nav-label {activeStates[item.path] ? 'sidebar-nav-label-active' : ''}">{translate(item.labelKey)}</span>
              </div>
            </a>
          </li>
        {/each}
      </ul>
    </nav>

    <div class="sidebar-bottom sidebar-toolbelt">
      <div class="sidebar-status sidebar-status-panel">
        <div class="sidebar-status-copy">
          <span class="sidebar-status-dot {isRecording && !isPaused ? 'sidebar-status-dot-live' : ''}" aria-hidden="true"></span>
          <span>{translate('sidebar.recordingStatus')}</span>
        </div>
        <button
          type="button"
          on:click={toggleRecording}
          class="sidebar-recording-action {isPaused ? 'sidebar-recording-action-resume' : ''}"
        >
          {#if isPaused}{translate('sidebar.resume')}{:else}{translate('sidebar.pause')}{/if}
        </button>
      </div>

      <div class="sidebar-footer w-full justify-between gap-y-2">

        <div class="relative" bind:this={localeMenuContainer}>
          <button
            type="button"
            class="locale-switch"
            aria-label={translate('sidebar.localeButtonTitle')}
            aria-haspopup="menu"
            aria-expanded={localeMenuOpen}
            title={translate('sidebar.localeButtonTitle')}
            on:click={toggleLocaleMenu}
          >
            <span class="leading-none">{currentLocaleLabel}</span>
            <svg class="h-3 w-3 shrink-0 text-slate-400 transition-transform {localeMenuOpen ? 'rotate-180' : ''}" fill="currentColor" viewBox="0 0 16 16">
              <path d="M3.22 5.47a.75.75 0 0 1 1.06 0L8 9.19l3.72-3.72a.75.75 0 1 1 1.06 1.06l-4.25 4.25a.75.75 0 0 1-1.06 0L3.22 6.53a.75.75 0 0 1 0-1.06Z" />
            </svg>
          </button>

          {#if localeMenuOpen}
            <div
              class="locale-menu absolute bottom-full left-0 mb-2 min-w-[156px]"
              role="menu"
            >
              {#each localeOptions as option}
                <button
                  type="button"
                  class="locale-menu-item {currentLocale === option.value ? 'locale-menu-item-active' : ''}"
                  role="menuitemradio"
                  aria-checked={currentLocale === option.value}
                  on:click={() => selectLocale(option.value)}
                >
                  <span class="locale-menu-code">{option.label}</span>
                  <span>{option.fullLabel}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>
