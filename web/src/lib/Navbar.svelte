<script lang="ts">
  import Logo from './Logo.svelte';

  let open = $state(false);

  const links = [
    { href: '#features', label: 'Features' },
    { href: '#how-it-works', label: 'How it works' },
    { href: '#architecture', label: 'Architecture' },
    { href: '#compare', label: 'Compare' },
    { href: '#faq', label: 'FAQ' }
  ];

  function close() {
    open = false;
  }
</script>

<header class="fixed inset-x-0 top-0 z-50 border-b border-white/[0.07] bg-ink/80 backdrop-blur-xl">
  <div class="container-shell flex h-[68px] items-center justify-between">
    <Logo />

    <nav class="hidden items-center gap-0.5 lg:flex" aria-label="Primary">
      {#each links as link}
        <a class="nav-link" href={link.href}>{link.label}</a>
      {/each}
    </nav>

    <div class="hidden items-center gap-2.5 lg:flex">
      <a
        class="button-quiet"
        href="https://github.com/verbatim-app/verbatim"
        target="_blank"
        rel="noopener noreferrer"
        aria-label="View Verbatim on GitHub"
      >
        <svg viewBox="0 0 20 20" class="h-4 w-4" fill="currentColor" aria-hidden="true">
          <path d="M10 1.6a8.4 8.4 0 0 0-2.65 16.37c.42.08.57-.18.57-.4v-1.43c-2.34.51-2.84-1-2.84-1-.38-.98-.92-1.24-.92-1.24-.76-.52.06-.51.06-.51.84.06 1.28.87 1.28.87.75 1.29 1.96.92 2.44.7.08-.54.29-.92.53-1.13-1.87-.21-3.84-.94-3.84-4.16 0-.92.33-1.67.87-2.26-.09-.21-.38-1.07.08-2.23 0 0 .7-.23 2.3.86A8 8 0 0 1 10 5.63a8 8 0 0 1 2.09.28c1.59-1.09 2.3-.86 2.3-.86.46 1.16.17 2.02.08 2.23.54.59.87 1.34.87 2.26 0 3.23-1.97 3.94-3.84 4.15.3.26.57.77.57 1.56v2.3c0 .22.15.49.58.4A8.4 8.4 0 0 0 10 1.6Z" />
        </svg>
        GitHub
      </a>
      <a class="button-primary py-2.5 text-[12px]" href="#download">
        Download
        <svg viewBox="0 0 16 16" class="h-3.5 w-3.5" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
          <path d="M8 2v8m0 0 3-3m-3 3L5 7M3 12v1.5h10V12" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </a>
    </div>

    <button
      type="button"
      class="flex h-9 w-9 items-center justify-center rounded-lg border border-white/10 text-muted transition hover:border-mint/30 hover:text-paper lg:hidden"
      aria-label={open ? 'Close navigation' : 'Open navigation'}
      aria-expanded={open}
      onclick={() => (open = !open)}
    >
      <svg viewBox="0 0 20 20" class="h-4 w-4" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
        {#if open}
          <path d="m5 5 10 10M15 5 5 15" stroke-linecap="round" />
        {:else}
          <path d="M4 6h12M4 10h12M4 14h12" stroke-linecap="round" />
        {/if}
      </svg>
    </button>
  </div>

  {#if open}
    <div class="border-t border-white/[0.07] bg-ink/95 px-5 py-4 backdrop-blur-xl lg:hidden">
      <nav class="flex flex-col" aria-label="Mobile">
        {#each links as link}
          <a
            class="rounded-lg px-3 py-3 text-[13px] font-medium text-muted transition hover:bg-white/[0.04] hover:text-paper"
            href={link.href}
            onclick={close}>{link.label}</a
          >
        {/each}
      </nav>
      <div class="mt-3 flex gap-2 border-t border-white/[0.07] pt-4">
        <a class="button-secondary flex-1 py-2.5 text-[12px]" href="https://github.com/verbatim-app/verbatim" target="_blank" rel="noopener noreferrer">GitHub</a>
        <a class="button-primary flex-1 py-2.5 text-[12px]" href="#download" onclick={close}>Download</a>
      </div>
    </div>
  {/if}
</header>
