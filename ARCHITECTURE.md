# Verbatim Architecture

This document describes the implemented architecture, contracts, and data flow as of the handoff. It is intentionally concrete: another engineer should be able to open the listed files and continue without reverse-engineering the UI.

---

## 1. System overview

Verbatim is one repository with four runtime surfaces, a static landing page, and two local AI microservices:

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                               User interfaces                                │
│                                                                              │
│  Svelte 5 UI in Tauri 2               Chrome/Edge MV3 extension              │
│  ┌────────────────────────┐           ┌────────────────────────┐             │
│  │ Dictation · Notes      │           │ context menus          │             │
│  │ Vision · Settings      │           │ popup + floating button│             │
│  └───────────┬────────────┘           └───────────┬────────────┘             │
│              │ Tauri IPC (invoke)                  │ HTTP + runtime messages  │
└──────────────┼─────────────────────────────────────┼──────────────────────────┘
               ▼                                     ▼
┌──────────────────────────────────┐   ┌───────────────────────────────────────┐
│ Rust shell                       │   │ Extension service worker              │
│ src-tauri/src/lib.rs             │   │ extension/src/background.ts           │
│                                  │   │                                       │
│ AppState                         │   │ fetch() → 127.0.0.1                   │
│ ├─ Arc<Database>                 │   └───────────────┬───────────────────────┘
│ ├─ Arc<AudioEngine>              │                   │
│ ├─ Arc<ServiceManager>           │                   │
│ ├─ reqwest::Client               │                   │
│ ├─ Mutex<Option<HotkeyService>>  │                   │
│ └─ SQLite connection             │                   │
└───────┬──────────────────────────┘                   │
        │ spawn / health / kill                        │
        ▼                                              ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                    Localhost-only microservices (no auth)                    │
│                                                                              │
│  ASR :8001                          OCR :8002                 Ollama :11434   │
│  services/asr/main.py               services/ocr/main.py       user install   │
│  FastAPI + FunASR SenseVoice        FastAPI + RapidOCR         /api/generate  │
│  /health /transcribe /unload        /health /ocr /ocr-batch    /api/chat      │
│                                      /ocr-pdf /pdf-searchable  /api/tags      │
│                                      /unload                                  │
└──────────────────────────────────┬───────────────────────────────────────────┘
                                   │
                                   ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                           ~/.verbatim/ local state                           │
│  notes.db (SQLite WAL, FTS5) · models/asr · models/ocr · logs · tmp          │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Ports

| Process | Bind | Port | Started by |
|---|---|---:|---|
| ASR FastAPI | `127.0.0.1` | 8001 | Tauri `ServiceManager`, or manually |
| OCR FastAPI | `127.0.0.1` | 8002 | Tauri `ServiceManager`, or manually |
| Ollama | `127.0.0.1` (or user URL) | 11434 | User installs/runs `ollama serve` |
| Vite dev server | `127.0.0.1` | 1420 | `npm run dev` / Tauri `beforeDevCommand` |
| MCP | — | — | Not built in this handoff |

No service uses authentication. Security comes from loopback binding plus a loopback-client middleware and a restrictive CORS allow-list. This is appropriate for a single-user desktop machine and should not be exposed to a network.

---

## 2. Repository map

```text
F:\deepseek\project
├── index.html                     Vite entry for the desktop webview
├── package.json                   scripts + Node dependencies
├── package-lock.json
├── vite.config.js                 Vite/Svelte, in-process TS plugin, esbuild disabled
├── svelte.config.js               in-process TypeScript preprocessor for Svelte
├── tsconfig.json
├── tailwind.config.js             desktop-era Tailwind config (landing has its own)
├── postcss.config.js
├── pytest.ini
├── README.md
├── ARCHITECTURE.md
├── AGENT.md
│
├── src/
│   ├── main.ts                    mount(App)
│   ├── App.svelte                 shell, nav, health polling, model modal
│   ├── app.css                    design tokens + global styles
│   └── lib/
│       ├── api.ts                 all IPC/HTTP/fallback operations
│       ├── types.ts               TypeScript data contracts
│       ├── settings.ts            defaults + URL/hotkey normalization
│       ├── models.ts              required model catalog for the UI
│       ├── markdown.ts            escaping Markdown renderer
│       ├── toast.svelte.ts        rune-based toast state
│       ├── tabs/                  DictationTab, NotesTab, VisionTab, SettingsTab
│       └── components/            Waveform, DropZone, NoteList, NoteViewer,
│                                  OcrResult, StatusBar, SettingsForm,
│                                  ModelSetup, ImageMasks, Markdown, ToastHost
│
├── src-tauri/
│   ├── Cargo.toml                 602-package resolved Tauri dependency graph
│   ├── Cargo.lock
│   ├── build.rs
│   ├── tauri.conf.json            window, CSP, bundle resources
│   ├── capabilities/default.json  Tauri 2 capability permissions
│   ├── icons/                     generated PNG/ICO assets
│   └── src/
│       ├── main.rs                calls verbatim_lib::run
│       ├── lib.rs                 AppState, builder, setup, shutdown, seed note
│       ├── models.rs              serde structs shared by IPC and events
│       ├── db.rs                  SQLite schema, notes/messages/settings
│       ├── settings.rs            settings load/save normalization
│       ├── services.rs            Python supervisor + health + restart
│       ├── audio.rs               cpal capture + WAV writer + level events
│       ├── dictation.rs           record → ASR → proofread → enigo
│       ├── injection.rs           enigo + xdotool/cliclick fallback
│       ├── hotkeys.rs             global-hotkey thread + rebinding
│       ├── screenshot.rs          OS region capture → OCR → clipboard
│       ├── model_manager.rs       check/download/delete models
│       ├── commands.rs            all #[tauri::command] IPC functions
│       └── logging.rs             daily rotating logs + 7-day cleanup
│
├── services/
│   ├── common/
│   │   ├── __init__.py
│   │   └── verbatim_common.py     paths, logging, CORS, loopback guard,
│   │                              cross-service model lease, audio helpers
│   ├── asr/
│   │   ├── main.py                FastAPI service + SenseVoice/mock engines
│   │   ├── requirements.txt
│   │   └── download_models.py
│   ├── ocr/
│   │   ├── main.py                FastAPI OCR/PDF/QR service
│   │   ├── requirements.txt
│   │   └── download_models.py
│   └── tests/
│       ├── conftest.py
│       ├── test_asr.py
│       └── test_ocr.py
│
├── extension/
│   ├── manifest.json              MV3 permissions, service worker, content script
│   ├── src/background.ts          localhost HTTP, context menus, OCR/capture/transcribe
│   ├── src/content.ts             page text/media, region selector, Shadow DOM panel
│   ├── background.js              checked-in bundle for Load unpacked
│   ├── content.js                 checked-in bundle
│   ├── build.mjs                  TypeScript compiler API build
│   ├── tsconfig.json
│   ├── popup/popup.html           plain HTML
│   ├── popup/popup.css            plain CSS
│   ├── popup/popup.js             plain JS
│   └── icons/                     16/48/128 px
│
├── scripts/
│   ├── make_icons.py              Pillow icon generation
│   └── pip_install_local.py       sandbox-only pip temp workaround
│
├── web/                           static Svelte + Tailwind landing page
│   ├── index.html
│   ├── vite.config.js
│   ├── svelte.config.js
│   ├── tailwind.config.js / postcss.config.js
│   ├── src/                       App, sections, shared Logo
│   ├── public/                    favicon + .nojekyll
│   └── dist/                      generated static output
└── mcp/                           empty; MCP out of scope this session
```

Generated/ignored directories: `node_modules/`, `dist/`, `.pydeps/`, `.venv/`, `.tmp/`, `.cargo/`, `.rustup/`, `.cargo-target/`, `target/`, `screenshots/`.

---

## 3. Svelte 5 frontend

The UI uses Svelte 5 runes exclusively (`$state`, `$derived`, `$effect`, `$props`, `$bindable`). There are no Svelte 4 stores.

### App shell

`src/App.svelte`:

- Loads settings from Tauri (`get_settings`) or `localStorage` in browser preview.
- Polls health every 6.5 seconds via `checkHealth()`:
  - ASR `GET /health`
  - OCR `GET /health`
  - Ollama `GET /api/tags`
- Loads `check_models()` on startup and opens the blocking `ModelSetup` modal if anything is missing.
- Downloads missing models sequentially via `download_model`, listening to `model-download-progress`.
- Maintains `modelsReady` and `readOnly`; read-only shows the banner and disables processing tabs.
- Renders the four tabs and the persistent `StatusBar`.
- Browser preview flags:
  - `?demo=1` skips the model modal and simulates all models ready.
  - `?tab=notes|vision|settings|dictation` opens a tab.
- `isTauri` is detected by `window.__TAURI_INTERNALS__`.

### Tab responsibilities

| Tab | File | Main operations |
|---|---|---|
| Dictation | `src/lib/tabs/DictationTab.svelte` | listen to `dictation:state`/`dictation:level`, invoke `dictate_start`/`dictate_stop`, browser MediaRecorder fallback, copy/undo |
| Notes | `src/lib/tabs/NotesTab.svelte` | list/get/save/delete notes, transcribe files/paths/URLs, summarize, chat, export |
| Vision | `src/lib/tabs/VisionTab.svelte` | queue up to 50 files, OCR images/PDFs, masks, QR, CSV, searchable PDF, capture |
| Settings | `src/lib/tabs/SettingsTab.svelte` + `SettingsForm.svelte` | models, services, hotkeys, Ollama, output, privacy |

### Shared components

- `Waveform.svelte` — level-driven animated bars.
- `DropZone.svelte` — HTML file input plus Tauri `onDragDropEvent` paths.
- `NoteList.svelte` — search + note library.
- `NoteViewer.svelte` — sanitized Markdown + local chat.
- `OcrResult.svelte` — textarea selection, copy, MD/TXT export, QR chips, searchable PDF trigger.
- `StatusBar.svelte` — live ASR/OCR/Ollama health + “Models not downloaded”.
- `SettingsForm.svelte` — models and preferences, saving through callbacks.
- `ModelSetup.svelte` — blocking first-launch/download prompt with per-model progress.
- `ImageMasks.svelte` — draw exclude rectangles over an image; coordinates are converted back to natural pixels.
- `Markdown.svelte` / `markdown.ts` — escapes all HTML, then renders headings/lists/tables/code/links.
- `ToastHost.svelte` + `toast.svelte.ts` — transient notifications.

### Browser preview versus Tauri

`src/lib/api.ts` branches on `isTauri`:

| Operation | Tauri | Browser preview |
|---|---|---|
| Notes | SQLite commands | `localStorage` |
| Settings | SQLite | `localStorage` |
| Dictation | Rust cpal + enigo | MediaRecorder, copy result |
| ASR/OCR | HTTP to localhost (same) | HTTP to localhost |
| URL import | Rust `process_url` | ASR URL + direct Ollama |
| Chat | direct Ollama HTTP | direct Ollama HTTP |
| Models | Tauri commands | simulated “missing” report |
| File drop | Tauri path events + commands | File objects |

This split is deliberate: all AI HTTP calls have the same shape, while OS-specific functions remain in Rust.

### Styling

Dark theme tokens in `src/app.css`:

```text
background #1a1a2e · cards #16213e · accent #0f3460
highlight/error #e94560 · text #eaeaea · success #4eccaa
```

The desktop UI is custom CSS (not Tailwind) with a compact 1000×700 layout. Tailwind is used by the independent static landing page in `web/`.

---

## 4. Rust/Tauri backend

### Startup sequence

`src-tauri/src/lib.rs`:

1. Initialize rotating logs (`~/.verbatim/logs/verbatim.log.YYYY-MM-DD`), cleanup older than 7 days.
2. Create `~/.verbatim/`, open `notes.db`, initialize schema, seed a welcome note only if the DB has zero notes.
3. Build a reqwest client (8 s connect timeout, 30 min request timeout).
4. Create `ServiceManager`, start ASR/OCR supervisors.
5. Create `AppState`, manage it with Tauri.
6. Start the global hotkey thread with persisted combos.
7. Run `check_models`, emit `models:status`.
8. On `RunEvent::Exit`, call `ServiceManager::shutdown()` to kill child services.

### AppState

```rust
pub struct AppState {
    pub db: Arc<Database>,
    pub audio: Arc<AudioEngine>,
    pub services: Arc<ServiceManager>,
    pub http: reqwest::Client,
    pub hotkeys: Mutex<Option<HotkeyService>>,
}
```

### IPC commands

All commands are registered in `lib.rs` with `tauri::generate_handler!`.

| Command | Arguments | Returns | File |
|---|---|---|---|
| `save_note` | `note: Note` | `Note` | commands.rs |
| `get_notes` | `query?: string` | `Note[]` | commands.rs |
| `list_notes` | `query?: string` | `Note[]` (alias) | commands.rs |
| `delete_note` | `id: string` | `void` | commands.rs |
| `list_messages` | `noteId: string` | `ChatMessage[]` | commands.rs |
| `save_message` | `message: ChatMessage` | `ChatMessage` | commands.rs |
| `get_settings` | — | `AppSettings` | commands.rs |
| `save_settings` | `settings: AppSettings` | `AppSettings` | commands.rs |
| `set_hotkeys` | `dictation, ocr` | `AppSettings` | commands.rs |
| `dictate_start` | `source?: "microphone"\|"system"` | `void` | commands.rs |
| `dictate_stop` | — | final transcript | commands.rs |
| `screenshot_ocr` | — | `CaptureResult` | commands.rs |
| `capture_region_ocr` | — | alias of `screenshot_ocr` | commands.rs |
| `undo_last_dictation` | — | `void` | commands.rs |
| `check_models` | — | `ModelsReport` | commands.rs |
| `get_setup_status` | — | `SetupStatus` | commands.rs |
| `download_model` | `model: string` | `void`, emits progress | commands.rs |
| `delete_model` | `model: string` | `void` | commands.rs |
| `service_status` | — | `ServiceRuntimeStatus[]` | commands.rs |
| `restart_service` | `name: "asr"\|"ocr"` | `void` | commands.rs |
| `transcribe_path` | `path, language?` | `Transcription` | commands.rs |
| `ocr_path` | `path, masks?` | `OcrResult` | commands.rs |
| `ocr_pdf_path` | `path, masks?` | `OcrResult` | commands.rs |
| `searchable_pdf_path` | `path` | base64 PDF | commands.rs |
| `read_image_preview` | `path` | data URL | commands.rs |
| `process_url` | `url` | `NoteDraft` | commands.rs |

Tauri 2 maps JS camelCase argument names to Rust snake_case parameters by default. For example, `invoke('transcribe_path', { path, language })` and `invoke('list_messages', { noteId })`.

### Events emitted to Svelte

| Event | Payload | Producer |
|---|---|---|
| `service:status` | `ServiceRuntimeStatus[]` | services.rs |
| `models:status` | `ModelsReport` | lib.rs |
| `model-download-progress` | `{model, downloadedMb, totalMb, status}` | model_manager.rs |
| `dictation:state` | `{state, text?, error?, duration?, source?}` | dictation.rs |
| `dictation:level` | `f32` 0..1 | audio.rs |
| `dictation:result` | `{text, duration, source}` | dictation.rs |
| `ocr:result` | `CaptureResult` | screenshot.rs |
| `ocr:error` | `string` | hotkeys.rs |
| `url:progress` | `{stage, progress, message}` | commands.rs |
| `hotkeys:error` | `string` | lib.rs setup |

### SQLite schema

`src-tauri/src/db.rs` uses `rusqlite` with bundled SQLite:

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;

CREATE TABLE notes (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  source TEXT NOT NULL DEFAULT '',
  source_type TEXT NOT NULL DEFAULT 'manual',
  markdown TEXT NOT NULL DEFAULT '',
  transcript TEXT NOT NULL DEFAULT '',
  duration REAL NOT NULL DEFAULT 0,
  tags TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE messages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE VIRTUAL TABLE notes_fts USING fts5(
  title, markdown, transcript,
  content='notes', content_rowid='rowid',
  tokenize='unicode61 remove_diacritics 2'
);
```

FTS5 triggers `notes_ai`, `notes_ad`, `notes_au` keep the index synchronized. Search builds a quoted-prefix OR query in `fts_query`; if FTS returns nothing, the code falls back to `LIKE` over title/markdown/transcript. The `app` settings row stores the serialized `AppSettings` JSON.

### Service supervisor

`src-tauri/src/services.rs`:

- Locates `services/` from `VERBATIM_SERVICES_DIR`, the development checkout (`CARGO_MANIFEST_DIR/../services`), or Tauri's resource directory.
- Resolves Python in this order:
  1. `VERBATIM_PYTHON`
  2. `.venv\Scripts\python.exe` / `.venv/bin/python`
  3. `.pydeps` + `python`/`python3` on PATH
  4. `python3` / `python`
- Spawns `asr/main.py` and `ocr/main.py` with `VERBATIM_DATA_DIR`, `VERBATIM_LOG_DIR`, `PYTHONUNBUFFERED=1`, and stdout/stderr appended to `*-service.log`.
- Uses `CREATE_NO_WINDOW` on Windows.
- If the port is already open, adopts the process as `external` instead of spawning.
- Monitor loop every 2 s checks `try_wait()`:
  - restart up to 3 times, then `failed` with a log-path message;
  - on app exit sets shutdown and kills children.
- `restart_service` kills, resets retries, and spawns again.
- Health is TCP-port based in Rust (fast); the UI does HTTP `/health` for model state.

### Audio capture

`src-tauri/src/audio.rs`:

- A dedicated thread owns the `cpal::Stream` (not `Send`-safe to move to async tasks).
- Start handshake via `mpsc` ensures errors are returned before the UI reports recording.
- Supports F32/I16/U16 sample formats, downmixes channels to mono, writes 16-bit PCM WAV to `~/.verbatim/tmp/`.
- Emits `dictation:level` from the capture thread every ~55 ms.
- Microphone uses the default input device.
- System audio enumerates inputs for names containing `loopback`, `stereo mix`, `monitor`, `what u hear`, or `cable output`. macOS system audio returns a clear unsupported error.

### Dictation flow

1. Hotkey pressed:
   - Tap mode toggles recording.
   - Push mode starts on press; release stops.
2. `dictate_start` → `audio.start` → `dictation:state {state:"recording"}`.
3. Stop:
   - `dictate_stop` stops and joins the audio thread.
   - Emits `{state:"processing"}`.
   - POSTs multipart WAV to ASR `/transcribe`.
   - Optionally calls Ollama `/api/generate` with the proofreading prompt.
   - Types through `enigo`; fallback `xdotool type` on Linux, `cliclick t:` on macOS.
   - Emits `{state:"idle", text, duration, source}` and `dictation:result`.
   - Deletes the temp WAV.
4. Errors emit `{state:"error", error}` and are logged.
5. `undo_last_dictation` sends Ctrl+Z through enigo.

### Screenshot OCR flow

`src-tauri/src/screenshot.rs`:

1. Hash current clipboard image.
2. Launch the OS region selector:
   - Windows: `cmd /C start "" ms-screenclip:`
   - macOS: `screencapture -i -c`
   - Linux: `gnome-screenshot -a -c`, `spectacle -r -b -n -c`, or `scrot -s` as available.
3. Poll clipboard for up to 90 s until a new image appears.
4. Encode PNG, POST multipart `/ocr`.
5. Copy extracted text to clipboard; emit `ocr:result`; return `CaptureResult` including `imageDataUrl`.

This avoids a custom full-screen selection overlay and relies on the platform's native region UX.

### Hotkeys

`src-tauri/src/hotkeys.rs`:

- A dedicated thread owns `GlobalHotKeyManager` (avoiding Send/Tauri-state constraints).
- A command channel handles rebinding and shutdown.
- Parses human strings such as `Ctrl+Shift+D` into `Modifiers`/`Code`.
- Supports letters, digits, F1–F12, arrows, Space, Enter, Tab, Escape, Backspace, Delete, Home, End, PageUp/Down.
- Stores a map of hotkey ID → action.
- Press/Release semantics implement tap and push-to-talk.
- Rebinding unregisters old keys, validates the new pair, registers both, and rolls back on failure.
- `save_settings` and `set_hotkeys` call `HotkeyService::rebind` before persisting.

### Model manager

`src-tauri/src/model_manager.rs`:

- Model IDs: `sensevoice`, `rapidocr`, `qwen2.5-coder:7b`.
- `check_models`:
  - ASR ready if all expected files exist and `model.pt` > 1 MB.
  - OCR ready if `~/.verbatim/models/ocr/.installed` exists.
  - LLM ready if `ollama list` contains the model name or `:latest`.
  - Reports Ollama binary presence separately.
- `download_model`:
  - ASR: HEAD-resolves all files from `huggingface.co/iic/SenseVoiceSmall` then `FunAudioLLM/SenseVoiceSmall`, sums real `Content-Length`, downloads with `reqwest::bytes_stream` to `.part`, renames, emits real progress.
  - OCR: runs `services/ocr/download_models.py` through the resolved Python, emits progress, then marks 100%.
  - LLM: `ollama pull qwen2.5-coder:7b` with stdout/stderr line parsing for percent; emits estimated progress.
- `delete_model` removes ASR/OCR directories or runs `ollama rm`.
- Progress event uses serde camelCase fields.

### Logging and error handling

- `tracing` + `tracing_appender::rolling::daily`, no console spam in release.
- 7-day retention cleanup in `logging.rs`.
- Service stdout/stderr is redirected to per-service log files.
- Service crash → UI status and optional restart; up to 3 automatic retries.
- OCR/ASR service `/health` returns 200 even when models are not loaded; `/transcribe` or `/ocr` returns 503 if loading fails.
- UI errors surface through toasts and inline banners.

---

## 5. Python microservices

### Shared module

`services/common/verbatim_common.py`:

- Data paths: `VERBATIM_DATA_DIR` or `~/.verbatim`.
- Rotating Python logs (`RotatingFileHandler`, 5 MB × 7).
- `LoopbackOnlyMiddleware` rejects non-loopback clients (`testclient` allowed for pytest).
- CORS allow-list:
  - `http://localhost:*`, `http://127.0.0.1:*`
  - `tauri://localhost`, `http://tauri.localhost`
  - `chrome-extension://...`, `moz-extension://...`
  - `null`
- `ModelLease` uses an OS file lock (`msvcrt.locking` / `fcntl.flock`) around load+inference.
- `unload_peer()` asks the other service `/unload` after the lease is acquired, preventing both models from being resident.
- Upload cap 2 GB; masks parser and overlap filter.
- ffmpeg conversion to 16 kHz mono WAV, duration helpers, WAV writer.

### ASR contract (`services/asr/main.py`)

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/health` | — | `{status:"ok", model:"sensevoice", loaded:bool}` |
| POST | `/transcribe` | multipart `file`, optional `language`; or JSON `{audio_base64, language?}`; or JSON `{url}` extension | `{text, duration, language}` |
| POST | `/unload` | — | `{status:"ok"}` |

Behavior:

- `VERBATIM_ASR_ENGINE=mock` returns deterministic text; default `sensevoice` imports FunASR lazily.
- SenseVoice loads on demand, formats `sentence_info` timestamps as `[MM:SS]` when available.
- URL input calls `yt-dlp -x --audio-format wav` as a fallback; the desktop URL path currently calls this service endpoint rather than downloading auto-subs itself.
- 400 for malformed input/missing audio; 503 for model load failure; 500 for inference failure.
- Idle unload after 5 minutes (`VERBATIM_MODEL_IDLE_SECONDS`).

### OCR contract (`services/ocr/main.py`)

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/health` | — | `{status:"ok", model:"paddle", loaded:bool}` |
| POST | `/ocr` | multipart `file`/`image`, `masks` form JSON | `{text, boxes, codes}` (codes additive) |
| POST | `/ocr-batch` | `files[]`, `masks_json` | `{results:[{filename,text,boxes,codes}]}` |
| POST | `/ocr-pdf` | multipart PDF, masks | `{text, boxes, codes}` |
| POST | `/pdf-searchable` | multipart PDF | `application/pdf` |
| POST | `/unload` | — | `{status:"ok"}` |

Behavior:

- Default engine `rapidocr` (RapidOCR-ONNX). `paddle` selectable; `mock` for tests.
- Converts polygon boxes to bounding boxes, groups text into reading-order lines, applies masks.
- QR/barcode via `pyzbar` if the native zbar library loads; otherwise silently returns no codes.
- PDFs render at 2× with PyMuPDF, OCR each page, and return `--- Page N ---` separators.
- Searchable PDF:
  1. Render each page at 2×.
  2. Insert the raster image to fill the page.
  3. Re-run OCR and keep internal text-bearing boxes.
  4. Group into lines, map pixels back to points, insert invisible text with `render_mode=3`.
  5. Copy rotation and set metadata.
- Batch limit 50; per-file 2 GB.
- `model:"paddle"` is kept as the stable contract ID even when RapidOCR is the engine.

> The bug fixed at handoff: `_searchable_pdf` previously used the public `payload["boxes"]`, which strips `text`. It now calls `engine.run(image)` directly to retain internal text before inserting the hidden layer.

### Model download scripts

- `services/asr/download_models.py`: ModelScope snapshot `iic/SenseVoiceSmall` into `~/.verbatim/models/asr`, Hugging Face fallback, `.installed` marker.
- `services/ocr/download_models.py`: warms RapidOCR (bundled ONNX) or initializes PaddleOCR, writes `~/.verbatim/models/ocr/.installed`.

Both print JSON progress lines for potential subprocess parsing; the Tauri OCR downloader currently executes the script and emits 0→100 around it.

---

## 6. Browser extension

### Manifest

- MV3, service worker `background.js`, content script `content.js` on `<all_urls>` top frame.
- Permissions: `activeTab`, `tabs`, `contextMenus`, `scripting`, `storage`, `clipboardWrite`.
- Host permissions: `http://127.0.0.1/*`, `http://localhost/*` (match patterns ignore port).
- Plain popup at `popup/popup.html`.
- CSP: `script-src 'self'; object-src 'self'`.

### Background service worker

`extension/src/background.ts`:

- Defaults: enabled true, ASR 8001, OCR 8002, Ollama 11434, floating button true.
- `chrome.storage.sync` for settings.
- Health probes with 1.8 s abort; updates action badge (`!` when ASR/OCR down, empty when ready).
- Creates context menus on install/startup:
  - `verbatim-ocr` contexts image/page/selection
  - `verbatim-capture` contexts selection/page
  - `verbatim-transcribe` contexts page/video/audio
- Message protocol:
  - popup/content → worker: `get-status`, `save-settings`, `capture-viewport`, `ocr-region`, `ocr-image`, `capture-text`, `transcribe-media`, `copy`
  - worker → content: `verbatim:show-panel`, `verbatim:copy`, `verbatim:context`
- OCR:
  - captures `chrome.tabs.captureVisibleTab`, optionally crops with `OffscreenCanvas` using CSS rect × devicePixelRatio;
  - POSTs multipart to OCR;
  - sends text to content for clipboard + panel.
- Capture:
  - sends visible text to Ollama `/api/generate` with a summarization prompt;
  - displays Markdown-ish text in the panel.
- Transcribe:
  - sends detected media/page URL to ASR `/transcribe` JSON `{url}`;
  - ASR uses yt-dlp.
- Errors are converted to “Verbatim not detected” when fetch fails.

### Content script

`extension/src/content.ts`:

- Injects no visible DOM until needed.
- `visibleText()` includes title + body innerText, limited to 20k.
- `detectMediaUrl()` uses `video/audio.currentSrc`, `src`, or `<source>`; blob/data URLs fall back to page URL for known video sites.
- Image context menu finds the matching visible `<img>` by normalized URL and returns its viewport rect.
- Region selector: fixed overlay + crosshair, drag rectangle, Esc cancel; sends `{rect, dpr}` to worker.
- Shadow DOM result panel with title, status dot, text, Copy/Close, loading spinner.
- Shadow DOM floating “V” button; configurable via storage.
- Clipboard write via `navigator.clipboard`, fallback `textarea + execCommand`.
- Host page CSS cannot affect the panel/float.

### Popup

`extension/popup/` (plain HTML/CSS/JS):

- Detection banner.
- Per-service health rows.
- “OCR visible tab” action.
- Enable and floating-button toggles.
- ASR/OCR/Ollama port editors and save.
- Privacy footer.
- Polls status every 15 s while open.

### Build

`extension/build.mjs` uses the TypeScript compiler API (`ts.transpileModule`) rather than esbuild, then writes `extension/background.js` and `extension/content.js`. It checks the 500 KB budget. Checked-in JS means **Load unpacked** works without a build step.

---

## 6.5 Static landing page

`web/` is a separate Svelte 5 + Tailwind 3 static site. It does not import the desktop app, Tauri APIs, or Python services.

```text
web/
├── index.html              static shell, meta, JSON-LD
├── vite.config.js          root=web, base=./, outDir=dist, esbuild disabled
├── svelte.config.js        in-process TS preprocessor (verbatim module syntax)
├── tailwind.config.js      landing palette/content
├── postcss.config.js       tailwindcss + autoprefixer
├── src/
│   ├── App.svelte          section composition
│   ├── app.css             Tailwind layers + hero/accordion animations
│   └── lib/                Navbar, Hero, Features, HowItWorks,
│                           Architecture, Comparison, Faq, Footer, Logo
├── public/                 favicon.png, .nojekyll
└── dist/                   generated output
```

- `npm run build:web` runs `vite build --configLoader native --config web/vite.config.js`.
- `base: './'` makes the output portable to Netlify or a GitHub Pages project subpath.
- Tailwind is compiled via PostCSS. No external fonts, images, or analytics.
- Sections: navbar, hero, four feature cards, three-step flow, architecture diagram, comparison vs Otter.ai, six-question FAQ accordion, footer.
- The hero animates a waveform/scan line with CSS; `prefers-reduced-motion` disables animation.
- The landing page's `svelte.config.js` must preserve imported component bindings in TypeScript output (`verbatimModuleSyntax: true`). Without it, TypeScript elides imports used only in markup and the page renders empty component stubs.

---

## 7. End-to-end flows

### Dictation into another app

```text
global-hotkey Pressed
  → HotkeyService thread
  → dictation::start
  → AudioEngine (cpal)
  → dictation:state recording
  → Hotkey Released / tap again
  → AudioEngine stop → WAV
  → ASR :8001 /transcribe
  → optional Ollama :11434 /api/generate
  → enigo text into focused window
  → dictation:state idle + result
  → UI updates last output
```

### File → note

```text
DropZone
  → browser File or Tauri path
  → ASR /transcribe
  → Ollama /api/generate with exact note-taker prompt
  → Note object
  → save_note (SQLite)
  → get_notes refreshes sidebar
  → NoteViewer renders sanitized Markdown
```

### URL → note

```text
NotesTab URL form
  → process_url (Tauri) or direct ASR URL endpoint (browser)
  → ASR /transcribe JSON {url}
      → yt-dlp audio extraction
  → Ollama structuring
  → NoteDraft → save_note
  → url:progress events during the Tauri path
```

### Image/PDF → text

```text
Drop images/PDFs
  → queue (max 50)
  → sequential OCR:
       image → /ocr
       PDF   → /ocr-pdf
  → masks applied in service
  → results panel
  → MD/TXT/CSV export
  → optional /pdf-searchable for the selected PDF
```

### Screen region → clipboard

```text
Ctrl+Shift+S
  → screenshot.rs launches native region selector
  → clipboard image polled
  → /ocr
  → text written to clipboard
  → ocr:result event + UI result
```

### Extension OCR

```text
context menu
  → content script loads
  → worker captureVisibleTab (+ crop)
  → OCR :8002
  → worker → content: copy + panel
```

---

## 8. Security and privacy

- ASR/OCR bind to `127.0.0.1`; Uvicorn is launched with that host.
- `LoopbackOnlyMiddleware` rejects non-loopback socket clients as a second control.
- CORS is restricted to localhost origins, Tauri origins, and extension origins.
- No authentication is used because the services are loopback-only. Do not expose them through a proxy or bind to `0.0.0.0`.
- No telemetry, analytics, crash reporting, cloud fallback, or API keys.
- Model downloads happen only after explicit user consent and go directly to the listed source.
- Remote Ollama is opt-in and is the only allowed outbound generation target.
- Markdown rendering escapes all model output before inserting allow-listed tags.
- Extension host permissions are limited to loopback; page text is only read on user action.

---

## 9. Configuration and design decisions

| Decision | Rationale |
|---|---|
| Svelte 5 runes only | Required by project brief; avoids store/runes mixing |
| Custom CSS for desktop | Small bundle, precise 1000×700 layout |
| Separate Python processes | Crash isolation, independent model stacks, easy `cargo` sidecar-less supervision |
| File lease for model exclusivity | Cross-process, no additional coordinator service |
| `model:"paddle"` contract ID | Stable API even though RapidOCR is default |
| `get_notes` + `list_notes` alias | UI uses the requested `get_notes`; alias preserves earlier contract |
| Camel-case event payloads | Matches the shipped Svelte consumers; differs from the brief’s snake_case sketch |
| `shell:allow-open` | Tauri 2 plugin permission; `core:shell:allow-open` from the brief is not a valid Tauri 2 permission |
| Vite `configLoader native` + in-process TS | Works around sandbox restrictions; normal machines can revert to stock Vite if desired |
| Checked-in extension JS | “Load unpacked from `./extension/`” works immediately |

---

## 10. Known architectural gaps

- Rust has not been type-checked/compiled; any of the above behavior is unverified until `cargo check` succeeds.
- The app’s Tauri runtime has not been launched.
- No integration test exercises the Rust ↔ Python lifecycle.
- No integration test exercises the extension ↔ services path.
- Model download URL/file list is speculative for the current SenseVoice HF repo.
- The `source_type` string is free-form rather than an enum; UI maps `file`, `url`, `dictation`, `manual`.
- The browser preview seeds `localStorage`; the Tauri DB seeds a welcome note. They are intentionally separate.
- FTS5 search falls back to `LIKE` for unusual queries; vector/hybrid search is not implemented (MCP out of scope).
- System-audio capture is device-name selection, not native WASAPI loopback.
- No auto-updater, code signing, or installer signing.
