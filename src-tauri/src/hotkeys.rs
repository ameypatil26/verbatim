use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::thread;
use std::time::Duration;

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tauri::{AppHandle, Emitter, Manager};

use crate::dictation;
use crate::screenshot;
use crate::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Dictation,
    Ocr,
}

enum HotkeyCommand {
    Rebind {
        dictation: String,
        ocr: String,
        reply: SyncSender<Result<(), String>>,
    },
    Stop,
}

pub struct HotkeyService {
    tx: Sender<HotkeyCommand>,
}

impl HotkeyService {
    pub fn start(app: AppHandle, dictation_hotkey: &str, ocr_hotkey: &str) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let dictation = dictation_hotkey.to_string();
        let ocr = ocr_hotkey.to_string();
        thread::Builder::new()
            .name("verbatim-hotkeys".into())
            .spawn(move || run(app, rx, dictation, ocr, ready_tx))
            .map_err(|error| format!("start hotkey thread: {error}"))?;
        match ready_rx.recv_timeout(Duration::from_secs(8)) {
            Ok(Ok(())) => Ok(Self { tx }),
            Ok(Err(error)) => Err(error),
            Err(error) => Err(format!("Global hotkeys did not initialize: {error}")),
        }
    }

    pub fn rebind(&self, dictation: String, ocr: String) -> Result<(), String> {
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        self.tx
            .send(HotkeyCommand::Rebind {
                dictation,
                ocr,
                reply: reply_tx,
            })
            .map_err(|_| "hotkey service is not running".to_string())?;
        reply_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|error| format!("hotkey rebind timed out: {error}"))?
    }
}

impl Drop for HotkeyService {
    fn drop(&mut self) {
        let _ = self.tx.send(HotkeyCommand::Stop);
    }
}

fn run(
    app: AppHandle,
    command_rx: Receiver<HotkeyCommand>,
    dictation_text: String,
    ocr_text: String,
    ready_tx: SyncSender<Result<(), String>>,
) {
    let manager = match GlobalHotKeyManager::new() {
        Ok(manager) => manager,
        Err(error) => {
            let _ = ready_tx.send(Err(format!("Could not initialize global hotkeys: {error}")));
            return;
        }
    };

    let mut action_by_id: HashMap<u32, Action> = HashMap::new();
    let mut current = match (parse_hotkey(&dictation_text), parse_hotkey(&ocr_text)) {
        (Ok(dictation), Ok(ocr)) => Binding::new(dictation, ocr),
        (Err(error), _) | (_, Err(error)) => {
            let _ = ready_tx.send(Err(error));
            return;
        }
    };
    if let Err(error) = register(&manager, &current, &mut action_by_id) {
        let _ = ready_tx.send(Err(error));
        return;
    }
    let _ = ready_tx.send(Ok(()));
    tracing::info!(
        dictation = %dictation_text,
        ocr = %ocr_text,
        "global hotkeys registered"
    );

    loop {
        while let Ok(command) = command_rx.try_recv() {
            match command {
                HotkeyCommand::Stop => {
                    let _ = unregister(&manager, &current);
                    return;
                }
                HotkeyCommand::Rebind {
                    dictation,
                    ocr,
                    reply,
                } => {
                    let result = rebind(&manager, &mut current, &mut action_by_id, &dictation, &ocr);
                    let _ = reply.send(result);
                }
            }
        }

        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            let Some(action) = action_by_id.get(&event.id()).copied() else {
                continue;
            };
            match (action, event.state()) {
                (Action::Dictation, HotKeyState::Pressed) => on_dictation_pressed(&app),
                (Action::Dictation, HotKeyState::Released) => on_dictation_released(&app),
                (Action::Ocr, HotKeyState::Pressed) => on_ocr_pressed(&app),
                (Action::Ocr, HotKeyState::Released) => {}
            }
        }
        thread::sleep(Duration::from_millis(18));
    }
}

struct Binding {
    dictation: HotKey,
    ocr: HotKey,
    dictation_id: u32,
    ocr_id: u32,
}

impl Binding {
    fn new(dictation: HotKey, ocr: HotKey) -> Self {
        Self {
            dictation_id: dictation.id(),
            ocr_id: ocr.id(),
            dictation,
            ocr,
        }
    }
}

fn register(
    manager: &GlobalHotKeyManager,
    binding: &Binding,
    action_by_id: &mut HashMap<u32, Action>,
) -> Result<(), String> {
    manager
        .register(binding.dictation)
        .map_err(|error| format!("register dictation hotkey: {error}"))?;
    if let Err(error) = manager.register(binding.ocr) {
        let _ = manager.unregister(binding.dictation);
        return Err(format!("register OCR hotkey: {error}"));
    }
    action_by_id.clear();
    action_by_id.insert(binding.dictation_id, Action::Dictation);
    action_by_id.insert(binding.ocr_id, Action::Ocr);
    Ok(())
}

fn unregister(manager: &GlobalHotKeyManager, binding: &Binding) -> Result<(), String> {
    let first = manager.unregister(binding.dictation);
    let second = manager.unregister(binding.ocr);
    first.and(second).map_err(|error| format!("unregister hotkeys: {error}"))
}

fn rebind(
    manager: &GlobalHotKeyManager,
    current: &mut Binding,
    action_by_id: &mut HashMap<u32, Action>,
    dictation_text: &str,
    ocr_text: &str,
) -> Result<(), String> {
    let dictation = parse_hotkey(dictation_text)?;
    let ocr = parse_hotkey(ocr_text)?;
    if dictation.id() == ocr.id() {
        return Err("Dictation and OCR hotkeys must be different.".into());
    }
    let next = Binding::new(dictation, ocr);
    let _ = unregister(manager, current);
    match register(manager, &next, action_by_id) {
        Ok(()) => {
            *current = next;
            tracing::info!(dictation = %dictation_text, ocr = %ocr_text, "hotkeys rebound");
            Ok(())
        }
        Err(error) => {
            let _ = register(manager, current, action_by_id);
            Err(format!("Could not bind those hotkeys: {error}"))
        }
    }
}

fn on_dictation_pressed(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = state.db.get_settings();
    let recording = state.audio.is_recording();
    if settings.dictation_mode == "push" {
        if !recording {
            if let Err(error) = dictation::start(app, &settings.audio_source) {
                dictation::emit_state(app, "error", None, Some(error), None, None);
            }
        }
    } else if recording {
        stop_dictation(app.clone());
    } else if let Err(error) = dictation::start(app, &settings.audio_source) {
        dictation::emit_state(app, "error", None, Some(error), None, None);
    }
}

fn on_dictation_released(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = state.db.get_settings();
    if settings.dictation_mode == "push" && state.audio.is_recording() {
        stop_dictation(app.clone());
    }
}

fn stop_dictation(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = dictation::stop_and_process(app.clone()).await {
            tracing::error!(error = %error, "hotkey dictation failed");
        }
    });
}

fn on_ocr_pressed(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = screenshot::capture_and_ocr(app.clone()).await {
            tracing::error!(error = %error, "hotkey screenshot OCR failed");
            let _ = app.emit("ocr:error", error);
        }
    });
}

fn parse_hotkey(value: &str) -> Result<HotKey, String> {
    let parts: Vec<&str> = value.split('+').map(str::trim).filter(|part| !part.is_empty()).collect();
    if parts.len() < 2 {
        return Err(format!("Invalid hotkey “{value}”. Use a modifier plus a key, e.g. Ctrl+Shift+D."));
    }
    let mut modifiers = Modifiers::empty();
    let mut code: Option<Code> = None;
    for (index, part) in parts.iter().enumerate() {
        let lower = part.to_ascii_lowercase();
        match lower.as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "meta" | "cmd" | "command" | "super" | "win" => modifiers |= Modifiers::SUPER,
            _ => {
                if index != parts.len() - 1 {
                    return Err(format!("Unknown modifier “{part}” in hotkey “{value}”."));
                }
                code = Some(parse_code(part)?);
            }
        }
    }
    let code = code.ok_or_else(|| format!("Hotkey “{value}” has no key."))?;
    if modifiers.is_empty() {
        return Err("Global hotkeys require at least one modifier (Ctrl, Alt, Shift, or Meta).".into());
    }
    Ok(HotKey::new(Some(modifiers), code))
}

fn parse_code(value: &str) -> Result<Code, String> {
    let upper = value.to_ascii_uppercase();
    if upper.len() == 1 {
        let character = upper.chars().next().unwrap_or('A');
        return match character {
            'A'..='Z' => Ok(match character {
                'A' => Code::KeyA,
                'B' => Code::KeyB,
                'C' => Code::KeyC,
                'D' => Code::KeyD,
                'E' => Code::KeyE,
                'F' => Code::KeyF,
                'G' => Code::KeyG,
                'H' => Code::KeyH,
                'I' => Code::KeyI,
                'J' => Code::KeyJ,
                'K' => Code::KeyK,
                'L' => Code::KeyL,
                'M' => Code::KeyM,
                'N' => Code::KeyN,
                'O' => Code::KeyO,
                'P' => Code::KeyP,
                'Q' => Code::KeyQ,
                'R' => Code::KeyR,
                'S' => Code::KeyS,
                'T' => Code::KeyT,
                'U' => Code::KeyU,
                'V' => Code::KeyV,
                'W' => Code::KeyW,
                'X' => Code::KeyX,
                'Y' => Code::KeyY,
                'Z' => Code::KeyZ,
                _ => unreachable!(),
            }),
            '0'..='9' => Ok(match character {
                '0' => Code::Digit0,
                '1' => Code::Digit1,
                '2' => Code::Digit2,
                '3' => Code::Digit3,
                '4' => Code::Digit4,
                '5' => Code::Digit5,
                '6' => Code::Digit6,
                '7' => Code::Digit7,
                '8' => Code::Digit8,
                '9' => Code::Digit9,
                _ => unreachable!(),
            }),
            _ => Err(format!("Unsupported hotkey key “{value}”.")),
        };
    }
    match upper.as_str() {
        "SPACE" => Ok(Code::Space),
        "ENTER" | "RETURN" => Ok(Code::Enter),
        "TAB" => Ok(Code::Tab),
        "ESC" | "ESCAPE" => Ok(Code::Escape),
        "BACKSPACE" => Ok(Code::Backspace),
        "DELETE" | "DEL" => Ok(Code::Delete),
        "HOME" => Ok(Code::Home),
        "END" => Ok(Code::End),
        "PAGEUP" => Ok(Code::PageUp),
        "PAGEDOWN" => Ok(Code::PageDown),
        "UP" => Ok(Code::ArrowUp),
        "DOWN" => Ok(Code::ArrowDown),
        "LEFT" => Ok(Code::ArrowLeft),
        "RIGHT" => Ok(Code::ArrowRight),
        other if other.starts_with('F') && other[1..].parse::<u8>().is_ok() => {
            match other[1..].parse::<u8>().unwrap_or(0) {
                1 => Ok(Code::F1),
                2 => Ok(Code::F2),
                3 => Ok(Code::F3),
                4 => Ok(Code::F4),
                5 => Ok(Code::F5),
                6 => Ok(Code::F6),
                7 => Ok(Code::F7),
                8 => Ok(Code::F8),
                9 => Ok(Code::F9),
                10 => Ok(Code::F10),
                11 => Ok(Code::F11),
                12 => Ok(Code::F12),
                _ => Err(format!("Unsupported function key “{value}”.")),
            }
        }
        _ => Err(format!("Unsupported hotkey key “{value}”.")),
    }
}
