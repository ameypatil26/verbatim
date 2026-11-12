<script lang="ts">
  const steps = [
    {
      n: '01',
      title: 'Install it. Own the models.',
      copy: 'Grab the desktop app. On first launch Verbatim asks before downloading SenseVoice, RapidOCR, and Qwen 2.5 Coder. One consent, no hidden pulls.',
      code: '~/verbatim/models   ~228 MB + ~15 MB + ~4.7 GB'
    },
    {
      n: '02',
      title: 'Talk, drop, or capture.',
      copy: 'Use the global hotkey, drag in a recording, paste a URL, or draw a rectangle over anything on screen. The right local service wakes up on demand.',
      code: 'Ctrl+Shift+D   Ctrl+Shift+S   drag · drop · paste'
    },
    {
      n: '03',
      title: 'Keep the clean Markdown.',
      copy: 'Structured notes, transcripts, OCR text, and searchable PDFs land back on your disk. Chat with any note. Export it anywhere. No account required.',
      code: '~/.verbatim/notes.db   →   your-markdown.md'
    }
  ];
</script>

<section id="how-it-works" class="section relative border-t border-white/[0.06]">
  <div class="pointer-events-none absolute inset-y-0 right-0 -z-10 w-1/2 bg-[radial-gradient(circle_at_70%_45%,rgba(15,52,96,0.22),transparent_62%)]"></div>
  <div class="container-shell">
    <div class="grid gap-12 lg:grid-cols-[0.78fr_1.22fr] lg:gap-20">
      <div>
        <div class="kicker">How it works</div>
        <h2 class="section-title">Three steps.<br />Then it disappears.</h2>
        <p class="section-copy">
          Verbatim is a workstation, not a subscription. It starts when you call it and stays out of the way
          otherwise. Services load models on demand and unload them after five idle minutes.
        </p>

        <div class="mt-9 rounded-2xl border border-signal/20 bg-[linear-gradient(150deg,rgba(233,69,96,0.08),rgba(233,69,96,0.02))] p-5 sm:p-6">
          <div class="flex items-center gap-2 font-mono text-[9px] uppercase tracking-[0.15em] text-signal">
            <span class="h-1.5 w-1.5 rounded-full bg-signal"></span>
            The whole pitch
          </div>
          <p class="mt-3 text-[18px] font-medium leading-[1.45] tracking-[-0.02em] text-white">
            No cloud.<br />No API key.<br />No subscription.
            <span class="text-signal">No data leaves.</span>
          </p>
          <div class="mt-5 flex items-center gap-3 border-t border-white/[0.08] pt-4 font-mono text-[8px] uppercase tracking-[0.1em] text-muted">
            <span>Except an Ollama endpoint</span>
            <span class="text-muted/60">/</span>
            <span>if you configure one</span>
          </div>
        </div>
      </div>

      <div class="relative">
        <div class="absolute bottom-8 left-[19px] top-8 w-px bg-gradient-to-b from-mint/[0.35] via-white/10 to-transparent sm:left-[23px]"></div>
        {#each steps as step, index}
          <div class="relative flex gap-5 pb-9 last:pb-0 sm:gap-7">
            <div class="relative z-10 flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-mint/25 bg-[#111d36] font-mono text-[10px] font-semibold text-mint sm:h-12 sm:w-12">
              {step.n}
            </div>
            <div class="min-w-0 pt-0.5">
              <div class="flex items-center gap-3">
                <h3 class="text-[17px] font-semibold tracking-[-0.02em] text-white sm:text-[19px]">{step.title}</h3>
                {#if index === 0}
                  <span class="hidden rounded border border-sand/20 bg-sand/[0.05] px-1.5 py-0.5 font-mono text-[7px] uppercase tracking-[0.1em] text-sand sm:inline">explicit consent</span>
                {/if}
              </div>
              <p class="mt-2.5 max-w-[520px] text-[12.5px] leading-6 text-muted">{step.copy}</p>
              <div class="mt-4 inline-flex max-w-full items-center gap-2 overflow-x-auto rounded-lg border border-white/[0.07] bg-ink-deep/60 px-3 py-2 font-mono text-[8.5px] text-mint">
                <span class="text-muted">$</span>
                <span class="whitespace-nowrap">{step.code}</span>
              </div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </div>
</section>
