<script lang="ts">
  import type {
    AppSettings,
    HealthSnapshot,
    ModelsReport,
    RequiredModelId,
    ServiceRuntimeStatus
  } from '../types';
  import { REQUIRED_MODELS } from '../models';
  import { formatBytes, testOllama } from '../api';
  import { hotkeyIsValid } from '../settings';
  import { toast } from '../toast.svelte';

  type ProgressState = {
    downloadedMb: number;
    totalMb: number;
    status: 'queued' | 'downloading' | 'done' | 'error';
    error?: string;
  };

  let {
    settings,
    health = null,
    models = null,
    serviceStatus = [],
    progress = {},
    downloading = false,
    onupdate,
    ondownload,
    ondelete,
    onrefresh,
    onrestart,
    onmodels
  }: {
    settings: AppSettings;
    health?: HealthSnapshot | null;
    models?: ModelsReport | null;
    serviceStatus?: ServiceRuntimeStatus[];
    progress?: Partial<Record<RequiredModelId, ProgressState>>;
    downloading?: boolean;
    onupdate?: (patch: Partial<AppSettings>) => void | Promise<void>;
    ondownload?: (model: RequiredModelId) => void | Promise<void>;
    ondelete?: (model: RequiredModelId) => void | Promise<void>;
    onrefresh?: () => void | Promise<void>;
    onrestart?: (name: 'asr' | 'ocr') => void | Promise<void>;
    onmodels?: () => void;
  } = $props();

  let dictationHotkey = $state(settings.hotkeyDictation);
  let ocrHotkey = $state(settings.hotkeyOcr);
  let ollamaUrl = $state(settings.ollamaUrl);
  let ollamaModel = $state(settings.ollamaModel);
  let testing = $state(false);
  let testResult = $state<{ ok: boolean; models: string[]; error?: string } | null>(null);
  let hotkeyError = $state('');

  $effect(() => {
    dictationHotkey = settings.hotkeyDictation;
    ocrHotkey = settings.hotkeyOcr;
    ollamaUrl = settings.ollamaUrl;
    ollamaModel = settings.ollamaModel;
  });

  let readyCount = $derived(models?.models.filter((model) => model.ready).length ?? 0);

  function update<K extends keyof AppSettings>(key: K, value: AppSettings[K]) {
    void onupdate?.({ [key]: value } as Partial<AppSettings>);
  }

  async function applyHotkeys() {
    hotkeyError = '';
    if (!hotkeyIsValid(dictationHotkey) || !hotkeyIsValid(ocrHotkey)) {
      hotkeyError = 'Use a modifier plus a key, for example Ctrl+Shift+D.';
      return;
    }
    await onupdate?.({
      hotkeyDictation: dictationHotkey.trim(),
      hotkeyOcr: ocrHotkey.trim()
    });
  }

  async function resetHotkeys() {
    dictationHotkey = 'Ctrl+Shift+D';
    ocrHotkey = 'Ctrl+Shift+S';
    await applyHotkeys();
  }

  async function test() {
    testing = true;
    testResult = null;
    try {
      testResult = await testOllama({ ...settings, ollamaUrl, ollamaModel });
      if (testResult.ok) toast('Ollama reachable. All generation stays local.', 'success');
      else toast(testResult.error || 'Ollama is not reachable.', 'error');
    } finally {
      testing = false;
    }
  }

  async function downloadAllMissing() {
    for (const model of REQUIRED_MODELS) {
      if (models?.models.find((entry) => entry.model === model.id)?.ready) continue;
      await ondownload?.(model.id);
    }
    await onrefresh?.();
  }

  function statusFor(id: RequiredModelId) {
    return models?.models.find((model) => model.model === id) ?? null;
  }

  function progressFor(id: RequiredModelId): ProgressState {
    return progress[id] ?? { downloadedMb: 0, totalMb: 0, status: 'queued' };
  }

  function modelPercent(id: RequiredModelId): number {
    const item = progressFor(id);
    if (item.status === 'done') return 100;
    if (!item.totalMb) return 0;
    return Math.min(100, (item.downloadedMb / item.totalMb) * 100);
  }

  function service(name: string) {
    return serviceStatus.find((entry) => entry.name === name);
  }
</script>

<div class="settings">
  <header class="settings-head">
    <div>
      <span class="eyebrow">SETTINGS · ALL LOCAL</span>
      <h1>Make it yours.</h1>
      <p>Models, hotkeys, and output preferences. Everything saves straight to <code>~/.verbatim/</code>.</p>
    </div>
    <span class="autosave"><span></span> Changes save automatically</span>
  </header>

  <section class="card models-card">
    <div class="card-head">
      <div>
        <span class="eyebrow">MODELS</span>
        <h2>Local AI stack</h2>
        <p>Download once. Every model runs on this machine and can be removed at any time.</p>
      </div>
      <div class="head-actions">
        <span class="ready-count" class:incomplete={readyCount < REQUIRED_MODELS.length}>
          {readyCount}/{REQUIRED_MODELS.length} ready
        </span>
        <button type="button" class="subtle" onclick={() => void onrefresh?.()}>Refresh</button>
        <button type="button" class="primary" disabled={downloading} onclick={() => void downloadAllMissing()}>
          {downloading ? 'Downloading…' : 'Download missing'}
        </button>
      </div>
    </div>

    {#if models?.message}
      <div class="model-notice" class:warning={!models.ready}>{models.message}</div>
    {/if}

    <div class="models-grid">
      {#each REQUIRED_MODELS as model}
        {@const check = statusFor(model.id)}
        {@const item = progressFor(model.id)}
        <div class="model-row {model.accent}" class:installed={check?.ready}>
          <div class="model-top">
            <div class="model-icon">{model.short.slice(0, 2).toUpperCase()}</div>
            <div class="model-copy">
              <strong>{model.name}</strong>
              <span>{model.role}</span>
            </div>
            <div class="model-meta">
              {#if check?.ready}
                <span class="status-ok"><i></i> READY</span>
                <em>{formatBytes(check.sizeBytes)} on disk</em>
              {:else}
                <span class="status-missing"><i></i> MISSING</span>
                <em>{model.size} · {model.source}</em>
              {/if}
            </div>
          </div>
          {#if item.status === 'downloading' || item.status === 'done' || item.status === 'error'}
            <div class="bar-row">
              <div class="bar">
                <span
                  class:error={item.status === 'error'}
                  style:width={`${item.status === 'error' ? 100 : modelPercent(model.id)}%`}
                ></span>
              </div>
              <span>
                {#if item.status === 'error'}Failed
                {:else if item.status === 'done'}100%
                {:else if item.totalMb}{Math.round(modelPercent(model.id))}% · {item.downloadedMb.toFixed(1)} MB
                {:else}Downloading…{/if}
              </span>
            </div>
          {/if}
          <div class="model-actions">
            {#if check?.ready}
              <button
                type="button"
                class="danger"
                onclick={async () => {
                  if (confirm(`Remove ${model.name} from this machine?`)) {
                    await ondelete?.(model.id);
                    await onrefresh?.();
                  }
                }}>Delete</button
              >
              <button type="button" onclick={() => void ondownload?.(model.id)}>Reinstall</button>
            {:else if item.status === 'error'}
              <button type="button" class="primary" onclick={() => void ondownload?.(model.id)}>Retry</button>
            {:else}
              <button type="button" class="primary" disabled={downloading || item.status === 'downloading'} onclick={() => void ondownload?.(model.id)}>
                {item.status === 'downloading' ? 'Downloading…' : 'Download'}
              </button>
            {/if}
            {#if item.status === 'error' && item.error}
              <span class="model-error">{item.error}</span>
            {/if}
          </div>
        </div>
      {/each}
    </div>
    <div class="models-foot">
      <span>Stored in <code>{models?.dataDir ?? '~/.verbatim/models'}</code></span>
      <button type="button" class="link" onclick={() => onmodels?.()}>Open first-launch setup</button>
    </div>
  </section>

  <div class="two-col">
    <section class="card">
      <div class="card-head small">
        <div>
          <span class="eyebrow">SERVICES</span>
          <h2>Localhost processes</h2>
        </div>
      </div>
      <div class="services">
        <div class="service-row">
          <span class="dot" class:ok={health?.asr.status === 'ok'} class:down={health?.asr.status === 'down'}></span>
          <div>
            <strong>ASR · SenseVoice</strong>
            <span>127.0.0.1:8001 · {health?.asr.loaded ? 'model loaded' : 'model idle'}</span>
          </div>
          <span class="service-state">{service('asr')?.state ?? health?.asr.status ?? 'unknown'}</span>
          <button type="button" onclick={() => void onrestart?.('asr')}>Restart</button>
        </div>
        <div class="service-row">
          <span class="dot" class:ok={health?.ocr.status === 'ok'} class:down={health?.ocr.status === 'down'}></span>
          <div>
            <strong>OCR · RapidOCR</strong>
            <span>127.0.0.1:8002 · {health?.ocr.loaded ? 'model loaded' : 'model idle'}</span>
          </div>
          <span class="service-state">{service('ocr')?.state ?? health?.ocr.status ?? 'unknown'}</span>
          <button type="button" onclick={() => void onrestart?.('ocr')}>Restart</button>
        </div>
        <div class="service-row">
          <span class="dot" class:ok={health?.ollama.status === 'ok'} class:down={health?.ollama.status === 'down'}></span>
          <div>
            <strong>Ollama</strong>
            <span>{settings.ollamaUrl.replace(/^https?:\/\//, '')}</span>
          </div>
          <span class="service-state">{health?.ollama.status ?? 'unknown'}</span>
          <button type="button" onclick={() => void test()}>Test</button>
        </div>
      </div>
      <p class="hint">Services bind to 127.0.0.1 only. Crashes restart automatically up to 3 times; logs stay in ~/.verbatim/logs/.</p>
    </section>

    <section class="card">
      <div class="card-head small">
        <div>
          <span class="eyebrow">HOTKEYS</span>
          <h2>Global shortcuts</h2>
        </div>
      </div>
      <label class="field">
        <span>Dictation</span>
        <div class="key-input">
          <input bind:value={dictationHotkey} onblur={() => void applyHotkeys()} aria-label="Dictation hotkey" />
          <kbd>mic</kbd>
        </div>
      </label>
      <label class="field">
        <span>Screen OCR</span>
        <div class="key-input">
          <input bind:value={ocrHotkey} onblur={() => void applyHotkeys()} aria-label="OCR hotkey" />
          <kbd>scan</kbd>
        </div>
      </label>
      {#if hotkeyError}<p class="field-error">{hotkeyError}</p>{/if}
      <div class="field-foot">
        <span>Tap or hold according to the Dictation tab.</span>
        <button type="button" class="link" onclick={() => void resetHotkeys()}>Reset defaults</button>
      </div>
    </section>
  </div>

  <div class="two-col">
    <section class="card">
      <div class="card-head small">
        <div>
          <span class="eyebrow">OLLAMA</span>
          <h2>Local language model</h2>
        </div>
      </div>
      <label class="field">
        <span>Endpoint</span>
        <input
          bind:value={ollamaUrl}
          onblur={() => update('ollamaUrl', ollamaUrl.trim())}
          placeholder="http://127.0.0.1:11434"
          spellcheck="false"
        />
      </label>
      <label class="field">
        <span>Model</span>
        <input
          bind:value={ollamaModel}
          onblur={() => update('ollamaModel', ollamaModel.trim())}
          placeholder="qwen2.5-coder:7b"
          spellcheck="false"
        />
      </label>
      <div class="test-row">
        <button type="button" class="subtle" disabled={testing} onclick={() => void test()}>
          {testing ? 'Testing…' : 'Test connection'}
        </button>
        {#if testResult}
          <span class:ok={testResult.ok} class:fail={!testResult.ok}>
            {testResult.ok ? `${testResult.models.length} model(s) available` : testResult.error || 'Unreachable'}
          </span>
        {/if}
      </div>
      <p class="hint">
        Point this at a remote Ollama host only if you intend to send note text there. It is the only permitted
        outbound connection.
      </p>
    </section>

    <section class="card">
      <div class="card-head small">
        <div>
          <span class="eyebrow">OUTPUT</span>
          <h2>Defaults</h2>
        </div>
      </div>
      <label class="field">
        <span>Transcription language</span>
        <select value={settings.language} onchange={(event) => update('language', event.currentTarget.value)}>
          <option value="auto">Auto-detect</option>
          <option value="en">English</option>
          <option value="zh">Chinese</option>
          <option value="ja">Japanese</option>
          <option value="ko">Korean</option>
          <option value="es">Spanish</option>
          <option value="fr">French</option>
          <option value="de">German</option>
        </select>
      </label>
      <label class="field">
        <span>Default export format</span>
        <select
          value={settings.defaultFormat}
          onchange={(event) => update('defaultFormat', event.currentTarget.value as AppSettings['defaultFormat'])}
        >
          <option value="md">Markdown (.md)</option>
          <option value="txt">Plain text (.txt)</option>
          <option value="csv">CSV (.csv)</option>
        </select>
      </label>
      <label class="toggle">
        <input type="checkbox" checked={settings.proofread} onchange={(event) => update('proofread', event.currentTarget.checked)} />
        <span class="switch"></span>
        <span>Proofread dictation with Ollama</span>
      </label>
    </section>
  </div>

  <section class="privacy-card">
    <div class="privacy-mark">
      <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M12 3 4 6.5v5.2c0 4.6 3.2 7.6 8 9.3 4.8-1.7 8-4.7 8-9.3V6.5L12 3Z" stroke-linejoin="round" />
      </svg>
    </div>
    <div>
      <strong>Verbatim has no telemetry.</strong>
      <p>
        No accounts, analytics, API keys, or cloud fallbacks. Notes, transcripts, OCR output, and settings live in
        SQLite under <code>~/.verbatim/</code>. Logs rotate after 7 days. Exported files are the only thing that leaves
        when you choose to save them.
      </p>
    </div>
    <span class="version">v1.0.0 · MIT</span>
  </section>
</div>

<style>
  .settings {
    height: 100%;
    overflow-y: auto;
    padding: 22px 24px 30px;
  }
  .settings > * {
    margin-left: auto;
    margin-right: auto;
    max-width: 940px;
  }
  .settings-head {
    align-items: flex-start;
    display: flex;
    justify-content: space-between;
    margin-bottom: 16px;
  }
  .settings-head h1 {
    color: #fff;
    font-size: 23px;
    font-weight: 650;
    letter-spacing: -0.03em;
    margin: 4px 0 4px;
  }
  .settings-head p {
    color: var(--muted);
    font-size: 9.5px;
    margin: 0;
  }
  code {
    color: var(--mint);
    font-family: var(--mono);
    font-size: 0.93em;
  }
  .autosave {
    align-items: center;
    color: var(--muted);
    display: flex;
    font-family: var(--mono);
    font-size: 6.8px;
    gap: 5px;
    letter-spacing: 0.04em;
    margin-top: 6px;
  }
  .autosave > span {
    background: var(--mint);
    border-radius: 50%;
    height: 4px;
    width: 4px;
  }
  .card {
    background: linear-gradient(165deg, rgba(22, 33, 62, 0.66), rgba(13, 21, 41, 0.6));
    border: 1px solid var(--border);
    border-radius: 11px;
    margin-bottom: 10px;
    padding: 14px 16px;
  }
  .card-head {
    align-items: flex-start;
    display: flex;
    gap: 14px;
    justify-content: space-between;
    margin-bottom: 11px;
  }
  .card-head.small {
    margin-bottom: 10px;
  }
  .card h2 {
    color: #fff;
    font-size: 13.5px;
    font-weight: 600;
    margin: 3px 0 2px;
  }
  .card p {
    color: var(--muted);
    font-size: 8.7px;
    line-height: 1.5;
    margin: 0;
  }
  .head-actions {
    align-items: center;
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    justify-content: flex-end;
  }
  .ready-count {
    border: 1px solid rgba(78, 204, 163, 0.22);
    border-radius: 3px;
    color: var(--mint);
    font-family: var(--mono);
    font-size: 7px;
    padding: 3px 5px;
  }
  .ready-count.incomplete {
    border-color: rgba(231, 181, 108, 0.22);
    color: #e7b56c;
  }
  button {
    border-radius: 4px;
    cursor: pointer;
    font-size: 8.5px;
    padding: 5px 8px;
  }
  button.primary {
    background: var(--accent);
    border: 1px solid rgba(78, 204, 163, 0.26);
    color: var(--mint);
  }
  button.primary:hover:not(:disabled) {
    background: #15457c;
  }
  button.subtle {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--muted);
  }
  button.subtle:hover {
    border-color: rgba(78, 204, 163, 0.3);
    color: var(--text);
  }
  button.danger {
    background: transparent;
    border: 1px solid rgba(233, 69, 96, 0.2);
    color: #e59aa6;
  }
  button:disabled {
    cursor: default;
    opacity: 0.4;
  }
  button.link {
    background: transparent;
    border: 0;
    color: var(--muted);
    font-size: 7.5px;
    padding: 0;
    text-decoration: underline;
    text-decoration-color: rgba(140, 152, 171, 0.35);
    text-underline-offset: 2px;
  }
  button.link:hover {
    color: var(--mint);
  }
  .model-notice {
    background: rgba(78, 204, 163, 0.05);
    border: 1px solid rgba(78, 204, 163, 0.13);
    border-radius: 5px;
    color: #a9d9c9;
    font-size: 8.2px;
    margin-bottom: 9px;
    padding: 6px 8px;
  }
  .model-notice.warning {
    background: rgba(231, 181, 108, 0.055);
    border-color: rgba(231, 181, 108, 0.16);
    color: #dbb887;
  }
  .models-grid {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .model-row {
    background: rgba(8, 13, 28, 0.3);
    border: 1px solid var(--border-soft);
    border-radius: 7px;
    padding: 8px 9px;
  }
  .model-row.installed {
    border-color: rgba(78, 204, 163, 0.13);
  }
  .model-top {
    align-items: center;
    display: flex;
    gap: 8px;
  }
  .model-icon {
    align-items: center;
    background: rgba(15, 52, 96, 0.48);
    border: 1px solid var(--border-soft);
    border-radius: 4px;
    color: var(--mint);
    display: flex;
    flex: 0 0 26px;
    font-family: var(--mono);
    font-size: 7px;
    height: 26px;
    justify-content: center;
  }
  .signal .model-icon {
    color: var(--signal);
  }
  .sand .model-icon {
    color: var(--sand);
  }
  .model-copy {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .model-copy strong {
    font-size: 10px;
    font-weight: 550;
  }
  .model-copy span {
    color: var(--muted);
    font-size: 7.8px;
  }
  .model-meta {
    align-items: flex-end;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .model-meta em {
    color: var(--muted-low);
    font-family: var(--mono);
    font-size: 6.3px;
    font-style: normal;
  }
  .status-ok,
  .status-missing {
    align-items: center;
    display: flex;
    font-family: var(--mono);
    font-size: 6.5px;
    gap: 3px;
    letter-spacing: 0.08em;
  }
  .status-ok {
    color: var(--mint);
  }
  .status-missing {
    color: #d9ab68;
  }
  .status-ok i,
  .status-missing i {
    background: currentColor;
    border-radius: 50%;
    height: 3px;
    width: 3px;
  }
  .bar-row {
    align-items: center;
    display: flex;
    gap: 7px;
    margin: 6px 0 0 34px;
  }
  .bar {
    background: rgba(234, 234, 234, 0.06);
    border-radius: 999px;
    flex: 1;
    height: 3px;
    overflow: hidden;
  }
  .bar span {
    background: var(--mint);
    display: block;
    height: 100%;
    transition: width 200ms ease;
  }
  .bar span.error {
    background: var(--signal);
  }
  .bar-row > span {
    color: var(--muted);
    font-family: var(--mono);
    font-size: 6.5px;
    min-width: 76px;
    text-align: right;
  }
  .model-actions {
    align-items: center;
    display: flex;
    gap: 5px;
    margin: 6px 0 0 34px;
  }
  .model-actions button {
    font-size: 7.5px;
    padding: 3px 6px;
  }
  .model-error {
    color: #e59aa6;
    font-size: 7px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .models-foot {
    align-items: center;
    border-top: 1px solid var(--border-soft);
    color: var(--muted);
    display: flex;
    font-size: 7.5px;
    justify-content: space-between;
    margin-top: 9px;
    padding-top: 8px;
  }
  .two-col {
    display: grid;
    gap: 10px;
    grid-template-columns: 1fr 1fr;
  }
  .services {
    display: flex;
    flex-direction: column;
  }
  .service-row {
    align-items: center;
    border-bottom: 1px solid var(--border-soft);
    display: flex;
    gap: 7px;
    padding: 6px 0;
  }
  .service-row:last-child {
    border-bottom: 0;
  }
  .dot {
    background: var(--muted-low);
    border-radius: 50%;
    flex: 0 0 auto;
    height: 5px;
    width: 5px;
  }
  .dot.ok {
    background: var(--mint);
    box-shadow: 0 0 4px rgba(78, 204, 163, 0.4);
  }
  .dot.down {
    background: var(--signal);
  }
  .service-row > div {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .service-row strong {
    font-size: 8.8px;
    font-weight: 500;
  }
  .service-row div span {
    color: var(--muted);
    font-family: var(--mono);
    font-size: 6.5px;
  }
  .service-state {
    color: var(--muted);
    font-family: var(--mono);
    font-size: 6.3px;
    text-transform: uppercase;
  }
  .service-row button {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--muted);
    font-size: 7px;
    padding: 2px 5px;
  }
  .hint {
    border-top: 1px solid var(--border-soft);
    color: var(--muted-low) !important;
    font-size: 7.2px !important;
    margin-top: 7px !important;
    padding-top: 7px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }
  .field > span {
    color: var(--muted);
    font-family: var(--mono);
    font-size: 6.8px;
    letter-spacing: 0.05em;
  }
  .field input,
  .field select {
    background: rgba(8, 13, 28, 0.36);
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--text);
    font-size: 9px;
    height: 28px;
    outline: 0;
    padding: 0 8px;
    width: 100%;
  }
  .field input:focus,
  .field select:focus {
    border-color: rgba(78, 204, 163, 0.4);
  }
  .key-input {
    align-items: center;
    display: flex;
    gap: 5px;
  }
  .key-input input {
    font-family: var(--mono);
    letter-spacing: 0.04em;
  }
  .key-input kbd {
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--muted-low);
    font-family: var(--mono);
    font-size: 6px;
    padding: 2px 4px;
  }
  .field-error {
    color: #e59aa6 !important;
    font-size: 7.5px !important;
    margin: 0 0 7px !important;
  }
  .field-foot {
    align-items: center;
    color: var(--muted-low);
    display: flex;
    font-size: 7.2px;
    justify-content: space-between;
  }
  .test-row {
    align-items: center;
    display: flex;
    gap: 7px;
  }
  .test-row button {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--muted);
    font-size: 7.5px;
    padding: 4px 7px;
  }
  .test-row span {
    font-size: 7.5px;
  }
  .test-row span.ok {
    color: var(--mint);
  }
  .test-row span.fail {
    color: #e59aa6;
  }
  .toggle {
    align-items: center;
    color: var(--text);
    cursor: pointer;
    display: flex;
    font-size: 8.5px;
    gap: 7px;
    margin-top: 10px;
  }
  .toggle input {
    height: 0;
    opacity: 0;
    position: absolute;
    width: 0;
  }
  .switch {
    background: rgba(140, 152, 171, 0.16);
    border: 1px solid var(--border);
    border-radius: 999px;
    flex: 0 0 auto;
    height: 14px;
    position: relative;
    width: 25px;
  }
  .switch::after {
    background: var(--muted);
    border-radius: 50%;
    content: '';
    height: 8px;
    left: 3px;
    position: absolute;
    top: 2px;
    transition: transform 140ms ease;
    width: 8px;
  }
  .toggle input:checked + .switch {
    background: rgba(78, 204, 163, 0.25);
    border-color: rgba(78, 204, 163, 0.4);
  }
  .toggle input:checked + .switch::after {
    background: var(--mint);
    transform: translateX(11px);
  }
  .privacy-card {
    align-items: flex-start;
    background: rgba(78, 204, 163, 0.03);
    border: 1px solid rgba(78, 204, 163, 0.11);
    border-radius: 10px;
    display: flex;
    gap: 11px;
    padding: 12px 14px;
  }
  .privacy-mark {
    color: var(--mint);
    flex: 0 0 auto;
  }
  .privacy-card > div:nth-child(2) {
    flex: 1;
  }
  .privacy-card strong {
    color: #dfe9e5;
    font-size: 9.5px;
    font-weight: 600;
  }
  .privacy-card p {
    color: var(--muted);
    font-size: 8px;
    line-height: 1.55;
    margin: 3px 0 0;
  }
  .version {
    color: var(--muted-low);
    font-family: var(--mono);
    font-size: 6.5px;
    white-space: nowrap;
  }
  @media (max-width: 820px) {
    .two-col {
      grid-template-columns: 1fr;
    }
    .settings-head,
    .card-head {
      flex-direction: column;
    }
  }
</style>
