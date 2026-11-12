<script lang="ts">
  const bars = Array.from({ length: 48 }, (_, index) => {
    const wave = Math.abs(Math.sin(index * 0.73)) * 0.55 + Math.abs(Math.sin(index * 0.23 + 1.2)) * 0.32;
    return Math.round(9 + wave * 44);
  });
</script>

<section id="top" class="relative overflow-hidden pb-20 pt-[122px] sm:pb-24 sm:pt-[138px] lg:pb-28">
  <div class="hero-grid absolute inset-0 -z-10"></div>
  <div class="pointer-events-none absolute left-1/2 top-[-260px] -z-10 h-[560px] w-[900px] -translate-x-1/2 rounded-full bg-accent/30 blur-[130px]"></div>
  <div class="noise pointer-events-none absolute inset-0 -z-10"></div>

  <div class="container-shell">
    <div class="grid items-center gap-14 lg:grid-cols-[1.02fr_0.98fr] lg:gap-12">
      <div class="relative z-10">
        <div class="pill mb-6">
          <span class="h-1.5 w-1.5 rounded-full bg-mint shadow-[0_0_7px_rgba(78,204,163,0.7)]"></span>
          Local-first · Open source · MIT
        </div>

        <h1 class="max-w-[640px] text-[42px] font-semibold leading-[0.98] tracking-[-0.055em] text-white sm:text-[58px] lg:text-[66px]">
          Verbatim —
          <span class="mt-1 block bg-gradient-to-r from-mint via-[#9ce8cf] to-[#d8fff1] bg-clip-text text-transparent">
            Every word,
          </span>
          <span class="block text-paper/95">on your machine.</span>
        </h1>

        <p class="mt-6 max-w-[520px] text-[16px] leading-7 text-muted sm:text-[17px]">
          Any media → clean text. <span class="text-paper">All local. All free.</span>
          Dictation, notes, OCR, searchable PDFs, and URL transcripts without a cloud in the loop.
        </p>

        <div id="download" class="mt-8 flex flex-wrap items-center gap-3">
          <a
            class="button-primary"
            href="https://github.com/verbatim-app/verbatim/releases/latest"
            target="_blank"
            rel="noopener noreferrer"
          >
            <svg viewBox="0 0 20 20" class="h-4 w-4" fill="currentColor" aria-hidden="true">
              <path d="M2 3.5 8.6 2.7v6.6H2V3.5Zm7.3-.9L18 1.4v7.9h-8.7V2.6ZM2 10.1h6.6v6.6L2 15.9v-5.8Zm7.3 0H18v7.9l-8.7-1.3v-6.6Z" />
            </svg>
            Download for Windows
          </a>
          <a
            class="button-secondary"
            href="https://github.com/verbatim-app/verbatim/releases/latest"
            target="_blank"
            rel="noopener noreferrer"
          >
            <svg viewBox="0 0 20 20" class="h-4 w-4" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true">
              <rect x="2.5" y="4" width="15" height="11" rx="1.5" />
              <path d="M7 17h6M10 15v2M5.5 7.5h3M5.5 10.5h5.5" stroke-linecap="round" />
            </svg>
            Linux
          </a>
          <a
            class="group inline-flex items-center gap-2 px-2 py-3 text-[12px] font-medium text-muted transition hover:text-paper"
            href="https://github.com/verbatim-app/verbatim"
            target="_blank"
            rel="noopener noreferrer"
          >
            <svg viewBox="0 0 20 20" class="h-4 w-4" fill="currentColor" aria-hidden="true">
              <path d="M10 1.6a8.4 8.4 0 0 0-2.65 16.37c.42.08.57-.18.57-.4v-1.43c-2.34.51-2.84-1-2.84-1-.38-.98-.92-1.24-.92-1.24-.76-.52.06-.51.06-.51.84.06 1.28.87 1.28.87.75 1.29 1.96.92 2.44.7.08-.54.29-.92.53-1.13-1.87-.21-3.84-.94-3.84-4.16 0-.92.33-1.67.87-2.26-.09-.21-.38-1.07.08-2.23 0 0 .7-.23 2.3.86A8 8 0 0 1 10 5.63a8 8 0 0 1 2.09.28c1.59-1.09 2.3-.86 2.3-.86.46 1.16.17 2.02.08 2.23.54.59.87 1.34.87 2.26 0 3.23-1.97 3.94-3.84 4.15.3.26.57.77.57 1.56v2.3c0 .22.15.49.58.4A8.4 8.4 0 0 0 10 1.6Z" />
            </svg>
            View on GitHub
            <span class="transition group-hover:translate-x-0.5">→</span>
          </a>
        </div>

        <div class="mt-7 flex flex-wrap items-center gap-x-5 gap-y-2 font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
          <span class="flex items-center gap-1.5"><span class="text-mint">✓</span> No account</span>
          <span class="flex items-center gap-1.5"><span class="text-mint">✓</span> No API key</span>
          <span class="flex items-center gap-1.5"><span class="text-mint">✓</span> No telemetry</span>
        </div>
      </div>

      <div class="relative mx-auto w-full max-w-[520px] lg:mx-0">
        <div class="absolute -inset-6 -z-10 rounded-[36px] bg-mint/[0.045] blur-3xl"></div>

        <div class="app-window relative">
          <div class="flex items-center gap-2 border-b border-white/[0.08] bg-white/[0.025] px-4 py-3">
            <span class="window-dot bg-signal/70"></span>
            <span class="window-dot bg-sand/50"></span>
            <span class="window-dot bg-mint/60"></span>
            <div class="ml-3 flex items-center gap-1.5">
              <span class="text-[8px] font-bold tracking-[0.18em] text-white/80">VERBATIM</span>
              <span class="rounded border border-mint/25 px-1.5 py-0.5 font-mono text-[6px] uppercase tracking-[0.12em] text-mint">LOCAL</span>
            </div>
            <span class="ml-auto font-mono text-[7px] text-muted">1000 × 700</span>
          </div>

          <div class="flex gap-1 border-b border-white/[0.06] px-4 py-2">
            <span class="rounded border-b border-mint px-2 py-1 text-[8px] font-medium text-white">Dictation</span>
            <span class="px-2 py-1 text-[8px] text-muted">Notes</span>
            <span class="px-2 py-1 text-[8px] text-muted">Vision</span>
            <span class="px-2 py-1 text-[8px] text-muted">Settings</span>
            <span class="ml-auto flex items-center gap-1.5 font-mono text-[7px] text-mint"><span class="h-1 w-1 rounded-full bg-mint"></span>ALL LOCAL</span>
          </div>

          <div class="relative overflow-hidden px-5 pb-5 pt-4">
            <div class="pointer-events-none absolute inset-x-0 top-0 h-20 bg-gradient-to-b from-mint/[0.04] to-transparent"></div>
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2 font-mono text-[7px] uppercase tracking-[0.12em] text-muted">
                <span class="h-1.5 w-1.5 animate-pulseSoft rounded-full bg-mint"></span>
                Recording
              </div>
              <span class="font-mono text-[8px] text-mint">00:14</span>
            </div>

            <div class="relative mt-5 flex h-[92px] items-center justify-center gap-[3px] overflow-hidden rounded-xl border border-white/[0.06] bg-ink-deep/60 px-4">
              {#each bars as height, index}
                <span
                  class="wave-bar"
                  style={`height:${height}px; animation-delay:-${(index % 17) * 0.08}s; opacity:${0.35 + (height / 90) * 0.65}`}
                ></span>
              {/each}
              <div class="scan-mask pointer-events-none absolute inset-x-4 top-0 h-px"></div>
            </div>

            <div class="mt-5 rounded-xl border border-white/[0.07] bg-white/[0.025] p-4">
              <div class="flex items-center justify-between">
                <span class="font-mono text-[7px] uppercase tracking-[0.13em] text-muted">Transcript</span>
                <span class="font-mono text-[7px] text-mint">typed → focused app</span>
              </div>
              <p class="mt-2 text-[10px] leading-[1.7] text-paper/80">
                “Turn this recording into a clean note. We can keep the action items and preserve the
                <span class="text-mint">[00:14]</span> timestamps.”
                <span class="ml-0.5 inline-block h-3 w-[1px] animate-pulseSoft bg-mint align-middle"></span>
              </p>
            </div>

            <div class="mt-3 flex items-center gap-1.5">
              <span class="mini-tag">SenseVoice</span>
              <span class="mini-tag">Qwen 2.5</span>
              <span class="mini-tag">localhost</span>
              <span class="ml-auto font-mono text-[7px] text-muted">0 bytes uploaded</span>
            </div>
          </div>
        </div>

        <div class="absolute -right-2 -top-4 hidden rounded-xl border border-mint/20 bg-[#101b33]/95 px-3 py-2 shadow-card backdrop-blur sm:block">
          <div class="flex items-center gap-2">
            <span class="flex h-6 w-6 items-center justify-center rounded-md bg-mint/10 text-[10px] text-mint">⌘</span>
            <div>
              <div class="font-mono text-[7px] uppercase tracking-[0.1em] text-muted">Global hotkey</div>
              <div class="font-mono text-[9px] text-paper">Ctrl+Shift+D</div>
            </div>
          </div>
        </div>

        <div class="absolute -bottom-5 -left-3 hidden rounded-xl border border-white/10 bg-[#101b33]/95 px-3.5 py-2.5 shadow-card backdrop-blur sm:block">
          <div class="flex items-center gap-2">
            <span class="h-1.5 w-1.5 rounded-full bg-mint shadow-[0_0_7px_rgba(78,204,163,0.8)]"></span>
            <div>
              <div class="font-mono text-[7px] uppercase tracking-[0.1em] text-muted">Services</div>
              <div class="font-mono text-[8px] text-paper">ASR 8001 · OCR 8002 · Ollama 11434</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="mt-16 flex flex-wrap items-center justify-between gap-5 border-t border-white/[0.07] pt-6 font-mono text-[9px] uppercase tracking-[0.12em] text-muted sm:mt-20">
      <span>One binary. Local services. Zero cloud.</span>
      <div class="flex flex-wrap items-center gap-x-5 gap-y-2">
        <span>Windows</span><span class="text-muted/60">/</span>
        <span>Linux</span><span class="text-muted/60">/</span>
        <span>macOS</span><span class="text-muted/60">/</span>
        <span>Chrome MV3</span>
      </div>
    </div>
  </div>
</section>
