<script lang="ts">
  import Logo from './Logo.svelte';

  const columns = [
    {
      title: 'Product',
      links: [
        { label: 'Features', href: '#features' },
        { label: 'How it works', href: '#how-it-works' },
        { label: 'Architecture', href: '#architecture' },
        { label: 'Compare', href: '#compare' },
        { label: 'Download', href: '#download' }
      ]
    },
    {
      title: 'Resources',
      links: [
        { label: 'GitHub', href: 'https://github.com/verbatim-app/verbatim', external: true },
        { label: 'Releases', href: 'https://github.com/verbatim-app/verbatim/releases', external: true },
        { label: 'Issues', href: 'https://github.com/verbatim-app/verbatim/issues', external: true },
        { label: 'FAQ', href: '#faq' }
      ]
    },
    {
      title: 'Open source',
      links: [
        { label: 'MIT license', href: 'https://github.com/verbatim-app/verbatim/blob/main/LICENSE', external: true },
        { label: 'Ollama', href: 'https://ollama.com', external: true },
        { label: 'FunASR / SenseVoice', href: 'https://github.com/modelscope/FunASR', external: true },
        { label: 'RapidOCR', href: 'https://github.com/RapidAI/RapidOCR', external: true }
      ]
    }
  ];
</script>

<footer class="relative overflow-hidden border-t border-white/[0.07]">
  <div class="pointer-events-none absolute -top-32 left-1/2 h-64 w-[720px] -translate-x-1/2 rounded-full bg-signal/[0.06] blur-[100px]"></div>

  <div class="container-shell relative">
    <div class="grid items-center gap-9 border-b border-white/[0.07] py-14 sm:py-16 lg:grid-cols-[1.2fr_0.8fr] lg:gap-16">
      <div>
        <div class="flex items-center gap-2 font-mono text-[9px] uppercase tracking-[0.15em] text-signal">
          <span class="h-1.5 w-1.5 rounded-full bg-signal"></span>
          Take the cloud out
        </div>
        <h2 class="mt-4 max-w-[620px] text-[32px] font-semibold leading-[1.04] tracking-[-0.045em] text-white sm:text-[42px]">
          Your voice.<br />
          <span class="text-mint">Your machine.</span><br />
          Your words.
        </h2>
      </div>
      <div class="lg:justify-self-end">
        <div class="flex flex-wrap gap-3">
          <a
            class="button-primary"
            href="https://github.com/verbatim-app/verbatim/releases/latest"
            target="_blank"
            rel="noopener noreferrer"
          >
            <svg viewBox="0 0 20 20" class="h-4 w-4" fill="currentColor" aria-hidden="true">
              <path d="M2 3.5 8.6 2.7v6.6H2V3.5Zm7.3-.9L18 1.4v7.9h-8.7V2.6ZM2 10.1h6.6v6.6L2 15.9v-5.8Zm7.3 0H18v7.9l-8.7-1.3v-6.6Z" />
            </svg>
            Download Windows
          </a>
          <a
            class="button-secondary"
            href="https://github.com/verbatim-app/verbatim/releases/latest"
            target="_blank"
            rel="noopener noreferrer"
          >
            Linux
          </a>
        </div>
        <p class="mt-3 font-mono text-[8px] uppercase tracking-[0.1em] text-muted">
          Free forever · MIT · no account
        </p>
      </div>
    </div>

    <div class="grid gap-10 py-12 sm:grid-cols-2 lg:grid-cols-[1.4fr_1fr_1fr_1fr]">
      <div>
        <Logo />
        <p class="mt-5 max-w-[260px] text-[11px] leading-5 text-muted">
          A universal media-to-text workstation that never phones home. Dictation, notes, vision, and URLs —
          in one local app.
        </p>
        <div class="mt-5 flex items-center gap-1.5">
          <span class="rounded border border-mint/20 bg-mint/[0.04] px-2 py-1 font-mono text-[7px] uppercase tracking-[0.1em] text-mint">LOCAL</span>
          <span class="rounded border border-white/10 px-2 py-1 font-mono text-[7px] uppercase tracking-[0.1em] text-muted">FREE</span>
          <span class="rounded border border-white/10 px-2 py-1 font-mono text-[7px] uppercase tracking-[0.1em] text-muted">MIT</span>
        </div>
      </div>

      {#each columns as column}
        <div>
          <div class="font-mono text-[8px] uppercase tracking-[0.14em] text-muted">{column.title}</div>
          <nav class="mt-3" aria-label={column.title}>
            {#each column.links as link}
              <a
                class="footer-link"
                href={link.href}
                target={link.external ? '_blank' : undefined}
                rel={link.external ? 'noopener noreferrer' : undefined}>{link.label}</a
              >
            {/each}
          </nav>
        </div>
      {/each}
    </div>

    <div class="flex flex-col items-start justify-between gap-3 border-t border-white/[0.07] py-6 sm:flex-row sm:items-center">
      <div class="flex flex-wrap items-center gap-x-4 gap-y-2 font-mono text-[8px] uppercase tracking-[0.08em] text-muted">
        <span>© 2025 Verbatim</span>
        <span class="text-muted/60">/</span>
        <span>MIT license</span>
        <span class="text-muted/60">/</span>
        <span>No telemetry</span>
      </div>
      <p class="text-[10px] text-muted">Built for people who want AI without the cloud.</p>
    </div>
  </div>
</footer>
