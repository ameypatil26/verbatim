<script lang="ts">
  let { href = '#top', compact = false }: { href?: string; compact?: boolean } = $props();
</script>

<a {href} class="group inline-flex items-center gap-2.5" aria-label="Verbatim home">
  <span
    class="flex h-8 w-8 items-center justify-center gap-[2px] rounded-lg border border-mint/25 bg-accent/70 shadow-[inset_0_1px_0_rgba(255,255,255,0.06)] transition group-hover:border-mint/50"
  >
    <span class="h-1.5 w-[2px] rounded-full bg-mint/80"></span>
    <span class="h-3 w-[2px] rounded-full bg-mint"></span>
    <span class="h-[18px] w-[2px] rounded-full bg-mint"></span>
    <span class="h-[22px] w-[2px] rounded-full bg-mint"></span>
    <span class="h-3 w-[2px] rounded-full bg-paper/80"></span>
    <span class="h-2 w-[2px] rounded-full bg-mint/80"></span>
    <span class="h-1 w-[2px] rounded-full bg-mint/50"></span>
  </span>
  <span class="flex flex-col leading-none">
    <span class="text-[11px] font-bold tracking-[0.22em] text-white">VERBATIM</span>
    {#if !compact}
      <span class="mt-1 text-[8px] tracking-[0.02em] text-muted">Every word, on your machine.</span>
    {/if}
  </span>
</a>
