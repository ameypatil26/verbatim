<script lang="ts">
  import { onMount } from 'svelte';
  import type {
    AppSettings,
    HealthSnapshot,
    ModelDownloadProgress,
    ModelsReport,
    RequiredModelId,
    ServiceRuntimeStatus,
    TabId
  } from './lib/types';
  import {
    checkHealth,
    checkModels,
    deleteModel,
    downloadModel,
    isTauri,
    loadSettings,
    onModelProgress,
    saveSettings,
    serviceRuntimeStatus
  } from './lib/api';
  import { DEFAULT_SETTINGS } from './lib/settings';
  import { REQUIRED_MODELS } from './lib/models';
  import { toast } from './lib/toast.svelte';
  import DictationTab from './lib/tabs/DictationTab.svelte';
  import NotesTab from './lib/tabs/NotesTab.svelte';
  import VisionTab from './lib/tabs/VisionTab.svelte';
  import SettingsTab from './lib/tabs/SettingsTab.svelte';
  import ModelSetup from './lib/components/ModelSetup.svelte';
  import StatusBar from './lib/components/StatusBar.svelte';
  import ToastHost from './lib/components/ToastHost.svelte';

  type ProgressState = {
    downloadedMb: number;
    totalMb: number;
    status: 'queued' | 'downloading' | 'done' | 'error';
    error?: string;
  };

  const tabs: { id: TabId; label: string; shortcut: string; icon: string }[] = [
    { id: 'dictation', label: 'Dictation', shortcut: 'Ctrl+Shift+D', icon: 'mic' },
    { id: 'notes', label: 'Notes', shortcut: '', icon: 'note' },
    { id: 'vision', label: 'Vision', shortcut: 'Ctrl+Shift+S', icon: 'scan' },
    { id: 'settings', label: 'Settings', shortcut: '', icon: 'gear' }
  ];

  let settings = $state<AppSettings>({ ...DEFAULT_SETTINGS });
  let active = $state<TabId>('dictation');
  let health = $state<HealthSnapshot | null>(null);
  let serviceStatus = $state<ServiceRuntimeStatus[]>([]);
  let models = $state<ModelsReport | null>(null);
  let checking = $state(false);
  let loading = $state(true);
  let setupOpen = $state(false);
  let setupDismissed = $state(false);
  let downloading = $state(false);
  let downloadError = $state('');
  let progress = $state<Partial<Record<RequiredModelId, ProgressState>>>({});
  let interval: ReturnType<typeof setInterval> | undefined;
  const previewParams = typeof window !== 'undefined' ? new URLSearchParams(window.location.search) : new URLSearchParams();
  const demoMode = !isTauri && previewParams.get('demo') === '1';
  if (demoMode) setupDismissed = true;

  let readOnly = $derived(models !== null && !models.ready);
  let modelsReady = $derived(models?.ready ?? false);

  onMount(() => {
    const queryTab = previewParams.get('tab') as TabId | null;
    const savedTab = localStorage.getItem('verbatim.tab') as TabId | null;
    if (queryTab && tabs.some((tab) => tab.id === queryTab)) active = queryTab;
    else if (!demoMode && savedTab && tabs.some((tab) => tab.id === savedTab)) active = savedTab;

    const unlistenModels = onModelProgress(handleModelProgress);
    void boot();
    interval = setInterval(() => {
      void refreshHealth();
      void refreshServiceStatus();
    }, 6500);

    return () => {
      unlistenModels();
      if (interval) clearInterval(interval);
    };
  });

  async function boot() {
    try {
      settings = await loadSettings();
      await Promise.all([refreshHealth(), refreshServiceStatus()]);
      await refreshModels(true);
    } catch (error) {
      toast(`Startup warning: ${String(error)}`, 'error');
    } finally {
      loading = false;
    }
  }

  async function refreshHealth() {
    checking = true;
    try {
      health = await checkHealth(settings);
    } finally {
      checking = false;
    }
  }

  async function refreshServiceStatus() {
    try {
      serviceStatus = await serviceRuntimeStatus();
    } catch {
      // Health polling already surfaces a dead service.
    }
  }

  async function refreshModels(openIfMissing = false) {
    try {
      models = await checkModels();
      if (demoMode && models) {
        models = {
          ...models,
          ready: true,
          models: models.models.map((model) => ({ ...model, ready: true }))
        };
      }
      if (openIfMissing && models && !models.ready && !setupDismissed) {
        setupOpen = true;
      }
      if (models?.ready && settings.setupComplete !== true) {
        await updateSettings({ setupComplete: true }, false);
      }
    } catch (error) {
      toast(`Could not check local models: ${String(error)}`, 'error');
    }
  }

  function handleModelProgress(event: ModelDownloadProgress) {
    const previous = progress[event.model];
    progress[event.model] = {
      downloadedMb: event.downloadedMb,
      totalMb: event.totalMb,
      status: event.status,
      error: event.status === 'error' ? previous?.error : undefined
    };
  }

  async function updateSettings(patch: Partial<AppSettings>, notify = true) {
    try {
      settings = await saveSettings({ ...settings, ...patch });
      if (notify) toast('Settings saved', 'success', 1800);
    } catch (error) {
      toast(`Could not save settings: ${String(error)}`, 'error');
      throw error;
    }
  }

  async function downloadMissing() {
    if (!models) await refreshModels();
    downloadError = '';
    downloading = true;
    for (const required of REQUIRED_MODELS) {
      if (models?.models.find((model) => model.model === required.id)?.ready) continue;
      progress[required.id] = { downloadedMb: 0, totalMb: 0, status: 'queued' };
    }
    try {
      for (const required of REQUIRED_MODELS) {
        if (models?.models.find((model) => model.model === required.id)?.ready) continue;
        await downloadModel(required.id);
        progress[required.id] = {
          downloadedMb: progress[required.id]?.totalMb ?? 0,
          totalMb: progress[required.id]?.totalMb ?? 0,
          status: 'done'
        };
      }
      models = await checkModels();
      if (models.ready) {
        setupOpen = false;
        setupDismissed = true;
        await updateSettings({ setupComplete: true }, false);
        toast('All local models are ready. Verbatim is fully local now.', 'success', 5200);
      } else {
        downloadError = models.message || 'Some models are still missing. Try again from Settings → Models.';
      }
    } catch (error) {
      const message = String(error);
      downloadError = message;
      for (const required of REQUIRED_MODELS) {
        const item = progress[required.id];
        if (item && item.status === 'downloading') {
          progress[required.id] = { ...item, status: 'error', error: message };
        }
      }
      toast(`Download failed: ${message}`, 'error', 7000);
    } finally {
      downloading = false;
    }
  }

  async function downloadOne(model: RequiredModelId) {
    progress[model] = { downloadedMb: 0, totalMb: 0, status: 'downloading' };
    try {
      await downloadModel(model);
      await refreshModels();
      toast(`${model} is ready`, 'success');
    } catch (error) {
      progress[model] = {
        downloadedMb: 0,
        totalMb: 0,
        status: 'error',
        error: String(error)
      };
      toast(`Download failed: ${String(error)}`, 'error', 7000);
    }
  }

  function cancelSetup() {
    setupOpen = false;
    setupDismissed = true;
  }

  function switchTab(tab: TabId) {
    active = tab;
    localStorage.setItem('verbatim.tab', tab);
  }

  function iconPath(name: string): string {
    switch (name) {
      case 'mic':
        return 'M12 15a3 3 0 0 0 3-3V6a3 3 0 1 0-6 0v6a3 3 0 0 0 3 3Zm-6-3a6 6 0 0 0 12 0M12 18v3m-3 0h6';
      case 'note':
        return 'M6 3.5h8.5L19 8v12.5H6V3.5Zm8 0V8h5M9 12h7M9 15.5h7M9 8h2';
      case 'scan':
        return 'M4 8V5.5A1.5 1.5 0 0 1 5.5 4H8m8 0h2.5A1.5 1.5 0 0 1 20 5.5V8m0 8v2.5a1.5 1.5 0 0 1-1.5 1.5H16M8 20H5.5A1.5 1.5 0 0 1 4 18.5V16m3-4h10';
      default:
        return 'M10.5 13.5a4 4 0 0 0 5.7 0l2.1-2.1a4 4 0 0 0-5.7-5.7l-1 1m-2.2 4.8a4 4 0 0 0-5.7 0l-2.1 2.1a4 4 0 1 0 5.7 5.7l1-1';
    }
  }
</script>

<div class="app-shell">
  <header class="topbar">
    <div class="brand">
      <div class="logo" aria-hidden="true">
        <span></span><span></span><span></span><span></span><span></span><span></span><span></span>
      </div>
      <div class="brand-copy">
        <div class="wordmark">VERBATIM<span class="cursor"></span></div>
        <div class="tagline">Every word, on your machine.</div>
      </div>
    </div>

    <nav aria-label="Verbatim sections">
      {#each tabs as tab}
        <button
          type="button"
          class="tab"
          class:active={active === tab.id}
          onclick={() => switchTab(tab.id)}
          title={tab.shortcut ? `${tab.label} · ${tab.shortcut}` : tab.label}
        >
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.55">
            <path d={iconPath(tab.icon)} stroke-linecap="round" stroke-linejoin="round" />
          </svg>
          <span>{tab.label}</span>
          {#if tab.id === 'vision' && !modelsReady}
            <em class="tab-dot"></em>
          {/if}
        </button>
      {/each}
    </nav>

    <div class="top-right">
      {#if isTauri}
        <span class="local-chip" title="All processing stays on this machine">
          <span class="pulse"></span>
          LOCAL
        </span>
      {:else}
        <span class="local-chip preview" title="Browser preview — desktop commands are unavailable">
          <span class="pulse"></span>
          PREVIEW
        </span>
      {/if}
      <button class="models-chip" class:missing={!modelsReady} type="button" onclick={() => (setupOpen = true)}>
        <span></span>
        {models === null ? 'Checking models' : modelsReady ? 'Models ready' : 'Models missing'}
      </button>
    </div>
  </header>

  {#if readOnly}
    <div class="readonly-banner">
      <span class="readonly-icon">!</span>
      <span>Models are not downloaded. Verbatim is read-only until local AI models are installed.</span>
      <button type="button" onclick={() => (setupOpen = true)}>Open setup</button>
    </div>
  {/if}

  <main class="content">
    {#if loading}
      <div class="loading-screen">
        <div class="loading-mark">
          <span></span><span></span><span></span><span></span><span></span>
        </div>
        <p>Opening your local workspace…</p>
      </div>
    {:else if active === 'dictation'}
      <DictationTab {settings} {readOnly} onupdate={(patch) => updateSettings(patch, false)} />
    {:else if active === 'notes'}
      <NotesTab {settings} {readOnly} onmodels={() => (setupOpen = true)} />
    {:else if active === 'vision'}
      <VisionTab {settings} {readOnly} onmodels={() => (setupOpen = true)} />
    {:else}
      <SettingsTab
        {settings}
        {health}
        {models}
        {serviceStatus}
        {progress}
        {downloading}
        onupdate={(patch) => updateSettings(patch)}
        ondownload={downloadOne}
        ondelete={async (model) => {
          await deleteModel(model);
          await refreshModels();
          toast(`${model} removed`, 'info');
        }}
        onrefresh={() => refreshModels()}
        onrestart={async (name) => {
          const { restartService } = await import('./lib/api');
          await restartService(name);
          toast(`Restarting ${name.toUpperCase()}`, 'info');
          setTimeout(() => void refreshHealth(), 1200);
        }}
        onmodels={() => (setupOpen = true)}
      />
    {/if}
  </main>

  <StatusBar
    {health}
    {checking}
    {modelsReady}
    onmodels={() => (setupOpen = true)}
    onretry={() => {
      void refreshHealth();
      void refreshServiceStatus();
    }}
  />
</div>

<ModelSetup
  open={setupOpen}
  models={models?.models ?? []}
  {progress}
  {downloading}
  error={downloadError}
  dataDir={models?.dataDir ?? '~/.verbatim'}
  ondownload={downloadMissing}
  oncancel={cancelSetup}
/>

<ToastHost />

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    min-height: 0;
    overflow: hidden;
  }
  .topbar {
    align-items: center;
    background: rgba(17, 26, 47, 0.72);
    border-bottom: 1px solid var(--border);
    display: flex;
    flex: 0 0 61px;
    gap: 24px;
    padding: 0 17px;
    position: relative;
    z-index: 10;
  }
  .brand {
    align-items: center;
    display: flex;
    flex: 0 0 auto;
    gap: 10px;
    min-width: 205px;
  }
  .logo {
    align-items: center;
    background: linear-gradient(160deg, rgba(15, 52, 96, 0.8), rgba(11, 24, 46, 0.9));
    border: 1px solid rgba(78, 204, 163, 0.22);
    border-radius: 8px;
    display: flex;
    gap: 2px;
    height: 33px;
    justify-content: center;
    width: 33px;
  }
  .logo span {
    background: var(--mint);
    border-radius: 2px;
    height: 5px;
    opacity: 0.85;
    width: 1.5px;
  }
  .logo span:nth-child(2) {
    height: 11px;
  }
  .logo span:nth-child(3) {
    height: 18px;
  }
  .logo span:nth-child(4) {
    height: 22px;
  }
  .logo span:nth-child(5) {
    height: 13px;
  }
  .logo span:nth-child(6) {
    height: 8px;
  }
  .logo span:nth-child(7) {
    height: 4px;
    opacity: 0.5;
  }
  .wordmark {
    color: #fff;
    font-size: 12px;
    font-weight: 750;
    letter-spacing: 0.18em;
    line-height: 1.2;
  }
  .cursor {
    animation: blink 1.2s step-end infinite;
    background: var(--mint);
    display: inline-block;
    height: 10px;
    margin-left: 3px;
    opacity: 0.8;
    vertical-align: -1px;
    width: 1.5px;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
  .tagline {
    color: var(--muted);
    font-size: 8.5px;
    letter-spacing: 0.015em;
    line-height: 1.4;
  }
  nav {
    align-items: center;
    align-self: stretch;
    display: flex;
    gap: 2px;
  }
  .tab {
    align-items: center;
    align-self: stretch;
    background: transparent;
    border: 0;
    border-bottom: 2px solid transparent;
    color: var(--muted);
    cursor: pointer;
    display: flex;
    font-size: 10.5px;
    font-weight: 500;
    gap: 6px;
    padding: 0 10px;
    position: relative;
    transition: color 130ms ease;
  }
  .tab:hover {
    color: var(--text);
  }
  .tab.active {
    border-bottom-color: var(--mint);
    color: #fff;
  }
  .tab.active svg {
    color: var(--mint);
  }
  .tab-dot {
    background: var(--signal);
    border-radius: 50%;
    height: 3px;
    position: absolute;
    right: 5px;
    top: 18px;
    width: 3px;
  }
  .top-right {
    align-items: center;
    display: flex;
    gap: 8px;
    margin-left: auto;
  }
  .local-chip {
    align-items: center;
    border: 1px solid rgba(78, 204, 163, 0.23);
    border-radius: 999px;
    color: var(--mint);
    display: flex;
    font-family: var(--mono);
    font-size: 6.8px;
    gap: 5px;
    letter-spacing: 0.12em;
    padding: 4px 7px;
  }
  .local-chip.preview {
    border-color: rgba(243, 217, 164, 0.24);
    color: var(--sand);
  }
  .pulse {
    background: currentColor;
    border-radius: 50%;
    height: 4px;
    width: 4px;
  }
  .local-chip .pulse {
    animation: pulse 2.2s ease-in-out infinite;
    box-shadow: 0 0 0 3px rgba(78, 204, 163, 0.08);
  }
  @keyframes pulse {
    50% {
      opacity: 0.4;
    }
  }
  .models-chip {
    align-items: center;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--muted);
    cursor: pointer;
    display: flex;
    font-family: var(--mono);
    font-size: 7px;
    gap: 5px;
    letter-spacing: 0.04em;
    padding: 5px 7px;
  }
  .models-chip > span {
    background: var(--mint);
    border-radius: 50%;
    height: 4px;
    width: 4px;
  }
  .models-chip.missing {
    border-color: rgba(231, 181, 108, 0.2);
    color: #e7b56c;
  }
  .models-chip.missing > span {
    background: #e7b56c;
  }
  .models-chip:hover {
    border-color: rgba(78, 204, 163, 0.4);
    color: var(--text);
  }
  .readonly-banner {
    align-items: center;
    background: linear-gradient(90deg, rgba(233, 69, 96, 0.12), rgba(233, 69, 96, 0.04));
    border-bottom: 1px solid rgba(233, 69, 96, 0.18);
    color: #e8b2bc;
    display: flex;
    flex: 0 0 33px;
    font-size: 9.5px;
    gap: 8px;
    padding: 0 17px;
  }
  .readonly-icon {
    align-items: center;
    background: rgba(233, 69, 96, 0.18);
    border: 1px solid rgba(233, 69, 96, 0.3);
    border-radius: 50%;
    color: var(--signal);
    display: flex;
    font-family: var(--mono);
    font-size: 7px;
    height: 14px;
    justify-content: center;
    width: 14px;
  }
  .readonly-banner button {
    background: transparent;
    border: 1px solid rgba(233, 69, 96, 0.3);
    border-radius: 4px;
    color: #f4c7cf;
    cursor: pointer;
    font-size: 8.5px;
    margin-left: auto;
    padding: 3px 7px;
  }
  .readonly-banner button:hover {
    background: rgba(233, 69, 96, 0.12);
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }
  .loading-screen {
    align-items: center;
    color: var(--muted);
    display: flex;
    flex-direction: column;
    font-size: 10px;
    gap: 16px;
    height: 100%;
    justify-content: center;
    letter-spacing: 0.03em;
  }
  .loading-mark {
    align-items: center;
    display: flex;
    gap: 3px;
  }
  .loading-mark span {
    animation: load-wave 1s ease-in-out infinite;
    background: var(--mint);
    border-radius: 2px;
    height: 17px;
    opacity: 0.8;
    width: 2px;
  }
  .loading-mark span:nth-child(2) {
    animation-delay: 0.1s;
  }
  .loading-mark span:nth-child(3) {
    animation-delay: 0.2s;
  }
  .loading-mark span:nth-child(4) {
    animation-delay: 0.3s;
  }
  .loading-mark span:nth-child(5) {
    animation-delay: 0.4s;
  }
  @keyframes load-wave {
    0%,
    100% {
      transform: scaleY(0.4);
    }
    50% {
      transform: scaleY(1.3);
    }
  }
  @media (max-width: 920px) {
    .tagline {
      display: none;
    }
    .brand {
      min-width: auto;
    }
    .tab span {
      display: none;
    }
    .tab {
      padding: 0 12px;
    }
    .local-chip {
      display: none;
    }
  }
</style>
