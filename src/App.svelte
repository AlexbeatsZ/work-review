<script>
  import { onMount } from 'svelte';
  import {
    request,
    AuthError,
    dayRange,
    localDate,
    duration,
    safeUrl,
  } from './api.js';
  let token = '',
    key = '',
    search = '',
    device = '',
    error = '';
  let connected = false,
    connecting = false,
    busy = false;
  let date = localDate(),
    devices = [],
    items = [],
    next = null,
    updated = null;
  let overview = {
    active_seconds: 0,
    device_seconds: 0,
    count: 0,
    apps: [],
    devices: [],
  };
  let generation = 0,
    debounce,
    selected = null,
    imageUrl = '',
    imageError = '',
    note = '',
    saving = false,
    modalGeneration = 0;
  const states = {
    recording: '记录中',
    starting: '启动中',
    idle: '空闲',
    locked: '已锁屏',
    paused: '已暂停',
    private: '隐私暂停',
    permission: '需要权限',
    error: '采集异常',
    syncing: '正在同步',
  };
  const categories = {
    development: '开发',
    office: '办公',
    browser: '浏览',
    design: '设计',
    communication: '沟通',
    entertainment: '休闲',
    other: '其他',
  };
  $: range = dayRange(date);
  $: chosenDevices = devices.filter((item) => !device || item.id === device);
  $: online = devices.filter(
    (item) => updated && updated.getTime() / 1000 - item.last_seen < 120,
  ).length;
  $: maxApp = Math.max(1, ...overview.apps.map((app) => app.duration));
  $: hourCount = Math.ceil((range.to - range.from) / 3600);
  function parameters(cursor) {
    const params = new URLSearchParams({
      from: range.from,
      to: range.to,
      limit: 100,
    });
    if (device) params.set('device', device);
    if (search.trim()) params.set('q', search.trim());
    if (cursor) {
      params.set('before_time', cursor.timestamp);
      params.set('before_id', cursor.id);
      params.set('before_device', cursor.device_id);
    }
    return params.toString();
  }
  function handleError(problem) {
    if (problem instanceof AuthError) {
      connected = false;
      sessionStorage.removeItem('work-review-key');
    }
    error = problem.message;
  }
  async function refresh() {
    const current = ++generation;
    busy = true;
    error = '';
    const params = parameters();
    try {
      const result = await Promise.all([
        request('/api/devices', token).then((r) => r.json()),
        request(`/api/overview?${params}`, token).then((r) => r.json()),
        request(`/api/activities?${params}`, token).then((r) => r.json()),
      ]);
      if (current !== generation) return;
      [devices, overview] = result;
      items = result[2].items;
      next = result[2].next;
      updated = new Date();
      connected = true;
    } catch (problem) {
      if (current === generation) handleError(problem);
    } finally {
      if (current === generation) busy = false;
    }
  }
  async function connect() {
    connecting = true;
    token = key.trim();
    await refresh();
    if (connected) {
      sessionStorage.setItem('work-review-key', token);
      key = '';
    }
    connecting = false;
  }
  function disconnect() {
    generation++;
    closeDetail();
    sessionStorage.removeItem('work-review-key');
    connected = false;
    token = '';
    devices = [];
    items = [];
    error = '';
    busy = false;
  }
  function selectDevice(value) {
    device = value;
    refresh();
  }
  function changeSearch() {
    clearTimeout(debounce);
    debounce = setTimeout(refresh, 350);
  }
  function moveDay(delta) {
    const value = new Date(range.from * 1000);
    value.setDate(value.getDate() + delta);
    date = localDate(value);
    refresh();
  }
  async function loadMore() {
    if (!next || busy) return;
    const current = generation;
    busy = true;
    try {
      const page = await request(
        `/api/activities?${parameters(next)}`,
        token,
      ).then((r) => r.json());
      if (current === generation) {
        items = [...items, ...page.items];
        next = page.next;
      }
    } catch (problem) {
      if (current === generation) handleError(problem);
    } finally {
      if (current === generation) busy = false;
    }
  }
  async function openDetail(item) {
    closeDetail();
    selected = item;
    note = item.note || '';
    imageError = '';
    const current = ++modalGeneration;
    if (!item.screenshot) return;
    try {
      const blob = await request(
        `/api/screenshots/${item.device_id}/${item.id}`,
        token,
      ).then((r) => r.blob());
      if (current === modalGeneration) imageUrl = URL.createObjectURL(blob);
    } catch (problem) {
      if (current === modalGeneration) imageError = problem.message;
    }
  }
  function closeDetail() {
    modalGeneration++;
    selected = null;
    if (imageUrl) URL.revokeObjectURL(imageUrl);
    imageUrl = '';
    imageError = '';
    saving = false;
  }
  async function saveNote() {
    const item = selected,
      value = note;
    saving = true;
    try {
      await request(
        `/api/activities/${item.device_id}/${item.id}/note`,
        token,
        {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ note: value }),
        },
      );
      items = items.map((row) =>
        row.id === item.id && row.device_id === item.device_id
          ? { ...row, note: value }
          : row,
      );
      if (selected === item) selected = { ...item, note: value };
    } catch (problem) {
      handleError(problem);
    } finally {
      saving = false;
    }
  }
  async function exportReport() {
    try {
      const blob = await request(`/api/report?${parameters()}`, token).then(
        (r) => r.blob(),
      );
      const url = URL.createObjectURL(blob),
        link = document.createElement('a');
      link.href = url;
      link.download = `工作记录-${date}.md`;
      link.click();
      URL.revokeObjectURL(url);
    } catch (problem) {
      handleError(problem);
    }
  }
  function deviceName(id) {
    return devices.find((item) => item.id === id)?.name || '未知设备';
  }
  function dialog(node) {
    const previous = document.activeElement;
    node.showModal();
    return {
      destroy() {
        if (previous?.isConnected) previous.focus();
      },
    };
  }
  function time(timestamp) {
    return new Date(timestamp * 1000).toLocaleTimeString('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
    });
  }
  function hourLabel(hour) {
    return time(range.from + hour * 3600);
  }
  function deviceStatus(item) {
    return !updated || updated.getTime() / 1000 - item.last_seen >= 120
      ? '离线'
      : states[item.state] || '在线';
  }
  function hourSeconds(id, hour) {
    return (
      overview.devices.find((item) => item.device_id === id)?.hours[hour] || 0
    );
  }
  onMount(() => {
    token = sessionStorage.getItem('work-review-key') || '';
    if (token) refresh();
    const timer = setInterval(() => {
      if (connected && !busy && !selected && items.length <= 100) refresh();
    }, 30000);
    return () => {
      clearInterval(timer);
      clearTimeout(debounce);
      if (imageUrl) URL.revokeObjectURL(imageUrl);
    };
  });
</script>

<svelte:window
  on:keydown={(event) => {
    if (event.key === 'Escape') closeDetail();
  }}
/>
{#if !connected}
  <main class="login">
    <div class="login-art" aria-hidden="true">
      <span></span><span></span><span></span><i></i>
    </div>
    <form class="login-form" on:submit|preventDefault={connect}>
      <div class="brand"><span class="brand-mark">w</span> Work Review</div>
      <p class="eyebrow">个人工作记录</p>
      <h1>把每台设备上的<br />一天，放在一起。</h1>
      <p class="muted">采集在后台继续。需要回顾时，从这里开始。</p>
      <label for="view-key">查看密钥</label><input
        id="view-key"
        type="password"
        bind:value={key}
        required
        autocomplete="off"
        placeholder="输入中心服务的查看密钥"
      />
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <button class="primary" disabled={connecting}
        >{connecting ? '正在连接…' : '查看工作记录'} <span>↗</span></button
      >
      <p class="small muted">密钥仅在当前浏览器会话内保存。</p>
    </form>
  </main>
{:else}
  <div class="shell">
    <aside class="sidebar">
      <div class="brand">
        <span class="brand-mark">w</span>
        <div>Work Review<small>个人工作记录</small></div>
      </div>
      <div class="sidebar-label">设备 <span>{online} 台在线</span></div>
      <button
        class:active={!device}
        class="device-button all-devices"
        on:click={() => selectDevice('')}
        ><span class="device-icon">▦</span>
        <div>全部设备<small>统一查看工作轨迹</small></div></button
      >
      {#each devices as item}<button
          class:active={device === item.id}
          class="device-button"
          on:click={() => selectDevice(item.id)}
          ><span class="device-icon"
            >{item.platform === 'macos' ? '⌘' : '⊞'}</span
          >
          <div>
            {item.name}<small
              ><span
                class:online={deviceStatus(item) === '记录中'}
                class="status-dot"
              ></span>{deviceStatus(item)}</small
            >
          </div></button
        >{/each}
      {#if !devices.length}<p class="sidebar-empty">还没有设备接入</p>{/if}
      <div class="sidebar-footer">
        <span class="quiet-dot"></span>后台持续记录<button on:click={disconnect}
          >断开查看</button
        >
      </div>
    </aside>
    <main class="dashboard">
      <header class="page-header">
        <div>
          <p class="eyebrow">
            工作轨迹 / {device ? deviceName(device) : '全部设备'}
          </p>
          <h1>一天的足迹</h1>
        </div>
        <div class="header-actions">
          <span class="last-update"
            >{updated
              ? `${time(Math.floor(updated.getTime() / 1000))} 更新`
              : ''}</span
          ><button class="secondary" on:click={refresh} disabled={busy}
            >{busy ? '更新中…' : '刷新'}</button
          ><button class="secondary" on:click={exportReport}>导出记录 ↗</button>
        </div>
      </header>
      <div class="filter-bar">
        <div class="date-control">
          <button aria-label="前一天" on:click={() => moveDay(-1)}>‹</button
          ><input
            aria-label="记录日期"
            type="date"
            bind:value={date}
            on:change={refresh}
          /><button aria-label="后一天" on:click={() => moveDay(1)}>›</button
          ><button
            class="today"
            on:click={() => {
              date = localDate();
              refresh();
            }}>今天</button
          >
        </div>
        <label class="search"
          ><span aria-hidden="true">⌕</span><input
            aria-label="搜索记录"
            bind:value={search}
            on:input={changeSearch}
            placeholder="搜索应用、窗口或内容"
          /></label
        >
      </div>
      {#if error}<div class="error-banner" role="alert">
          {error}<button on:click={refresh}>重试</button>
        </div>{/if}
      <section class="metrics" aria-label="活动统计">
        <div class="metric primary-metric">
          <span>活跃时间</span><strong
            >{duration(overview.active_seconds)}</strong
          ><small>重叠时段合并计算</small>
        </div>
        <div class="metric">
          <span>设备合计</span><strong
            >{duration(overview.device_seconds)}</strong
          ><small>{chosenDevices.length} 台设备的记录时长</small>
        </div>
        <div class="metric">
          <span>应用 / 记录</span><strong
            >{overview.apps.length}<i>
              / {overview.count.toLocaleString('zh-CN')}</i
            ></strong
          ><small>{search.trim() ? '当前搜索结果' : date}</small>
        </div>
      </section>
      <section class="panel device-rhythm">
        <div class="section-header">
          <div>
            <h2>设备节奏</h2>
            <p>每台设备，沿着同一条时间轴。</p>
          </div>
          <div class="legend">
            空闲 <span></span><span></span><span></span> 活跃
          </div>
        </div>
        <div class="rhythm-scroll">
          <div class="hour-scale">
            <span></span>
            <div
              style={`grid-template-columns:repeat(${hourCount}, minmax(20px,1fr))`}
            >
              {#each Array(hourCount) as _, hour}<span
                  >{hour % 3 === 0 ? hourLabel(hour) : ''}</span
                >{/each}
            </div>
          </div>
          {#each chosenDevices as item}<div class="rhythm-row">
              <div class="rhythm-label">
                <strong>{item.name}</strong><small
                  >{duration(
                    overview.devices.find((row) => row.device_id === item.id)
                      ?.duration || 0,
                  )}</small
                >
              </div>
              <div
                class="hour-cells"
                style={`grid-template-columns:repeat(${hourCount}, minmax(20px,1fr))`}
              >
                {#each Array(hourCount) as _, hour}<div
                    class="hour-cell"
                    class:has-activity={hourSeconds(item.id, hour) > 0}
                    style={`--intensity:${0.2 + Math.min(1, hourSeconds(item.id, hour) / 3600) * 0.8}`}
                    title={`${item.name} · ${hourLabel(hour)} · ${duration(hourSeconds(item.id, hour))}`}
                  ></div>{/each}
              </div>
            </div>{/each}
        </div>
        {#if !devices.length}<div class="empty">
            启动设备上的采集程序后，工作记录会出现在这里。
          </div>{/if}
        {#each chosenDevices.filter((item) => item.last_error || item.pending > 0) as item}<p
            class="device-notice"
          >
            <strong>{item.name}</strong>{item.last_error ||
              `还有 ${item.pending} 条记录等待同步`}
          </p>{/each}
      </section>
      <div class="content-grid">
        <section class="panel timeline">
          <div class="section-header">
            <div>
              <h2>时间线</h2>
              <p>
                {search.trim()
                  ? '匹配当前搜索的记录'
                  : '从最近的活动，找回当时的上下文。'}
              </p>
            </div>
            <span class="count-label"
              >{overview.count.toLocaleString('zh-CN')} 条</span
            >
          </div>
          {#if items.length}<div class="timeline-list">
              {#each items as item (`${item.device_id}:${item.id}`)}<button
                  class="activity-row"
                  on:click={() => openDetail(item)}
                  ><div class="activity-time">
                    <strong>{time(item.timestamp)}</strong><small
                      >{duration(item.duration)}</small
                    >
                  </div>
                  <span class="app-letter"
                    >{item.app_name.slice(0, 1).toUpperCase()}</span
                  >
                  <div class="activity-copy">
                    <div>
                      <strong>{item.app_name}</strong><span class="category"
                        >{categories[item.category] || item.category}</span
                      >{#if item.screenshot}<span
                          class="image-indicator"
                          title="有截图">▧</span
                        >{/if}
                    </div>
                    <p>{item.window_title || '内容已脱敏'}</p>
                    {#if item.browser_url}<small>{item.browser_url}</small
                      >{/if}{#if item.note}<p class="note-preview">
                        ✎ {item.note}
                      </p>{/if}
                  </div>
                  <span class="activity-device"
                    >{deviceName(item.device_id)}</span
                  ></button
                >{/each}
            </div>
            {#if next}<button
                class="load-more"
                on:click={loadMore}
                disabled={busy}
                >{busy ? '正在加载…' : '加载更早的记录 ↓'}</button
              >{/if}
          {:else}<div class="empty">
              <span class="empty-mark">∿</span><strong
                >{search.trim() ? '没有匹配的记录' : '这一天还没有记录'}</strong
              >
              <p>
                {search.trim()
                  ? '换一个关键词，或清空搜索再看看。'
                  : '选择其他日期，或等待设备同步。'}
              </p>
            </div>{/if}
        </section>
        <section class="panel app-usage">
          <div class="section-header">
            <div>
              <h2>时间花在哪里</h2>
              <p>按应用汇总设备时长</p>
            </div>
          </div>
          {#each overview.apps.slice(0, 12) as app}<div class="usage-row">
              <div>
                <strong>{app.app_name}</strong><span
                  >{duration(app.duration)}</span
                >
              </div>
              <div class="usage-track">
                <span style={`width:${(app.duration / maxApp) * 100}%`}></span>
              </div>
            </div>{/each}{#if !overview.apps.length}<p class="empty compact">
              有记录后会显示应用分布。
            </p>{/if}
        </section>
      </div>
      <footer class="page-footer">
        <span>Work Review · 记录留在自己的设备与中心服务</span><span
          >{Intl.DateTimeFormat().resolvedOptions().timeZone}</span
        >
      </footer>
    </main>
  </div>
{/if}
{#if selected}
  <dialog
    use:dialog
    class="detail"
    aria-labelledby="detail-title"
    on:cancel|preventDefault={closeDetail}
  >
    <header>
      <div>
        <p class="eyebrow">
          {deviceName(selected.device_id)} · {time(selected.timestamp)} · {duration(
            selected.duration,
          )}
        </p>
        <h2 id="detail-title">{selected.app_name}</h2>
      </div>
      <button class="close" aria-label="关闭详情" on:click={closeDetail}
        >×</button
      >
    </header>
    <p class="detail-title">{selected.window_title || '内容已脱敏'}</p>
    {#if safeUrl(selected.browser_url)}<a
        class="detail-url"
        href={safeUrl(selected.browser_url)}
        target="_blank"
        rel="noreferrer">{selected.browser_url} ↗</a
      >{/if}
    {#if selected.screenshot}<div class="screenshot-preview">
        {#if imageUrl}<img src={imageUrl} alt="该时段的屏幕截图" />{:else}<p>
            {imageError || '正在加载截图…'}
          </p>{/if}
      </div>{/if}
    {#if selected.ocr_text}<details>
        <summary>识别到的文字</summary>
        <pre>{selected.ocr_text}</pre>
      </details>{/if}
    <label for="record-note">备注</label><textarea
      id="record-note"
      bind:value={note}
      maxlength="16000"
      placeholder="记下这段时间在做什么"
      rows="3"
    ></textarea>
    <div class="detail-actions">
      <button class="secondary" on:click={closeDetail}>关闭</button><button
        class="primary"
        disabled={saving}
        on:click={saveNote}>{saving ? '正在保存…' : '保存备注'}</button
      >
    </div>
  </dialog>
{/if}
