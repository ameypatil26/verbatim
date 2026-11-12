# Verbatim

**Every word, on your machine. All local. All free.**

Verbatim is a local-first media-to-text workstation. It turns dictation, audio, video, images, PDFs, and media URLs into clean, searchable Markdown without sending content to a cloud service.

One codebase, five surfaces in this handoff:

- **Desktop app** — Tauri 2 + Svelte 5, with four tabs: Dictation, Notes, Vision, Settings.
- **ASR microservice** — FastAPI + FunASR/SenseVoice on `127.0.0.1:8001`.
- **OCR microservice** — FastAPI + RapidOCR-ONNX + PyMuPDF + pyzbar on `127.0.0.1:8002`.
- **Browser extension** — Chrome/Edge MV3, talks only to the localhost services.
- **Landing page** — static Svelte + Tailwind marketing site in `web/`, deployable to Netlify or GitHub Pages.

Ollama supplies local structuring, proofreading, and note chat on `127.0.0.1:11434`.

> **Handoff status:** the Svelte app builds, the extension type-checks and builds, and the Python service contracts are covered by tests. The Rust/Tauri backend source is complete but was **not compiled** in the build sandbox. See [Known limitations](#known-limitations) and [AGENT.md](AGENT.md) before running the desktop app.

---

## Contents

- [Status at handoff](#status-at-handoff)
- [Feature map](#feature-map)
- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Install dependencies](#install-dependencies)
- [Run the Python microservices](#run-the-python-microservices)
- [Run the Tauri desktop app](#run-the-tauri-desktop-app)
- [Models and first launch](#models-and-first-launch)
- [Load the Chrome extension](#load-the-chrome-extension)
- [Where data lives](#where-data-lives)
- [Development commands](#development-commands)
- [Troubleshooting](#troubleshooting)
- [Project layout](#project-layout)
- [Known limitations](#known-limitations)
- [License](#license)

---

## Status at handoff

| Area | State | Evidence |
|---|---|---|
| Svelte 5 UI, four tabs, model setup modal | Implemented | `npm run build` succeeds |
| Type checking | Passes with non-blocking warnings | `npm run check` — 0 errors |
| ASR service contract | Implemented, tested with mock engine | `services/tests/test_asr.py` |
| OCR service, QR, PDF dual layer | Implemented, 12/12 service tests pass | `services/tests/test_ocr.py` |
| Model check/download UI and backend | Implemented | Not exercised against real downloads |
| Tauri Rust backend | Source complete, **not compiled** | `cargo check` blocked by sandbox linking |
| Browser extension | Implemented, type-checked, bundled | 27 KB JS bundle |
| Extension loaded in Chrome | Not manually tested in sandbox | Chrome cannot spawn under this sandbox |
| End-to-end desktop run | Not performed | Requires compiling Tauri outside sandbox |
| MCP plugin | Out of scope for this handoff | Not built |
| Static landing page | Implemented | `npm run build:web` → `web/dist/` |

The Python service tests were run with `.pydeps` and the mock ASR engine. See [AGENT.md](AGENT.md) for the exact verification history.

---

## Feature map

### Dictation

- Global hotkey `Ctrl+Shift+D` (rebindable).
- Tap-to-talk and push-to-talk modes.
- Microphone capture through `cpal`.
- System-audio source attempts to use a loopback/monitor input device (`Stereo Mix`, virtual cable, PulseAudio monitor).
- Local ASR through `POST http://127.0.0.1:8001/transcribe`.
- Optional Ollama proofreading.
- Text typed into the focused application with `enigo`; Linux uses `xdotool` and macOS uses `cliclick` as fallbacks.
- Last output, copy, and undo in the UI.
- Live level waveform derived from Rust `dictation:level` events.

### Notes

- Drag/drop audio or video.
- Paste a YouTube/TikTok URL.
- URL path uses the ASR service's `yt-dlp` fallback, then Ollama.
- Exact structuring prompt is stored in `src/lib/api.ts`:
  `"You are a note-taker. Convert this raw transcript into ... Do not add information not in the transcript."`
- Markdown rendering uses a small sanitizing renderer in `src/lib/markdown.ts`.
- Chat with the transcript using the exact system prompt: `"Answer only from the provided transcript. If the answer is not present, say so."`
- Notes and chat messages persist in SQLite through Tauri commands.
- Browser preview stores notes in `localStorage` instead.

### Vision

- Hotkey `Ctrl+Shift+S` opens the OS region-capture tool, OCRs the selected region, and copies the text.
- Batch queue, sequential processing, maximum 50 files, 2 GB per file.
- Images and PDFs.
- Draw ignore regions on an image; masks are sent to `/ocr` as `[{x,y,w,h}]`.
- QR/barcode detection through `pyzbar` when available.
- Searchable PDF export through PyMuPDF dual-layer rendering (page image plus invisible text).
- Export current result as MD/TXT, batch as CSV.

### Settings

- Model status, download, reinstall, and delete.
- ASR/OCR service restart.
- Ollama endpoint/model test.
- Hotkey rebinding with validation.
- Language, default format, and proofread defaults.
- Read-only state and re-openable model setup when models are missing.

### Browser extension

- Right-click **Verbatim OCR** on an image or page region.
- Right-click **Verbatim Capture** to summarize visible page text with Ollama.
- Right-click **Verbatim Transcribe** to send a detected media URL to ASR.
- Configurable floating button.
- Popup with detection, service health, ports, master enable, and floating-button settings.
- All requests go to `127.0.0.1`/`localhost`.

---

## Architecture

```text
┌──────────────────────┐   ┌──────────────────────┐   ┌──────────────────────┐
│  Tauri desktop       │   │ Chrome MV3 extension │   │  Local tools/tests   │
│  Svelte 5 UI         │   │ background/content   │   │  curl / pytest       │
└──────────┬───────────┘   └──────────┬───────────┘   └──────────┬───────────┘
           │ IPC (Tauri commands)     │ HTTP                      │ HTTP
           ▼                          │                           │
┌──────────────────────┐              │                           │
│ Rust shell           │              │                           │
│ SQLite, cpal, enigo  │              │                           │
│ global-hotkey        │              │                           │
│ service supervisor   │              │                           │
└──────────┬───────────┘              │                           │
           │ spawn + health           │                           │
           ▼                          ▼                           ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ Localhost only · no auth · all data stays on this machine                    │
│                                                                             │
│  ASR 127.0.0.1:8001          OCR 127.0.0.1:8002          Ollama 127.0.0.1:11434│
│  FastAPI + FunASR            FastAPI + RapidOCR          /api/generate        │
│  /health /transcribe         /health /ocr /ocr-batch      /api/chat           │
│  /unload                     /ocr-pdf /pdf-searchable     /api/tags           │
│                              /unload                                         │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
                                   ▼
                     ~/.verbatim/notes.db (SQLite + FTS5)
                     ~/.verbatim/models/
                     ~/.verbatim/logs/
```

- Services bind to `127.0.0.1` only.
- A cross-process file lease in `services/common/verbatim_common.py` ensures ASR and OCR models are never resident at the same time. A service unloads its peer before loading.
- Both services unload after 5 minutes idle by default (`VERBATIM_MODEL_IDLE_SECONDS`).
- The only permitted outbound connection is a user-configured remote Ollama endpoint.

See [ARCHITECTURE.md](ARCHITECTURE.md) for IPC commands, event payloads, database schema, and process details.

---

## Prerequisites

### Required for the desktop app

- **Windows 10/11**, macOS, or Linux. This handoff was authored on Windows.
- **Node.js 20.19+** and npm.
- **Python 3.10–3.12** (3.11 recommended for FunASR/torch compatibility).
- **Rust stable** with the platform linker:
  - Windows: Rust MSVC toolchain + Visual Studio Build Tools with “Desktop development with C++”.
  - macOS: Xcode Command Line Tools.
  - Linux: `build-essential`, `libwebkit2gtk-4.1-dev`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`.
- **WebView2 runtime** on Windows (normally present on Windows 11).
- **Ollama** from <https://ollama.com>.
- **ffmpeg** and **ffprobe** on `PATH` for audio conversion and URL media.
- **yt-dlp** in the Python environment (installed by `services/asr/requirements.txt`).
- **zbar** if `pyzbar` cannot load its bundled DLL (Linux package `libzbar0`).

### Recommended Ollama model

```powershell
ollama pull qwen2.5-coder:7b
```

### Optional development tools

- Chrome or Edge for the extension.
- `curl` for service health checks.
- Git for version control.

---

## Install dependencies

From `F:\deepseek\project` (or the repository root):

```powershell
# Node / Svelte / Tauri CLI
npm install

# Python environment
py -3.12 -m venv .venv
.\.venv\Scripts\Activate.ps1

python -m pip install --upgrade pip

# ASR (FunASR, torch, yt-dlp). This is a large install.
python -m pip install -r services\asr\requirements.txt

# OCR (FastAPI, PyMuPDF, RapidOCR, pyzbar)
python -m pip install -r services\ocr\requirements.txt
```

> The ASR dependency install downloads PyTorch and FunASR and can take several GB. The OCR install is much smaller. If you only need the UI/contract smoke test, you can skip FunASR and run the ASR service with `VERBATIM_ASR_ENGINE=mock`; install at least `fastapi`, `uvicorn`, `python-multipart`, and `httpx`.

### Install Rust (if not already installed)

Windows, PowerShell:

```powershell
winget install Rustlang.Rustup
winget install Microsoft.VisualStudio.2022.BuildTools
```

In the Visual Studio Installer, add the **Desktop development with C++** workload. Restart the terminal and verify:

```powershell
rustc --version
cargo --version
```

If you use the GNU toolchain that was installed inside this workspace during the sandbox attempt, see [AGENT.md](AGENT.md) for the local `RUSTUP_HOME`/`CARGO_HOME` paths and the linker caveat.

---

## Run the Python microservices

The Tauri supervisor normally starts these for you. To run or debug them independently, open **two terminals** from the repository root and activate the virtual environment in each.

### ASR — port 8001

```powershell
cd F:\deepseek\project
.\.venv\Scripts\Activate.ps1
python services\asr\main.py
```

Health check:

```powershell
curl.exe http://127.0.0.1:8001/health
# {"status":"ok","model":"sensevoice","loaded":false}
```

If FunASR is not installed but you want to exercise the HTTP contract:

```powershell
$env:VERBATIM_ASR_ENGINE = "mock"
python services\asr\main.py
```

### OCR — port 8002

```powershell
cd F:\deepseek\project
.\.venv\Scripts\Activate.ps1
python services\ocr\main.py
```

Health check:

```powershell
curl.exe http://127.0.0.1:8002/health
# {"status":"ok","model":"paddle","loaded":false}
```

### Test a transcription / OCR without the desktop app

```powershell
# ASR: replace sample.wav with a real file
curl.exe -X POST http://127.0.0.1:8001/transcribe -F "file=@sample.wav" -F "language=auto"

# OCR
curl.exe -X POST http://127.0.0.1:8002/ocr -F "file=@scan.png" -F 'masks=[]'
```

### Environment variables

| Variable | Default | Purpose |
|---|---|---|
| `VERBATIM_DATA_DIR` | `~/.verbatim` | SQLite, models, logs, temp |
| `VERBATIM_LOG_DIR` | `~/.verbatim/logs` | Service and app logs |
| `VERBATIM_MODEL_DIR` | `~/.verbatim/models` | Model storage root |
| `VERBATIM_ASR_MODEL` | `~/.verbatim/models/asr` | Local SenseVoice directory |
| `VERBATIM_OCR_MODEL` | `~/.verbatim/models/ocr` | OCR marker/models directory |
| `VERBATIM_ASR_ENGINE` | `sensevoice` | Set `mock` for contract testing |
| `VERBATIM_OCR_ENGINE` | `rapidocr` | `rapidocr`, `paddle`, or `mock` |
| `VERBATIM_MODEL_IDLE_SECONDS` | `300` | Idle unload delay |
| `VERBATIM_DISABLE_IDLE` | unset | Set `1` to keep models loaded |
| `VERBATIM_ASR_PORT` | `8001` | ASR port |
| `VERBATIM_OCR_PORT` | `8002` | OCR port |
| `VERBATIM_PYTHON` | unset | Explicit Python executable for the Tauri supervisor |
| `VERBATIM_SERVICES_DIR` | unset | Explicit services directory for the Tauri supervisor |
| `VERBATIM_FFMPEG` | `ffmpeg` on PATH | Explicit ffmpeg path |

---

## Run the Tauri desktop app

Once Python dependencies, Ollama, ffmpeg, and Rust are installed:

```powershell
cd F:\deepseek\project
npm run tauri dev
```

What happens:

1. Tauri runs `npm run dev` (Vite on `http://127.0.0.1:1420`).
2. The Rust setup hook opens `~/.verbatim/notes.db`, seeds a welcome note if empty, checks models, installs hotkeys, and starts the service supervisor.
3. The supervisor locates `.venv\Scripts\python.exe` (or `VERBATIM_PYTHON`), starts ASR and OCR, writes their stdout/stderr to `~/.verbatim/logs/asr-service.log` and `ocr-service.log`, and health-checks ports 8001/8002. If a service is already running, it is adopted as an external process.
4. If any model is missing, a blocking modal appears. Choose **Download now** or **Cancel**. Cancel opens the app in read-only mode; Settings → Models can re-open the setup.
5. The three localhost services power the four tabs.

Build a release bundle (requires a working native Rust toolchain):

```powershell
npm run tauri build
```

Artifacts are written under `src-tauri/target/release/bundle/` (MSVC) or `.cargo-target` if `CARGO_TARGET_DIR` is set.

### Front-end-only preview

```powershell
npm run dev
```

Open `http://127.0.0.1:1420`. Tauri commands are unavailable, so the app falls back to browser behavior: transcription/OCR use the localhost services directly, notes use `localStorage`, and dictation only records into the browser tab. For UI screenshots without the model modal, append:

```text
http://127.0.0.1:1420/?demo=1&tab=dictation
```

Valid tabs: `dictation`, `notes`, `vision`, `settings`.

### Global hotkeys

- `Ctrl+Shift+D` — dictation.
- `Ctrl+Shift+S` — screenshot region OCR.
- Rebind in Settings → Hotkeys. The Rust hotkey thread unregisters the old keys and registers the new ones without restarting.

---

## Models and first launch

The app does **not** download models silently. It checks the three required assets and asks first.

| Model ID | Name | Approx. size | Source / mechanism |
|---|---|---|---|
| `sensevoice` | SenseVoice Small | ~228 MB | Hugging Face `iic/SenseVoiceSmall`, fallback `FunAudioLLM/SenseVoiceSmall` |
| `rapidocr` | RapidOCR ONNX | ~15 MB | Bundled in the `rapidocr-onnxruntime` wheel; script warms and marks it ready |
| `qwen2.5-coder:7b` | Qwen 2.5 Coder 7B | ~4.7 GB | `ollama pull qwen2.5-coder:7b` |

### Manual model installation

```powershell
# SenseVoice through FunASR / ModelScope
python services\asr\download_models.py

# RapidOCR warm-up + marker
python services\ocr\download_models.py

# LLM
ollama pull qwen2.5-coder:7b
```

Model files live under `~/.verbatim/models/`. Check status with:

```powershell
ollama list
Get-ChildItem $HOME\.verbatim\models -Recurse
```

### What the UI receives

The Tauri command `download_model(model)` emits:

```json
{
  "model": "sensevoice",
  "downloadedMb": 12.4,
  "totalMb": 228.0,
  "status": "downloading"
}
```

`status` is `downloading`, `done`, or `error`. The event name is `model-download-progress`. The Svelte side uses camelCase properties; see [ARCHITECTURE.md](ARCHITECTURE.md).

If a user cancels the blocking prompt:

- the app stays open in a read-only state;
- the StatusBar shows **“Models not downloaded”**;
- Settings → Models and the header chip can re-open setup;
- no model download occurs without another click.

---

## Load the Chrome extension

### Build

```powershell
cd F:\deepseek\project
npm run extension:build
npm run extension:check
```

`extension:build` transpiles `extension/src/background.ts` and `extension/src/content.ts` in-process with the TypeScript compiler API and writes `extension/background.js` and `extension/content.js`. The extension bundle is under 500 KB (currently ~27 KB).

### Load unpacked

1. Open Chrome/Edge and go to `chrome://extensions` or `edge://extensions`.
2. Turn on **Developer mode**.
3. Click **Load unpacked**.
4. Select `F:\deepseek\project\extension` (the folder containing `manifest.json`).
5. Pin the Verbatim extension.
6. Start the desktop app or both Python services first.

The popup reports **“Verbatim detected”** only when both ASR (`:8001`) and OCR (`:8002`) answer. Ollama is reported separately. If a port is changed in the app, update the extension popup’s ports to match.

### Context actions

- **Verbatim OCR** on an image crops that image out of a viewport screenshot; on a page/selection it opens a crosshair region selector. Text is copied and shown in a page panel.
- **Verbatim Capture** summarizes visible text with Ollama.
- **Verbatim Transcribe** finds a media URL, or falls back to the page URL for YouTube/TikTok/Vimeo, and asks ASR (which uses `yt-dlp`).
- The floating button captures the visible viewport and OCRs it.

### Permissions

`activeTab`, `tabs`, `contextMenus`, `scripting`, `storage`, `clipboardWrite`. Host access is limited to `http://127.0.0.1/*` and `http://localhost/*`.

> `chrome.tabs.captureVisibleTab` needs an activeTab grant. Context-menu actions and the popup receive it. A click on the in-page floating button is not an extension UI gesture on every Chrome build; if capture is denied, use the toolbar popup’s **OCR visible tab** button.

---

## Build the landing page

The static marketing site lives in `web/` (Svelte 5 + Tailwind CSS 3).

```powershell
npm run web:dev      # http://127.0.0.1:5173
npm run build:web    # static output in web/dist/
```

Deploy `web/dist/` to Netlify or GitHub Pages. The build uses a relative base, so it works at a domain root or a project subpath. See `web/README.md` for deploy settings.

The site is separate from the desktop webview and is not bundled into the Tauri binary.

## Where data lives

```text
~/.verbatim/
├── notes.db                 SQLite (WAL) with notes, messages, settings, FTS5
├── notes.db-wal / -shm
├── models/
│   ├── asr/                 SenseVoice files + .installed
│   └── ocr/                 .installed marker (RapidOCR ships in the wheel)
├── logs/
│   ├── verbatim.log.*       Rotating app logs, 7-day cleanup
│   ├── asr-service.log
│   └── ocr-service.log
├── tmp/                     Recordings and transient media
└── model-lease.lock         Cross-service model arbitration
```

No telemetry, analytics, accounts, API keys, or cloud fallbacks are implemented. The only outbound request is to the configured Ollama URL or a one-time model download when the user explicitly clicks Download.

---

## Development commands

```powershell
# Svelte type checking (0 errors expected; a few state-capture warnings remain)
npm run check

# Build the desktop webview
npm run build

# Vite dev server only
npm run dev

# Extension
npm run extension:build
npm run extension:check

# Static landing page (outputs web/dist/)
npm run build:web

# Python tests (use the local .pydeps fallback if .venv is unavailable)
$env:PYTHONPATH = (Resolve-Path .pydeps).Path
$env:TEMP = (Resolve-Path .tmp).Path
$env:TMP = $env:TEMP
python -m pytest services\tests -q

# Rust
cargo check --manifest-path src-tauri\Cargo.toml
```

### Verification evidence from the handoff

- `npm run build`: succeeds (Vite 6.4.5, 118 modules; `dist/` produced).
- `npm run check`: 0 errors, 7 warnings (the command exits non-zero because warnings are present).
- `npm run extension:check`: succeeds.
- `npm run extension:build`: succeeds, 27.0 KB.
- `node --check` on all extension JS: succeeds.
- `pytest services/tests -q`: **12 passed**, 1 third-party deprecation warning.
- `cargo check`: attempted; 602 crates resolved and downloaded, then the GNU linker failed because Windows could not create a temp file. See [AGENT.md](AGENT.md).

---

## Troubleshooting

### “Verbatim not detected” in the extension

- Start `npm run tauri dev`, or run both Python services.
- Check `http://127.0.0.1:8001/health` and `http://127.0.0.1:8002/health`.
- Confirm the popup ports match the desktop Settings URLs.
- Extension host permissions only cover loopback, by design.

### Port 8001/8002 already in use

The supervisor adopts a healthy external service and leaves it running when the app exits. To debug cleanly, stop the standalone servers before starting Tauri.

### ASR returns 503 “model failed to load”

- Run `python services\asr\download_models.py`.
- Confirm `~/.verbatim/models/asr/model.pt` exists.
- Install `funasr`, `modelscope`, `torch`, and `torchaudio`.
- Check `~/.verbatim/logs/asr-service.log`.
- For UI-only testing, set `VERBATIM_ASR_ENGINE=mock`.

### OCR returns 503

- Run `python services\ocr\download_models.py`.
- Confirm `rapidocr-onnxruntime` imports in the same Python environment.
- Check `~/.verbatim/logs/ocr-service.log`.

### ffmpeg / yt-dlp errors

Install ffmpeg and put it on `PATH`; confirm `yt-dlp --version`. The ASR container uses ffmpeg for non-WAV input and URL media.

### Searchable PDF has an image but no selectable text

- PyMuPDF must place invisible text under the page image. The implementation groups OCR boxes into lines and inserts with `render_mode=3`.
- If text is empty, inspect OCR output first; the hidden layer is generated from the internal OCR boxes.
- Regenerate after the fix described in `services/ocr/main.py` (`_searchable_pdf` now calls `engine.run` directly and keeps text-bearing boxes).

### Hotkeys do not register

- Another app owns the combination; choose another in Settings.
- On Linux, a running X11/Wayland portal may restrict global shortcuts.
- The hotkey thread logs errors to `~/.verbatim/logs/verbatim.log.*`; check for `global hotkeys unavailable`.

### System audio is unavailable

- Windows: enable **Stereo Mix**, or install a virtual loopback device. The app selects an input whose name contains loopback/monitor/stereo mix/cable output.
- Linux: choose a PulseAudio monitor source.
- macOS: system-audio capture is explicitly not implemented; use microphone input.

### Tauri build fails on Windows

Recommended path: install Visual Studio Build Tools with **Desktop development with C++** and use the MSVC Rust toolchain. The sandbox attempt used a GNU toolchain and failed at the linker temp-file step, not in application code. See `AGENT.md` for the exact error.

### Vite build in a restricted Windows shell

`vite.config.js` uses `esbuild: false`, an in-process TypeScript transform, and the native config loader because this sandbox blocks esbuild's child process. A local `node_modules/vite` patch skips `net use` probing. Normal developer machines do not need the patch; `npm install` restores stock Vite.

---

## Project layout

```text
verbatim/
├── src/                         Svelte 5 desktop UI
│   ├── App.svelte               shell, tabs, model modal, health polling
│   ├── app.css                  dark theme tokens
│   ├── main.ts
│   └── lib/
│       ├── api.ts               Tauri IPC + HTTP + browser fallbacks
│       ├── types.ts             shared TypeScript contracts
│       ├── settings.ts          defaults and validation
│       ├── models.ts            required model catalog
│       ├── markdown.ts          sanitizing Markdown renderer
│       ├── tabs/                Dictation, Notes, Vision, Settings
│       └── components/          shared UI + model setup
├── src-tauri/                   Rust/Tauri backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/default.json
│   └── src/                     audio, hotkeys, DB, services, models, etc.
├── services/
│   ├── common/                  loopback/CORS, model lease, audio helpers
│   ├── asr/                     FastAPI SenseVoice service + downloader
│   ├── ocr/                     FastAPI OCR/PDF/QR service + downloader
│   └── tests/                   pytest contract tests
├── extension/                   MV3 extension (TS source + checked-in JS)
│   ├── manifest.json
│   ├── background.js / content.js
│   ├── src/background.ts / content.ts
│   ├── popup/                   plain HTML/CSS/JS
│   └── icons/
├── web/                         static landing page (Svelte + Tailwind)
│   ├── index.html
│   ├── vite.config.js
│   ├── tailwind.config.js
│   ├── src/                     App + section components
│   ├── public/                  favicon + .nojekyll
│   └── dist/                    generated static build
├── scripts/                     icon generation, local pip helper
├── index.html
├── package.json
├── vite.config.js
├── svelte.config.js
└── README.md
```

---

## Known limitations

- **Rust code was never compiled.** Treat `src-tauri/` as review-required until `cargo check` passes on a normal toolchain. The sandbox failure occurred in the GNU linker before reaching the crate.
- **The Tauri app was never launched.** No end-to-end verification of hotkeys, enigo injection, service supervision, screenshot capture, or model download.
- **The extension was not loaded into Chrome.** It type-checks and bundles, but browser behavior is untested.
- Model downloads have not been exercised against live servers. The SenseVoice URL candidate and required file list may need adjustment for the current Hugging Face repository layout.
- Model download progress events use camelCase (`downloadedMb`, `totalMb`) in the shipped Svelte contract, not the snake_case sketch in the original brief.
- Tauri capabilities use `shell:allow-open` (Tauri 2 plugin permission). The brief’s `core:shell:allow-open` is a Tauri 1-era path and was not used.
- System-audio capture is monitor-device selection through `cpal`, not a dedicated WASAPI loopback implementation.
- macOS system audio is intentionally unsupported.
- The browser extension’s in-page floating button may lack an activeTab grant for `captureVisibleTab`; use the popup button or a context-menu gesture.
- No MCP server (`mcp/`) is implemented; it was explicitly out of scope for this session.
- `package.json` references `scripts/setup_services.py` and `scripts/smoke_test.py`, which do not exist. Use the manual commands in this README instead.
- `npm run check` has non-blocking Svelte warnings about initial prop capture and a few state references.
- The workspace `.venv` created during the sandbox attempt has ACL-restricted packages. Use `.pydeps` for the existing tests, or create a fresh `.venv` outside the sandbox. See `AGENT.md`.

---

## License

MIT. There is no `LICENSE` file in the tree yet; add one before distribution.
