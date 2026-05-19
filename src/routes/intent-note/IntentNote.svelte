<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { push } from 'svelte-spa-router';
  import { showToast } from '$lib/stores/toast.js';
  import { advanceIntentBoundary, enterIntentSession } from '$lib/stores/intentSession.js';

  let boundary = enterIntentSession();
  let saved = [];
  let purposes = [];
  let loading = true;
  let completingId = null;

  function formatTime(ts) {
    return new Date(ts * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  function updateBoundary(ts) {
    boundary = advanceIntentBoundary(ts);
  }

  async function loadPurposes() {
    loading = true;
    try {
      purposes = await invoke('get_manual_followups', { dateFrom: null, dateTo: null });
    } catch (error) {
      console.error('读取目的列表失败:', error);
      showToast(`读取目的列表失败: ${error}`, 'error');
    } finally {
      loading = false;
    }
  }

  async function deletePurpose(item) {
    try {
      await invoke('delete_manual_followup', { id: item.id });
      purposes = purposes.filter((candidate) => candidate.id !== item.id);
    } catch (error) {
      console.error('删除目的失败:', error);
      showToast(`删除目的失败: ${error}`, 'error');
    }
  }

  async function completePurpose(item) {
    if (!item?.title?.trim() || completingId) return;

    completingId = item.id;
    try {
      const result = await invoke('save_intent_note_interval', {
        input: {
          purpose: item.title.trim(),
          note: item.note?.trim() || null,
          startTimestamp: boundary,
        },
      });
      await invoke('update_manual_followup_status', { id: item.id, status: 'done' });
      saved = [
        {
          purpose: item.title.trim(),
          note: item.note?.trim() || '',
          start: result.startTimestamp,
          end: result.endTimestamp,
          annotatedSegments: result.annotatedSegments,
        },
        ...saved,
      ];
      updateBoundary(result.endTimestamp);
      purposes = purposes.filter((candidate) => candidate.id !== item.id);
      showToast('目的已回填到活动记录', 'success');
    } catch (error) {
      console.error('保存目的备注失败:', error);
      showToast(`保存目的备注失败: ${error}`, 'error');
    } finally {
      completingId = null;
    }
  }

  onMount(() => {
    boundary = enterIntentSession();
    loadPurposes();
  });
</script>

<div class="intent-page">
  <section class="intent-header">
    <div>
      <h1>目的备注</h1>
      <p>进入本页开始计时。点击队列里的完成后，会把上一段时间的目的回填到真实活动记录上；离开后未完成的时间边界会丢弃。</p>
    </div>
    <div class="intent-boundary">
      <span>当前边界</span>
      <strong>{formatTime(boundary)}</strong>
    </div>
  </section>

  <section class="intent-panel">
    <div class="intent-library">
      <div class="intent-library-title">
        <h2>长期目的列表</h2>
        <span>{purposes.filter((item) => item.status !== 'done').length}</span>
      </div>
      <button class="intent-add-link" on:click={() => push('/intent-note/new')}>添加目的</button>

      {#if loading}
        <p class="intent-empty">读取中...</p>
      {:else if purposes.length === 0}
        <p class="intent-empty">还没有长期目的。</p>
      {:else}
        <div class="intent-list">
          {#each purposes.filter((item) => item.status !== 'done') as item}
            <div class="intent-item">
              <div class="intent-item-main">
                <span>{item.title}</span>
                {#if item.note}<small>{item.note}</small>{/if}
              </div>
              <button class="intent-done" on:click={() => completePurpose(item)} disabled={completingId === item.id}>
                {completingId === item.id ? '保存中' : '完成'}
              </button>
              <button class="intent-delete" on:click={() => deletePurpose(item)}>删除</button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </section>

  {#if saved.length > 0}
    <section class="intent-saved">
      <h2>本次已完成</h2>
      {#each saved as item}
        <div class="intent-saved-item">
          <strong>{item.purpose}</strong>
          <span>{formatTime(item.start)} - {formatTime(item.end)} · {item.annotatedSegments} 段活动</span>
          {#if item.note}<p>{item.note}</p>{/if}
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  .intent-page {
    min-height: 100%;
    padding: 24px;
    color: rgb(226, 232, 240);
  }

  .intent-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 16px;
  }

  .intent-header h1 {
    margin: 0;
    font-size: 24px;
    font-weight: 700;
  }

  .intent-header p,
  .intent-empty,
  .intent-saved-item span,
  .intent-saved-item p {
    color: rgb(148, 163, 184);
  }

  .intent-boundary,
  .intent-panel,
  .intent-saved {
    border: 1px solid rgba(148, 163, 184, 0.22);
    background: rgba(15, 23, 42, 0.72);
    border-radius: 10px;
  }

  .intent-boundary {
    min-width: 132px;
    padding: 12px;
    text-align: right;
  }

  .intent-boundary span {
    display: block;
    font-size: 12px;
    color: rgb(148, 163, 184);
  }

  .intent-boundary strong {
    font-size: 20px;
  }

  .intent-panel { padding: 16px; }

  .intent-library,
  .intent-list,
  .intent-saved {
    display: grid;
    gap: 12px;
  }

  .intent-library-title h2,
  .intent-saved h2 {
    display: block;
    margin: 0 0 6px;
    font-size: 13px;
    font-weight: 700;
  }

  button {
    border: 0;
    border-radius: 8px;
    cursor: pointer;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .intent-library-title,
  .intent-item {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .intent-library-title {
    justify-content: space-between;
  }

  .intent-add-link {
    justify-self: start;
    padding: 10px 14px;
    background: rgb(59, 130, 246);
    color: white;
    font-weight: 700;
  }

  .intent-item {
    justify-content: space-between;
    padding: 8px;
    border: 1px solid rgba(148, 163, 184, 0.16);
    border-radius: 8px;
  }

  .intent-item-main {
    min-width: 0;
    flex: 1;
    color: rgb(226, 232, 240);
  }

  .intent-item-main span,
  .intent-item-main small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .intent-item-main small {
    color: rgb(148, 163, 184);
  }

  .intent-done,
  .intent-delete {
    padding: 9px 12px;
  }

  .intent-done {
    background: rgba(16, 185, 129, 0.16);
    color: rgb(110, 231, 183);
  }

  .intent-delete {
    background: rgba(244, 63, 94, 0.14);
    color: rgb(253, 164, 175);
  }

  .intent-saved {
    margin-top: 16px;
    padding: 16px;
  }

  .intent-saved-item {
    padding: 10px 0;
    border-top: 1px solid rgba(148, 163, 184, 0.16);
  }

  @media (max-width: 860px) {
    .intent-panel,
    .intent-header {
      grid-template-columns: 1fr;
      display: grid;
    }
  }
</style>
