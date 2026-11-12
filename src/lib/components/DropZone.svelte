<script lang="ts">
  import { onMount } from 'svelte';
  import { isTauri } from '../api';

  let {
    accept = 'audio/*,video/*',
    multiple = false,
    disabled = false,
    label = 'Drop audio or video here',
    hint = 'WAV, MP3, M4A, MP4, MOV, WebM · or click to browse',
    compact = false,
    onfiles,
    onpaths
  }: {
    accept?: string;
    multiple?: boolean;
    disabled?: boolean;
    label?: string;
    hint?: string;
    compact?: boolean;
    onfiles?: (files: File[]) => void;
    onpaths?: (paths: string[]) => void;
  } = $props();

  let dragDepth = $state(0);
  let input: HTMLInputElement;
  let active = $state(false);

  function emitFiles(fileList: FileList | null) {
    if (disabled || !fileList?.length) return;
    const files = Array.from(fileList);
    onfiles?.(multiple ? files : files.slice(0, 1));
  }

  function onDrop(event: DragEvent) {
    event.preventDefault();
    dragDepth = 0;
    active = false;
    emitFiles(event.dataTransfer?.files ?? null);
  }

  function onDragEnter(event: DragEvent) {
    event.preventDefault();
    dragDepth++;
    active = true;
  }

  function onDragLeave(event: DragEvent) {
    event.preventDefault();
    dragDepth = Math.max(0, dragDepth - 1);
    if (dragDepth === 0) active = false;
  }

  onMount(() => {
    if (!isTauri || !onpaths) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    import('@tauri-apps/api/webview').then(async ({ getCurrentWebview }) => {
      const stop = await getCurrentWebview().onDragDropEvent((event) => {
        if (disabled) return;
        if (event.payload.type === 'enter' || event.payload.type === 'over') {
          active = true;
        } else if (event.payload.type === 'leave') {
          active = false;
        } else if (event.payload.type === 'drop') {
          active = false;
          const paths = event.payload.paths ?? [];
          if (paths.length) onpaths(multiple ? paths : paths.slice(0, 1));
        }
      });
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<button
  type="button"
  class="dropzone"
  class:active
  class:compact
  class:disabled
  aria-label={label}
  {disabled}
  onclick={() => input?.click()}
  ondragover={(event) => {
    event.preventDefault();
    active = true;
  }}
  ondragenter={onDragEnter}
  ondragleave={onDragLeave}
  ondrop={onDrop}
>
  <div class="drop-icon" aria-hidden="true">
    <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="1.6">
      <path d="M12 16V4m0 0L7.5 8.5M12 4l4.5 4.5" stroke-linecap="round" stroke-linejoin="round" />
      <path d="M4 15v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3" stroke-linecap="round" />
    </svg>
  </div>
  <div class="drop-copy">
    <strong>{label}</strong>
    {#if !compact}<span>{hint}</span>{/if}
  </div>
  {#if !compact}<div class="drop-key">BROWSE</div>{/if}
  <input
    bind:this={input}
    type="file"
    {accept}
    {multiple}
    hidden
    onchange={(event) => {
      emitFiles(event.currentTarget.files);
      event.currentTarget.value = '';
    }}
  />
</button>

<style>
  .dropzone {
    align-items: center;
    background: linear-gradient(180deg, rgba(15, 52, 96, 0.15), rgba(22, 33, 62, 0.2));
    border: 1px dashed rgba(234, 234, 234, 0.16);
    border-radius: 14px;
    color: var(--text);
    cursor: pointer;
    display: flex;
    gap: 16px;
    min-height: 136px;
    padding: 20px;
    text-align: left;
    transition: border-color 160ms ease, background 160ms ease, transform 160ms ease;
    width: 100%;
  }
  .dropzone:hover,
  .dropzone.active {
    background: linear-gradient(180deg, rgba(78, 204, 163, 0.08), rgba(22, 33, 62, 0.32));
    border-color: rgba(78, 204, 163, 0.72);
  }
  .dropzone.active {
    transform: translateY(-1px);
  }
  .dropzone.compact {
    min-height: 62px;
    padding: 12px 14px;
  }
  .dropzone.disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
  .drop-icon {
    align-items: center;
    background: rgba(15, 52, 96, 0.55);
    border: 1px solid rgba(78, 204, 163, 0.24);
    border-radius: 12px;
    color: var(--mint);
    display: flex;
    flex: 0 0 48px;
    height: 48px;
    justify-content: center;
  }
  .compact .drop-icon {
    flex-basis: 34px;
    height: 34px;
  }
  .compact .drop-icon svg {
    height: 18px;
    width: 18px;
  }
  .drop-copy {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }
  .drop-copy strong {
    font-size: 13.5px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }
  .drop-copy span {
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.5;
  }
  .drop-key {
    border: 1px solid rgba(234, 234, 234, 0.14);
    border-radius: 5px;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 9px;
    letter-spacing: 0.12em;
    padding: 4px 6px;
  }
  .compact .drop-key {
    display: none;
  }
</style>
