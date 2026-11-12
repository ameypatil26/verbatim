<script lang="ts">
  const rows = [
    { label: 'Price', verbatim: 'Free · MIT', other: 'Free tier + paid plans' },
    { label: 'Works offline', verbatim: 'Yes, after model download', other: 'No' },
    { label: 'Dictation into any app', verbatim: 'Global hotkey + local typing', other: 'Web / app only' },
    { label: 'OCR + searchable PDF', verbatim: 'Built in, local', other: 'Not offered' },
    { label: 'YouTube / TikTok → notes', verbatim: 'Subtitles or local ASR', other: 'Not offered' },
    { label: 'Chat with your transcript', verbatim: 'Local Ollama, transcript-only', other: 'Cloud AI Chat' },
    { label: 'Your data stays local', verbatim: 'SQLite in ~/.verbatim', other: 'Uploaded to cloud' },
    { label: 'Open source / no account', verbatim: 'MIT · anonymous', other: 'Proprietary · account' }
  ];
</script>

<section id="compare" class="section relative border-t border-white/[0.06]">
  <div class="container-shell">
    <div class="grid gap-8 lg:grid-cols-[0.8fr_1.2fr] lg:items-end">
      <div>
        <div class="kicker">Compare</div>
        <h2 class="section-title">The cloud is optional.<br /><span class="text-signal">That's the point.</span></h2>
      </div>
      <p class="section-copy mt-0 lg:justify-self-end lg:text-right">
        Otter.ai is a good meeting recorder. Verbatim is a local media workstation. One needs your account and
        your upload; the other needs your hardware.
      </p>
    </div>

    <div class="mt-10 overflow-hidden rounded-2xl border border-white/[0.09] bg-card/40 shadow-card">
      <div class="overflow-x-auto">
        <table class="w-full min-w-[640px] border-collapse text-left">
          <caption class="sr-only">
            Comparison of Verbatim and Otter.ai
          </caption>
          <thead>
            <tr class="border-b border-white/[0.08]">
              <th scope="col" class="w-[30%] px-5 py-4 font-mono text-[8px] font-medium uppercase tracking-[0.14em] text-muted sm:px-6">Capability</th>
              <th scope="col" class="w-[35%] border-x border-white/[0.06] bg-mint/[0.035] px-5 py-4 sm:px-6">
                <div class="flex items-center gap-2 text-[12px] font-semibold text-white">
                  <span class="flex h-5 w-5 items-center justify-center rounded border border-mint/25 bg-mint/[0.08] font-mono text-[7px] text-mint">V</span>
                  Verbatim
                </div>
                <div class="mt-1 font-mono text-[7px] uppercase tracking-[0.1em] text-mint/80">Local · free</div>
              </th>
              <th scope="col" class="w-[35%] px-5 py-4 sm:px-6">
                <div class="text-[12px] font-semibold text-paper/70">Otter.ai</div>
                <div class="mt-1 font-mono text-[7px] uppercase tracking-[0.1em] text-muted">Cloud · subscription</div>
              </th>
            </tr>
          </thead>
          <tbody>
            {#each rows as row, index}
              <tr class={index % 2 === 1 ? 'bg-white/[0.015]' : ''}>
                <th scope="row" class="border-b border-white/[0.055] px-5 py-4 text-[11px] font-medium text-paper/75 sm:px-6">{row.label}</th>
                <td class="border-x border-b border-white/[0.055] bg-mint/[0.025] px-5 py-4 sm:px-6">
                  <div class="flex items-center gap-2.5">
                    <span class="compare-yes">
                      <svg viewBox="0 0 14 14" class="h-3 w-3" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
                        <path d="m3 7.5 2.5 2.5L11 4" stroke-linecap="round" stroke-linejoin="round" />
                      </svg>
                    </span>
                    <span class="text-[10.5px] leading-5 text-paper/80">{row.verbatim}</span>
                  </div>
                </td>
                <td class="border-b border-white/[0.055] px-5 py-4 sm:px-6">
                  <div class="flex items-center gap-2.5">
                    <span class="compare-no">
                      <svg viewBox="0 0 14 14" class="h-3 w-3" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                        <path d="M4 7h6" stroke-linecap="round" />
                      </svg>
                    </span>
                    <span class="text-[10.5px] leading-5 text-muted">{row.other}</span>
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <div class="flex flex-col justify-between gap-2 border-t border-white/[0.07] px-5 py-3.5 font-mono text-[7.5px] uppercase tracking-[0.08em] text-muted sm:flex-row sm:items-center sm:px-6">
        <span>Publicly listed Otter.ai features at time of writing. Always verify current plans.</span>
        <a href="#download" class="text-mint/80 transition hover:text-mint">Skip the upload →</a>
      </div>
    </div>

    <div class="mt-6 grid gap-3 sm:grid-cols-3">
      {#each [
        ['Your disk', 'Transcripts, models, and FTS5 all live under ~/.verbatim.'],
        ['Your choice', 'Models unload after idle. Delete them and they are gone.'],
        ['Your terms', 'MIT license. No seat count, minute cap, or usage meter.']
      ] as item}
        <div class="rounded-xl border border-white/[0.07] bg-white/[0.02] px-4 py-3.5">
          <div class="font-mono text-[8px] uppercase tracking-[0.12em] text-mint/80">{item[0]}</div>
          <p class="mt-2 text-[10.5px] leading-5 text-muted">{item[1]}</p>
        </div>
      {/each}
    </div>
  </div>
</section>
