<script>
  import { invoke } from '@tauri-apps/api/core';
  import { push } from 'svelte-spa-router';
  import { showToast } from '$lib/stores/toast.js';

  let purpose = '';
  let note = '';
  let saving = false;

  const today = new Date().toISOString().slice(0, 10);

  async function savePurpose() {
    const title = purpose.trim();
    if (!title || saving) return;

    saving = true;
    try {
      await invoke('add_manual_followup', {
        input: {
          title,
          note: note.trim() || null,
          date: today,
          sourceApp: '',
          sourceTitle: '',
          projectKey: title,
        },
      });
      showToast('目的已加入列表', 'success');
      push('/intent-note');
    } catch (error) {
      console.error('新增目的失败:', error);
      showToast(`新增目的失败: ${error}`, 'error');
    } finally {
      saving = false;
    }
  }
</script>

<div class="intent-new-page">
  <section class="intent-new-header">
    <button class="intent-back" on:click={() => push('/intent-note')}>返回</button>
    <div>
      <h1>添加目的</h1>
      <p>目的和备注会作为一个整体加入列表。只有在列表中打勾完成时，才会写入活动记录。</p>
    </div>
  </section>

  <section class="intent-new-panel">
    <label>
      <span>目的</span>
      <input bind:value={purpose} placeholder="例如：看某课程第 3 节" />
    </label>
    <label>
      <span>备注，可选</span>
      <textarea bind:value={note} rows="5" placeholder="补充材料、章节、疑问或上下文"></textarea>
    </label>
    <div class="intent-actions">
      <button class="intent-cancel" on:click={() => push('/intent-note')}>取消</button>
      <button class="intent-save" on:click={savePurpose} disabled={!purpose.trim() || saving}>
        {saving ? '保存中...' : '加入目的列表'}
      </button>
    </div>
  </section>
</div>

<style>
  .intent-new-page {
    min-height: 100%;
    padding: 24px;
    color: rgb(226, 232, 240);
  }

  .intent-new-header {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    margin-bottom: 16px;
  }

  .intent-new-header h1 {
    margin: 0;
    font-size: 24px;
    font-weight: 700;
  }

  .intent-new-header p {
    margin: 6px 0 0;
    color: rgb(148, 163, 184);
  }

  .intent-new-panel {
    display: grid;
    gap: 14px;
    max-width: 720px;
    padding: 16px;
    border: 1px solid rgba(148, 163, 184, 0.22);
    background: rgba(15, 23, 42, 0.72);
    border-radius: 10px;
  }

  label span {
    display: block;
    margin-bottom: 6px;
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

  .intent-back,
  .intent-cancel {
    padding: 9px 12px;
    background: rgba(148, 163, 184, 0.14);
    color: rgb(203, 213, 225);
  }

  .intent-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }

  .intent-save {
    padding: 10px 14px;
    background: rgb(59, 130, 246);
    color: white;
    font-weight: 700;
  }
</style>
