<script>
  export let title = '';
  export let value = '';
  export let icon = '';
  export let color = 'indigo';
  export let href = null;
  export let embedded = false;

  const iconColors = {
    indigo: 'text-indigo-500 dark:text-indigo-400',
    emerald: 'text-emerald-500 dark:text-emerald-400',
    amber: 'text-amber-500 dark:text-amber-400',
    rose: 'text-rose-500 dark:text-rose-400',
    blue: 'text-blue-500 dark:text-blue-400',
    violet: 'text-violet-500 dark:text-violet-400',
  };

  const iconBgs = {
    indigo: 'bg-indigo-50 dark:bg-indigo-950/40',
    emerald: 'bg-emerald-50 dark:bg-emerald-950/40',
    amber: 'bg-amber-50 dark:bg-amber-950/40',
    rose: 'bg-rose-50 dark:bg-rose-950/40',
    blue: 'bg-blue-50 dark:bg-blue-950/40',
    violet: 'bg-violet-50 dark:bg-violet-950/40',
  };

  $: cardClass = embedded
    ? 'block min-h-[108px] rounded-[22px] bg-slate-50/88 px-4 py-4 dark:bg-slate-900/30'
    : 'block min-h-[116px] rounded-2xl border border-slate-100 bg-white p-5 dark:border-slate-700/60 dark:bg-slate-800/80';
  $: iconClass = embedded
    ? `flex h-10 w-10 items-center justify-center rounded-2xl ${iconBgs[color]} ${iconColors[color]} shrink-0`
    : `flex h-11 w-11 items-center justify-center rounded-2xl border border-white/80 shadow-sm dark:border-slate-700/80 ${iconBgs[color]} ${iconColors[color]} shrink-0`;
  $: valueClass = embedded
    ? 'mt-4 text-[1.9rem] font-semibold tracking-tight leading-none text-slate-800 dark:text-white'
    : 'mt-6 text-[2.15rem] font-semibold tracking-tight leading-none text-slate-800 dark:text-white';
  $: interactiveClass = embedded
    ? 'transition-all duration-200 hover:bg-slate-100/90 hover:-translate-y-0.5 dark:hover:bg-slate-900/50'
    : 'transition-all duration-200 hover:shadow-md hover:-translate-y-0.5 cursor-pointer group';
</script>

{#if href}
  <a
    {href}
    class="{cardClass} {interactiveClass}"
  >
    <div class="flex h-full items-center justify-between gap-4">
      <div class="min-w-0 flex-1">
        <span class="text-[13px] font-medium text-slate-400 dark:text-slate-500 leading-none">{title}</span>
        <p class={valueClass}>{value}</p>
      </div>
      <div class={iconClass}>
        {#if icon === 'duration'}
          <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
            <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8Zm.5-13a.75.75 0 0 0-.75.75v4.5c0 .2.08.39.22.53l3 3a.75.75 0 0 0 1.06-1.06L13.25 11.94V7.75A.75.75 0 0 0 12.5 7Z" />
          </svg>
        {:else if icon === 'focus'}
          <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
            <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8Zm0-13a5 5 0 1 0 0 10 5 5 0 0 0 0-10Zm0 8.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Zm0-5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3Z" />
          </svg>
        {:else if icon === 'browser'}
          <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
            <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm7.93 9h-3.47a15.7 15.7 0 0 0-1.42-5.06A8.02 8.02 0 0 1 19.93 11ZM12 3.5c.87 1.4 1.6 3.75 1.95 6.5H10.05c.35-2.75 1.08-5.1 1.95-6.5ZM8.96 5.94A15.7 15.7 0 0 0 7.54 11H4.07a8.02 8.02 0 0 1 4.89-5.06ZM4.07 13h3.47c.3 1.9.8 3.72 1.42 5.06A8.02 8.02 0 0 1 4.07 13Zm7.93 7.5c-.87-1.4-1.6-3.75-1.95-6.5h3.9c-.35 2.75-1.08 5.1-1.95 6.5Zm3.04-2.44c.62-1.34 1.12-3.16 1.42-5.06h3.47a8.02 8.02 0 0 1-4.89 5.06Z" />
          </svg>
        {:else if icon === 'apps'}
          <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
            <path d="M4 4.5A2.5 2.5 0 0 1 6.5 2h2A2.5 2.5 0 0 1 11 4.5v2A2.5 2.5 0 0 1 8.5 9h-2A2.5 2.5 0 0 1 4 6.5v-2ZM6.5 3.5a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h2a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1h-2Zm6.5 1A2.5 2.5 0 0 1 15.5 2h2A2.5 2.5 0 0 1 20 4.5v2A2.5 2.5 0 0 1 17.5 9h-2A2.5 2.5 0 0 1 13 6.5v-2ZM15.5 3.5a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h2a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1h-2ZM4 15.5A2.5 2.5 0 0 1 6.5 13h2a2.5 2.5 0 0 1 2.5 2.5v2a2.5 2.5 0 0 1-2.5 2.5h-2A2.5 2.5 0 0 1 4 17.5v-2Zm2.5-1a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h2a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1h-2Zm6.5 1a2.5 2.5 0 0 1 2.5-2.5h2a2.5 2.5 0 0 1 2.5 2.5v2a2.5 2.5 0 0 1-2.5 2.5h-2a2.5 2.5 0 0 1-2.5-2.5v-2Zm2.5-1a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h2a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1h-2Z" />
          </svg>
        {:else}
          {icon}
        {/if}
      </div>
    </div>
  </a>
{:else}
  <div class={cardClass}>
    <div class="flex h-full items-center justify-between gap-4">
      <div class="min-w-0 flex-1">
        <span class="text-[13px] font-medium text-slate-400 dark:text-slate-500 leading-none">{title}</span>
        <p class={valueClass}>{value}</p>
      </div>
      <div class={iconClass}>
        {#if icon === 'duration'}
          <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
            <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8Zm.5-13a.75.75 0 0 0-.75.75v4.5c0 .2.08.39.22.53l3 3a.75.75 0 0 0 1.06-1.06L13.25 11.94V7.75A.75.75 0 0 0 12.5 7Z" />
          </svg>
        {:else if icon === 'focus'}
          <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
            <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8Zm0-13a5 5 0 1 0 0 10 5 5 0 0 0 0-10Zm0 8.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Zm0-5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3Z" />
          </svg>
        {:else if icon === 'browser'}
          <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
            <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm7.93 9h-3.47a15.7 15.7 0 0 0-1.42-5.06A8.02 8.02 0 0 1 19.93 11ZM12 3.5c.87 1.4 1.6 3.75 1.95 6.5H10.05c.35-2.75 1.08-5.1 1.95-6.5ZM8.96 5.94A15.7 15.7 0 0 0 7.54 11H4.07a8.02 8.02 0 0 1 4.89-5.06ZM4.07 13h3.47c.3 1.9.8 3.72 1.42 5.06A8.02 8.02 0 0 1 4.07 13Zm7.93 7.5c-.87-1.4-1.6-3.75-1.95-6.5h3.9c-.35 2.75-1.08 5.1-1.95 6.5Zm3.04-2.44c.62-1.34 1.12-3.16 1.42-5.06h3.47a8.02 8.02 0 0 1-4.89 5.06Z" />
          </svg>
        {:else if icon === 'apps'}
          <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
            <path d="M4 4.5A2.5 2.5 0 0 1 6.5 2h2A2.5 2.5 0 0 1 11 4.5v2A2.5 2.5 0 0 1 8.5 9h-2A2.5 2.5 0 0 1 4 6.5v-2ZM6.5 3.5a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h2a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1h-2Zm6.5 1A2.5 2.5 0 0 1 15.5 2h2A2.5 2.5 0 0 1 20 4.5v2A2.5 2.5 0 0 1 17.5 9h-2A2.5 2.5 0 0 1 13 6.5v-2ZM15.5 3.5a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h2a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1h-2ZM4 15.5A2.5 2.5 0 0 1 6.5 13h2a2.5 2.5 0 0 1 2.5 2.5v2a2.5 2.5 0 0 1-2.5 2.5h-2A2.5 2.5 0 0 1 4 17.5v-2Zm2.5-1a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h2a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1h-2Zm6.5 1a2.5 2.5 0 0 1 2.5-2.5h2a2.5 2.5 0 0 1 2.5 2.5v2a2.5 2.5 0 0 1-2.5 2.5h-2a2.5 2.5 0 0 1-2.5-2.5v-2Zm2.5-1a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h2a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1h-2Z" />
          </svg>
        {:else}
          {icon}
        {/if}
      </div>
    </div>
  </div>
{/if}
