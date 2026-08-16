<script>
  import { toast, clearToast } from '$lib/stores/toast.js';

  const iconMap = {
    success: 'M10 2a8 8 0 1 1 0 16 8 8 0 0 1 0-16Zm0 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13Zm3.78 4.72a.75.75 0 0 1 0 1.06l-4.5 4.5a.75.75 0 0 1-1.06 0l-2-2a.75.75 0 1 1 1.06-1.06L8.75 12.19l3.97-3.97a.75.75 0 0 1 1.06 0Z',
    error: 'M10 2a8 8 0 1 1 0 16 8 8 0 0 1 0-16Zm0 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13Zm2.47 4.03a.75.75 0 0 1 0 1.06L11.06 10l1.41 1.41a.75.75 0 1 1-1.06 1.06L10 11.06l-1.41 1.41a.75.75 0 0 1-1.06-1.06L8.94 10 7.53 8.59a.75.75 0 0 1 1.06-1.06L10 8.94l1.41-1.41a.75.75 0 0 1 1.06 0Z',
    warning: 'M10 2a8 8 0 1 1 0 16 8 8 0 0 1 0-16Zm0 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM10 6a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 10 6Zm0 6.5a.88.88 0 1 1 0 1.75.88.88 0 0 1 0-1.75Z',
    info: 'M10 2a8 8 0 1 1 0 16 8 8 0 0 1 0-16Zm0 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM10 6a.88.88 0 1 1 0 1.75.88.88 0 0 1 0-1.75ZM9.25 9.5a.75.75 0 0 1 .75-.75h.5a.75.75 0 0 1 .75.75v3.75a.75.75 0 0 1-1.5 0v-3H9.25Z',
  };

  const colorMap = {
    success: 'bg-slate-800 dark:bg-slate-200 text-white dark:text-slate-800',
    error: 'bg-red-600 text-white',
    warning: 'bg-amber-500 text-white',
    info: 'bg-sky-600 text-white',
  };

  $: toastState = $toast;
  $: iconPath = iconMap[toastState?.type] || iconMap.info;
  $: toastClass = colorMap[toastState?.type] || colorMap.info;
</script>

{#if toastState}
  <div class="fixed inset-x-0 bottom-6 z-[200] flex justify-center px-4 animate-fadeIn pointer-events-none">
    <button
      type="button"
      on:click={clearToast}
      class={`max-w-full min-h-11 px-4 py-2.5 rounded-xl shadow-lg text-sm font-medium leading-none flex items-center gap-2 pointer-events-auto ${toastClass}`}
    >
      <svg class="w-4 h-4 shrink-0" fill="currentColor" viewBox="0 0 20 20">
        <path d={iconPath} />
      </svg>
      <span class="leading-none text-center">{toastState.message}</span>
    </button>
  </div>
{/if}
