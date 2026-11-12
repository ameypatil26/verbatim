<script lang="ts">
  const interfaces = [
    {
      name: 'Desktop app',
      detail: 'Tauri 2 · Svelte 5',
      icon: 'window'
    },
    {
      name: 'Browser extension',
      detail: 'Chrome / Edge MV3',
      icon: 'browser'
    },
    {
      name: 'Local clients',
      detail: 'HTTP / scripts / local tools',
      icon: 'terminal'
    }
  ];

  const services = [
    { name: 'ASR', port: '8001', engine: 'SenseVoice', routes: '/health · /transcribe · /unload' },
    { name: 'OCR', port: '8002', engine: 'RapidOCR + PyMuPDF', routes: '/ocr · /ocr-pdf · /pdf-searchable' },
    { name: 'OLLAMA', port: '11434', engine: 'Qwen 2.5 Coder 7B', routes: '/api/generate · /api/chat' }
  ];
</script>

<section id="architecture" class="section relative border-t border-white/[0.06]">
  <div class="container-shell">
    <div class="mx-auto max-w-3xl text-center">
      <div class="kicker justify-center before:hidden">Architecture</div>
      <h2 class="section-title mx-auto">One loopback.<br /><span class="text-mint">Zero cloud.</span></h2>
      <p class="section-copy mx-auto">
        Every interface speaks the same boring thing: HTTP to 127.0.0.1. No auth tokens, no queues, no
        account server. The desktop app supervises both Python services and shuts them down on exit.
      </p>
    </div>

    <div class="relative mt-12 overflow-hidden rounded-3xl border border-white/[0.09] bg-[linear-gradient(160deg,rgba(22,33,62,0.68),rgba(10,16,32,0.82))] p-5 shadow-card sm:p-8 lg:p-10">
      <div class="hero-grid pointer-events-none absolute inset-0 opacity-50"></div>
      <div class="pointer-events-none absolute left-1/2 top-0 h-56 w-[500px] -translate-x-1/2 rounded-full bg-accent/40 blur-[100px]"></div>

      <div class="relative">
        <div class="mx-auto mb-3 flex max-w-[240px] items-center justify-center gap-2 font-mono text-[8px] uppercase tracking-[0.17em] text-muted">
          <span class="h-px w-7 bg-white/[0.15]"></span>
          three front doors
          <span class="h-px w-7 bg-white/[0.15]"></span>
        </div>

        <div class="grid gap-3 sm:grid-cols-3">
          {#each interfaces as item}
            <div class="group flex items-center gap-3 rounded-xl border border-white/[0.09] bg-white/[0.035] px-4 py-3.5 transition hover:border-mint/30">
              <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-mint/20 bg-accent/50 text-mint">
                {#if item.icon === 'window'}
                  <svg viewBox="0 0 20 20" class="h-4 w-4" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true">
                    <rect x="2.5" y="3.5" width="15" height="12" rx="1.5" />
                    <path d="M2.5 7h15M5.5 9.5h4" stroke-linecap="round" />
                  </svg>
                {:else if item.icon === 'browser'}
                  <svg viewBox="0 0 20 20" class="h-4 w-4" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true">
                    <circle cx="10" cy="10" r="7" />
                    <path d="M3 10h14M10 3a11 11 0 0 1 0 14M10 3a11 11 0 0 0 0 14" />
                  </svg>
                {:else}
                  <svg viewBox="0 0 20 20" class="h-4 w-4" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true">
                    <rect x="2.5" y="3.5" width="15" height="12" rx="1.5" />
                    <path d="m5.5 7.5 2 2-2 2m4 0h3" stroke-linecap="round" stroke-linejoin="round" />
                  </svg>
                {/if}
              </span>
              <div class="min-w-0">
                <div class="text-[11px] font-semibold text-white">{item.name}</div>
                <div class="mt-0.5 font-mono text-[7px] uppercase tracking-[0.08em] text-muted">{item.detail}</div>
              </div>
            </div>
          {/each}
        </div>

        <div class="relative flex h-16 items-center justify-center">
          <div class="absolute inset-y-0 left-1/2 w-px bg-gradient-to-b from-transparent via-mint/40 to-mint/60"></div>
          <div class="relative z-10 flex items-center gap-2 rounded-full border border-mint/25 bg-ink px-3 py-1.5 font-mono text-[7px] uppercase tracking-[0.12em] text-mint">
            <span class="h-1.5 w-1.5 rounded-full bg-mint shadow-[0_0_6px_rgba(78,204,163,0.8)]"></span>
            HTTP · 127.0.0.1 only
          </div>
        </div>

        <div class="grid gap-3 lg:grid-cols-3">
          {#each services as service}
            <div class="relative rounded-xl border border-white/[0.09] bg-ink-deep/70 p-4">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <span class="flex h-6 w-6 items-center justify-center rounded border border-mint/20 bg-mint/[0.05] font-mono text-[7px] font-bold text-mint">
                    {service.name.slice(0, 2)}
                  </span>
                  <span class="text-[11px] font-semibold text-white">{service.name}</span>
                </div>
                <span class="rounded border border-white/10 px-1.5 py-0.5 font-mono text-[7px] text-muted">:{service.port}</span>
              </div>
              <div class="mt-4 text-[10px] text-paper/70">{service.engine}</div>
              <div class="mt-2 font-mono text-[7.5px] leading-5 text-muted">{service.routes}</div>
              <div class="mt-4 flex items-center gap-1.5 border-t border-white/[0.06] pt-3 font-mono text-[6.5px] uppercase tracking-[0.08em] text-muted">
                <span class="h-1 w-1 rounded-full bg-mint/70"></span>
                loopback · no auth
              </div>
            </div>
          {/each}
        </div>

        <div class="relative flex h-16 items-center justify-center">
          <div class="absolute inset-y-0 left-1/2 w-px bg-gradient-to-b from-mint/50 via-white/20 to-white/10"></div>
          <div class="relative z-10 rounded-full border border-white/10 bg-ink px-3 py-1.5 font-mono text-[7px] uppercase tracking-[0.12em] text-muted">
            local state
          </div>
        </div>

        <div class="mx-auto flex max-w-2xl flex-col items-center justify-between gap-3 rounded-xl border border-sand/[0.15] bg-sand/[0.035] px-5 py-4 sm:flex-row">
          <div class="flex items-center gap-3">
            <span class="flex h-8 w-8 items-center justify-center rounded-lg border border-sand/20 bg-sand/[0.05] text-sand">
              <svg viewBox="0 0 20 20" class="h-4 w-4" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true">
                <ellipse cx="10" cy="5" rx="6" ry="2.5" />
                <path d="M4 5v10c0 1.4 2.7 2.5 6 2.5s6-1.1 6-2.5V5M4 10c0 1.4 2.7 2.5 6 2.5s6-1.1 6-2.5" />
              </svg>
            </span>
            <div>
              <div class="text-[11px] font-semibold text-white">SQLite + FTS5</div>
              <div class="mt-0.5 font-mono text-[7.5px] text-muted">~/.verbatim/notes.db</div>
            </div>
          </div>
          <div class="flex flex-wrap items-center gap-1.5 font-mono text-[6.5px] uppercase tracking-[0.1em] text-muted">
            <span class="rounded border border-white/10 px-2 py-1">notes</span>
            <span class="rounded border border-white/10 px-2 py-1">messages</span>
            <span class="rounded border border-white/10 px-2 py-1">settings</span>
            <span class="rounded border border-white/10 px-2 py-1">fts5</span>
          </div>
        </div>
      </div>
    </div>

    <div class="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      {#each [
        ['Model lease', 'ASR and OCR never hold both models at once.'],
        ['Idle unload', 'Models free RAM after 5 minutes by default.'],
        ['Crash recovery', 'Up to 3 automatic restarts, then clear logs.'],
        ['No egress', 'Only downloads you approve and your Ollama URL.']
      ] as item}
        <div class="rounded-xl border border-white/[0.07] bg-white/[0.02] px-4 py-3">
          <div class="flex items-center gap-2 text-[10px] font-semibold text-paper">
            <span class="h-1 w-1 rounded-full bg-mint"></span>
            {item[0]}
          </div>
          <p class="mt-1.5 text-[10.5px] leading-5 text-muted">{item[1]}</p>
        </div>
      {/each}
    </div>

    <div class="mt-5 flex flex-col items-center justify-between gap-3 rounded-xl border border-white/[0.07] bg-[#0c1425]/60 px-5 py-4 font-mono text-[8px] uppercase tracking-[0.09em] text-muted sm:flex-row">
      <span>desktop supervisor → spawn services → health check → kill on exit</span>
      <span class="text-mint/80">no cloud in the diagram</span>
    </div>
  </div>
</section>
