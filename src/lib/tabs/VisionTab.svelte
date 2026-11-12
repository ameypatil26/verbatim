<script lang="ts">
  import { onMount } from 'svelte';
  import type { AppSettings, Mask, OcrResult } from '../types';
  import {
    captureRegionOcr,
    copyText,
    downloadBlob,
    downloadText,
    formatBytes,
    isTauri,
    ocrFile,
    ocrPath,
    ocrPdf,
    ocrPdfPath,
    readImagePreview,
    searchablePdf,
    searchablePdfPath
  } from '../api';
  import { toast } from '../toast.svelte';
  import DropZone from '../components/DropZone.svelte';
  import ImageMasks from '../components/ImageMasks.svelte';
  import OcrResultPanel from '../components/OcrResult.svelte';

  type ItemKind = 'image' | 'pdf' | 'capture';
  type ItemStatus = 'pending' | 'processing' | 'done' | 'error';

  interface QueueItem {
    id: string;
    name: string;
    size: number;
    kind: ItemKind;
    file?: File;
    path?: string;
    preview?: string;
    status: ItemStatus;
    result?: OcrResult;
    error?: string;
    masks: Mask[];
  }

  let {
    settings,
    readOnly = false,
    onmodels
  }: {
    settings: AppSettings;
    readOnly?: boolean;
    onmodels?: () => void;
  } = $props();

  let items = $state<QueueItem[]>([]);
  let selectedId = $state('');
  let masks = $state<Mask[]>([]);
  let processing = $state(false);
  let searchableBusy = $state(false);
  let lastSelected = '';

  let selected = $derived(items.find((item) => item.id === selectedId) ?? null);
  let completed = $derived(items.filter((item) => item.status === 'done' || item.status === 'error').length);
  let progress = $derived(items.length ? Math.round((completed / items.length) * 100) : 0);
  let hasResults = $derived(items.some((item) => item.result?.text));

  $effect(() => {
    if (selectedId !== lastSelected) {
      masks = selected?.masks ? [...selected.masks] : [];
      lastSelected = selectedId;
    }
  });

  onMount(() => {
    return () => {
      for (const item of items) {
        if (item.preview?.startsWith('blob:')) URL.revokeObjectURL(item.preview);
      }
    };
  });

  function newId(): string {
    return typeof crypto !== 'undefined' && 'randomUUID' in crypto
      ? crypto.randomUUID()
      : `ocr-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  }

  function itemKind(file: File): ItemKind {
    return file.type === 'application/pdf' || /\.pdf$/i.test(file.name) ? 'pdf' : 'image';
  }

  function pathKind(path: string): ItemKind {
    return /\.pdf$/i.test(path) ? 'pdf' : 'image';
  }

  async function addFiles(files: File[]) {
    if (readOnly) {
      toast('Download the local models before running OCR.', 'error');
      onmodels?.();
      return;
    }
    const room = 50 - items.length;
    if (room <= 0) {
      toast('A batch is limited to 50 files.', 'error');
      return;
    }
    const accepted: QueueItem[] = [];
    for (const file of files.slice(0, room)) {
      if (file.size > 2 * 1024 * 1024 * 1024) {
        toast(`${file.name} exceeds the 2 GB limit.`, 'error');
        continue;
      }
      const kind = itemKind(file);
      accepted.push({
        id: newId(),
        name: file.name,
        size: file.size,
        kind,
        file,
        preview: kind === 'image' ? URL.createObjectURL(file) : undefined,
        status: 'pending',
        masks: []
      });
    }
    if (files.length > room) toast(`Only ${room} more file${room === 1 ? '' : 's'} fit in this batch.`, 'info');
    if (!accepted.length) return;
    items = [...items, ...accepted];
    if (!selectedId) selectedId = accepted[0].id;
    void processQueue();
  }

  async function addPaths(paths: string[]) {
    if (readOnly) {
      toast('Download the local models before running OCR.', 'error');
      onmodels?.();
      return;
    }
    const room = 50 - items.length;
    if (room <= 0) {
      toast('A batch is limited to 50 files.', 'error');
      return;
    }
    const accepted: QueueItem[] = [];
    for (const path of paths.slice(0, room)) {
      const name = path.split(/[\\/]/).pop() || path;
      const kind = pathKind(path);
      let preview: string | undefined;
      if (kind === 'image' && isTauri) {
        try {
          preview = await readImagePreview(path);
        } catch {
          preview = undefined;
        }
      }
      accepted.push({
        id: newId(),
        name,
        size: 0,
        kind,
        path,
        preview,
        status: 'pending',
        masks: []
      });
    }
    if (!accepted.length) return;
    items = [...items, ...accepted];
    if (!selectedId) selectedId = accepted[0].id;
    void processQueue();
  }

  async function processQueue() {
    if (processing) return;
    processing = true;
    try {
      while (true) {
        const next = items.find((item) => item.status === 'pending');
        if (!next) break;
        await runItem(next);
      }
    } finally {
      processing = false;
    }
  }

  async function runItem(item: QueueItem) {
    item.status = 'processing';
    item.error = undefined;
    try {
      let result: OcrResult;
      if (item.kind === 'pdf') {
        result = item.path
          ? await ocrPdfPath(item.path, settings, item.masks)
          : await ocrPdf(item.file!, settings, item.masks);
      } else {
        result = item.path
          ? await ocrPath(item.path, settings, item.masks)
          : await ocrFile(item.file!, settings, item.masks);
      }
      item.result = result;
      item.status = 'done';
      if (!selectedId) selectedId = item.id;
      if (item.id === selectedId) {
        masks = [...item.masks];
      }
    } catch (problem) {
      item.status = 'error';
      item.error = String(problem);
      toast(`OCR failed for ${item.name}: ${item.error}`, 'error', 6500);
    }
  }

  async function rerunSelected() {
    if (!selected) return;
    if (selected.kind === 'capture' && !selected.file && !selected.path) {
      toast('Re-capture the region to change ignore masks.', 'info');
      return;
    }
    selected.masks = [...masks];
    selected.result = undefined;
    selected.status = 'pending';
    await runItem(selected);
  }

  function selectItem(item: QueueItem) {
    selectedId = item.id;
    masks = [...item.masks];
  }

  function removeItem(id: string) {
    const item = items.find((entry) => entry.id === id);
    if (item?.preview?.startsWith('blob:')) URL.revokeObjectURL(item.preview);
    items = items.filter((entry) => entry.id !== id);
    if (selectedId === id) selectedId = items[0]?.id ?? '';
  }

  function clearAll() {
    if (!items.length) return;
    if (!confirm('Clear this OCR batch? Extracted text that was not exported will be lost.')) return;
    for (const item of items) {
      if (item.preview?.startsWith('blob:')) URL.revokeObjectURL(item.preview);
    }
    items = [];
    selectedId = '';
    masks = [];
  }

  function exportCsv() {
    const rows = [['filename', 'text', 'regions', 'qr_codes']];
    for (const item of items) {
      if (!item.result) continue;
      rows.push([
        item.name,
        item.result.text,
        String(item.result.boxes.length),
        String(item.result.codes.length)
      ]);
    }
    if (rows.length === 1) {
      toast('Run OCR before exporting.', 'info');
      return;
    }
    const csv = rows
      .map((row) => row.map((cell) => `"${cell.replace(/"/g, '""')}"`).join(','))
      .join('\n');
    downloadText('verbatim-ocr-batch.csv', csv, 'text/csv;charset=utf-8');
    toast('Batch CSV exported', 'success');
  }

  async function capture() {
    if (readOnly) {
      toast('Download the local models before capturing.', 'error');
      onmodels?.();
      return;
    }
    try {
      const result = await captureRegionOcr(settings);
      const item: QueueItem = {
        id: newId(),
        name: `Screen capture ${new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`,
        size: 0,
        kind: 'capture',
        preview: result.imageDataUrl,
        status: 'done',
        result: {
          text: result.text,
          boxes: result.boxes,
          codes: result.codes
        },
        masks: []
      };
      items = [item, ...items];
      selectedId = item.id;
      if (!isTauri && result.text) {
        await copyText(result.text);
      }
      toast('Region captured and OCR’d locally', 'success');
    } catch (problem) {
      toast(String(problem), 'error', 7000);
    }
  }

  async function makeSearchable() {
    if (!selected || selected.kind !== 'pdf') return;
    searchableBusy = true;
    try {
      const baseName = selected.name.replace(/\.pdf$/i, '') || 'document';
      if (selected.path) {
        const base64 = await searchablePdfPath(selected.path);
        const binary = atob(base64);
        const bytes = new Uint8Array(binary.length);
        for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);
        downloadBlob(new Blob([bytes], { type: 'application/pdf' }), `${baseName}-searchable.pdf`);
      } else if (selected.file) {
        const pdf = await searchablePdf(selected.file, settings);
        downloadBlob(pdf, `${baseName}-searchable.pdf`);
      }
      toast('Searchable PDF generated — the text layer is hidden under the page image', 'success', 4500);
    } catch (problem) {
      toast(`Searchable PDF failed: ${String(problem)}`, 'error', 7000);
    } finally {
      searchableBusy = false;
    }
  }

  async function copySelected() {
    if (!selected?.result?.text) return;
    await copyText(selected.result.text);
    toast('OCR text copied', 'success');
  }
</script>

<div class="vision">
  <aside class="queue-panel">
    <header>
      <div>
        <span class="eyebrow">VISION · OCR</span>
        <h2>Batch queue</h2>
      </div>
      {#if items.length}<span class="count">{items.length}/50</span>{/if}
    </header>

    <button class="capture-button" type="button" onclick={() => void capture()} disabled={readOnly}>
      <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M3 7V5a2 2 0 0 1 2-2h2m6 0h2a2 2 0 0 1 2 2v2m0 6v2a2 2 0 0 1-2 2h-2M7 17H5a2 2 0 0 1-2-2v-2" stroke-linecap="round" />
        <circle cx="10" cy="10" r="2.4" />
      </svg>
      Capture region
      <kbd>{settings.hotkeyOcr}</kbd>
    </button>

    <div class="drop">
      <DropZone
        compact
        accept="image/*,.pdf,application/pdf"
        multiple
        label={readOnly ? 'Models required for OCR' : 'Drop images or PDFs'}
        hint=""
        disabled={readOnly}
        onfiles={(files) => void addFiles(files)}
        onpaths={(paths) => void addPaths(paths)}
      />
    </div>

    <div class="queue-list">
      {#each items as item (item.id)}
        <div
          class="queue-item"
          class:selected={selectedId === item.id}
          role="button"
          tabindex="0"
          onclick={() => selectItem(item)}
          onkeydown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              selectItem(item);
            }
          }}
        >
          <span class="type-icon {item.kind}">
            {#if item.kind === 'pdf'}PDF{:else}IMG{/if}
          </span>
          <span class="item-copy">
            <strong>{item.name}</strong>
            <span>
              {#if item.size}{formatBytes(item.size)} · {/if}
              {#if item.status === 'processing'}OCR running…
              {:else if item.status === 'pending'}Waiting
              {:else if item.status === 'error'}Failed
              {:else if item.result}{item.result.boxes.length} regions · {item.result.codes.length} codes
              {:else}Ready{/if}
            </span>
          </span>
          <span class="status-dot {item.status}"></span>
          <button
            class="remove"
            type="button"
            title="Remove"
            aria-label="Remove {item.name}"
            onclick={(event) => {
              event.stopPropagation();
              removeItem(item.id);
            }}>×</button
          >
        </div>
      {:else}
        <div class="queue-empty">
          <p>No files in this batch.</p>
          <span>Images and scanned PDFs are processed one at a time locally.</span>
        </div>
      {/each}
    </div>

    <footer>
      <button type="button" onclick={exportCsv} disabled={!hasResults}>Export CSV</button>
      <button type="button" class="ghost" onclick={clearAll} disabled={!items.length}>Clear</button>
    </footer>
  </aside>

  <section class="main">
    <header class="toolbar">
      <div class="toolbar-copy">
        <span class="eyebrow">RESULT</span>
        <h1>{selected?.name ?? 'Local OCR workspace'}</h1>
      </div>
      <div class="toolbar-actions">
        {#if processing}
          <span class="batch-progress"><span class="spinner"></span>{completed + 1} of {items.length}</span>
        {:else if items.length}
          <span class="batch-progress">{completed} of {items.length} processed</span>
        {/if}
        <button type="button" onclick={() => void copySelected()} disabled={!selected?.result?.text}>
          Copy
        </button>
        <button type="button" onclick={exportCsv} disabled={!hasResults}>CSV</button>
      </div>
    </header>

    {#if items.length}
      <div class="progress-line">
        <span style:width={`${progress}%`} class:active={processing}></span>
      </div>
    {/if}

    <div class="workspace-view">
      {#if selected}
        <div class="result-column">
          {#if selected.preview}
            <div class="preview-block">
              <ImageMasks src={selected.preview} bind:masks onchange={(value) => {
                if (selected) {
                  selected.masks = [...value];
                  masks = [...value];
                }
              }} />
              <div class="preview-actions">
                <span>Excluded regions are sent to the OCR service as masks.</span>
                <button type="button" onclick={() => void rerunSelected()} disabled={selected.status === 'processing'}>
                  Re-run OCR with masks
                </button>
              </div>
            </div>
          {/if}

          {#if selected.status === 'processing'}
            <div class="processing-state">
              <div class="scan-loader"><span></span></div>
              <h3>Reading pixels locally</h3>
              <p>RapidOCR is scanning {selected.name}. No image leaves this machine.</p>
            </div>
          {:else if selected.status === 'error'}
            <div class="error-state">
              <h3>OCR failed</h3>
              <p>{selected.error}</p>
              <button type="button" onclick={() => void rerunSelected()}>Try again</button>
            </div>
          {:else if selected.result}
            <OcrResultPanel
              result={selected.result}
              title={selected.name}
              sourceIsPdf={selected.kind === 'pdf'}
              searchableBusy={searchableBusy}
              onsearchable={() => void makeSearchable()}
            />
          {:else}
            <div class="pending-state">
              <h3>Waiting in the queue</h3>
              <p>This file will run automatically, one file at a time.</p>
            </div>
          {/if}
        </div>
      {:else}
        <div class="empty-state">
          <div class="scan-art"><span></span></div>
          <h2>Turn any image into text</h2>
          <p>Drop screenshots, scans, or PDFs. Draw red ignore regions to skip watermarks and headers.</p>
          <div class="empty-tips">
            <span><kbd>{settings.hotkeyOcr}</kbd> capture a region</span>
            <span>QR + barcode detection</span>
            <span>Searchable PDF export</span>
          </div>
        </div>
      {/if}
    </div>
  </section>
</div>

<style>
  .vision {
    display: flex;
    height: 100%;
    min-height: 0;
  }
  .queue-panel {
    background: rgba(22, 33, 62, 0.42);
    border-right: 1px solid var(--border);
    display: flex;
    flex: 0 0 260px;
    flex-direction: column;
    min-height: 0;
  }
  .queue-panel header {
    align-items: flex-start;
    display: flex;
    justify-content: space-between;
    padding: 15px 13px 9px;
  }
  .queue-panel h2 {
    color: #fff;
    font-size: 14px;
    font-weight: 600;
    margin: 3px 0 0;
  }
  .count {
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 7px;
    padding: 2px 4px;
  }
  .capture-button {
    align-items: center;
    background: linear-gradient(180deg, rgba(15, 52, 96, 0.75), rgba(13, 38, 70, 0.7));
    border: 1px solid rgba(78, 204, 163, 0.22);
    border-radius: 6px;
    color: var(--mint);
    cursor: pointer;
    display: flex;
    font-size: 9px;
    gap: 6px;
    height: 34px;
    margin: 0 11px;
    padding: 0 9px;
  }
  .capture-button:hover:not(:disabled) {
    border-color: rgba(78, 204, 163, 0.45);
  }
  .capture-button:disabled {
    cursor: default;
    opacity: 0.4;
  }
  .capture-button kbd {
    margin-left: auto;
  }
  kbd {
    background: rgba(8, 13, 28, 0.45);
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 6.5px;
    padding: 2px 4px;
  }
  .drop {
    padding: 8px 11px 5px;
  }
  .drop :global(.dropzone) {
    border-style: solid;
    min-height: 44px;
  }
  .queue-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 3px 7px 8px;
  }
  .queue-item {
    align-items: center;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--text);
    cursor: pointer;
    display: flex;
    gap: 8px;
    margin-bottom: 2px;
    padding: 7px 7px;
    position: relative;
    text-align: left;
    width: 100%;
  }
  .queue-item:hover {
    background: rgba(15, 52, 96, 0.22);
  }
  .queue-item.selected {
    background: rgba(15, 52, 96, 0.5);
    border-color: rgba(78, 204, 163, 0.2);
  }
  .type-icon {
    align-items: center;
    background: rgba(15, 52, 96, 0.45);
    border: 1px solid var(--border-soft);
    border-radius: 4px;
    color: var(--mint);
    display: flex;
    flex: 0 0 28px;
    font-family: var(--mono);
    font-size: 6.5px;
    height: 26px;
    justify-content: center;
    letter-spacing: 0.04em;
  }
  .type-icon.pdf {
    color: var(--signal);
  }
  .item-copy {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .item-copy strong {
    font-size: 9.5px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .item-copy span {
    color: var(--muted);
    font-size: 7.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status-dot {
    border-radius: 50%;
    flex: 0 0 auto;
    height: 4px;
    width: 4px;
  }
  .status-dot.done {
    background: var(--mint);
  }
  .status-dot.processing {
    animation: blink 1s infinite;
    background: var(--sand);
  }
  .status-dot.error {
    background: var(--signal);
  }
  .status-dot.pending {
    background: var(--muted-low);
  }
  @keyframes blink {
    50% {
      opacity: 0.25;
    }
  }
  .remove {
    background: transparent;
    border: 0;
    color: var(--muted-low);
    display: none;
    font-size: 13px;
    padding: 0 2px;
  }
  .queue-item:hover .remove {
    display: block;
  }
  .remove:hover {
    color: var(--signal);
  }
  .queue-empty {
    color: var(--muted);
    font-size: 8.5px;
    line-height: 1.6;
    padding: 20px 9px;
    text-align: center;
  }
  .queue-empty p {
    color: var(--text);
    font-size: 9.5px;
    margin: 0 0 4px;
  }
  .queue-panel footer {
    border-top: 1px solid var(--border-soft);
    display: flex;
    gap: 5px;
    padding: 8px 11px;
  }
  .queue-panel footer button {
    background: var(--accent);
    border: 1px solid rgba(78, 204, 163, 0.18);
    border-radius: 4px;
    color: var(--mint);
    cursor: pointer;
    flex: 1;
    font-size: 8px;
    padding: 5px;
  }
  .queue-panel footer button.ghost {
    background: transparent;
    border-color: var(--border);
    color: var(--muted);
  }
  .queue-panel footer button:disabled {
    cursor: default;
    opacity: 0.35;
  }
  .main {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }
  .toolbar {
    align-items: center;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex: 0 0 54px;
    justify-content: space-between;
    padding: 0 16px;
  }
  .toolbar h1 {
    color: #fff;
    font-size: 12px;
    font-weight: 600;
    margin: 3px 0 0;
    max-width: 380px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .toolbar-actions {
    align-items: center;
    display: flex;
    gap: 5px;
  }
  .batch-progress {
    align-items: center;
    color: var(--muted);
    display: flex;
    font-family: var(--mono);
    font-size: 7px;
    gap: 5px;
    margin-right: 4px;
  }
  .spinner {
    animation: spin 0.8s linear infinite;
    border: 1px solid rgba(78, 204, 163, 0.22);
    border-radius: 50%;
    border-top-color: var(--mint);
    height: 8px;
    width: 8px;
  }
  .toolbar-actions button {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--muted);
    cursor: pointer;
    font-size: 8px;
    padding: 4px 7px;
  }
  .toolbar-actions button:hover:not(:disabled) {
    border-color: rgba(78, 204, 163, 0.35);
    color: var(--text);
  }
  .toolbar-actions button:disabled {
    cursor: default;
    opacity: 0.35;
  }
  .progress-line {
    background: rgba(234, 234, 234, 0.04);
    flex: 0 0 2px;
  }
  .progress-line span {
    background: var(--mint);
    display: block;
    height: 100%;
    transition: width 300ms ease;
  }
  .workspace-view {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .result-column {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow-y: auto;
  }
  .preview-block {
    border-bottom: 1px solid var(--border-soft);
    flex: 0 0 auto;
  }
  .preview-actions {
    align-items: center;
    color: var(--muted);
    display: flex;
    font-size: 8px;
    gap: 10px;
    justify-content: space-between;
    padding: 7px 12px;
  }
  .preview-actions button {
    background: transparent;
    border: 1px solid rgba(78, 204, 163, 0.24);
    border-radius: 4px;
    color: var(--mint);
    cursor: pointer;
    font-size: 7.5px;
    padding: 3px 7px;
  }
  .preview-actions button:disabled {
    opacity: 0.4;
  }
  .result-column :global(.ocr-result) {
    flex: 1;
    min-height: 320px;
  }
  .processing-state,
  .pending-state,
  .error-state {
    align-items: center;
    color: var(--muted);
    display: flex;
    flex-direction: column;
    flex: 1;
    justify-content: center;
    min-height: 300px;
    padding: 30px;
    text-align: center;
  }
  .processing-state h3,
  .pending-state h3,
  .error-state h3 {
    color: var(--text);
    font-size: 13px;
    margin: 15px 0 5px;
  }
  .processing-state p,
  .pending-state p,
  .error-state p {
    font-size: 9px;
    line-height: 1.6;
    margin: 0;
    max-width: 330px;
  }
  .error-state p {
    color: #e9a1ad;
  }
  .error-state button {
    background: var(--accent);
    border: 1px solid rgba(78, 204, 163, 0.24);
    border-radius: 4px;
    color: var(--mint);
    cursor: pointer;
    font-size: 8px;
    margin-top: 12px;
    padding: 5px 9px;
  }
  .scan-loader {
    border: 1px solid rgba(78, 204, 163, 0.3);
    border-radius: 40px;
    height: 76px;
    overflow: hidden;
    position: relative;
    width: 110px;
  }
  .scan-loader::before {
    border: 1px solid rgba(78, 204, 163, 0.2);
    border-radius: 3px;
    content: '';
    inset: 8px;
    position: absolute;
  }
  .scan-loader span {
    animation: scanline 2.2s ease-in-out infinite;
    background: linear-gradient(90deg, transparent, var(--mint), transparent);
    box-shadow: 0 0 12px rgba(78, 204, 163, 0.5);
    height: 1px;
    left: 10px;
    position: absolute;
    right: 10px;
    top: 50%;
  }
  @keyframes scanline {
    0%,
    100% {
      transform: translateY(-26px);
    }
    50% {
      transform: translateY(26px);
    }
  }
  .empty-state {
    align-items: center;
    background:
      linear-gradient(rgba(78, 204, 163, 0.018) 1px, transparent 1px),
      linear-gradient(90deg, rgba(78, 204, 163, 0.018) 1px, transparent 1px);
    background-size: 34px 34px;
    color: var(--muted);
    display: flex;
    flex-direction: column;
    height: 100%;
    justify-content: center;
    padding: 30px;
    text-align: center;
  }
  .scan-art {
    border: 1px solid rgba(78, 204, 163, 0.28);
    border-radius: 12px;
    height: 80px;
    margin-bottom: 18px;
    position: relative;
    transform: rotate(-3deg);
    width: 118px;
  }
  .scan-art::before,
  .scan-art::after {
    border: 1px solid rgba(78, 204, 163, 0.35);
    content: '';
    height: 13px;
    position: absolute;
    width: 13px;
  }
  .scan-art::before {
    border-bottom: 0;
    border-right: 0;
    left: 9px;
    top: 9px;
  }
  .scan-art::after {
    border-left: 0;
    border-top: 0;
    bottom: 9px;
    right: 9px;
  }
  .scan-art span {
    animation: scanline 3.2s ease-in-out infinite;
    background: linear-gradient(90deg, transparent, var(--mint), transparent);
    height: 1px;
    left: 10px;
    position: absolute;
    right: 10px;
    top: 50%;
  }
  .empty-state h2 {
    color: #fff;
    font-size: 18px;
    font-weight: 600;
    margin: 0 0 6px;
  }
  .empty-state p {
    font-size: 10px;
    line-height: 1.6;
    margin: 0;
    max-width: 340px;
  }
  .empty-tips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    justify-content: center;
    margin-top: 18px;
  }
  .empty-tips span {
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: 7.5px;
    padding: 4px 8px;
  }
  @media (max-width: 840px) {
    .queue-panel {
      flex-basis: 215px;
    }
  }
</style>
