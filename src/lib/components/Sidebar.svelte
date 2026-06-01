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
    { path: '/intent-note', labelKey: 'sidebar.nav.intentNote', icon: 'intent' },
    { path: '/settings', labelKey: 'sidebar.nav.settings', icon: 'settings' },
  ];

  $: currentLocale = $locale;
  $: translate = (key, params = {}) => {
    currentLocale;
    return t(key, params);
  };
  $: sidebarTagSegments = translate('sidebar.tagline')
    .split('·')
    .map((item) => item.trim())
    .filter(Boolean);
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
  <div class="sidebar-top">
    <!-- Logo 区域 -->
    <div class="sidebar-brand sidebar-brand-panel">
      <div class="sidebar-brand-row flex items-center gap-3 min-w-0">
        <div class="flex items-center gap-3 min-w-0">
          <div class="w-10 h-10 rounded-xl overflow-hidden shadow-md shrink-0 ring-1 ring-slate-200/50 dark:ring-slate-700/50">
            <img src="/icons/256x256.png" alt="Work Review" class="w-full h-full object-cover" />
          </div>
          <div class="min-w-0">
            <h1 class="sidebar-brand-title">Work Review Lite</h1>
            <p class="sidebar-brand-line" aria-label={translate('sidebar.tagline')}>
              {#each sidebarTagSegments as segment, index}
                <span class="sidebar-brand-segment">{segment}</span>
                {#if index < sidebarTagSegments.length - 1}
                  <span class="sidebar-brand-separator">·</span>
                {/if}
              {/each}
            </p>
          </div>
        </div>
      </div>
    </div>

    <!-- 录制状态 -->
    <div class="sidebar-status sidebar-status-panel">
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-2 min-w-0">
          <span class="relative flex h-2.5 w-2.5">
            {#if isRecording && !isPaused}
              <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
              <span class="relative inline-flex rounded-full h-2.5 w-2.5 bg-emerald-500"></span>
            {:else}
              <span class="relative inline-flex rounded-full h-2.5 w-2.5 bg-slate-300 dark:bg-slate-600"></span>
            {/if}
          </span>
          <span class="text-[12px] font-semibold tracking-[0.08em] text-slate-500 dark:text-slate-400">
            {translate('sidebar.recordingStatus')}
          </span>
        </div>
        <button
          on:click={toggleRecording}
          class="mt-0.5 shrink-0 px-3 py-1.5 text-[11px] font-semibold rounded-full transition-all
            {isPaused 
              ? 'bg-emerald-100 text-emerald-700 hover:bg-emerald-200 dark:bg-emerald-900/40 dark:text-emerald-300' 
              : 'bg-slate-100 text-slate-600 hover:bg-slate-200 dark:bg-slate-700 dark:text-slate-300'}"
        >
          {#if isPaused}{translate('sidebar.resume')}{:else}{translate('sidebar.pause')}{/if}
        </button>
      </div>
    </div>
  </div>

  <div class="sidebar-main">
    <!-- 导航菜单 -->
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
                <!-- SVG 图标 -->
                <div class="sidebar-nav-icon {activeStates[item.path] ? 'text-indigo-600 dark:text-indigo-400' : 'text-slate-400 group-hover:text-slate-500 dark:group-hover:text-slate-300'}">
                  {#if item.icon === 'timeline'}
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                  {:else if item.icon === 'intent'}
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
                    </svg>
                  {:else if item.icon === 'settings'}
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
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

    <!-- 底部工具栏 -->
    <div class="sidebar-bottom sidebar-toolbelt">
      <div class="sidebar-footer w-full justify-between gap-y-2">

        <div class="relative" bind:this={localeMenuContainer}>
          <button
            type="button"
            class="locale-switch inline-flex h-8 min-w-[54px] items-center justify-center gap-1.5 rounded-full border border-slate-200/80 bg-white/90 px-3 text-[11px] font-semibold tracking-[0.08em] text-slate-600 shadow-[inset_0_1px_0_rgba(255,255,255,0.6)] outline-none transition hover:border-slate-300 hover:text-slate-800 focus:ring-2 focus:ring-slate-300 dark:border-slate-700/80 dark:bg-slate-900/80 dark:text-slate-200 dark:hover:border-slate-600 dark:hover:text-white dark:focus:ring-slate-600"
            aria-label={translate('sidebar.localeButtonTitle')}
            aria-haspopup="menu"
            aria-expanded={localeMenuOpen}
            title={translate('sidebar.localeButtonTitle')}
            on:click={toggleLocaleMenu}
          >
            <span class="leading-none">{currentLocaleLabel}</span>
            <svg class="h-3 w-3 shrink-0 text-slate-400 transition-transform {localeMenuOpen ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 9l6 6 6-6" />
            </svg>
          </button>

          {#if localeMenuOpen}
            <div
              class="absolute bottom-full left-0 mb-2 min-w-[148px] rounded-2xl border border-slate-200/80 bg-white/96 p-1.5 shadow-xl shadow-slate-900/12 backdrop-blur dark:border-slate-700/80 dark:bg-slate-900/96"
              role="menu"
            >
              {#each localeOptions as option}
                <button
                  type="button"
                  class="flex w-full items-center gap-2.5 whitespace-nowrap rounded-xl px-3 py-2 text-left text-xs font-medium transition-colors {currentLocale === option.value ? 'bg-slate-100 text-slate-900 dark:bg-slate-800 dark:text-white' : 'text-slate-500 hover:bg-slate-50 hover:text-slate-800 dark:text-slate-300 dark:hover:bg-slate-800/80 dark:hover:text-white'}"
                  role="menuitemradio"
                  aria-checked={currentLocale === option.value}
                  on:click={() => selectLocale(option.value)}
                >
                  <span class="font-semibold tracking-[0.08em] text-slate-500 dark:text-slate-400">{option.label}</span>
                  <span class="text-slate-700 dark:text-slate-200">{option.fullLabel}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>
