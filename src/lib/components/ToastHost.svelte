<script lang="ts">
  import { dismissToast, toasts } from '../toast.svelte';
</script>

<div class="toasts" aria-live="polite" aria-atomic="false">
  {#each toasts as item (item.id)}
    <button type="button" class="toast {item.kind}" onclick={() => dismissToast(item.id)}>
      <span class="mark"></span>
      <span>{item.message}</span>
      <span class="close" aria-hidden="true">×</span>
    </button>
  {/each}
</div>

<style>
  .toasts {
    bottom: 38px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    pointer-events: none;
    position: fixed;
    right: 16px;
    z-index: 100;
  }
  .toast {
    align-items: center;
    animation: toast-in 180ms ease-out;
    background: #1b294a;
    border: 1px solid var(--border);
    border-radius: 7px;
    box-shadow: 0 12px 34px rgba(0, 0, 0, 0.34);
    color: var(--text);
    cursor: pointer;
    display: flex;
    font-size: 10.5px;
    gap: 8px;
    max-width: 360px;
    padding: 9px 11px;
    pointer-events: auto;
    text-align: left;
  }
  .mark {
    background: var(--mint);
    border-radius: 50%;
    flex: 0 0 auto;
    height: 5px;
    width: 5px;
  }
  .toast.error .mark {
    background: var(--signal);
  }
  .toast.info .mark {
    background: #7da7d9;
  }
  .close {
    color: var(--muted);
    font-size: 14px;
    line-height: 1;
    margin-left: 4px;
  }
  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
</style>
