<script lang="ts">
  import type { RequiredModelId } from '../types';
  import { REQUIRED_MODELS } from '../models';

  interface Progress {
    downloadedMb: number;
    totalMb: number;
    status: 'queued' | 'downloading' | 'done' | 'error';
    error?: string;
  }

  let {
    open = false,
    models = [],
    progress = {},
    downloading = false,
    error = '',
    dataDir = '~/.verbatim',
    ondownload,
    oncancel
  }: {
    open?: boolean;
    models?: { model: RequiredModelId; ready: boolean; detail: string }[];
    progress?: Partial<Record<RequiredModelId, Progress>>;
    downloading?: boolean;
    error?: string;
    dataDir?: string;
    ondownload?: () => void;
    oncancel?: () => void;
  } = $props();

  let readyCount = $derived(models.filter((model) => model.ready).length);
  let missingCount = $derived(REQUIRED_MODELS.length - readyCount);

  function stateFor(id: RequiredModelId): Progress {
    return progress[id] ?? { downloadedMb: 0, totalMb: 0, status: 'queued' };
  }

  function percent(id: RequiredModelId): number {
    const item = stateFor(id);
    if (item.status === 'done') return 100;
    if (!item.totalMb) return 0;
    return Math.max(0, Math.min(100, (item.downloadedMb / item.totalMb) * 100));
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && open && !downloading) oncancel?.();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="backdrop" role="presentation">
    <div class="modal" role="dialog" aria-modal="true" aria-labelledby="model-title">
      <div class="glow"></div>
      <header>
        <div class="mark" aria-hidden="true">
          <span></span><span></span><span></span><span></span><span></span>
        </div>
        <div class="heading">
          <span class="eyebrow">LOCAL MODELS · ONE-TIME SETUP</span>
          <h1 id="model-title">Verbatim needs its models.</h1>
        </div>
        {#if !downloading}
          <button class="close" type="button" aria-label="Close setup" onclick={() => oncancel?.()}>×</button>
        {/if}
      </header>

      <p class="lede">
        Verbatim needs to download required AI models to function locally. Download now?
      </p>
      <p class="privacy">
        Nothing is sent to a cloud service. Downloads happen once, directly from the listed source, and every
        model then runs on this machine. Files live in <code>{dataDir}</code>.
      </p>

      <div class="models">
        {#each REQUIRED_MODELS as model}
          {@const status = stateFor(model.id)}
          {@const installed = models.find((item) => item.model === model.id)?.ready}
          <div class="model {model.accent}" class:done={installed} class:active={status.status === 'downloading'}>
            <div class="model-top">
              <div class="model-badge">{model.short.slice(0, 2).toUpperCase()}</div>
              <div class="model-copy">
                <strong>{model.name}</strong>
                <span>{model.role}</span>
              </div>
              <div class="model-size">
                {#if installed}
                  <span class="installed">READY</span>
                {:else}
                  <span>{model.size}</span>
                  <em>{model.source}</em>
                {/if}
              </div>
            </div>
            {#if status.status === 'downloading' || status.status === 'done' || status.status === 'error'}
              <div class="progress-row">
                <div class="track">
                  <span
                    class:error={status.status === 'error'}
                    class:indeterminate={status.status === 'downloading' && !status.totalMb}
                    style:width={`${status.status === 'error' ? 100 : percent(model.id)}%`}
                  ></span>
                </div>
                <span class="progress-label">
                  {#if status.status === 'error'}
                    Failed
                  {:else if status.status === 'done'}
                    100%
                  {:else if status.totalMb}
                    {Math.round(percent(model.id))}% · {status.downloadedMb.toFixed(1)} / {status.totalMb.toFixed(1)} MB
                  {:else}
                    Downloading…
                  {/if}
                </span>
              </div>
            {/if}
            {#if status.status === 'error' && status.error}
              <p class="model-error">{status.error}</p>
            {/if}
          </div>
        {/each}
      </div>

      {#if error}
        <p class="error-box">{error}</p>
      {/if}

      <footer>
        <div class="foot-copy">
          <span class="dot"></span>
          {#if readyCount === REQUIRED_MODELS.length}
            All models ready
          {:else}
            {readyCount} of {REQUIRED_MODELS.length} models installed · {missingCount} missing
          {/if}
        </div>
        <div class="buttons">
          <button type="button" class="ghost" disabled={downloading} onclick={() => oncancel?.()}>
            Cancel
          </button>
          <button type="button" class="primary" disabled={downloading} onclick={() => ondownload?.()}>
            {#if downloading}
              <span class="spinner"></span>
              Downloading…
            {:else if readyCount > 0}
              Download missing models
            {:else}
              Download now
            {/if}
          </button>
        </div>
      </footer>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    align-items: center;
    animation: fade-in 180ms ease-out;
    backdrop-filter: blur(13px) saturate(0.85);
    background: rgba(6, 10, 20, 0.72);
    display: flex;
    inset: 0;
    justify-content: center;
    padding: 26px;
    position: fixed;
    z-index: 80;
  }
  .modal {
    animation: modal-in 220ms cubic-bezier(0.2, 0.8, 0.2, 1);
    background: linear-gradient(175deg, rgba(26, 41, 74, 0.98), rgba(15, 21, 42, 0.99));
    border: 1px solid rgba(234, 234, 234, 0.13);
    border-radius: 15px;
    box-shadow: 0 28px 90px rgba(0, 0, 0, 0.55);
    max-height: calc(100vh - 50px);
    max-width: 690px;
    overflow-y: auto;
    padding: 25px 26px 20px;
    position: relative;
    width: 100%;
  }
  .glow {
    background: radial-gradient(circle, rgba(78, 204, 163, 0.11), transparent 68%);
    height: 260px;
    pointer-events: none;
    position: absolute;
    right: -80px;
    top: -120px;
    width: 320px;
  }
  header {
    align-items: flex-start;
    display: flex;
    gap: 14px;
    position: relative;
  }
  .mark {
    align-items: center;
    background: rgba(15, 52, 96, 0.7);
    border: 1px solid rgba(78, 204, 163, 0.28);
    border-radius: 11px;
    display: flex;
    flex: 0 0 44px;
    gap: 2px;
    height: 44px;
    justify-content: center;
  }
  .mark span {
    animation: mark-wave 1.8s ease-in-out infinite;
    background: var(--mint);
    border-radius: 3px;
    height: 8px;
    width: 2px;
  }
  .mark span:nth-child(2) {
    animation-delay: 0.15s;
    height: 17px;
  }
  .mark span:nth-child(3) {
    animation-delay: 0.3s;
    height: 24px;
  }
  .mark span:nth-child(4) {
    animation-delay: 0.45s;
    height: 14px;
  }
  .mark span:nth-child(5) {
    animation-delay: 0.6s;
    height: 6px;
  }
  @keyframes mark-wave {
    0%,
    100% {
      transform: scaleY(0.65);
    }
    50% {
      transform: scaleY(1);
    }
  }
  .heading {
    flex: 1;
    min-width: 0;
  }
  h1 {
    color: #fff;
    font-size: 22px;
    font-weight: 650;
    letter-spacing: -0.025em;
    line-height: 1.15;
    margin: 5px 0 0;
  }
  .close {
    background: transparent;
    border: 0;
    color: var(--muted);
    cursor: pointer;
    font-size: 21px;
    line-height: 1;
    padding: 2px 5px;
  }
  .close:hover {
    color: var(--text);
  }
  .lede {
    color: #dfe5ed;
    font-size: 13px;
    line-height: 1.55;
    margin: 20px 0 8px;
    max-width: 580px;
  }
  .privacy {
    color: var(--muted);
    font-size: 10.5px;
    line-height: 1.6;
    margin: 0 0 16px;
    max-width: 610px;
  }
  code {
    color: var(--mint);
    font-family: var(--mono);
    font-size: 9.5px;
  }
  .models {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .model {
    background: rgba(8, 14, 29, 0.34);
    border: 1px solid var(--border-soft);
    border-radius: 9px;
    padding: 10px 12px;
  }
  .model.active {
    border-color: rgba(78, 204, 163, 0.32);
  }
  .model.done {
    opacity: 0.75;
  }
  .model-top {
    align-items: center;
    display: flex;
    gap: 10px;
  }
  .model-badge {
    align-items: center;
    background: rgba(15, 52, 96, 0.5);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--mint);
    display: flex;
    flex: 0 0 29px;
    font-family: var(--mono);
    font-size: 8px;
    height: 29px;
    justify-content: center;
    letter-spacing: 0.05em;
  }
  .signal .model-badge {
    color: var(--signal);
  }
  .sand .model-badge {
    color: var(--sand);
  }
  .model-copy {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .model-copy strong {
    color: var(--text);
    font-size: 11.5px;
    font-weight: 600;
  }
  .model-copy span {
    color: var(--muted);
    font-size: 9px;
  }
  .model-size {
    align-items: flex-end;
    display: flex;
    flex-direction: column;
    flex: 0 0 auto;
    gap: 2px;
  }
  .model-size > span {
    color: var(--text);
    font-family: var(--mono);
    font-size: 8.5px;
  }
  .model-size em {
    color: var(--muted-low);
    font-family: var(--mono);
    font-size: 6.5px;
    font-style: normal;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .model-size .installed {
    color: var(--mint);
    font-size: 7.5px;
    letter-spacing: 0.1em;
  }
  .progress-row {
    align-items: center;
    display: flex;
    gap: 9px;
    margin-top: 8px;
  }
  .track {
    background: rgba(234, 234, 234, 0.07);
    border-radius: 999px;
    flex: 1;
    height: 3px;
    overflow: hidden;
  }
  .track span {
    background: linear-gradient(90deg, var(--mint), #8be8c6);
    border-radius: inherit;
    display: block;
    height: 100%;
    transition: width 240ms ease;
  }
  .track span.error {
    background: var(--signal);
  }
  .track span.indeterminate {
    animation: indeterminate 1.2s ease-in-out infinite;
    width: 35% !important;
  }
  @keyframes indeterminate {
    0% {
      margin-left: -35%;
    }
    100% {
      margin-left: 100%;
    }
  }
  .progress-label {
    color: var(--muted);
    font-family: var(--mono);
    font-size: 7.5px;
    min-width: 116px;
    text-align: right;
  }
  .model-error {
    color: #f19bad;
    font-size: 9px;
    line-height: 1.5;
    margin: 8px 0 0 39px;
  }
  .error-box {
    background: rgba(233, 69, 96, 0.08);
    border: 1px solid rgba(233, 69, 96, 0.22);
    border-radius: 7px;
    color: #f4a4b2;
    font-size: 10px;
    line-height: 1.5;
    margin: 12px 0 0;
    padding: 8px 10px;
  }
  footer {
    align-items: center;
    border-top: 1px solid var(--border-soft);
    display: flex;
    gap: 16px;
    justify-content: space-between;
    margin-top: 18px;
    padding-top: 14px;
  }
  .foot-copy {
    align-items: center;
    color: var(--muted);
    display: flex;
    font-family: var(--mono);
    font-size: 7.5px;
    gap: 6px;
    letter-spacing: 0.04em;
  }
  .dot {
    background: var(--mint);
    border-radius: 50%;
    box-shadow: 0 0 6px rgba(78, 204, 163, 0.5);
    height: 4px;
    width: 4px;
  }
  .buttons {
    display: flex;
    gap: 7px;
  }
  .buttons button {
    border-radius: 6px;
    cursor: pointer;
    font-size: 10.5px;
    font-weight: 600;
    padding: 8px 14px;
  }
  .ghost {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--muted);
  }
  .ghost:hover:not(:disabled) {
    border-color: rgba(234, 234, 234, 0.25);
    color: var(--text);
  }
  .primary {
    align-items: center;
    background: linear-gradient(180deg, #1b5f43, #134d36);
    border: 1px solid rgba(78, 204, 163, 0.4);
    color: #c9f5e5;
    display: inline-flex;
    gap: 7px;
    min-width: 148px;
    justify-content: center;
  }
  .primary:hover:not(:disabled) {
    background: linear-gradient(180deg, #227355, #175a40);
  }
  .buttons button:disabled {
    cursor: default;
    opacity: 0.55;
  }
  .spinner {
    animation: spin 0.85s linear infinite;
    border: 1.5px solid rgba(201, 245, 229, 0.3);
    border-radius: 50%;
    border-top-color: #c9f5e5;
    height: 11px;
    width: 11px;
  }
  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }
  @keyframes modal-in {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.99);
    }
  }
  @media (max-width: 640px) {
    .backdrop {
      padding: 12px;
    }
    .modal {
      padding: 18px;
    }
    footer {
      align-items: stretch;
      flex-direction: column;
    }
    .buttons {
      justify-content: flex-end;
    }
  }
</style>
