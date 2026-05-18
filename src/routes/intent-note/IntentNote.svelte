<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { showToast } from '$lib/stores/toast.js';

  let boundary = Math.floor(Date.now() / 1000);
  let purpose = '';
  let note = '';
  let saved = [];
  let purposes = [];
  let newPurpose = '';
  let loading = true;
  let saving = false;

  const today = new Date().toISOString().slice(0, 10);

  function formatTime(ts) {
    return new Date(ts * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
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

  async function addPurpose() {
    const title = newPurpose.trim();
    if (!title) return;

    try {
      const item = await invoke('add_manual_followup', {
        input: {
          title,
          date: today,
          sourceApp: '',
          sourceTitle: '',
          projectKey: title,
        },
      });
      purposes = [item, ...purposes];
      purpose = title;
      newPurpose = '';
    } catch (error) {
      console.error('新增目的失败:', error);
      showToast(`新增目的失败: ${error}`, 'error');
    }
  }

  async function deletePurpose(item) {
    try {
      await invoke('delete_manual_followup', { id: item.id });
      purposes = purposes.filter((candidate) => candidate.id !== item.id);
      if (purpose === item.title) {
        purpose = '';
      }
    } catch (error) {
      console.error('删除目的失败:', error);
      showToast(`删除目的失败: ${error}`, 'error');
    }
  }

  async function completePurpose() {
    const trimmedPurpose = purpose.trim();
    if (!trimmedPurpose || saving) return;

    saving = true;
    try {
      const result = await invoke('save_intent_note_interval', {
        input: {
          purpose: trimmedPurpose,
          note: note.trim() || null,
          startTimestamp: boundary,
        },
      });
      saved = [
        {
          purpose: trimmedPurpose,
          note: note.trim(),
          start: result.startTimestamp,
          end: result.endTimestamp,
          annotatedSegments: result.annotatedSegments,
        },
        ...saved,
      ];
      boundary = result.endTimestamp;
      note = '';
      showToast('目的已回填到活动记录', 'success');
    } catch (error) {
      console.error('保存目的备注失败:', error);
      showToast(`保存目的备注失败: ${error}`, 'error');
    } finally {
      saving = false;
    }
  }

  onMount(() => {
    boundary = Math.floor(Date.now() / 1000);
    loadPurposes();
  });
</script>

<div class="intent-page">
  <section class="intent-header">
    <div>
      <h1>目的备注</h1>
      <p>从进入本页开始计时。点击打勾后，会把上一段时间的目的回填到真实活动记录上；未打勾离开不会写入。</p>
    </div>
    <div class="intent-boundary">
      <span>当前边界</span>
      <strong>{formatTime(boundary)}</strong>
    </div>
  </section>

  <section class="intent-panel">
    <div class="intent-current">
      <label>
        <span>当前目的</span>
        <input bind:value={purpose} placeholder="例如：看某课程第 3 节" />
      </label>
      <label>
        <span>备注，可选</span>
        <textarea bind:value={note} rows="3" placeholder="补充材料、章节、疑问或上下文"></textarea>
      </label>
      <button class="intent-complete" on:click={completePurpose} disabled={saving || !purpose.trim()}>
        {saving ? '保存中...' : '打勾并回填这段时间'}
      </button>
    </div>

    <div class="intent-library">
      <div class="intent-library-title">
        <h2>长期目的列表</h2>
        <span>{purposes.filter((item) => item.status !== 'done').length}</span>
      </div>
      <div class="intent-add-row">
        <input bind:value={newPurpose} placeholder="新增一个长期目的" on:keydown={(event) => event.key === 'Enter' && addPurpose()} />
        <button on:click={addPurpose} disabled={!newPurpose.trim()}>新增</button>
      </div>

      {#if loading}
        <p class="intent-empty">读取中...</p>
      {:else if purposes.length === 0}
        <p class="intent-empty">还没有长期目的。</p>
      {:else}
        <div class="intent-list">
          {#each purposes.filter((item) => item.status !== 'done') as item}
            <div class="intent-item">
              <button class="intent-pick" on:click={() => (purpose = item.title)}>
                <span>{item.title}</span>
                {#if item.source_title}<small>{item.source_title}</small>{/if}
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

  .intent-panel {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(280px, 0.8fr);
    gap: 16px;
    padding: 16px;
  }

  .intent-current,
  .intent-library,
  .intent-list,
  .intent-saved {
    display: grid;
    gap: 12px;
  }

  label span,
  .intent-library-title h2,
  .intent-saved h2 {
    display: block;
    margin: 0 0 6px;
    font-size: 13px;
    font-weight: 700;
  }

  input,
  textarea {
    width: 100%;
    border: 1px solid rgba(148, 163, 184, 0.28);
    border-radius: 8px;
    background: rgba(15, 23, 42, 0.9);
    color: rgb(241, 245, 249);
    padding: 10px 12px;
    outline: none;
  }

  textarea {
    resize: vertical;
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

  .intent-complete {
    min-height: 44px;
    background: rgb(16, 185, 129);
    color: white;
    font-weight: 700;
  }

  .intent-library-title,
  .intent-add-row,
  .intent-item {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .intent-library-title {
    justify-content: space-between;
  }

  .intent-add-row input {
    flex: 1;
  }

  .intent-add-row button,
  .intent-delete {
    padding: 9px 12px;
    background: rgba(59, 130, 246, 0.16);
    color: rgb(191, 219, 254);
  }

  .intent-item {
    justify-content: space-between;
    padding: 8px;
    border: 1px solid rgba(148, 163, 184, 0.16);
    border-radius: 8px;
  }

  .intent-pick {
    min-width: 0;
    flex: 1;
    background: transparent;
    color: rgb(226, 232, 240);
    text-align: left;
  }

  .intent-pick span,
  .intent-pick small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .intent-pick small {
    color: rgb(148, 163, 184);
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
