<script lang="ts">
  let {
    active = false,
    level = 0,
    bars = 56,
    height = 72,
    color = 'mint'
  }: {
    active?: boolean;
    level?: number;
    bars?: number;
    height?: number;
    color?: 'mint' | 'signal' | 'paper';
  } = $props();

  const heights = Array.from({ length: 96 }, (_, index) => {
    const wave = Math.sin(index * 0.83) * 0.28 + Math.sin(index * 0.31 + 1.7) * 0.2;
    return 0.25 + Math.abs(wave) + ((index * 37) % 17) / 50;
  });

  let visible = $derived(heights.slice(0, bars));
</script>

<div
  class="waveform {color}"
  class:active
  style:--level={Math.min(1, Math.max(active ? 0.08 : 0.03, level))}
  style:height={`${height}px`}
  aria-label={active ? 'Live audio waveform' : 'Audio waveform, idle'}
  role="img"
>
  {#each visible as bar, index}
    <span
      class="bar"
      style:height={`${Math.round(bar * 100)}%`}
      style:animation-delay={`${(index % 13) * -0.11}s`}
      style:animation-duration={`${0.6 + (index % 7) * 0.09}s`}
    ></span>
  {/each}
</div>

<style>
  .waveform {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 3px;
    width: 100%;
    overflow: hidden;
  }
  .bar {
    flex: 1 1 2px;
    min-width: 2px;
    max-width: 5px;
    border-radius: 999px;
    background: var(--mint);
    opacity: 0.33;
    transform: scaleY(0.35);
    transition: opacity 180ms ease, background 180ms ease;
  }
  .bar:nth-child(3n) {
    background: var(--paper);
  }
  .waveform.active .bar {
    opacity: calc(0.45 + var(--level) * 0.55);
    animation-name: pulse;
    animation-timing-function: ease-in-out;
    animation-iteration-count: infinite;
  }
  .signal .bar {
    background: var(--signal);
  }
  .paper .bar {
    background: var(--paper);
  }
  .active .bar:nth-child(4n) {
    background: var(--signal);
    opacity: 0.58;
  }
  @keyframes pulse {
    0%,
    100% {
      transform: scaleY(calc(0.2 + var(--level) * 0.5));
    }
    50% {
      transform: scaleY(calc(0.55 + var(--level) * 1.6));
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bar {
      animation: none !important;
    }
  }
</style>
