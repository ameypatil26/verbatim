<script lang="ts">
  import type { Mask } from '../types';

  let {
    src = '',
    masks = $bindable([]),
    onchange
  }: {
    src?: string;
    masks?: Mask[];
    onchange?: (masks: Mask[]) => void;
  } = $props();

  let naturalWidth = $state(1);
  let naturalHeight = $state(1);
  let dragging = $state(false);
  let start = $state({ x: 0, y: 0 });
  let current = $state({ x: 0, y: 0 });
  let overlay = $state<HTMLDivElement>();

  function point(event: PointerEvent): { x: number; y: number } {
    if (!overlay) return { x: 0, y: 0 };
    const bounds = overlay.getBoundingClientRect();
    return {
      x: Math.max(0, Math.min(bounds.width, event.clientX - bounds.left)),
      y: Math.max(0, Math.min(bounds.height, event.clientY - bounds.top))
    };
  }

  function onPointerDown(event: PointerEvent) {
    if (!src || !overlay) return;
    event.preventDefault();
    overlay.setPointerCapture(event.pointerId);
    const value = point(event);
    start = value;
    current = value;
    dragging = true;
  }

  function onPointerMove(event: PointerEvent) {
    if (!dragging) return;
    current = point(event);
  }

  function onPointerUp(event: PointerEvent) {
    if (!dragging || !overlay) return;
    dragging = false;
    const bounds = overlay.getBoundingClientRect();
    const end = point(event);
    const x = Math.min(start.x, end.x);
    const y = Math.min(start.y, end.y);
    const w = Math.abs(end.x - start.x);
    const h = Math.abs(end.y - start.y);
    if (w < 6 || h < 6) return;
    const scaleX = naturalWidth / Math.max(1, bounds.width);
    const scaleY = naturalHeight / Math.max(1, bounds.height);
    const mask: Mask = {
      x: Math.round(x * scaleX),
      y: Math.round(y * scaleY),
      w: Math.round(w * scaleX),
      h: Math.round(h * scaleY)
    };
    masks = [...masks, mask];
    onchange?.(masks);
  }

  function clear() {
    masks = [];
    onchange?.(masks);
  }

  function removeAt(index: number) {
    masks = masks.filter((_, position) => position !== index);
    onchange?.(masks);
  }

  function boxStyle(mask: Mask): string {
    return `left:${(mask.x / naturalWidth) * 100}%;top:${(mask.y / naturalHeight) * 100}%;width:${(mask.w / naturalWidth) * 100}%;height:${(mask.h / naturalHeight) * 100}%`;
  }

  let dragStyle = $derived(
    `left:${Math.min(start.x, current.x)}px;top:${Math.min(start.y, current.y)}px;width:${Math.abs(
      current.x - start.x
    )}px;height:${Math.abs(current.y - start.y)}px`
  );
</script>

{#if src}
  <div class="mask-editor">
    <div class="canvas-wrap">
      <img
        {src}
        alt="OCR source preview"
        draggable="false"
        onload={(event) => {
          const image = event.currentTarget as HTMLImageElement;
          naturalWidth = image.naturalWidth || 1;
          naturalHeight = image.naturalHeight || 1;
        }}
      />
      <div
        class="overlay"
        bind:this={overlay}
        role="application"
        aria-label="Draw ignore regions on the image"
        onpointerdown={onPointerDown}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        onpointercancel={() => (dragging = false)}
      >
        {#each masks as mask, index}
          <button
            type="button"
            class="mask"
            style={boxStyle(mask)}
            title="Remove ignore region"
            aria-label={`Remove ignore region ${index + 1}`}
            onpointerdown={(event) => event.stopPropagation()}
            onclick={(event) => {
              event.stopPropagation();
              removeAt(index);
            }}
          ></button>
        {/each}
        {#if dragging}
          <span class="drag-box" style={dragStyle}></span>
        {/if}
      </div>
    </div>
    <div class="mask-toolbar">
      <span>{masks.length ? `${masks.length} ignore region${masks.length === 1 ? '' : 's'}` : 'Drag to exclude watermarks or headers'}</span>
      {#if masks.length}
        <button type="button" onclick={clear}>Clear masks</button>
      {/if}
    </div>
  </div>
{:else}
  <div class="no-preview">Text extracted from this document. Ignore regions are available for images.</div>
{/if}

<style>
  .mask-editor {
    border-bottom: 1px solid var(--border-soft);
  }
  .canvas-wrap {
    background: #0a0f1e;
    max-height: 285px;
    overflow: auto;
    position: relative;
  }
  img {
    display: block;
    max-width: 100%;
    user-select: none;
  }
  .overlay {
    cursor: crosshair;
    inset: 0;
    position: absolute;
    touch-action: none;
  }
  .mask {
    background: rgba(233, 69, 96, 0.3);
    border: 1px solid rgba(233, 69, 96, 0.9);
    cursor: pointer;
    padding: 0;
    position: absolute;
  }
  .mask:hover {
    background: rgba(233, 69, 96, 0.5);
  }
  .mask::after {
    color: #fff;
    content: '×';
    font-family: var(--mono);
    font-size: 10px;
    left: 50%;
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
  }
  .drag-box {
    background: rgba(233, 69, 96, 0.22);
    border: 1px dashed rgba(233, 69, 96, 0.95);
    pointer-events: none;
    position: absolute;
  }
  .mask-toolbar {
    align-items: center;
    color: var(--muted);
    display: flex;
    font-size: 8.5px;
    justify-content: space-between;
    padding: 6px 12px;
  }
  .mask-toolbar button {
    background: transparent;
    border: 1px solid rgba(233, 69, 96, 0.3);
    border-radius: 3px;
    color: #eda5b1;
    cursor: pointer;
    font-size: 7.5px;
    padding: 2px 6px;
  }
  .no-preview {
    border-bottom: 1px solid var(--border-soft);
    color: var(--muted);
    font-size: 8.5px;
    padding: 10px 14px;
  }
</style>
