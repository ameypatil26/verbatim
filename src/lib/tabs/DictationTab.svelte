<script lang="ts">
  import { onMount } from 'svelte';
  import type { AppSettings, DictationEvent } from '../types';
  import {
    copyText,
    isTauri,
    onDictation,
    onDictationLevel,
    proofreadText,
    startDictation,
    stopDictation,
    transcribeFile,
    undoLastDictation
  } from '../api';
  import { toast } from '../toast.svelte';
  import Waveform from '../components/Waveform.svelte';

  let {
    settings,
    readOnly = false,
    onupdate
  }: {
    settings: AppSettings;
    readOnly?: boolean;
    onupdate?: (patch: Partial<AppSettings>) => void | Promise<void>;
  } = $props();

  let status = $state<'idle' | 'recording' | 'processing' | 'error'>('idle');
  let level = $state(0);
  let lastText = $state('');
  let error = $state('');
  let elapsed = $state(0);
  let source = $state<'microphone' | 'system'>(settings.audioSource);
  let mode = $state<'tap' | 'push'>(settings.dictationMode);
  let proofread = $state(settings.proofread);
  let busy = $state(false);
  let browserRecording = false;
  let mediaRecorder: MediaRecorder | null = null;
  let mediaStream: MediaStream | null = null;
  let audioContext: AudioContext | null = null;
  let analyserTimer: ReturnType<typeof setInterval> | undefined;
  let elapsedTimer: ReturnType<typeof setInterval> | undefined;
  let startedAt = 0;
  let chunks: Blob[] = [];

  $effect(() => {
    source = settings.audioSource;
    mode = settings.dictationMode;
    proofread = settings.proofread;
  });

  onMount(() => {
    const unlistenState = onDictation(handleDictationEvent);
    const unlistenLevel = onDictationLevel((value) => {
      level = Math.max(0, Math.min(1, value));
    });
    const keyHandler = (event: KeyboardEvent) => {
      if (isTauri) return;
      if (event.ctrlKey && event.shiftKey && event.key.toLowerCase() === 'd') {
        event.preventDefault();
        void toggle();
      }
    };
    window.addEventListener('keydown', keyHandler);
    return () => {
      unlistenState();
      unlistenLevel();
      window.removeEventListener('keydown', keyHandler);
      cleanupBrowserAudio();
    };
  });

  function handleDictationEvent(event: DictationEvent) {
    if (event.state === 'recording') {
      status = 'recording';
      error = '';
      startElapsed();
    } else if (event.state === 'processing') {
      status = 'processing';
      stopElapsed();
    } else if (event.state === 'idle') {
      status = 'idle';
      stopElapsed();
      if (event.text) {
        lastText = event.text;
        level = 0;
      }
      if (event.error) error = event.error;
    } else if (event.state === 'error') {
      status = 'error';
      error = event.error || 'Dictation failed.';
      stopElapsed();
      if (event.text) lastText = event.text;
    }
    if (event.source === 'microphone' || event.source === 'system') source = event.source;
  }

  function startElapsed() {
    if (elapsedTimer) clearInterval(elapsedTimer);
    startedAt = Date.now();
    elapsed = 0;
    elapsedTimer = setInterval(() => {
      elapsed = Math.floor((Date.now() - startedAt) / 1000);
    }, 250);
  }

  function stopElapsed() {
    if (elapsedTimer) clearInterval(elapsedTimer);
    elapsedTimer = undefined;
  }

  async function toggle() {
    if (status === 'processing' || busy) return;
    if (readOnly) {
      toast('Download the local models before using dictation.', 'error');
      return;
    }
    if (status === 'recording') {
      await stop();
    } else {
      await start();
    }
  }

  async function start() {
    error = '';
    busy = true;
    try {
      if (isTauri) {
        await startDictation(source);
        status = 'recording';
        startElapsed();
      } else {
        await startBrowserRecording();
      }
    } catch (problem) {
      error = String(problem);
      status = 'error';
      toast(error, 'error');
    } finally {
      busy = false;
    }
  }

  async function stop() {
    busy = true;
    error = '';
    stopElapsed();
    try {
      if (isTauri) {
        status = 'processing';
        const text = await stopDictation();
        if (text) lastText = text;
        status = 'idle';
      } else {
        await stopBrowserRecording();
      }
    } catch (problem) {
      error = String(problem);
      status = 'error';
      toast(error, 'error', 6000);
    } finally {
      busy = false;
    }
  }

  async function startBrowserRecording() {
    if (!navigator.mediaDevices?.getUserMedia) {
      throw new Error('Microphone capture is not available in this browser.');
    }
    mediaStream =
      source === 'system' && navigator.mediaDevices.getDisplayMedia
        ? await navigator.mediaDevices.getDisplayMedia({ audio: true, video: true })
        : await navigator.mediaDevices.getUserMedia({ audio: true });
    chunks = [];
    const mime = MediaRecorder.isTypeSupported('audio/webm;codecs=opus') ? 'audio/webm;codecs=opus' : '';
    mediaRecorder = new MediaRecorder(mediaStream, mime ? { mimeType: mime } : undefined);
    mediaRecorder.ondataavailable = (event) => {
      if (event.data.size) chunks.push(event.data);
    };
    mediaRecorder.start(250);
    browserRecording = true;
    status = 'recording';
    startElapsed();
    level = 0.25;

    audioContext = new AudioContext();
    const context = audioContext;
    const analyser = context.createAnalyser();
    analyser.fftSize = 512;
    const input = context.createMediaStreamSource(mediaStream);
    input.connect(analyser);
    const data = new Uint8Array(analyser.fftSize);
    analyserTimer = setInterval(() => {
      analyser.getByteTimeDomainData(data);
      let sum = 0;
      for (const value of data) {
        const normalized = (value - 128) / 128;
        sum += normalized * normalized;
      }
      const rms = Math.sqrt(sum / data.length);
      level = Math.max(0.04, Math.min(1, rms * 4.8));
    }, 60);
  }

  function stopBrowserRecording(): Promise<void> {
    return new Promise((resolve, reject) => {
      const recorder = mediaRecorder;
      if (!recorder) {
        reject(new Error('No browser recording is active.'));
        return;
      }
      status = 'processing';
      recorder.onstop = async () => {
        try {
          cleanupBrowserAudio();
          const blob = new Blob(chunks, { type: recorder.mimeType || 'audio/webm' });
          if (!blob.size) throw new Error('The recording was empty.');
          const transcription = await transcribeFile(blob, settings, settings.language, 'dictation.webm');
          let text = transcription.text.trim();
          if (proofread && text) {
            try {
              text = await proofreadText(text, settings);
            } catch {
              // Keep the raw transcript if Ollama is unavailable.
            }
          }
          lastText = text;
          status = 'idle';
          await copyText(text);
          toast('Transcript copied. Type into another app from the desktop build.', 'success', 5200);
          resolve();
        } catch (error) {
          status = 'error';
          reject(error);
        }
      };
      recorder.stop();
    });
  }

  function cleanupBrowserAudio() {
    browserRecording = false;
    if (analyserTimer) clearInterval(analyserTimer);
    analyserTimer = undefined;
    if (audioContext) void audioContext.close();
    audioContext = null;
    mediaStream?.getTracks().forEach((track) => track.stop());
    mediaStream = null;
    mediaRecorder = null;
    level = 0;
  }

  async function copyLast() {
    if (!lastText) return;
    await copyText(lastText);
    toast('Copied the last transcript', 'success');
  }

  async function undo() {
    if (!lastText) return;
    if (!isTauri) {
      toast('Undo is available in the desktop app.', 'info');
      return;
    }
    try {
      await undoLastDictation();
      toast('Sent undo to the focused app', 'info');
    } catch (problem) {
      toast(String(problem), 'error');
    }
  }

  function setSource(next: 'microphone' | 'system') {
    if (status === 'recording' || status === 'processing') return;
    source = next;
    void onupdate?.({ audioSource: next });
  }

  function setMode(next: 'tap' | 'push') {
    mode = next;
    void onupdate?.({ dictationMode: next });
  }

  function setProofread(next: boolean) {
    proofread = next;
    void onupdate?.({ proofread: next });
  }

  let title = $derived(
    status === 'recording'
      ? 'Listening locally'
      : status === 'processing'
        ? 'Turning speech into text'
        : status === 'error'
          ? 'Dictation needs attention'
          : 'Ready when you are'
  );
  let subtitle = $derived(
    status === 'recording'
      ? source === 'system'
        ? 'Capturing system audio · nothing leaves this machine'
        : 'Microphone live · nothing leaves this machine'
      : status === 'processing'
        ? 'ASR is running locally, then proofread is optional'
        : status === 'error'
          ? error
          : 'Press the hotkey, speak, and Verbatim types into the focused app.'
  );
</script>

<div class="dictation">
  <div class="title-row">
    <div>
      <span class="eyebrow">DICTATION · LOCAL SPEECH</span>
      <h1>Your voice, typed anywhere.</h1>
    </div>
    <div class="hotkey-badge">
      <span>HOTKEY</span>
      <kbd>{settings.hotkeyDictation}</kbd>
    </div>
  </div>

  <div class="grid">
    <section class="hero-card" class:recording={status === 'recording'} class:error={status === 'error'}>
      <div class="hero-glow"></div>
      <div class="status-line">
        <span class="status-dot" class:live={status === 'recording'} class:processing={status === 'processing'}></span>
        <span>{status === 'idle' ? 'IDLE' : status.toUpperCase()}</span>
        {#if status === 'recording'}
          <span class="elapsed">{Math.floor(elapsed / 60).toString().padStart(2, '0')}:{(elapsed % 60)
              .toString()
              .padStart(2, '0')}</span>
        {/if}
        {#if !isTauri}<span class="preview-tag">PREVIEW</span>{/if}
      </div>

      <div class="hero-copy">
        <h2>{title}</h2>
        <p>{subtitle}</p>
      </div>

      <div class="orb-wrap">
        <div class="orb-rings" class:active={status === 'recording'}>
          <span></span><span></span><span></span>
        </div>
        <button
          type="button"
          class="mic-button"
          class:recording={status === 'recording'}
          class:processing={status === 'processing'}
          disabled={busy || status === 'processing' || readOnly}
          onclick={toggle}
          aria-label={status === 'recording' ? 'Stop dictation' : 'Start dictation'}
        >
          {#if status === 'processing'}
            <span class="spinner"></span>
          {:else}
            <svg viewBox="0 0 32 32" width="29" height="29" fill="none" stroke="currentColor" stroke-width="1.7">
              <rect x="11" y="5" width="10" height="15" rx="5" />
              <path d="M7 15.5a9 9 0 0 0 18 0M16 24.5V28m-5 0h10" stroke-linecap="round" />
            </svg>
          {/if}
        </button>
      </div>

      <Waveform active={status === 'recording'} level={level} bars={54} height={60} />

      <div class="quick-controls">
        <div class="control-group">
          <span class="control-label">SOURCE</span>
          <div class="segmented">
            <button
              type="button"
              class:active={source === 'microphone'}
              disabled={status === 'recording' || status === 'processing'}
              onclick={() => setSource('microphone')}>Microphone</button
            >
            <button
              type="button"
              class:active={source === 'system'}
              disabled={status === 'recording' || status === 'processing'}
              onclick={() => setSource('system')}>System audio</button
            >
          </div>
        </div>
        <div class="control-group">
          <span class="control-label">MODE</span>
          <div class="segmented compact">
            <button type="button" class:active={mode === 'tap'} onclick={() => setMode('tap')}>Tap</button>
            <button type="button" class:active={mode === 'push'} onclick={() => setMode('push')}>Hold</button>
          </div>
        </div>
        <label class="proofread">
          <input type="checkbox" checked={proofread} onchange={(event) => setProofread(event.currentTarget.checked)} />
          <span class="switch"></span>
          Proofread
        </label>
      </div>

      {#if readOnly}
        <div class="readonly-note">Models are not installed. Dictation is disabled until setup completes.</div>
      {/if}
    </section>

    <aside class="side">
      <section class="output-card">
        <header>
          <div>
            <span class="eyebrow">LAST OUTPUT</span>
            <h3>{lastText ? 'Typed into the focused app' : 'Nothing yet'}</h3>
          </div>
          {#if lastText}
            <span class="word-count">{lastText.trim().split(/\s+/).length} words</span>
          {/if}
        </header>
        <div class="output-text" class:empty={!lastText}>
          {#if lastText}
            {lastText}
          {:else}
            <span>Your next transcript will appear here. Press <kbd>{settings.hotkeyDictation}</kbd> anywhere.</span>
          {/if}
        </div>
        <footer>
          <button type="button" class="primary" onclick={copyLast} disabled={!lastText}>
            <svg viewBox="0 0 18 18" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.4">
              <rect x="6" y="6" width="8" height="9" rx="1.5" />
              <path d="M4 12H3.5A1.5 1.5 0 0 1 2 10.5v-7A1.5 1.5 0 0 1 3.5 2h7A1.5 1.5 0 0 1 12 3.5V4" />
            </svg>
            Copy
          </button>
          <button type="button" onclick={undo} disabled={!lastText || !isTauri}>Undo</button>
          <span class="auto-note">AUTO-TYPED</span>
        </footer>
      </section>

      <section class="info-card">
        <div class="info-icon">
          <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M12 3 4 6.5v5.2c0 4.6 3.2 7.6 8 9.3 4.8-1.7 8-4.7 8-9.3V6.5L12 3Z" stroke-linejoin="round" />
            <path d="m8.5 12 2.2 2.2 4.8-5" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </div>
        <div>
          <strong>Nothing leaves this machine</strong>
          <p>SenseVoice transcribes on-device. Proofreading uses your configured local Ollama model — never a cloud API.</p>
        </div>
      </section>

      <section class="system-card">
        <span class="eyebrow">SYSTEM AUDIO</span>
        <p>
          {#if source === 'system'}
            Verbatim looks for a monitor / loopback input. On Windows enable Stereo Mix or a virtual loopback device; on Linux choose a PulseAudio monitor source.
          {:else}
            Microphone is capturing from your default input. Switch to system audio to transcribe calls, videos, or anything playing.
          {/if}
        </p>
      </section>
    </aside>
  </div>
</div>

<style>
  .dictation {
    display: flex;
    flex-direction: column;
    gap: 17px;
    height: 100%;
    overflow-y: auto;
    padding: 20px 22px 22px;
  }
  .title-row {
    align-items: flex-end;
    display: flex;
    flex: 0 0 auto;
    justify-content: space-between;
  }
  h1 {
    color: #fff;
    font-size: 22px;
    font-weight: 650;
    letter-spacing: -0.025em;
    margin: 4px 0 0;
  }
  .hotkey-badge {
    align-items: center;
    border: 1px solid var(--border);
    border-radius: 6px;
    display: flex;
    gap: 7px;
    padding: 5px 8px;
  }
  .hotkey-badge span {
    color: var(--muted);
    font-family: var(--mono);
    font-size: 6.5px;
    letter-spacing: 0.12em;
  }
  kbd {
    background: rgba(15, 52, 96, 0.5);
    border: 1px solid rgba(234, 234, 234, 0.14);
    border-radius: 4px;
    color: var(--text);
    font-family: var(--mono);
    font-size: 8px;
    padding: 2px 5px;
  }
  .grid {
    display: grid;
    flex: 1;
    gap: 14px;
    grid-template-columns: minmax(0, 1.55fr) minmax(280px, 0.8fr);
    min-height: 0;
  }
  .hero-card {
    background: linear-gradient(160deg, rgba(22, 33, 62, 0.8), rgba(12, 20, 39, 0.72));
    border: 1px solid var(--border);
    border-radius: 14px;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    padding: 16px 20px 17px;
    position: relative;
  }
  .hero-card::before {
    border: 1px solid rgba(78, 204, 163, 0.08);
    border-radius: 14px;
    content: '';
    inset: 5px;
    pointer-events: none;
    position: absolute;
  }
  .hero-card.recording {
    border-color: rgba(78, 204, 163, 0.23);
  }
  .hero-card.error {
    border-color: rgba(233, 69, 96, 0.24);
  }
  .hero-glow {
    background: radial-gradient(circle, rgba(78, 204, 163, 0.12), transparent 70%);
    height: 360px;
    left: 50%;
    pointer-events: none;
    position: absolute;
    top: 24%;
    transform: translateX(-50%);
    width: 460px;
  }
  .status-line {
    align-items: center;
    color: var(--muted);
    display: flex;
    font-family: var(--mono);
    font-size: 7.5px;
    gap: 6px;
    letter-spacing: 0.12em;
    position: relative;
    z-index: 1;
  }
  .status-dot {
    background: var(--muted-low);
    border-radius: 50%;
    height: 5px;
    width: 5px;
  }
  .status-dot.live {
    animation: live 1.3s ease-in-out infinite;
    background: var(--mint);
    box-shadow: 0 0 0 3px rgba(78, 204, 163, 0.1);
  }
  .status-dot.processing {
    background: var(--sand);
  }
  @keyframes live {
    50% {
      opacity: 0.35;
    }
  }
  .elapsed {
    color: var(--mint);
    margin-left: auto;
  }
  .preview-tag {
    border: 1px solid rgba(243, 217, 164, 0.26);
    border-radius: 3px;
    color: var(--sand);
    font-size: 6px;
    padding: 2px 4px;
  }
  .hero-copy {
    margin: 17px 0 4px;
    position: relative;
    text-align: center;
    z-index: 1;
  }
  .hero-copy h2 {
    color: #fff;
    font-size: 19px;
    font-weight: 600;
    letter-spacing: -0.02em;
    margin: 0 0 5px;
  }
  .hero-copy p {
    color: var(--muted);
    font-size: 10px;
    line-height: 1.5;
    margin: 0 auto;
    max-width: 340px;
  }
  .hero-card.error .hero-copy p {
    color: #e39aa6;
  }
  .orb-wrap {
    align-items: center;
    display: flex;
    flex: 1;
    justify-content: center;
    min-height: 150px;
    position: relative;
  }
  .orb-rings {
    height: 154px;
    position: absolute;
    width: 154px;
  }
  .orb-rings span {
    border: 1px solid rgba(78, 204, 163, 0.09);
    border-radius: 50%;
    inset: 0;
    position: absolute;
  }
  .orb-rings span:nth-child(2) {
    inset: 15px;
    border-color: rgba(78, 204, 163, 0.07);
  }
  .orb-rings span:nth-child(3) {
    inset: 30px;
    border-color: rgba(78, 204, 163, 0.05);
  }
  .orb-rings.active span {
    animation: ring 2.1s ease-out infinite;
    border-color: rgba(78, 204, 163, 0.34);
  }
  .orb-rings.active span:nth-child(2) {
    animation-delay: 0.35s;
  }
  .orb-rings.active span:nth-child(3) {
    animation-delay: 0.7s;
  }
  @keyframes ring {
    from {
      opacity: 0.75;
      transform: scale(0.75);
    }
    to {
      opacity: 0;
      transform: scale(1.12);
    }
  }
  .mic-button {
    align-items: center;
    background: linear-gradient(160deg, #164c6e, #0f3460);
    border: 1px solid rgba(78, 204, 163, 0.38);
    border-radius: 50%;
    box-shadow:
      0 0 0 8px rgba(78, 204, 163, 0.025),
      0 14px 40px rgba(0, 0, 0, 0.32),
      inset 0 1px 0 rgba(255, 255, 255, 0.06);
    color: var(--mint);
    cursor: pointer;
    display: flex;
    height: 93px;
    justify-content: center;
    position: relative;
    transition: transform 140ms ease, box-shadow 140ms ease;
    width: 93px;
    z-index: 1;
  }
  .mic-button:hover:not(:disabled) {
    box-shadow:
      0 0 0 11px rgba(78, 204, 163, 0.04),
      0 16px 44px rgba(0, 0, 0, 0.38);
    transform: translateY(-1px);
  }
  .mic-button.recording {
    background: linear-gradient(160deg, #1b6453, #0f3f3a);
    border-color: rgba(78, 204, 163, 0.75);
    color: #bff5e3;
    box-shadow:
      0 0 0 10px rgba(78, 204, 163, 0.055),
      0 0 45px rgba(78, 204, 163, 0.2);
  }
  .mic-button.processing {
    color: var(--sand);
    cursor: wait;
  }
  .mic-button:disabled {
    cursor: default;
    opacity: 0.5;
  }
  .spinner {
    animation: spin 0.9s linear infinite;
    border: 2px solid rgba(243, 217, 164, 0.22);
    border-radius: 50%;
    border-top-color: var(--sand);
    height: 25px;
    width: 25px;
  }
  .quick-controls {
    align-items: flex-end;
    border-top: 1px solid var(--border-soft);
    display: flex;
    gap: 18px;
    padding-top: 13px;
    position: relative;
    z-index: 1;
  }
  .control-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .control-label {
    color: var(--muted);
    font-family: var(--mono);
    font-size: 6.5px;
    letter-spacing: 0.13em;
  }
  .segmented {
    background: rgba(8, 13, 28, 0.4);
    border: 1px solid var(--border-soft);
    border-radius: 5px;
    display: flex;
    padding: 2px;
  }
  .segmented button {
    background: transparent;
    border: 0;
    border-radius: 3px;
    color: var(--muted);
    cursor: pointer;
    font-size: 8.5px;
    padding: 4px 7px;
  }
  .segmented button.active {
    background: rgba(15, 52, 96, 0.75);
    color: var(--text);
  }
  .segmented button:disabled {
    cursor: default;
    opacity: 0.45;
  }
  .segmented.compact button {
    padding: 4px 9px;
  }
  .proofread {
    align-items: center;
    color: var(--muted);
    cursor: pointer;
    display: flex;
    font-size: 9px;
    gap: 7px;
    margin-left: auto;
    padding-bottom: 4px;
  }
  .proofread input {
    height: 0;
    opacity: 0;
    position: absolute;
    width: 0;
  }
  .switch {
    background: rgba(140, 152, 171, 0.16);
    border: 1px solid var(--border);
    border-radius: 999px;
    height: 14px;
    position: relative;
    transition: background 140ms ease;
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
    transition: transform 140ms ease, background 140ms ease;
    width: 8px;
  }
  .proofread input:checked + .switch {
    background: rgba(78, 204, 163, 0.25);
    border-color: rgba(78, 204, 163, 0.4);
  }
  .proofread input:checked + .switch::after {
    background: var(--mint);
    transform: translateX(11px);
  }
  .readonly-note {
    color: #e6b36b;
    font-size: 9px;
    margin-top: 10px;
    text-align: center;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
  }
  .output-card {
    background: linear-gradient(165deg, rgba(22, 33, 62, 0.72), rgba(13, 21, 41, 0.7));
    border: 1px solid var(--border);
    border-radius: 12px;
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 210px;
    overflow: hidden;
  }
  .output-card header {
    align-items: flex-start;
    border-bottom: 1px solid var(--border-soft);
    display: flex;
    justify-content: space-between;
    padding: 12px 14px 10px;
  }
  .output-card h3 {
    color: var(--text);
    font-size: 11px;
    font-weight: 600;
    margin: 3px 0 0;
  }
  .word-count {
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 7px;
    padding: 2px 4px;
  }
  .output-text {
    color: #d3dae3;
    flex: 1;
    font-size: 11px;
    line-height: 1.65;
    overflow-y: auto;
    padding: 12px 14px;
    white-space: pre-wrap;
  }
  .output-text.empty {
    align-items: center;
    color: var(--muted-low);
    display: flex;
    font-size: 9.5px;
    font-style: italic;
    justify-content: center;
    line-height: 1.7;
    padding: 22px;
    text-align: center;
  }
  .output-text kbd {
    font-style: normal;
  }
  .output-card footer {
    align-items: center;
    border-top: 1px solid var(--border-soft);
    display: flex;
    gap: 5px;
    padding: 8px 10px;
  }
  .output-card footer button {
    align-items: center;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--muted);
    cursor: pointer;
    display: flex;
    font-size: 8.5px;
    gap: 4px;
    padding: 4px 7px;
  }
  .output-card footer button.primary {
    background: var(--accent);
    border-color: rgba(78, 204, 163, 0.24);
    color: var(--mint);
  }
  .output-card footer button:disabled {
    cursor: default;
    opacity: 0.35;
  }
  .auto-note {
    color: var(--muted-low);
    font-family: var(--mono);
    font-size: 6px;
    letter-spacing: 0.1em;
    margin-left: auto;
  }
  .info-card {
    align-items: flex-start;
    background: rgba(78, 204, 163, 0.035);
    border: 1px solid rgba(78, 204, 163, 0.11);
    border-radius: 10px;
    display: flex;
    gap: 10px;
    padding: 11px 12px;
  }
  .info-icon {
    color: var(--mint);
    flex: 0 0 auto;
    margin-top: 1px;
  }
  .info-card strong {
    color: #dbe6e2;
    display: block;
    font-size: 9.5px;
    font-weight: 600;
    margin-bottom: 3px;
  }
  .info-card p {
    color: var(--muted);
    font-size: 8.5px;
    line-height: 1.55;
    margin: 0;
  }
  .system-card {
    border: 1px solid var(--border-soft);
    border-radius: 9px;
    padding: 9px 11px;
  }
  .system-card p {
    color: var(--muted);
    font-size: 8.5px;
    line-height: 1.55;
    margin: 4px 0 0;
  }
  @media (max-height: 680px) {
    .hero-copy {
      margin-top: 8px;
    }
    .orb-wrap {
      min-height: 120px;
    }
    .orb-rings {
      height: 126px;
      width: 126px;
    }
    .mic-button {
      height: 78px;
      width: 78px;
    }
    .system-card {
      display: none;
    }
  }
  @media (max-width: 880px) {
    .grid {
      grid-template-columns: minmax(0, 1fr);
    }
    .side {
      display: none;
    }
  }
</style>
