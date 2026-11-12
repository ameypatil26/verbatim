<script lang="ts">
  import { copyText, downloadText, formatBytes } from '../api';
  import { toast } from '../toast.svelte';
  import type { OcrResult } from '../types';

  let {
    result = null,
    title = 'OCR result',
    sourceIsPdf = false,
    onsearchable,
    searchableBusy = false
  }: {
    result?: OcrResult | null;
    title?: string;
    sourceIsPdf?: boolean;
    onsearchable?: () => void;
    searchableBusy?: boolean;
  } = $props();

  let text = $derived(result?.text ?? '');
  let words = $derived(text.trim() ? text.trim().split(/\s+/).length : 0);
  let averageConfidence = $derived(
    result?.boxes.length
      ? Math.round((result.boxes.reduce((sum, box) => sum + box.confidence, 0) / result.boxes.length) * 100)
      : 0
  );
  let baseName = $derived(
    (title.replace(/\.[^.]+$/, '') || 'verbatim-ocr')
      .replace(/[^a-z0-9-_]+/gi, '-')
      .replace(/^-+|-+$/g, '')
      .toLowerCase()
  );

  async function copy() {
    try {
      await copyText(text);
      toast('OCR text copied to the clipboard', 'success');
    } catch (error) {
      toast(`Copy failed: ${String(error)}`, 'error');
    }
  }

  function exportMd() {
    const md = `# ${title}\n\n${text}\n\n---\n\nExtracted locally with Verbatim.\n`;
    downloadText(`${baseName || 'verbatim-ocr'}.md`, md, 'text/markdown;charset=utf-8');
    toast('Markdown exported', 'success');
  }

  function exportTxt() {
    downloadText(`${baseName || 'verbatim-ocr'}.txt`, text);
    toast('Text exported', 'success');
  }
</script>

{#if result}
  <section class="ocr-result">
    <header>
      <div class="head-copy">
        <span class="eyebrow">EXTRACTED TEXT</span>
        <h3>{title}</h3>
      </div>
      <div class="badges">
        <span>{result.boxes.length} regions</span>
        {#if averageConfidence}<span>{averageConfidence}% conf.</span>{/if}
        <span>{words} words</span>
        {#if result.pageCount}<span>{result.pageCount} pages</span>{/if}
      </div>
    </header>

    {#if result.codes.length}
      <div class="codes">
        {#each result.codes as code}
          <button
            type="button"
            class="code-chip"
            title="Copy {code.type}"
            onclick={async () => {
              await copyText(code.data);
              toast(`${code.type} copied`, 'success');
            }}
          >
            <span>{code.type}</span>
            <strong>{code.data.length > 48 ? `${code.data.slice(0, 48)}…` : code.data}</strong>
          </button>
        {/each}
      </div>
    {/if}

    <textarea readonly value={text} aria-label="Extracted text" spellcheck="false"></textarea>

    <footer>
      <div class="exports">
        <button type="button" class="primary" onclick={copy}>
          <svg viewBox="0 0 18 18" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.4">
            <rect x="6" y="6" width="8" height="9" rx="1.5" />
            <path d="M4 12H3.5A1.5 1.5 0 0 1 2 10.5v-7A1.5 1.5 0 0 1 3.5 2h7A1.5 1.5 0 0 1 12 3.5V4" />
          </svg>
          Copy
        </button>
        <button type="button" onclick={exportMd}>Export MD</button>
        <button type="button" onclick={exportTxt}>Export TXT</button>
        {#if sourceIsPdf && onsearchable}
          <button type="button" class="pdf" onclick={() => onsearchable?.()} disabled={searchableBusy}>
            {#if searchableBusy}<span class="spinner"></span>{/if}
            Searchable PDF
          </button>
        {/if}
      </div>
      <span class="privacy">Processed on this machine</span>
    </footer>
  </section>
{:else}
  <div class="empty">
    <div class="scan-frame" aria-hidden="true">
      <span></span>
    </div>
    <h3>No text extracted yet</h3>
    <p>Drop an image or PDF, or capture a region with your hotkey.</p>
  </div>
{/if}

<style>
  .ocr-result {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  header {
    align-items: flex-start;
    border-bottom: 1px solid var(--border);
    display: flex;
    gap: 12px;
    justify-content: space-between;
    padding: 13px 16px 11px;
  }
  .head-copy {
    min-width: 0;
  }
  .eyebrow {
    color: var(--mint);
    font-family: var(--mono);
    font-size: 7.5px;
    letter-spacing: 0.13em;
  }
  h3 {
    color: #fff;
    font-size: 12px;
    font-weight: 600;
    margin: 3px 0 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    justify-content: flex-end;
  }
  .badges span {
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--muted);
    font-size: 8px;
    padding: 2px 5px;
    white-space: nowrap;
  }
  .codes {
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 9px 16px 0;
  }
  .code-chip {
    align-items: center;
    background: rgba(78, 204, 163, 0.07);
    border: 1px solid rgba(78, 204, 163, 0.24);
    border-radius: 5px;
    color: var(--text);
    cursor: pointer;
    display: flex;
    gap: 8px;
    overflow: hidden;
    padding: 5px 7px;
    text-align: left;
  }
  .code-chip span {
    color: var(--mint);
    flex: 0 0 auto;
    font-family: var(--mono);
    font-size: 7px;
    letter-spacing: 0.08em;
  }
  .code-chip strong {
    font-family: var(--mono);
    font-size: 8.5px;
    font-weight: 400;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  textarea {
    background: rgba(8, 13, 28, 0.35);
    border: 0;
    border-bottom: 1px solid var(--border);
    color: #d6dde6;
    flex: 1;
    font-family: var(--mono);
    font-size: 11px;
    line-height: 1.7;
    min-height: 180px;
    outline: 0;
    padding: 14px 16px;
    resize: none;
    white-space: pre-wrap;
  }
  textarea::selection {
    background: rgba(78, 204, 163, 0.28);
  }
  footer {
    align-items: center;
    display: flex;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 12px;
  }
  .exports {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  .exports button {
    align-items: center;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--text);
    cursor: pointer;
    display: flex;
    font-size: 9px;
    gap: 4px;
    padding: 5px 8px;
  }
  .exports button:hover {
    border-color: rgba(78, 204, 163, 0.42);
  }
  .exports button.primary {
    background: var(--accent);
    border-color: rgba(78, 204, 163, 0.3);
    color: var(--mint);
  }
  .exports button.pdf {
    color: #f3d9a4;
  }
  .exports button:disabled {
    cursor: wait;
    opacity: 0.55;
  }
  .privacy {
    color: var(--muted);
    font-size: 8px;
    white-space: nowrap;
  }
  .empty {
    align-items: center;
    color: var(--muted);
    display: flex;
    flex-direction: column;
    height: 100%;
    justify-content: center;
    padding: 30px;
    text-align: center;
  }
  .scan-frame {
    border: 1px solid var(--border);
    border-radius: 8px;
    height: 74px;
    margin-bottom: 17px;
    overflow: hidden;
    position: relative;
    width: 112px;
  }
  .scan-frame::before,
  .scan-frame::after {
    border: 1px solid rgba(78, 204, 163, 0.28);
    content: '';
    height: 10px;
    position: absolute;
    width: 10px;
  }
  .scan-frame::before {
    border-bottom: 0;
    border-right: 0;
    left: 8px;
    top: 8px;
  }
  .scan-frame::after {
    border-left: 0;
    border-top: 0;
    bottom: 8px;
    right: 8px;
  }
  .scan-frame span {
    animation: scan 2.8s ease-in-out infinite;
    background: linear-gradient(90deg, transparent, var(--mint), transparent);
    height: 1px;
    left: 8px;
    opacity: 0.6;
    position: absolute;
    right: 8px;
    top: 50%;
  }
  @keyframes scan {
    0%,
    100% {
      transform: translateY(-27px);
    }
    50% {
      transform: translateY(27px);
    }
  }
  .empty h3 {
    color: var(--text);
    font-size: 13px;
    margin: 0 0 5px;
  }
  .empty p {
    font-size: 10px;
    line-height: 1.55;
    margin: 0;
  }
  .spinner {
    animation: spin 0.8s linear infinite;
    border: 1px solid rgba(243, 217, 164, 0.35);
    border-radius: 50%;
    border-top-color: currentColor;
    height: 9px;
    width: 9px;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
