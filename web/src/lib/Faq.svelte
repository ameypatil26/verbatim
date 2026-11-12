<script lang="ts">
  const faqs = [
    {
      q: 'Is Verbatim really free?',
      a: 'Yes. The app, extension, and local services are MIT licensed. There is no paid tier, minute counter, seat count, or hosted backend. You provide the hardware and the one-time model downloads.'
    },
    {
      q: 'Does my audio or text go to the cloud?',
      a: 'No. ASR, OCR, note structuring, and chat run through localhost services and models on your machine. The only outbound connection is an Ollama endpoint if you explicitly configure a remote one.'
    },
    {
      q: 'Do I need an API key?',
      a: 'No. No OpenAI key, no Anthropic key, no account. Verbatim talks to localhost only, and model downloads are direct from the listed source after you approve them.'
    },
    {
      q: 'Which models does it use?',
      a: 'SenseVoice Small for speech, RapidOCR-ONNX for text detection and recognition, and Qwen 2.5 Coder 7B through Ollama for structuring, proofreading, and note chat. The OCR path uses PyMuPDF for PDFs and searchable text layers.'
    },
    {
      q: 'What can I feed it?',
      a: 'Microphone speech, system audio, WAV/MP3/M4A/MP4/MOV/WebM files, YouTube and TikTok URLs, screenshots, images, scanned PDFs, and QR/barcodes. The batch queue accepts up to 50 files at 2 GB each.'
    },
    {
      q: 'Can I use it fully offline?',
      a: 'After the app and models are installed, yes. ASR, OCR, note structuring, chat, and search all work without a network. Model downloads and URL imports are the only operations that need connectivity.'
    }
  ];
</script>

<section id="faq" class="section relative border-t border-white/[0.06]">
  <div class="container-shell">
    <div class="grid gap-10 lg:grid-cols-[0.72fr_1.28fr] lg:gap-20">
      <div>
        <div class="kicker">FAQ</div>
        <h2 class="section-title">Reasonable<br />questions only.</h2>
        <p class="section-copy">
          Everything runs locally, but that does not mean you have to guess what is happening. Here are the
          short answers.
        </p>
        <a
          class="mt-7 inline-flex items-center gap-2 text-[12px] font-medium text-muted transition hover:text-mint"
          href="https://github.com/verbatim-app/verbatim/issues"
          target="_blank"
          rel="noopener noreferrer"
        >
          Ask on GitHub
          <span>↗</span>
        </a>

        <div class="mt-9 hidden rounded-xl border border-white/[0.07] bg-white/[0.02] p-4 lg:block">
          <div class="font-mono text-[8px] uppercase tracking-[0.12em] text-mint/80">Privacy, concretely</div>
          <p class="mt-2 text-[10.5px] leading-5 text-muted">
            No telemetry SDK. No analytics script. No crash uploader. No API keys in the bundle. Logs stay in
            <span class="font-mono text-paper/70">~/.verbatim/logs/</span> and rotate after seven days.
          </p>
        </div>
      </div>

      <div class="border-t border-white/[0.08]">
        {#each faqs as faq, index}
          <details class="faq-item" name="verbatim-faq" open={index === 0}>
            <summary>
              <span>{faq.q}</span>
              <span class="faq-plus" aria-hidden="true"></span>
            </summary>
            <div class="faq-answer">{faq.a}</div>
          </details>
        {/each}
      </div>
    </div>
  </div>
</section>
