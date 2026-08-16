<script>
  import { confirmDialog, resolveConfirm } from '$lib/stores/confirm.js';

  const toneMap = {
    info: {
      iconBg: 'bg-blue-50 text-blue-600 dark:bg-blue-950/50 dark:text-blue-300',
      button: 'bg-indigo-500 hover:bg-indigo-600 text-white',
      path: 'M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8Zm0-13a1 1 0 1 0 0 2 1 1 0 0 0 0-2Zm-1 5a.75.75 0 0 1 .75-.75h.5a.75.75 0 0 1 .75.75v4.5a.75.75 0 0 1-1.5 0V13H11a.75.75 0 0 1-.75-.75Z',
    },
    warning: {
      iconBg: 'bg-amber-50 text-amber-600 dark:bg-amber-950/50 dark:text-amber-300',
      button: 'bg-amber-500 hover:bg-amber-600 text-white',
      path: 'M12.98 2.37a1.75 1.75 0 0 0-2.96 0l-8.5 14.5A1.75 1.75 0 0 0 3 19.5h17a1.75 1.75 0 0 0 1.48-2.63l-8.5-14.5Zm-1.68 1.25a.25.25 0 0 1 .4 0l8.5 14.5a.25.25 0 0 1-.2.38H3a.25.25 0 0 1-.21-.38l8.5-14.5ZM12 8a.75.75 0 0 0-.75.75v4.5a.75.75 0 0 0 1.5 0v-4.5A.75.75 0 0 0 12 8Zm0 8a1 1 0 1 0 0 2 1 1 0 0 0 0-2Z',
    },
    error: {
      iconBg: 'bg-red-50 text-red-600 dark:bg-red-950/50 dark:text-red-300',
      button: 'bg-red-500 hover:bg-red-600 text-white',
      path: 'M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8Zm3.28-11.28a.75.75 0 0 0-1.06-1.06L12 9.94 9.78 7.72a.75.75 0 0 0-1.06 1.06L10.94 11l-2.22 2.22a.75.75 0 1 0 1.06 1.06L12 12.06l2.22 2.22a.75.75 0 0 0 1.06-1.06L13.06 11l2.22-2.28Z',
    },
  };

  $: dialogState = $confirmDialog;
  $: tone = toneMap[dialogState?.tone] || toneMap.info;

  function handleKeydown(event) {
    if (event.key === 'Escape') {
      resolveConfirm(false);
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if dialogState}
  <div class="fixed inset-0 z-[200] flex items-center justify-center px-4 py-6 bg-slate-950/48 backdrop-blur-md animate-fadeIn">
    <div
      class="w-full max-w-md rounded-3xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 shadow-2xl shadow-slate-950/24 dark:shadow-black/50 p-6"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirm-dialog-title"
    >
      <div class="flex items-start gap-4">
        <div class={`flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl ${tone.iconBg}`}>
          <svg class="h-6 w-6" fill="currentColor" viewBox="0 0 24 24">
            <path d={tone.path} />
          </svg>
        </div>
        <div class="min-w-0 flex-1">
          <h3 id="confirm-dialog-title" class="text-lg font-semibold tracking-tight text-slate-800 dark:text-white">
            {dialogState.title}
          </h3>
          <p class="mt-2 text-sm leading-6 text-slate-500 dark:text-slate-400 whitespace-pre-line">
            {dialogState.message}
          </p>
        </div>
      </div>

      <div class="mt-6 flex items-center justify-end gap-3">
        <button
          type="button"
          on:click={() => resolveConfirm(false)}
          class="inline-flex min-h-11 items-center justify-center rounded-2xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-5 text-sm font-medium text-slate-600 dark:text-slate-300 transition-colors hover:bg-slate-50 dark:hover:bg-slate-700"
        >
          {dialogState.cancelText}
        </button>
        <button
          type="button"
          on:click={() => resolveConfirm(true)}
          class={`inline-flex min-h-11 items-center justify-center rounded-2xl px-5 text-sm font-medium transition-colors ${tone.button}`}
        >
          {dialogState.confirmText}
        </button>
      </div>
    </div>
  </div>
{/if}
