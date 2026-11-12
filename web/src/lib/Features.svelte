<script lang="ts">
  const features = [
    {
      n: '01',
      title: 'Dictation that types anywhere',
      copy: 'Press Ctrl+Shift+D, speak, and Verbatim types into whatever app has focus. Microphone or system audio. Local proofreading, optional.',
      tags: ['cpal', 'SenseVoice', 'enigo'],
      icon: 'mic',
      accent: 'mint'
    },
    {
      n: '02',
      title: 'Any recording becomes a note',
      copy: 'Drop audio, video, or a YouTube/TikTok URL. Get a clean Markdown note with key points, detailed notes, action items, and inline timestamps.',
      tags: ['yt-dlp', 'Ollama', 'Markdown'],
      icon: 'note',
      accent: 'sand'
    },
    {
      n: '03',
      title: 'OCR without the upload',
      copy: 'Screenshot a region or batch 50 files. Ignore watermarks with masks, decode QR codes, and export searchable PDFs with a hidden text layer.',
      tags: ['RapidOCR', 'PyMuPDF', 'pyzbar'],
      icon: 'scan',
      accent: 'signal'
    },
    {
      n: '04',
      title: 'Paste a URL, keep the knowledge',
      copy: 'Subtitles first, ASR when they are missing. Turn talks, tutorials, and interviews into searchable notes and chat with the transcript.',
      tags: ['subtitles', 'ASR fallback', 'chat'],
      icon: 'link',
      accent: 'mint'
    }
  ];
</script>

<section id="features" class="section relative border-t border-white/[0.06]">
  <div class="container-shell">
    <div class="flex flex-col justify-between gap-8 sm:flex-row sm:items-end">
      <div>
        <div class="kicker">What it does</div>
        <h2 class="section-title">Four front doors.<br />One local core.</h2>
      </div>
      <p class="section-copy mt-0 max-w-[380px] sm:text-right">
        Same engine behind every surface. Nothing is uploaded, queued, or metered.
      </p>
    </div>

    <div class="mt-12 grid gap-4 sm:grid-cols-2 lg:mt-14">
      {#each features as feature}
        <article class="feature-card group">
          <div class="flex items-start justify-between">
            <div class="icon-box">
              {#if feature.icon === 'mic'}
                <svg viewBox="0 0 24 24" class="h-5 w-5" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                  <rect x="9" y="3" width="6" height="11" rx="3" />
                  <path d="M5.5 11.5a6.5 6.5 0 0 0 13 0M12 18v3m-4 0h8" stroke-linecap="round" />
                </svg>
              {:else if feature.icon === 'note'}
                <svg viewBox="0 0 24 24" class="h-5 w-5" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                  <path d="M6 3.5h8.5L19 8v12.5H6V3.5Z" stroke-linejoin="round" />
                  <path d="M14 3.5V8h5M9 12h7M9 15.5h7M9 8h2" stroke-linecap="round" />
                </svg>
              {:else if feature.icon === 'scan'}
                <svg viewBox="0 0 24 24" class="h-5 w-5" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                  <path d="M4 8V5.5A1.5 1.5 0 0 1 5.5 4H8m8 0h2.5A1.5 1.5 0 0 1 20 5.5V8m0 8v2.5a1.5 1.5 0 0 1-1.5 1.5H16M8 20H5.5A1.5 1.5 0 0 1 4 18.5V16m3-4h10" stroke-linecap="round" />
                </svg>
              {:else}
                <svg viewBox="0 0 24 24" class="h-5 w-5" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                  <path d="M9.5 14.5a3.5 3.5 0 0 0 5 0l3-3a3.5 3.5 0 0 0-5-5l-1 1m-2 6.5a3.5 3.5 0 0 0-5 0l-3 3a3.5 3.5 0 0 0 5 5l1-1" stroke-linecap="round" />
                </svg>
              {/if}
            </div>
            <span class="font-mono text-[9px] uppercase tracking-[0.16em] text-muted">{feature.n}</span>
          </div>

          <h3 class="mt-6 text-[19px] font-semibold tracking-[-0.02em] text-white">{feature.title}</h3>
          <p class="mt-3 max-w-[440px] text-[12.5px] leading-6 text-muted">{feature.copy}</p>

          <div class="mt-6 flex flex-wrap gap-1.5">
            {#each feature.tags as tag}
              <span class="mini-tag">{tag}</span>
            {/each}
          </div>

          <div class="mt-7 h-px w-full bg-gradient-to-r from-white/[0.08] to-transparent"></div>
          <div class="mt-4 flex items-center justify-between">
            <span class="font-mono text-[8px] uppercase tracking-[0.12em] text-muted">Runs on-device</span>
            <span class="text-mint/80 transition group-hover:translate-x-1 group-hover:text-mint">→</span>
          </div>
        </article>
      {/each}
    </div>

    <div class="mt-6 flex flex-wrap items-center justify-between gap-4 rounded-xl border border-white/[0.07] bg-white/[0.02] px-5 py-4">
      <div class="flex flex-wrap items-center gap-x-5 gap-y-2 font-mono text-[9px] uppercase tracking-[0.1em] text-muted">
        <span class="text-mint/80">Also included</span>
        <span>Global hotkeys</span>
        <span class="text-muted/60">/</span>
        <span>Batch queue</span>
        <span class="text-muted/60">/</span>
        <span>Markdown chat</span>
        <span class="text-muted/60">/</span>
        <span>FTS5 search</span>
      </div>
      <a href="#architecture" class="text-[11px] font-medium text-muted transition hover:text-mint">See the architecture →</a>
    </div>
  </div>
</section>
