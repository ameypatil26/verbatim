<script lang="ts">
  import { formatDate, formatDuration } from '../api';
  import type { Note } from '../types';

  let {
    notes = [],
    selectedId = '',
    query = $bindable(''),
    onselect,
    ondelete,
    onnew,
    onquery
  }: {
    notes?: Note[];
    selectedId?: string;
    query?: string;
    onselect?: (note: Note) => void;
    ondelete?: (note: Note) => void;
    onnew?: () => void;
    onquery?: (value: string) => void;
  } = $props();
</script>

<aside class="note-list" aria-label="Saved notes">
  <div class="list-head">
    <div>
      <span class="eyebrow">LIBRARY</span>
      <span class="count">{notes.length} {notes.length === 1 ? 'note' : 'notes'}</span>
    </div>
    <button class="new-button" type="button" onclick={() => onnew?.()} title="Start a new note">
      <svg viewBox="0 0 20 20" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8">
        <path d="M10 4v12M4 10h12" stroke-linecap="round" />
      </svg>
      New
    </button>
  </div>

  <label class="search">
    <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.6">
      <circle cx="8.5" cy="8.5" r="5.5" />
      <path d="m13 13 4 4" stroke-linecap="round" />
    </svg>
    <input
      type="search"
      placeholder="Search notes"
      bind:value={query}
      oninput={() => onquery?.(query)}
      aria-label="Search notes"
    />
  </label>

  <div class="items">
    {#each notes as note (note.id)}
      <div
        class="note-item"
        class:selected={note.id === selectedId}
        role="button"
        tabindex="0"
        onclick={() => onselect?.(note)}
        onkeydown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            onselect?.(note);
          }
        }}
      >
        <div class="title">{note.title || 'Untitled note'}</div>
        <div class="meta">
          <span class="source">{note.sourceType === 'url' ? 'URL' : note.sourceType === 'dictation' ? 'MIC' : 'FILE'}</span>
          <span>{formatDate(note.updatedAt)}</span>
          {#if note.duration}<span>·</span><span>{formatDuration(note.duration)}</span>{/if}
        </div>
        {#if ondelete}
          <button
            class="delete"
            type="button"
            title="Delete note"
            aria-label="Delete {note.title}"
            onclick={(event) => {
              event.stopPropagation();
              ondelete?.(note);
            }}
          >
            <svg viewBox="0 0 18 18" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.4">
              <path d="M4 5h10M7 5V3.5h4V5m-6 0 .6 9h6.8l.6-9" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          </button>
        {/if}
      </div>
    {:else}
      <div class="empty">
        {#if query}
          <p>No notes match “{query}”.</p>
        {:else}
          <p>No notes yet.</p>
          <span>Drop a file or paste a URL to build your first one.</span>
        {/if}
      </div>
    {/each}
  </div>
</aside>

<style>
  .note-list {
    background: rgba(22, 33, 62, 0.5);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    min-height: 0;
    width: 272px;
  }
  .list-head {
    align-items: center;
    display: flex;
    justify-content: space-between;
    padding: 16px 14px 10px;
  }
  .list-head > div {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .eyebrow {
    color: var(--muted);
    font-size: 8.5px;
    font-weight: 650;
    letter-spacing: 0.16em;
  }
  .count {
    color: var(--text);
    font-size: 12px;
    font-weight: 550;
  }
  .new-button {
    align-items: center;
    background: var(--accent);
    border: 1px solid rgba(78, 204, 163, 0.22);
    border-radius: 6px;
    color: var(--mint);
    cursor: pointer;
    display: flex;
    font-size: 10.5px;
    font-weight: 600;
    gap: 4px;
    padding: 5px 8px;
  }
  .new-button:hover {
    background: #15457c;
  }
  .search {
    align-items: center;
    background: rgba(10, 16, 34, 0.4);
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--muted);
    display: flex;
    gap: 7px;
    margin: 0 12px 10px;
    padding: 0 9px;
  }
  .search:focus-within {
    border-color: rgba(78, 204, 163, 0.5);
  }
  .search input {
    background: transparent;
    border: 0;
    color: var(--text);
    font-size: 11px;
    height: 31px;
    outline: 0;
    width: 100%;
  }
  .items {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 8px 12px;
  }
  .note-item {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 7px;
    color: var(--text);
    cursor: pointer;
    display: block;
    margin-bottom: 3px;
    padding: 10px 10px 9px;
    position: relative;
    text-align: left;
    width: 100%;
  }
  .note-item:focus-visible {
    outline: 2px solid rgba(78, 204, 163, 0.6);
    outline-offset: 1px;
  }
  .note-item:hover {
    background: rgba(15, 52, 96, 0.25);
  }
  .note-item.selected {
    background: rgba(15, 52, 96, 0.55);
    border-color: rgba(78, 204, 163, 0.25);
  }
  .title {
    color: #fff;
    font-size: 11.5px;
    font-weight: 550;
    line-height: 1.35;
    overflow: hidden;
    padding-right: 18px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    align-items: center;
    color: var(--muted);
    display: flex;
    font-size: 9px;
    gap: 5px;
    margin-top: 5px;
  }
  .source {
    color: var(--mint);
    font-family: var(--mono);
    font-size: 7.5px;
    letter-spacing: 0.08em;
    opacity: 0.7;
  }
  .delete {
    background: transparent;
    border: 0;
    border-radius: 4px;
    color: var(--muted);
    display: none;
    padding: 3px;
    position: absolute;
    right: 7px;
    top: 7px;
  }
  .note-item:hover .delete,
  .note-item:focus-visible .delete {
    display: flex;
  }
  .delete:hover {
    background: rgba(233, 69, 96, 0.14);
    color: var(--signal);
  }
  .empty {
    color: var(--muted);
    font-size: 11px;
    line-height: 1.6;
    padding: 28px 12px;
    text-align: center;
  }
  .empty p {
    color: var(--text);
    margin: 0 0 4px;
  }
  .empty span {
    color: var(--muted);
    font-size: 10px;
  }
</style>
