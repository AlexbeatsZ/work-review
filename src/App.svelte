<script>
  import { onMount } from 'svelte';
  import Router, { location } from 'svelte-spa-router';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Toast from './lib/components/Toast.svelte';
  import ConfirmDialog from './lib/components/ConfirmDialog.svelte';
  import Timeline from './routes/timeline/Timeline.svelte';
  import Summary from './routes/timeline/Summary.svelte';
  import Settings from './routes/settings/Settings.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { cache, getLocalDate } from './lib/stores/cache.js';
  import { applyLocaleToDocument, initializeLocale, locale } from '$lib/i18n/index.js';
  import { preloadAppIcons } from './lib/stores/iconCache.js';

  const appWindow = getCurrentWebviewWindow();

  // 視窗拖拽（Linux WebKitGTK 不支援 -webkit-app-region: drag，改用 Tauri API）
  let lastDragClick = 0;
  async function startDrag(e) {
    if (e.button !== 0 || e.target.closest('button')) return;
    const now = Date.now();
    if (now - lastDragClick < 350) {
      lastDragClick = 0;
      await maximizeWindow();
      return;
    }
    lastDragClick = now;
    await appWindow.startDragging();
  }

  // 窗口控制函数
  async function closeWindow() {
    await appWindow.close();
  }

  async function minimizeWindow() {
    await appWindow.minimize();
  }

  async function maximizeWindow() {
    const isMaximized = await appWindow.isMaximized();
    if (isMaximized) {
      await appWindow.unmaximize();
    } else {
      await appWindow.maximize();
    }
  }

  // 预加载核心数据
  async function preloadApp() {
    console.log('开始预加载数据...');
    const today = getLocalDate();
    
    Promise.all([
      invoke('get_timeline', { date: today, limit: 20, offset: 0 }),
      invoke('get_hourly_summaries', { date: today })
    ]).then(([activities, summaries]) => {
      cache.setTimeline(today, activities, summaries);
      preloadAppIcons(
        (activities || []).slice(0, 12).map((activity) => ({
          appName: activity.app_name,
          executablePath: activity.executable_path,
        })),
        invoke,
        { priority: true }
      );
      console.log('预加载完成');
    }).catch(e => {
      console.warn('预加载时间线失败:', e);
    });
  }

  const routes = {
    '/': Timeline,
    '/timeline': Timeline,
    '/timeline/summary': Summary,
    '/settings': Settings,
  };

  let theme = 'system';
  let isDark = false;
  let isRecording = true;
  let isPaused = false;
  let platform = '';
  let unsubscribeLocale = () => {};
  $: currentLocale = $locale;
  const lowPowerMode = true;

  function applyTheme(_newTheme) {
    theme = 'dark';
    isDark = true;
    document.documentElement.classList.add('dark');
  }

  // 阻止文件拖拽到窗口时 WebView 导航到文件 URL
  function preventFileDrop(e) {
    e.preventDefault();
  }

  onMount(() => {
    // 全局阻止文件拖放导致页面导航（如拖入 PDF 会替换整个应用）
    window.addEventListener('dragover', preventFileDrop);
    window.addEventListener('drop', preventFileDrop);

    initializeLocale();
    unsubscribeLocale = locale.subscribe((nextLocale) => {
      applyLocaleToDocument(nextLocale);
    });

    let disposed = false;
    const pendingCleanup = [];

    // 同步注册的 locale subscription 立即可清理
    pendingCleanup.push(() => unsubscribeLocale());
    pendingCleanup.push(() => window.removeEventListener('dragover', preventFileDrop));
    pendingCleanup.push(() => window.removeEventListener('drop', preventFileDrop));

    (async () => {
      // 获取平台信息
      try {
        platform = await invoke('get_platform');
        console.log('当前平台:', platform);
      } catch (e) {
        console.error('获取平台信息失败:', e);
      }
      if (disposed) return;

      // 加载配置并应用主题
      let config;
      try {
        config = await invoke('get_config');
        cache.setConfig(config);
        applyTheme(config.theme || 'system');
      } catch (e) {
        console.error('加载配置失败:', e);
        applyTheme('system');
        config = {};
      }
      if (disposed) return;

      try {
        const [recording, paused] = await invoke('get_recording_state');
        isRecording = recording;
        isPaused = paused;
      } catch (e) {
        console.error('获取录制状态失败:', e);
      }
      if (disposed) return;

      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      const handleSystemThemeChange = () => {
        applyTheme('dark');
      };
      mediaQuery.addEventListener('change', handleSystemThemeChange);
      pendingCleanup.push(() => mediaQuery.removeEventListener('change', handleSystemThemeChange));

      const unsubscribeCache = cache.subscribe((state) => {
        if (!state.config) return;

        if (state.config.theme && state.config.theme !== theme) {
          applyTheme(state.config.theme);
        }
      });
      pendingCleanup.push(unsubscribeCache);

      const unlistenRecordingState = await listen('recording-state-changed', (event) => {
        isRecording = event.payload.isRecording;
        isPaused = event.payload.isPaused;
      });
      if (disposed) return;
      pendingCleanup.push(unlistenRecordingState);

      const unlistenConfigChanged = await listen('config-changed', (event) => {
        cache.setConfig(event.payload);
      });
      if (disposed) return;
      pendingCleanup.push(unlistenConfigChanged);

      // 启动预加载
      preloadApp();

      const unlisten = await listen('screenshot-taken', (event) => {
        console.log('截屏完成:', event.payload);

        // 1. 增量更新时间线缓存
        cache.addActivity(event.payload);

        // 2. 使概览缓存过期（下次访问或当前页面监听时刷新）
        cache.invalidate('overview');

        // 3. 发射自定义事件，通知当前页面实时更新
        window.dispatchEvent(new CustomEvent('activity-added', { detail: event.payload }));

        // 4. 抢先预热当前应用图标，浏览器记录优先级更高
        preloadAppIcons(
          [{
            appName: event.payload?.app_name,
            executablePath: event.payload?.executable_path,
          }],
          invoke,
          { priority: Boolean(event.payload?.browser_url) }
        );
      });
      if (disposed) return;
      pendingCleanup.push(unlisten);
    })();

    return () => {
      disposed = true;
      pendingCleanup.forEach(fn => { try { fn(); } catch {} });
    };
  });
</script>

<div class="app-shell work-ledger-shell flex h-screen overflow-hidden relative {lowPowerMode ? 'lite-low-power' : ''}">
  <div class="work-ledger-atmosphere pointer-events-none absolute inset-0 z-0 {lowPowerMode ? 'hidden' : ''}">
    <div class="work-ledger-rule work-ledger-rule-top"></div>
    <div class="work-ledger-rule work-ledger-rule-bottom"></div>
  </div>
  <!--
    全局顶部拖拽层 (Invisible Drag Layer)
    1. 覆盖在所有内容之上 (z-50)
    2. 负责处理窗口拖动 (-webkit-app-region: drag)
    3. 按钮区域排除拖动 (-webkit-app-region: no-drag)
  -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="app-shell-windowbar absolute top-0 left-0 w-full h-7 z-50" style="-webkit-app-region: drag;" on:mousedown={startDrag}>
    <!-- 仅 Windows/Linux 平台显示自定义窗口控制按钮，macOS 使用原生控件 -->
    {#if platform && platform !== 'macos'}
    <!-- Windows 风格窗口控制按钮 (右上角) -->
    <div class="app-shell-window-controls absolute right-0 top-0 flex items-stretch h-7" style="-webkit-app-region: no-drag;">
      <!-- Minimize -->
      <button
        on:click={minimizeWindow}
        class="app-shell-window-btn"
        title="最小化"
      >
        <svg fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14" />
        </svg>
      </button>

      <!-- Maximize -->
      <button
        on:click={maximizeWindow}
        class="app-shell-window-btn"
        title="最大化"
      >
        <svg fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <rect x="4" y="4" width="16" height="16" rx="1" />
        </svg>
      </button>

      <!-- Close -->
      <button
        on:click={closeWindow}
        class="app-shell-window-btn app-shell-window-btn-close"
        title="关闭"
      >
        <svg fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
    {/if}
  </div>

  <div class="app-shell-stage relative z-10 flex-1 grid grid-cols-[13.5rem_minmax(0,1fr)] gap-3 m-2 {platform !== 'macos' ? 'pt-7' : 'pt-2'}">
    <!-- 左侧边栏 -->
    <aside class="app-shell-sidebar-frame min-h-0">
      <div class="app-shell-sidebar h-full flex flex-col overflow-hidden">
        <Sidebar {isRecording} {isPaused} />
      </div>
    </aside>

    <!-- 右侧主内容区域 -->
    <section class="app-shell-main-frame min-h-0">
      <div class="app-shell-main relative h-full flex flex-col overflow-hidden">
        <main class="app-shell-main-scroll flex-1 overflow-auto">
          {#key currentLocale}
            <Router {routes} />
          {/key}
        </main>
        <Toast />
        <ConfirmDialog />
      </div>
    </section>
  </div>
</div>
