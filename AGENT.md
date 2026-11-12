# AGENT.md — Verbatim Handoff

**Audience:** the next AI agent or developer taking over this repository.
**Repo root:** `F:\deepseek\project`
**Handoff date:** 2026-11-05
**Scope of this session:** finish the Rust/Tauri backend, browser extension, model setup UI, and wire Svelte to Tauri. The MCP plugin was explicitly excluded. A follow-up session added the static landing page in `web/`.

Read this before touching code. Nothing in `src-tauri/` has been compiled, so do not assume the Rust backend is correct just because it is written.

---

## 0. TL;DR

Implemented and verified:

- Svelte 5 frontend with four tabs, model setup modal, settings, notes, vision queue, dictation UI.
- Python ASR/OCR microservices with contract tests — **12/12 pass**.
- Chrome MV3 extension with TypeScript source, checked-in JS bundles, plain popup.
- Full Rust/Tauri backend source: SQLite, service supervisor, cpal, global-hotkey, enigo, model downloads, screenshot OCR, URL pipeline.
- `npm run build` succeeds; `npm run extension:check` and `npm run extension:build` succeed.
- Static Svelte + Tailwind landing page in `web/`; `npm run build:web` produces `web/dist/`.

Not verified / blocked:

- `cargo check` never compiled the application crate. Dependencies downloaded, then the Windows GNU linker failed creating a temp file in the sandbox.
- `npm run tauri dev` was never run.
- The extension was never loaded into Chrome (Chrome cannot spawn child processes under the sandbox).
- Model downloads, hotkeys, audio capture, enigo typing, screenshot capture, and service supervision are untested at runtime.
- MCP is absent by design for this handoff. The landing page is now implemented in `web/`.

**Do not run `cargo check` or try to compile in the current sandbox unless the user asks.** The last user instruction was to stop compiling and finish documentation.

---

## 1. What was built

### 1.1 Desktop frontend (Svelte 5)

Four tabs:

1. **Dictation** — tap/hold, mic/system source, waveform, level events, proofread toggle, last output copy/undo.
2. **Notes** — drop media or paste URL, transcribe, Ollama structuring, Markdown viewer, local chat, SQLite persistence.
3. **Vision** — image/PDF queue (max 50, 2 GB/file), sequential OCR, mask drawing, QR/barcode chips, CSV/MD/TXT export, searchable PDF.
4. **Settings** — model status/download/delete/reinstall, ASR/OCR restart, Ollama test, hotkey rebinding, language/format/proofread, privacy.

Additional UI:

- Blocking first-launch **model setup modal** with per-model download progress and Cancel → read-only state.
- StatusBar with ASR/OCR/Ollama dots and “Models not downloaded”.
- Browser preview fallback when Tauri IPC is unavailable.
- `?demo=1&tab=notes` browser-preview mode skips model checks.

The theme uses the requested dark palette. Desktop CSS is custom; the landing page uses Tailwind.

### 1.2 Rust/Tauri backend

- `AppState`, setup hook, window config, capabilities.
- SQLite with notes/messages/settings + FTS5 and triggers.
- Service supervisor for ASR/OCR, health polling, logs, max 3 automatic restarts, clean shutdown.
- cpal audio capture with WAV writer and level events.
- global-hotkey thread with rebind support.
- enigo typing with xdotool/cliclick fallback; Ctrl+Z undo.
- Dictation pipeline: record → ASR → optional Ollama proofread → type.
- Screenshot OCR via native region tool + clipboard + OCR + clipboard text.
- Model manager: required model IDs, Ollama list parsing, dynamic HTTP download progress, script-based OCR preparation, model deletion.
- URL transcription and Ollama note structuring.
- IPC commands consumed by the Svelte API.
- Rotating logs with 7-day retention.

### 1.3 Python microservices

- ASR FastAPI service on 8001 with lazy FunASR/SenseVoice, mock engine, URL fallback, timestamps, idle unload.
- OCR FastAPI service on 8002 with RapidOCR/Paddle/mock, masks, batch, PDF OCR, QR/barcode, dual-layer searchable PDF.
- Shared loopback guard, restricted CORS, cross-service model lease, ffmpeg helpers, 2 GB limits.
- Download scripts for SenseVoice and RapidOCR/Paddle.
- Pytest suite for contracts, masks, batch, PDF, QR, and the searchable PDF fix.

### 1.4 Chrome MV3 extension

- Context menus for OCR, page capture/summarize, and media transcription.
- Region selector in an isolated shadow DOM.
- Image crop from viewport screenshots.
- Ollama page summary and ASR URL transcription.
- Floating OCR button (configurable).
- Popup with health detection, enable/floating toggles, ports, and OCR-visible-tab action.
- Built with TypeScript compiler API; checked-in JS so “Load unpacked” works immediately.
- Bundle 27 KB, far below the 500 KB limit.

---

## 2. The blockage: Windows sandbox linker/temp error

### 2.1 Exact error

After resolving and downloading **602 crates**, `cargo check` reached build-script linking and failed:

```text
error: linking with `x86_64-w64-mingw32-gcc` failed: exit code: 1
  |
  = note: "x86_64-w64-mingw32-gcc" ... "-o" "F:\\deepseek\\project\\.cargo-target\\debug\\build\\...\\build_script_build-....exe" ...
  = note: some arguments are omitted. use `--verbose` to show all linker arguments
  = note: Cannot create temporary file in C:\Users\PRAJAK~1\AppData\Local\Temp\: Permission denied
```

The same failure occurred for `serde_core`, `proc-macro2`, `quote`, `parking_lot_core`, `icu_properties_data`, and `icu_normalizer_data` build scripts.

Earlier, purely fetching crates failed with:

```text
warning: spurious network error (3 tries remaining):
[35] SSL connect error (schannel: AcquireCredentialsHandle failed:
SEC_E_NO_CREDENTIALS (0x8009030e) - No credentials are available in the security package)
```

Crates were successfully downloaded only after running the command with wider sandbox access. The linker failure persisted at the temp-file step. The user then explicitly said:

> Do not run `cargo check` or try to compile again. Stop trying to fix the temp folder or linker errors.

Respect that. Do not spend another session fighting the sandbox.

### 2.2 Environment created during the attempt

Rust was installed inside the workspace (not system-wide):

```text
RUSTUP_HOME = F:\deepseek\project\.rustup
CARGO_HOME  = F:\deepseek\project\.cargo
toolchain   = stable-x86_64-pc-windows-gnu (rustc 1.99.1)
gcc         = F:\msys64\ucrt64\bin\gcc.exe (MSYS2 UCRT64, gcc 13.2.0)
```

This was a workaround for the sandbox's TLS/toolchain restrictions. It is **not** the recommended build setup for a normal Windows machine.

### 2.3 Recommended fix

On the local machine, outside the sandbox:

1. Install **Rust stable MSVC** and **Visual Studio Build Tools** with “Desktop development with C++”.
2. Use the standard MSVC linker (`link.exe`), not the GNU toolchain.
3. Run from a normal terminal with a normal `%TEMP%`:
   ```powershell
   cd F:\deepseek\project
   npm run tauri dev
   ```
4. If compile errors appear, fix them normally. The code has never been through `rustc` as a crate, so expect some.
5. Do not reuse the local `.rustup`/`.cargo` unless you intentionally want GNU.

---

## 3. Verification performed (evidence)

| Check | Command | Result |
|---|---|---|
| Frontend build | `npm run build` | ✅ succeeded, 118 modules, `dist/` |
| Svelte diagnostics | `npm run check` | ⚠️ 0 errors, 7 warnings; process exits non-zero because warnings exist |
| Extension type check | `npm run extension:check` | ✅ exit 0 |
| Extension build | `npm run extension:build` | ✅ 27.0 KB |
| Extension JS syntax | `node --check extension/background.js`, `content.js`, `popup/popup.js` | ✅ exit 0 |
| Landing page build | `npm run build:web` | ✅ `web/dist/` produced, 110 modules |
| Service tests | `pytest services/tests -q` | ✅ 12 passed, 1 third-party deprecation warning |
| Rust compile | `cargo check` | ❌ linker temp permission (see above) |
| Tauri launch | `npm run tauri dev` | ❌ not attempted / needs native toolchain |
| Extension in Chrome | manual | ❌ not attempted; Chrome cannot spawn in sandbox |

The service tests used:

```powershell
$env:PYTHONPATH = (Resolve-Path .pydeps).Path
$env:TEMP = (Resolve-Path .tmp).Path
$env:TMP = $env:TEMP
python -m pytest services\tests -q
```

Test result after the final searchable-PDF fix:

```text
............                                                             [100%]
12 passed, 1 warning in 54.92s
```

### 3.1 Frontend warnings

`npm run check` reports seven `state_referenced_locally` warnings in `DictationTab.svelte` and `SettingsForm.svelte` because local `$state` values are initialized from props. They are non-blocking. The production build succeeds.

### 3.2 Searchable PDF bug fixed

`services/ocr/main.py` `_searchable_pdf` now calls `engine.run(image)` directly and uses the internal text-bearing boxes. It previously used public response boxes from `_engine_run`, which intentionally strip `text`, leaving the hidden layer empty. The regression test passes.

---

## 4. Complete file inventory and responsibilities

Generated/ignored files (`node_modules/`, `dist/`, `.pydeps/`, `.venv/`, `.tmp/`, `.cargo/`, `.rustup/`, `.cargo-target/`, `screenshots/`) are not listed except where important.

### 4.1 Root

| File | Purpose |
|---|---|
| `package.json` | Scripts and Node dependencies. References some missing helper scripts; see §6. |
| `package-lock.json` | npm lockfile. |
| `index.html` | Vite/Tauri webview entry. |
| `vite.config.js` | Svelte plugin, in-process TypeScript plugin, `esbuild: false`, native config loader, port 1420. |
| `svelte.config.js` | In-process TypeScript preprocessor; avoids Vite/esbuild preprocessor. |
| `tsconfig.json` | Frontend TS config. |
| `postcss.config.js` | Tailwind/PostCSS config for future web page. |
| `tailwind.config.js` | Tailwind theme with Verbatim colors for future web page. |
| `pytest.ini` | Test paths, no cache provider, warning filters. |
| `.gitignore` | Does **not** yet ignore `.cargo/`, `.rustup/`, `.cargo-target/`, `.pydeps/`, `.pip-cache/`, `.npm-cache/`, `.tools/`; add these before committing. |
| `README.md` | Install/run/troubleshoot documentation. |
| `ARCHITECTURE.md` | System design, IPC, events, schema, flows. |
| `AGENT.md` | This handoff. |

### 4.2 Svelte desktop UI (`src/`)

| File | Purpose |
|---|---|
| `src/main.ts` | Mounts Svelte App. |
| `src/App.svelte` | Shell, header/nav, healthy polling, model report, download orchestration, read-only banner, tab switching, preview flags. |
| `src/app.css` | Dark theme tokens, reset, global utilities. |
| `src/lib/api.ts` | Tauri invoke + localhost HTTP + browser localStorage fallbacks; event subscriptions; prompts; download helpers. |
| `src/lib/types.ts` | Note, OCR, model, health, dictation, settings types. |
| `src/lib/settings.ts` | Defaults, URL normalization, hotkey validation. |
| `src/lib/models.ts` | `REQUIRED_MODELS` catalog (must match Rust constants). |
| `src/lib/markdown.ts` | Escaping Markdown renderer and title extraction. |
| `src/lib/toast.svelte.ts` | Runes toast queue. |
| `src/lib/tabs/DictationTab.svelte` | Dictation UI, MediaRecorder fallback, Tauri event listeners, copy/undo. |
| `src/lib/tabs/NotesTab.svelte` | Notes library, file/path/URL import, summarization, persistence, chat. |
| `src/lib/tabs/VisionTab.svelte` | OCR queue, sequential processing, masks, capture, searchable PDF, CSV. |
| `src/lib/tabs/SettingsTab.svelte` | Prop bridge to `SettingsForm`. |
| `src/lib/components/Waveform.svelte` | Animated level bars. |
| `src/lib/components/DropZone.svelte` | File input + Tauri drag-drop paths. |
| `src/lib/components/NoteList.svelte` | Search/sidebar list/delete/new. |
| `src/lib/components/NoteViewer.svelte` | Markdown document + local chat. |
| `src/lib/components/OcrResult.svelte` | Selectable text, copy, exports, QR chips, searchable PDF. |
| `src/lib/components/StatusBar.svelte` | ASR/OCR/Ollama health, models warning, retry. |
| `src/lib/components/SettingsForm.svelte` | Model/service/hotkey/Ollama/output/privacy settings UI. |
| `src/lib/components/ModelSetup.svelte` | Blocking download prompt with per-model progress. |
| `src/lib/components/ImageMasks.svelte` | Draw/remove ignore rectangles over an image. |
| `src/lib/components/Markdown.svelte` | Sanitized Markdown display + typography. |
| `src/lib/components/ToastHost.svelte` | Toast rendering. |

### 4.3 Rust/Tauri (`src-tauri/`)

| File | Purpose |
|---|---|
| `src-tauri/Cargo.toml` | Tauri 2, plugins, rusqlite bundled, reqwest, cpal, enigo, global-hotkey, arboard, image, logging. |
| `src-tauri/Cargo.lock` | Generated with 602 crates; use for reproducible builds. |
| `src-tauri/build.rs` | `tauri_build::build()`. |
| `src-tauri/tauri.conf.json` | Product, 1000×700 window, CSP, bundle resources, icons. |
| `src-tauri/capabilities/default.json` | `core:default`, window close/minimize/focus/drag, `shell:allow-open`, `opener:default`. |
| `src-tauri/icons/*` | Generated icons: 32, 128, 256, 512, ICO. |
| `src-tauri/src/main.rs` | `verbatim_lib::run()`. |
| `src-tauri/src/lib.rs` | Modules, `AppState`, builder/setup, logging, DB seed, hotkeys, services, exit shutdown. |
| `src-tauri/src/models.rs` | Serde structs (Note, ChatMessage, OCR, settings, models, events). |
| `src-tauri/src/db.rs` | SQLite schema/CRUD/FTS5/settings/messages. |
| `src-tauri/src/settings.rs` | Defaults/normalization/persistence. |
| `src-tauri/src/services.rs` | Python resolution, spawn, logs, port checks, restart, shutdown, status events. |
| `src-tauri/src/audio.rs` | cpal capture, device selection, level events, WAV writing. |
| `src-tauri/src/dictation.rs` | start/stop, ASR call, proofreading, enigo injection, state events. |
| `src-tauri/src/injection.rs` | enigo + xdotool/cliclick; Ctrl+Z undo. |
| `src-tauri/src/hotkeys.rs` | global-hotkey thread, parser, rebinding, actions. |
| `src-tauri/src/screenshot.rs` | Region capture via OS tools + clipboard, OCR, clipboard text. |
| `src-tauri/src/model_manager.rs` | Model checks, HTTP streaming downloads, Ollama pull, OCR script, delete. |
| `src-tauri/src/commands.rs` | All registered IPC commands and local service HTTP helpers. |
| `src-tauri/src/logging.rs` | Daily rotating app logs, 7-day cleanup. |

### 4.4 Python services (`services/`)

| File | Purpose |
|---|---|
| `services/common/__init__.py` | Package marker. |
| `services/common/verbatim_common.py` | Paths, logging, CORS, loopback middleware, model lease, masks, ffmpeg/WAV, idle unloader. |
| `services/asr/__init__.py` | Package marker. |
| `services/asr/main.py` | FastAPI ASR, FunASR SenseVoice/mock, URL yt-dlp fallback, `/health`, `/transcribe`, `/unload`. |
| `services/asr/requirements.txt` | FastAPI, FunASR, torch, torchaudio, modelscope, numpy, soundfile, yt-dlp. |
| `services/asr/download_models.py` | ModelScope/HF SenseVoice download with JSON progress. |
| `services/ocr/__init__.py` | Package marker. |
| `services/ocr/main.py` | FastAPI OCR, RapidOCR/Paddle/mock, masks, batch, PDF OCR, dual-layer PDF, QR, unload. |
| `services/ocr/requirements.txt` | FastAPI, RapidOCR, Pillow, PyMuPDF, pyzbar, numpy. |
| `services/ocr/download_models.py` | Warms RapidOCR/Paddle and writes install marker. |
| `services/tests/conftest.py` | Test env/data dir, mock ASR, disabled idle unload. |
| `services/tests/test_asr.py` | Health, multipart/JSON/base64, unload, 400 cases. |
| `services/tests/test_ocr.py` | Health, OCR, masks, batch, PDF OCR, searchable PDF text layer, QR, limits. |

### 4.5 Extension (`extension/`)

| File | Purpose |
|---|---|
| `extension/manifest.json` | MV3 manifest, permissions, host permissions, worker/content/popup. |
| `extension/src/background.ts` | Service worker source. |
| `extension/src/content.ts` | Page integration source. |
| `extension/background.js` | Generated/checked-in worker. |
| `extension/content.js` | Generated/checked-in content script. |
| `extension/build.mjs` | TS compiler API build + 500 KB budget. |
| `extension/tsconfig.json` | Extension TS config, Chrome types. |
| `extension/popup/popup.html` | Plain popup markup. |
| `extension/popup/popup.css` | Plain popup styling. |
| `extension/popup/popup.js` | Popup settings/status/actions. |
| `extension/icons/icon16.png` | Toolbar icon. |
| `extension/icons/icon48.png` | Extension icon. |
| `extension/icons/icon128.png` | Store icon. |

### 4.6 Landing page (`web/`)

| File | Purpose |
|---|---|
| `web/index.html` | Static HTML shell, meta tags, JSON-LD |
| `web/vite.config.js` | Svelte + in-process TS, relative base, outputs `web/dist` |
| `web/svelte.config.js` | Landing-specific TS preprocessor (preserves component imports) |
| `web/tailwind.config.js` | Landing palette/extensions |
| `web/postcss.config.js` | Tailwind + autoprefixer |
| `web/src/main.ts` | Mounts landing App |
| `web/src/App.svelte` | Section composition |
| `web/src/app.css` | Tailwind layers + custom hero/accordion/animation styles |
| `web/src/lib/Navbar.svelte` | Sticky nav + mobile menu |
| `web/src/lib/Hero.svelte` | Headline, CTAs, animated app/waveform mock |
| `web/src/lib/Features.svelte` | Four feature cards |
| `web/src/lib/HowItWorks.svelte` | Three-step flow + privacy callout |
| `web/src/lib/Architecture.svelte` | Interfaces → localhost services → SQLite diagram |
| `web/src/lib/Comparison.svelte` | Verbatim vs Otter.ai table |
| `web/src/lib/Faq.svelte` | Six-question native accordion |
| `web/src/lib/Footer.svelte` | Final CTA + link columns |
| `web/src/lib/Logo.svelte` | Shared wordmark |
| `web/public/favicon.png` | Favicon |
| `web/public/.nojekyll` | GitHub Pages compatibility |
| `web/README.md` | Build/deploy notes |
| `web/dist/` | Generated static site (gitignored) |

### 4.7 Scripts and generated assets

| File | Purpose |
|---|---|
| `scripts/make_icons.py` | Pillow icon generator for Tauri, extension, web favicon. |
| `scripts/pip_install_local.py` | Sandbox-only pip temp workaround with in-process `mkdtemp` patch. Keep; does not affect normal installs. |
| `web/favicon.png` | Generated 64 px favicon used by the landing page. |

### 4.8 Empty/out-of-scope directories

- `mcp/` — empty.
- `web/` — implemented static landing page; see `web/README.md`.
- `scripts/setup_services.py` — referenced by `package.json`, **does not exist**.
- `scripts/smoke_test.py` — referenced by `package.json`, **does not exist**.

---

## 5. What is left to do

### Priority 0 — get a working compile

Do this outside the sandbox, ideally with MSVC:

```powershell
cd F:\deepseek\project
npm install
# use a normal Rust MSVC toolchain + VS Build Tools
cargo check --manifest-path src-tauri\Cargo.toml
```

Expect fixes in these likely areas (unverified, since `rustc` never saw the crate):

- Tauri 2 API details in `services.rs` (`Arc` receiver use, app handle, resource paths).
- `db.rs` generic `collect_notes` signature with `impl FnMut`.
- `commands.rs` `tauri` argument naming / `State` + async Send bounds.
- `hotkeys.rs` `global-hotkey` API versions (0.6.4 resolved), `Modifiers::SUPER`, event IDs.
- `enigo` 0.2 import/API (`Enigo::new`, `Keyboard::text`, `Key::Unicode`).
- `cpal` 0.15 StreamConfig/sample format APIs.
- `screenshot.rs` `arboard::ImageData` ownership and `image` crate API.
- `model_manager.rs` partial moves in `setup_status`, reqwest stream handling, `tokio::select!`.
- Tauri capability validation: `shell:allow-open` requires the shell plugin; capabilities currently omit the brief’s invalid Tauri 1 `core:shell:allow-open`.
- Tauri config resource path/glob.

### Priority 1 — runtime test

1. Create a fresh Python venv outside the sandbox.
2. Install ASR/OCR requirements.
3. Pull `qwen2.5-coder:7b`.
4. Run `npm run tauri dev`.
5. Verify:
   - ASR/OCR auto-start, health turns green, logs written.
   - Model modal appears and download progress is sane.
   - `Ctrl+Shift+D` captures microphone, transcribes, types into Notepad.
   - `Ctrl+Shift+S` opens native snipping, OCRs, copies text.
   - Notes file/URL import persists after restart.
   - Vision batch, masks, QR, searchable PDF.
   - Settings hotkey rebind works and persists.
   - Service kill → UI error → restart.

### Priority 2 — extension test

1. Start services.
2. Load unpacked `extension/`.
3. Test each context menu and popup.
4. Verify “Verbatim not detected” when services stop.
5. Check `captureVisibleTab` behavior for the floating button (activeTab caveat).

### Priority 3 — model download reality check

- Confirm the actual current SenseVoice HF repo file list. `ASR_FILES` currently expects:
  `model.pt`, `config.yaml`, `tokens.json`, `am.mvn`, `configuration.json`.
- Confirm `iic/SenseVoiceSmall` exists; fallback is `FunAudioLLM/SenseVoiceSmall`.
- HEAD requests may return no `Content-Length`; progress may show 0 total for some files.
- OCR progress wraps a script and jumps 0→100; acceptable but not granular.
- Event payload is camelCase (`downloadedMb`), not the snake_case in the original brief.
- Delete/reinstall behavior has not been tested.

### Priority 4 — missing pieces from the original product brief

- MCP binary (`verbatim-mcp`) — out of scope this session.
- `scripts/setup_services.py` and `scripts/smoke_test.py` — missing; either implement or remove scripts.
- `.gitignore` additions for local Rust/npm/pip caches and `.pydeps`.
- `LICENSE` file (package says MIT).
- Installers/code signing.

### Priority 5 — polish

- Reduce `svelte-check` warnings (7).
- Add real screenshot/browser tests once not sandboxed.
- Add Rust unit tests for FTS queries, URL parsing, hotkey parsing, model status.
- Add Python integration test that launches Uvicorn and checks CORS/loopback.
- Refine screenshot capture fallback on Linux (scrot path currently writes a file, not clipboard).
- Consider true WASAPI loopback rather than device-name selection.
- Add request cancellation for model downloads.
- Add `DELETE`/`POST` model cancellation command if desired.

---

## 6. Known inconsistencies / traps

1. **`package.json` description may display mojibake** (`â€”`). Check the file bytes before trusting it; not functionally relevant.
2. **Missing scripts referenced by package.json**:
   - `services:install` → `python scripts/setup_services.py` (missing)
   - `smoke` → `python scripts/smoke_test.py` (missing)
   - `web:*` → no web source
3. **`.venv` is broken in this workspace.** It was created by a sandbox process; pip-installed dirs are ACL-restricted and cannot be imported. Use `.pydeps` for the current tests. Outside the sandbox, create a fresh `.venv`.
4. **`.pydeps` is a working dependency target**, used with `PYTHONPATH` and the bundled/base Python. It includes FastAPI, PyMuPDF, RapidOCR, pytest, qrcode, etc.
5. **Vite hack**: `vite.config.js` uses `esbuild: false`, an in-process TypeScript plugin, and `--configLoader native` in npm scripts. A local patch in `node_modules/vite/dist/node/chunks/dep-CkSfqg8D.js` removes the `net use` child process. Reinstalling `node_modules` loses that patch. On a normal machine, stock Vite is fine; the current scripts should still work.
6. **Svelte warnings are expected** and cause `npm run check` to exit 1 despite 0 errors.
7. **Build command exit code weirdness**: `npm run build` printed success but the shell reported exit 1 in this sandbox (likely npm update notice/stderr). The artifacts exist in `dist/`.
8. **Extension `.js` files are generated and checked in.** Edit `src/*.ts`, then run `npm run extension:build`; do not hand-edit generated JS.
9. **`manifest.json` permissions are minimal by design.** No `<all_urls>` host permission, so `captureVisibleTab` from the in-page floating button may be denied. Use context menus/popup.
10. **Tauri capabilities use `shell:allow-open`, not `core:shell:allow-open`.** The latter came from the brief but is not valid Tauri 2.
11. **Model event shape** is camelCase because of serde `rename_all = "camelCase"`; update Svelte if changed.
12. **`model:"paddle"`** is returned by OCR health even though default engine is RapidOCR.
13. **Service tests use mock ASR** but real RapidOCR; they do not test FunASR or Ollama.
14. **No integration test** covers Rust ↔ Python or extension ↔ service.

---

## 7. Commands for a developer outside the sandbox

### 7.1 Python venv

```powershell
cd F:\deepseek\project
py -3.12 -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install -r services\asr\requirements.txt
python -m pip install -r services\ocr\requirements.txt
python services\asr\download_models.py
python services\ocr\download_models.py
ollama pull qwen2.5-coder:7b
```

### 7.2 Start the services manually

Terminal 1:

```powershell
cd F:\deepseek\project
.\.venv\Scripts\Activate.ps1
python services\asr\main.py
```

Terminal 2:

```powershell
cd F:\deepseek\project
.\.venv\Scripts\Activate.ps1
python services\ocr\main.py
```

### 7.3 Start Ollama

Terminal 3 (if it is not already running as a service):

```powershell
ollama serve
```

### 7.4 Start the Tauri app

Terminal 4:

```powershell
cd F:\deepseek\project
npm install
npm run tauri dev
```

If services are already running, Tauri adopts them as external. Otherwise it starts them from `.venv`.

### 7.5 Build the extension and load it

```powershell
cd F:\deepseek\project
npm run extension:build
npm run extension:check
```

Then Chrome/Edge → `chrome://extensions` → Developer mode → **Load unpacked** → `F:\deepseek\project\extension`.

### 7.6 Build the landing page

```powershell
cd F:\deepseek\project
npm run build:web
```

Static output: `F:\deepseek\project\web\dist`. Preview with `npm run web:preview` or deploy that folder.

### 7.7 Verify services

```powershell
curl.exe http://127.0.0.1:8001/health
curl.exe http://127.0.0.1:8002/health
curl.exe http://127.0.0.1:11434/api/tags
```

### 7.8 Frontend-only check

```powershell
cd F:\deepseek\project
npm run check
npm run build
npm run dev
```

Open `http://127.0.0.1:1420/?demo=1&tab=dictation`.

---

## 8. Environment notes for the next agent

- Working directory: `F:\deepseek\project`.
- Node: `C:\Program Files\nodejs\node.exe` v20.19.5; npm 10.8.2.
- Bundled Python used for tests:
  `C:\Users\Prajakta Patil\.dsh\dsh-runtimes\dsh-primary-runtime\dependencies\python\python.exe`
- Working test deps: `F:\deepseek\project\.pydeps`.
- Broken venv: `F:\deepseek\project\.venv` (ACL restricted; do not rely on it).
- Rust install: `.rustup` / `.cargo`, GNU target.
- MSYS2 UCRT64: `F:\msys64\ucrt64\bin` (gcc, windres, ar, pkg-config).
- Cargo target dir: `.cargo-target` (large; not yet gitignored).
- `node_modules` was installed with `--ignore-scripts` in the sandbox; a normal `npm install` outside is recommended.
- Build/screenshot attempts using Chrome/Edge failed because the sandbox blocks child process/named-pipe creation. Do not interpret a missing screenshot as a UI failure.
- User instruction at cutoff: **“Do not write any more application code.”** Respect it until asked otherwise.
- Last user instruction before that: **“Do not run cargo check or try to compile again.”**

---

## 9. Suggested first prompt for the next agent

> Read `README.md`, `ARCHITECTURE.md`, and `AGENT.md`. The Svelte UI and MV3 extension build. The Python services pass 12 tests. The Rust/Tauri backend has never compiled because of a Windows sandbox linker/temp-file failure. Work outside the sandbox with an MSVC Rust toolchain, run `cargo check`, fix compile errors only (no feature redesign), then launch `npm run tauri dev` and verify the service supervisor, model modal, dictation hotkey, and screenshot OCR. Do not rebuild the extension or rewrite the UI unless a concrete failure requires it.

---

## 10. Final state

The project is a **complete, reviewable implementation** of the desktop app, local microservices, model setup, browser extension, and static landing page, with one hard external verification gap: the Rust crate has not been compiled in this environment. Treat `src-tauri/` as high-quality but unverified source until that gap is closed. `npm run build:web` produces the deployable landing page in `web/dist/`.
