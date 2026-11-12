<script lang="ts">
  import { onMount } from 'svelte';
  import type { AppSettings, ChatMessage, Note } from '../types';
  import {
    chatWithNote,
    copyText,
    deleteNote,
    downloadText,
    listMessages,
    listNotes,
    noteFromDraft,
    noteFromTranscript,
    processUrl,
    saveMessage,
    saveNote,
    summarizeText,
    transcribeFile,
    transcribePath,
    uid
  } from '../api';
  import { toast } from '../toast.svelte';
  import DropZone from '../components/DropZone.svelte';
  import NoteList from '../components/NoteList.svelte';
  import NoteViewer from '../components/NoteViewer.svelte';

  let {
    settings,
    readOnly = false,
    onmodels
  }: {
    settings: AppSettings;
    readOnly?: boolean;
    onmodels?: () => void;
  } = $props();

  let notes = $state<Note[]>([]);
  let selectedId = $state('');
  let query = $state('');
  let loading = $state(true);
  let busy = $state(false);
  let busyLabel = $state('');
  let progress = $state(0);
  let error = $state('');
  let url = $state('');
  let messages = $state<ChatMessage[]>([]);
  let chatBusy = $state(false);
  let searchTimer: ReturnType<typeof setTimeout> | undefined;

  let selected = $derived(notes.find((note) => note.id === selectedId) ?? null);

  onMount(() => {
    void refresh('');
  });

  async function refresh(search = query, selectId = selectedId) {
    loading = true;
    try {
      const result = await listNotes(search);
      notes = result;
      if (selectId && result.some((note) => note.id === selectId)) {
        selectedId = selectId;
      } else if (result.length) {
        selectedId = result[0].id;
      } else {
        selectedId = '';
      }
      if (selectedId) await loadMessages(selectedId);
    } catch (problem) {
      toast(`Could not load notes: ${String(problem)}`, 'error');
    } finally {
      loading = false;
    }
  }

  async function loadMessages(noteId: string) {
    try {
      messages = await listMessages(noteId);
    } catch {
      messages = [];
    }
  }

  function searchChanged(value: string) {
    query = value;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      void refresh(query, selectedId);
    }, 250);
  }

  async function select(note: Note) {
    selectedId = note.id;
    await loadMessages(note.id);
  }

  async function newNote() {
    const now = new Date().toISOString();
    const note: Note = {
      id: uid(),
      title: 'Untitled note',
      source: 'Manual',
      sourceType: 'manual',
      markdown: '# Untitled note\n\n',
      transcript: '',
      duration: 0,
      createdAt: now,
      updatedAt: now,
      tags: []
    };
    try {
      const saved = await saveNote(note);
      await refresh(query, saved.id);
      toast('New note created', 'success', 1800);
    } catch (problem) {
      toast(String(problem), 'error');
    }
  }

  function guardModels(): boolean {
    if (readOnly) {
      toast('Download the local models before importing media.', 'error');
      onmodels?.();
      return true;
    }
    return false;
  }

  async function importFile(file: File) {
    if (guardModels()) return;
    if (file.size > 2 * 1024 * 1024 * 1024) {
      toast('That file is larger than the 2 GB limit.', 'error');
      return;
    }
    busy = true;
    busyLabel = 'Transcribing locally';
    progress = 12;
    error = '';
    try {
      const transcription = await transcribeFile(file, settings, settings.language, file.name);
      progress = 64;
      busyLabel = 'Structuring notes with Ollama';
      let markdown = '';
      try {
        markdown = await summarizeText(transcription.text, settings);
      } catch (problem) {
        markdown = `# ${file.name.replace(/\.[^.]+$/, '')}\n\n${transcription.text}`;
        toast(`Ollama unavailable — saved the raw transcript instead. ${String(problem)}`, 'info', 6000);
      }
      progress = 92;
      const note = noteFromTranscript(transcription, file.name, markdown, 'file');
      const saved = await saveNote(note);
      await refresh('', saved.id);
      progress = 100;
      toast('Note saved locally', 'success');
    } catch (problem) {
      error = String(problem);
      toast(error, 'error', 7000);
    } finally {
      busy = false;
      busyLabel = '';
    }
  }

  async function importPath(path: string) {
    if (guardModels()) return;
    const name = path.split(/[\\/]/).pop() || path;
    busy = true;
    busyLabel = 'Transcribing locally';
    progress = 12;
    error = '';
    try {
      const transcription = await transcribePath(path, settings);
      progress = 64;
      busyLabel = 'Structuring notes with Ollama';
      let markdown = '';
      try {
        markdown = await summarizeText(transcription.text, settings);
      } catch {
        markdown = `# ${name.replace(/\.[^.]+$/, '')}\n\n${transcription.text}`;
      }
      progress = 92;
      const note = noteFromTranscript(transcription, name, markdown, 'file');
      const saved = await saveNote(note);
      await refresh('', saved.id);
      progress = 100;
      toast('Note saved locally', 'success');
    } catch (problem) {
      error = String(problem);
      toast(error, 'error', 7000);
    } finally {
      busy = false;
      busyLabel = '';
    }
  }

  async function importUrl() {
    if (guardModels()) return;
    const value = url.trim();
    if (!value) return;
    if (!/^https?:\/\//i.test(value)) {
      toast('Paste a full http(s) URL.', 'error');
      return;
    }
    busy = true;
    busyLabel = 'Fetching media';
    progress = 4;
    error = '';
    try {
      const draft = await processUrl(value, settings, (update) => {
        busyLabel = update.message;
        progress = update.progress;
      });
      progress = 94;
      const saved = await saveNote(noteFromDraft(draft));
      url = '';
      await refresh('', saved.id);
      progress = 100;
      toast('Imported and structured locally', 'success');
    } catch (problem) {
      error = String(problem);
      toast(error, 'error', 7000);
    } finally {
      busy = false;
      busyLabel = '';
    }
  }

  async function deleteSelected(note = selected) {
    if (!note) return;
    if (!confirm(`Delete “${note.title}”? This removes it from local storage.`)) return;
    try {
      await deleteNote(note.id);
      await refresh(query, '');
      toast('Note deleted', 'info');
    } catch (problem) {
      toast(String(problem), 'error');
    }
  }

  async function copySelected() {
    if (!selected) return;
    await copyText(selected.markdown);
    toast('Markdown copied', 'success');
  }

  function exportSelected() {
    if (!selected) return;
    const name = selected.title.replace(/[^a-z0-9-_]+/gi, '-').toLowerCase() || 'verbatim-note';
    downloadText(`${name}.md`, selected.markdown, 'text/markdown;charset=utf-8');
    toast('Markdown exported', 'success');
  }

  async function chat(question: string) {
    if (!selected) return;
    if (readOnly) {
      toast('Download the local models before chatting with a note.', 'error');
      onmodels?.();
      return;
    }
    const userMessage: ChatMessage = {
      noteId: selected.id,
      role: 'user',
      content: question,
      createdAt: new Date().toISOString()
    };
    messages = [...messages, userMessage];
    void saveMessage(userMessage).catch(() => {});
    chatBusy = true;
    try {
      const answer = await chatWithNote(question, selected, messages.slice(0, -1), settings);
      const assistant: ChatMessage = {
        noteId: selected.id,
        role: 'assistant',
        content: answer || 'The local model returned no answer.',
        createdAt: new Date().toISOString()
      };
      messages = [...messages, assistant];
      void saveMessage(assistant).catch(() => {});
    } catch (problem) {
      toast(`Chat failed: ${String(problem)}`, 'error', 6000);
    } finally {
      chatBusy = false;
    }
  }

</script>

<div class="notes">
  <NoteList
    {notes}
    {selectedId}
    bind:query
    onselect={(note) => void select(note)}
    ondelete={(note) => void deleteSelected(note)}
    onnew={() => void newNote()}
    onquery={searchChanged}
  />

  <section class="workspace">
    <div class="import-strip">
      <DropZone
        compact
        accept="audio/*,video/*,.wav,.mp3,.m4a,.mp4,.mov,.webm,.ogg"
        label="Drop audio or video to transcribe"
        hint=""
        disabled={busy || readOnly}
        onfiles={(files) => files[0] && void importFile(files[0])}
        onpaths={(paths) => paths[0] && void importPath(paths[0])}
      />
      <form
        class="url-form"
        onsubmit={(event) => {
          event.preventDefault();
          void importUrl();
        }}
      >
        <svg viewBox="0 0 18 18" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.4">
          <path d="M7.8 10.2a2.8 2.8 0 0 0 4 0l1.5-1.5a2.8 2.8 0 0 0-4-4l-.7.7" stroke-linecap="round" />
          <path d="M10.2 7.8a2.8 2.8 0 0 0-4 0L4.7 9.3a2.8 2.8 0 0 0 4 4l.7-.7" stroke-linecap="round" />
        </svg>
        <input
          bind:value={url}
          placeholder="YouTube or TikTok URL"
          aria-label="Media URL"
          disabled={busy || readOnly}
        />
        <button type="submit" disabled={!url.trim() || busy || readOnly}>Import</button>
      </form>
    </div>

    {#if busy}
      <div class="progress-banner">
        <div class="progress-copy">
          <span class="spinner"></span>
          <span>{busyLabel || 'Working locally…'}</span>
        </div>
        <div class="progress-track"><span style:width={`${Math.max(3, progress)}%`}></span></div>
        <span class="progress-percent">{Math.round(progress)}%</span>
      </div>
    {/if}

    {#if error && !busy}
      <div class="error-banner">
        <span>{error}</span>
        <button type="button" onclick={() => (error = '')}>Dismiss</button>
      </div>
    {/if}

    <div class="viewer-wrap">
      {#if loading}
        <div class="empty-state"><span class="spinner light"></span><p>Opening your notes…</p></div>
      {:else}
        <NoteViewer
          note={selected}
          {messages}
          {chatBusy}
          onchat={(question) => void chat(question)}
          oncopy={() => void copySelected()}
          onexport={exportSelected}
          ondelete={() => void deleteSelected()}
        />
      {/if}
    </div>
  </section>
</div>

<style>
  .notes {
    display: flex;
    height: 100%;
    min-height: 0;
  }
  .workspace {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }
  .import-strip {
    align-items: center;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex: 0 0 54px;
    gap: 8px;
    padding: 8px 12px;
  }
  .import-strip :global(.dropzone) {
    flex: 0 0 270px;
    min-height: 37px;
  }
  .url-form {
    align-items: center;
    background: rgba(8, 13, 28, 0.34);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--muted);
    display: flex;
    flex: 1;
    gap: 7px;
    height: 37px;
    min-width: 0;
    padding: 0 5px 0 10px;
  }
  .url-form:focus-within {
    border-color: rgba(78, 204, 163, 0.45);
  }
  .url-form input {
    background: transparent;
    border: 0;
    color: var(--text);
    flex: 1;
    font-size: 10px;
    min-width: 0;
    outline: 0;
  }
  .url-form button {
    background: var(--accent);
    border: 1px solid rgba(78, 204, 163, 0.24);
    border-radius: 4px;
    color: var(--mint);
    cursor: pointer;
    font-size: 8.5px;
    height: 25px;
    padding: 0 10px;
  }
  .url-form button:disabled {
    cursor: default;
    opacity: 0.35;
  }
  .progress-banner,
  .error-banner {
    align-items: center;
    border-bottom: 1px solid var(--border-soft);
    display: flex;
    flex: 0 0 30px;
    font-size: 9px;
    gap: 10px;
    padding: 0 14px;
  }
  .progress-banner {
    background: rgba(78, 204, 163, 0.035);
    color: var(--muted);
  }
  .progress-copy {
    align-items: center;
    display: flex;
    gap: 7px;
    min-width: 190px;
  }
  .spinner {
    animation: spin 0.8s linear infinite;
    border: 1px solid rgba(78, 204, 163, 0.25);
    border-radius: 50%;
    border-top-color: var(--mint);
    height: 10px;
    width: 10px;
  }
  .spinner.light {
    border-color: rgba(234, 234, 234, 0.14);
    border-top-color: var(--mint);
    height: 18px;
    width: 18px;
  }
  .progress-track {
    background: rgba(234, 234, 234, 0.06);
    border-radius: 999px;
    flex: 1;
    height: 3px;
    overflow: hidden;
  }
  .progress-track span {
    background: var(--mint);
    border-radius: inherit;
    display: block;
    height: 100%;
    transition: width 240ms ease;
  }
  .progress-percent {
    color: var(--mint);
    font-family: var(--mono);
    font-size: 7.5px;
    min-width: 30px;
    text-align: right;
  }
  .error-banner {
    background: rgba(233, 69, 96, 0.08);
    color: #edaab5;
  }
  .error-banner span {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .error-banner button {
    background: transparent;
    border: 1px solid rgba(233, 69, 96, 0.25);
    border-radius: 3px;
    color: #f0b3be;
    cursor: pointer;
    font-size: 7.5px;
    padding: 2px 6px;
  }
  .viewer-wrap {
    flex: 1;
    min-height: 0;
  }
  .empty-state {
    align-items: center;
    color: var(--muted);
    display: flex;
    flex-direction: column;
    font-size: 10px;
    gap: 12px;
    height: 100%;
    justify-content: center;
  }
  @media (max-width: 800px) {
    .import-strip {
      align-items: stretch;
      flex-direction: column;
      flex-basis: 100px;
      height: auto;
    }
    .import-strip :global(.dropzone) {
      flex: 0 0 37px;
    }
  }
</style>
