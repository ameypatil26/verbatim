<script lang="ts">
  import { renderMarkdown } from '../markdown';

  let { source = '', class: className = '' }: { source?: string; class?: string } = $props();
  let html = $derived(renderMarkdown(source));
</script>

<div class="markdown {className}">
  {@html html}
</div>

<style>
  .markdown {
    color: var(--text);
    font-size: 13.5px;
    line-height: 1.75;
    overflow-wrap: anywhere;
  }
  .markdown :global(h1) {
    color: #fff;
    font-size: 24px;
    font-weight: 650;
    letter-spacing: -0.025em;
    line-height: 1.22;
    margin: 0 0 18px;
  }
  .markdown :global(h2) {
    border-top: 1px solid var(--border-soft);
    color: var(--mint);
    font-size: 15px;
    font-weight: 650;
    letter-spacing: 0.02em;
    margin: 24px 0 10px;
    padding-top: 16px;
  }
  .markdown :global(h3) {
    color: #fff;
    font-size: 13.5px;
    font-weight: 650;
    margin: 18px 0 8px;
  }
  .markdown :global(h4) {
    color: var(--text);
    font-size: 12.5px;
    font-weight: 600;
    margin: 14px 0 6px;
  }
  .markdown :global(p) {
    color: #cfd6df;
    margin: 0 0 12px;
  }
  .markdown :global(ul),
  .markdown :global(ol) {
    color: #cfd6df;
    margin: 0 0 14px;
    padding-left: 20px;
  }
  .markdown :global(li) {
    margin-bottom: 5px;
    padding-left: 2px;
  }
  .markdown :global(li::marker) {
    color: var(--mint);
  }
  .markdown :global(li.task) {
    list-style: none;
    margin-left: -18px;
  }
  .markdown :global(li.task input) {
    accent-color: var(--mint);
    margin-right: 6px;
    vertical-align: -1px;
  }
  .markdown :global(strong) {
    color: #fff;
    font-weight: 650;
  }
  .markdown :global(em) {
    color: #d8e6e1;
  }
  .markdown :global(a) {
    color: var(--mint);
    text-decoration: underline;
    text-decoration-color: rgba(78, 204, 163, 0.4);
    text-underline-offset: 2px;
  }
  .markdown :global(code) {
    background: rgba(15, 52, 96, 0.52);
    border: 1px solid var(--border-soft);
    border-radius: 4px;
    color: #f3d9a4;
    font-family: var(--mono);
    font-size: 0.88em;
    padding: 1px 5px;
  }
  .markdown :global(pre.code-block) {
    background: var(--bg-deep);
    border: 1px solid var(--border-soft);
    border-radius: 9px;
    margin: 14px 0;
    overflow-x: auto;
    padding: 13px 15px;
    position: relative;
  }
  .markdown :global(pre.code-block code) {
    background: none;
    border: 0;
    color: #c8d3df;
    font-size: 11.5px;
    line-height: 1.65;
    padding: 0;
  }
  .markdown :global(blockquote) {
    border-left: 2px solid var(--mint);
    color: var(--muted);
    margin: 12px 0;
    padding: 2px 0 2px 14px;
  }
  .markdown :global(blockquote p) {
    color: var(--muted);
    margin: 0;
  }
  .markdown :global(hr) {
    border: 0;
    border-top: 1px solid var(--border-soft);
    margin: 22px 0;
  }
  .markdown :global(.timestamp) {
    color: var(--mint);
    font-family: var(--mono);
    font-size: 0.85em;
    opacity: 0.86;
  }
  .markdown :global(.table-wrap) {
    margin: 14px 0;
    overflow-x: auto;
  }
  .markdown :global(table) {
    border-collapse: collapse;
    font-size: 12px;
    width: 100%;
  }
  .markdown :global(th) {
    background: rgba(15, 52, 96, 0.4);
    border: 1px solid var(--border-soft);
    color: #fff;
    font-weight: 600;
    padding: 8px 10px;
    text-align: left;
  }
  .markdown :global(td) {
    border: 1px solid var(--border-soft);
    color: #cfd6df;
    padding: 7px 10px;
  }
</style>
