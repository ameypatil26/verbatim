<script lang="ts">
  import type { HealthSnapshot, ServiceHealth } from '../types';

  let {
    health = null,
    onretry,
    checking = false,
    modelsReady = true,
    onmodels
  }: {
    health?: HealthSnapshot | null;
    onretry?: () => void;
    checking?: boolean;
    modelsReady?: boolean;
    onmodels?: () => void;
  } = $props();

  const services: { key: 'asr' | 'ocr' | 'ollama'; label: string; model: string }[] = [
    { key: 'asr', label: 'ASR', model: '8001' },
    { key: 'ocr', label: 'OCR', model: '8002' },
    { key: 'ollama', label: 'OLLAMA', model: '11434' }
  ];

  function state(service: 'asr' | 'ocr' | 'ollama'): ServiceHealth | null {
    return health ? health[service] : null;
  }

  function title(key: string, value: ServiceHealth | null): string {
    if (!value) return `${key.toUpperCase()} checking`;
    if (value.status === 'down') return `${key.toUpperCase()} unavailable: ${value.error ?? 'connection refused'}`;
    return `${key.toUpperCase()} ${value.status}${value.loaded !== undefined ? (value.loaded ? ' · model loaded' : ' · model idle') : ''}${
      value.latencyMs !== undefined ? ` · ${value.latencyMs}ms` : ''
    }`;
  }
</script>

<footer class="statusbar">
  <div class="left">
    <span class="privacy-lock" title="No telemetry. No cloud.">
      <svg viewBox="0 0 14 14" width="10" height="10" fill="none" stroke="currentColor" stroke-width="1.2">
        <rect x="3" y="6" width="8" height="6" rx="1" />
        <path d="M4.7 6V4.3a2.3 2.3 0 0 1 4.6 0V6" />
      </svg>
      ALL LOCAL
    </span>
    <span class="divider"></span>
    <span class="privacy-copy">no accounts · no API keys · no telemetry</span>
    {#if !modelsReady}
      <span class="divider"></span>
      <button class="models-warning" type="button" onclick={() => onmodels?.()}>
        <span></span>
        Models not downloaded
      </button>
    {/if}
  </div>

  <div class="right">
    {#each services as service}
      <button type="button" class="service" title={title(service.key, state(service.key))} onclick={() => onretry?.()}>
        <span
          class="dot"
          class:ok={state(service.key)?.status === 'ok'}
          class:down={state(service.key)?.status === 'down'}
          class:degraded={state(service.key)?.status === 'degraded'}
        ></span>
        <span class="name">{service.label}</span>
        <span class="port">{service.model}</span>
      </button>
    {/each}
    <button class="retry" type="button" onclick={() => onretry?.()} disabled={checking} title="Re-check local services">
      <svg viewBox="0 0 16 16" width="11" height="11" fill="none" stroke="currentColor" stroke-width="1.4" class:spinning={checking}>
        <path d="M13 8a5 5 0 1 1-1.5-3.6M13 2.5v3h-3" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>
  </div>
</footer>

<style>
  .statusbar {
    align-items: center;
    background: rgba(11, 17, 34, 0.88);
    border-top: 1px solid var(--border);
    display: flex;
    flex: 0 0 27px;
    justify-content: space-between;
    padding: 0 9px 0 13px;
    user-select: none;
  }
  .left,
  .right {
    align-items: center;
    display: flex;
    min-width: 0;
  }
  .left {
    gap: 7px;
  }
  .privacy-lock {
    align-items: center;
    color: var(--mint);
    display: flex;
    font-family: var(--mono);
    font-size: 6.7px;
    gap: 4px;
    letter-spacing: 0.13em;
    opacity: 0.82;
  }
  .divider {
    background: var(--border);
    height: 9px;
    width: 1px;
  }
  .privacy-copy {
    color: var(--muted);
    font-size: 7.5px;
    letter-spacing: 0.01em;
  }
  .models-warning {
    align-items: center;
    background: transparent;
    border: 0;
    color: #e7b56c;
    cursor: pointer;
    display: flex;
    font-family: var(--mono);
    font-size: 7px;
    gap: 5px;
    letter-spacing: 0.04em;
    padding: 2px 3px;
  }
  .models-warning:hover {
    color: #ffd18a;
  }
  .models-warning > span {
    background: #e7b56c;
    border-radius: 50%;
    height: 4px;
    width: 4px;
  }
  .right {
    gap: 2px;
  }
  .service {
    align-items: center;
    background: transparent;
    border: 0;
    border-radius: 3px;
    color: var(--muted);
    cursor: pointer;
    display: flex;
    font-family: var(--mono);
    font-size: 6.5px;
    gap: 4px;
    letter-spacing: 0.06em;
    padding: 3px 5px;
  }
  .service:hover {
    background: rgba(15, 52, 96, 0.25);
  }
  .name {
    color: #aab4c2;
  }
  .port {
    color: #5d6878;
  }
  .dot {
    background: #59606f;
    border-radius: 50%;
    height: 4px;
    width: 4px;
  }
  .dot.ok {
    background: var(--mint);
    box-shadow: 0 0 4px rgba(78, 204, 163, 0.55);
  }
  .dot.down {
    background: var(--signal);
  }
  .dot.degraded {
    background: #e2b341;
  }
  .retry {
    align-items: center;
    background: transparent;
    border: 0;
    border-radius: 3px;
    color: var(--muted);
    cursor: pointer;
    display: flex;
    height: 21px;
    justify-content: center;
    margin-left: 3px;
    width: 21px;
  }
  .retry:hover {
    background: rgba(15, 52, 96, 0.35);
    color: var(--text);
  }
  .retry:disabled {
    cursor: default;
  }
  .spinning {
    animation: rotate 0.9s linear infinite;
  }
  @keyframes rotate {
    to {
      transform: rotate(360deg);
    }
  }
</style>
