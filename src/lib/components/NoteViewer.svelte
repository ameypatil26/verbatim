<script lang="ts">
  import { formatDate, formatDuration } from '../api';
  import type { ChatMessage, Note } from '../types';
  import Markdown from './Markdown.svelte';

  let {
    note = null,
    messages = [],
    chatBusy = false,
    onchat,
    oncopy,
    onexport,
    ondelete
  }: {
    note?: Note | null;
    messages?: ChatMessage[];
    chatBusy?: boolean;
    onchat?: (question: string) => void;
    oncopy?: () => void;
    onexport?: () => void;
    ondelete?: () => void;
  } = $props();

  let question = $state('');
  let chatScroll = $state<HTMLDivElement>();
  let bodyScroll = $state<HTMLDivElement>();

  $effect(() => {
    void messages.length;
    void chatBusy;
    if (chatScroll) {
      const element = chatScroll;
      requestAnimationFrame(() => {
        element.scrollTop = element.scrollHeight;
      });
    }
  });

  $effect(() => {
    void note?.id;
    if (bodyScroll) bodyScroll.scrollTop = 0;
  });

  function submit() {
    const text = question.trim();
    if (!text || chatBusy) return;
    question = '';
    onchat?.(text);
  }
</script>

{#if note}
  <section class="viewer">
    <header class="viewer-head">
      <div class="meta">
        <span class="source-type">{note.sourceType === 'url' ? 'URL IMPORT' : note.sourceType === 'dictation' ? 'DICTATION' : 'FILE / TRANSCRIPT'}</span>
        <span>{formatDate(note.updatedAt)}</span>
        {#if note.duration}<span>·</span><span>{formatDuration(note.duration)}</span>{/if}
        <span class="local">LOCAL</span>
      </div>
      <div class="actions">
        <button type="button" onclick={() => oncopy?.()} title="Copy Markdown">
          <svg viewBox="0 0 18 18" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.4">
            <rect x="6" y="6" width="8" height="9" rx="1.5" />
            <path d="M4 12H3.5A1.5 1.5 0 0 1 2 10.5v-7A1.5 1.5 0 0 1 3.5 2h7A1.5 1.5 0 0 1 12 3.5V4" />
          </svg>
          Copy
        </button>
        <button type="button" onclick={() => onexport?.()} title="Export Markdown">
          <svg viewBox="0 0 18 18" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.4">
            <path d="M9 2v9m0 0 3-3m-3 3L6 8M3 13v2h12v-2" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
          Export
        </button>
        <button class="danger" type="button" onclick={() => ondelete?.()} title="Delete note">
          <svg viewBox="0 0 18 18" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.4">
            <path d="M4 5h10M7 5V3.5h4V5m-6 0 .6 9h6.8l.6-9" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
      </div>
    </header>

    <div class="document" bind:this={bodyScroll}>
      <Markdown source={note.markdown} />
      {#if note.source !== 'Verbatim'}
        <div class="source-line">
          <span>SOURCE</span>
          <span class="source-value" title={note.source}>{note.source}</span>
        </div>
      {/if}
    </div>

    <section class="chat" aria-label="Chat with this note">
      <div class="chat-head">
        <div class="chat-title">
          <span class="pulse"></span>
          Ask this note
        </div>
        <span class="chat-model">LOCAL CONTEXT</span>
      </div>
      <div class="messages" bind:this={chatScroll}>
        {#if messages.length === 0}
          <div class="chat-empty">
            Questions are answered from this transcript only. Nothing leaves your machine.
          </div>
        {/if}
        {#each messages as message, index (message.id ?? index)}
          <div class="message {message.role}">
            <div class="role">{message.role === 'user' ? 'YOU' : 'VERBATIM'}</div>
            {#if message.role === 'assistant'}
              <Markdown source={message.content} />
            {:else}
              <p>{message.content}</p>
            {/if}
          </div>
        {/each}
        {#if chatBusy}
          <div class="thinking">
            <span></span><span></span><span></span>
            <em>Answering locally…</em>
          </div>
        {/if}
      </div>
      <form
        class="chat-form"
        onsubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <input
          bind:value={question}
          placeholder="Ask a question about this note…"
          aria-label="Ask a question about this note"
          disabled={chatBusy}
        />
        <button type="submit" disabled={!question.trim() || chatBusy} aria-label="Send question">
          <svg viewBox="0 0 20 20" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.7">
            <path d="m3 10 14-6-5 13-2.7-5.3L3 10Z" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
      </form>
    </section>
  </section>
{:else}
  <div class="empty-viewer">
    <div class="empty-mark" aria-hidden="true">
      <svg viewBox="0 0 34 34" width="34" height="34" fill="none" stroke="currentColor" stroke-width="1.2">
        <path d="M6 8.5A2.5 2.5 0 0 1 8.5 6h6L18 9.5h7.5A2.5 2.5 0 0 1 28 12v12a2.5 2.5 0 0 1-2.5 2.5h-17A2.5 2.5 0 0 1 6 24V8.5Z" />
        <path d="M11 16h12M11 20h8" stroke-linecap="round" />
      </svg>
    </div>
    <h2>Select a note</h2>
    <p>Or drop a recording, video, or URL to make a new one.</p>
  </div>
{/if}

<style>
  .viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    min-width: 0;
  }
  .viewer-head {
    align-items: center;
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    min-height: 45px;
    padding: 0 18px;
  }
  .meta {
    align-items: center;
    color: var(--muted);
    display: flex;
    font-size: 9px;
    gap: 6px;
    min-width: 0;
  }
  .source-type {
    color: var(--mint);
    font-family: var(--mono);
    font-size: 7.5px;
    letter-spacing: 0.1em;
  }
  .local {
    border: 1px solid rgba(78, 204, 163, 0.22);
    border-radius: 3px;
    color: var(--mint);
    font-family: var(--mono);
    font-size: 7px;
    letter-spacing: 0.12em;
    padding: 2px 4px;
  }
  .actions {
    align-items: center;
    display: flex;
    gap: 5px;
  }
  .actions button {
    align-items: center;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--muted);
    cursor: pointer;
    display: flex;
    font-size: 9.5px;
    gap: 4px;
    padding: 4px 7px;
  }
  .actions button:hover {
    border-color: rgba(78, 204, 163, 0.4);
    color: var(--text);
  }
  .actions button.danger:hover {
    border-color: rgba(233, 69, 96, 0.4);
    color: var(--signal);
  }
  .document {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 24px 30px 32px;
    scrollbar-gutter: stable;
  }
  .source-line {
    border-top: 1px solid var(--border-soft);
    color: var(--muted);
    display: flex;
    font-size: 9px;
    gap: 10px;
    margin-top: 26px;
    padding-top: 12px;
  }
  .source-line span:first-child {
    color: var(--mint);
    font-family: var(--mono);
    font-size: 7.5px;
    letter-spacing: 0.1em;
  }
  .source-value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chat {
    background: rgba(13, 21, 43, 0.6);
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    flex: 0 0 auto;
    max-height: 260px;
  }
  .chat-head {
    align-items: center;
    display: flex;
    justify-content: space-between;
    padding: 9px 18px 5px;
  }
  .chat-title {
    align-items: center;
    color: var(--text);
    display: flex;
    font-size: 10px;
    font-weight: 600;
    gap: 7px;
    letter-spacing: 0.03em;
  }
  .pulse {
    background: var(--mint);
    border-radius: 50%;
    box-shadow: 0 0 0 3px rgba(78, 204, 163, 0.1);
    height: 5px;
    width: 5px;
  }
  .chat-model {
    color: var(--muted);
    font-family: var(--mono);
    font-size: 7px;
    letter-spacing: 0.12em;
  }
  .messages {
    flex: 1;
    min-height: 36px;
    overflow-y: auto;
    padding: 3px 18px 8px;
  }
  .chat-empty {
    color: var(--muted);
    font-size: 9.5px;
    font-style: italic;
    line-height: 1.6;
    padding: 6px 0;
  }
  .message {
    margin-bottom: 8px;
  }
  .message .role {
    color: var(--mint);
    font-family: var(--mono);
    font-size: 6.5px;
    letter-spacing: 0.13em;
    margin-bottom: 2px;
  }
  .message.user .role {
    color: var(--muted);
  }
  .message p {
    color: #d4dbe4;
    font-size: 11px;
    line-height: 1.55;
    margin: 0;
    white-space: pre-wrap;
  }
  .message :global(.markdown) {
    font-size: 11px;
    line-height: 1.55;
  }
  .message :global(.markdown p) {
    font-size: 11px;
    margin-bottom: 4px;
  }
  .thinking {
    align-items: center;
    color: var(--muted);
    display: flex;
    gap: 4px;
    padding: 4px 0;
  }
  .thinking span {
    animation: blink 1.1s infinite;
    background: var(--mint);
    border-radius: 50%;
    height: 4px;
    width: 4px;
  }
  .thinking span:nth-child(2) {
    animation-delay: 0.15s;
  }
  .thinking span:nth-child(3) {
    animation-delay: 0.3s;
  }
  .thinking em {
    font-size: 9px;
    margin-left: 4px;
  }
  @keyframes blink {
    0%,
    70%,
    100% {
      opacity: 0.22;
    }
    35% {
      opacity: 1;
    }
  }
  .chat-form {
    align-items: center;
    background: rgba(10, 16, 34, 0.55);
    border: 1px solid var(--border);
    border-radius: 7px;
    display: flex;
    margin: 0 14px 11px;
    padding: 0 4px 0 10px;
  }
  .chat-form:focus-within {
    border-color: rgba(78, 204, 163, 0.45);
  }
  .chat-form input {
    background: transparent;
    border: 0;
    color: var(--text);
    flex: 1;
    font-size: 10.5px;
    height: 33px;
    min-width: 0;
    outline: 0;
  }
  .chat-form button {
    align-items: center;
    background: var(--accent);
    border: 1px solid rgba(78, 204, 163, 0.28);
    border-radius: 5px;
    color: var(--mint);
    cursor: pointer;
    display: flex;
    height: 25px;
    justify-content: center;
    width: 28px;
  }
  .chat-form button:disabled {
    cursor: default;
    opacity: 0.35;
  }
  .empty-viewer {
    align-items: center;
    color: var(--muted);
    display: flex;
    flex-direction: column;
    height: 100%;
    justify-content: center;
    padding: 30px;
    text-align: center;
  }
  .empty-mark {
    align-items: center;
    border: 1px solid var(--border);
    border-radius: 16px;
    color: var(--accent-soft);
    display: flex;
    height: 74px;
    justify-content: center;
    margin-bottom: 18px;
    transform: rotate(-3deg);
    width: 74px;
  }
  .empty-viewer h2 {
    color: var(--text);
    font-size: 14px;
    font-weight: 600;
    margin: 0 0 6px;
  }
  .empty-viewer p {
    font-size: 11px;
    line-height: 1.6;
    margin: 0;
  }
</style>
