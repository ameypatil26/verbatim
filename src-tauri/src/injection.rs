use std::process::Command;

/// Type text into whatever application currently has focus.
///
/// enigo is the primary implementation on every platform. If it fails to
/// initialize or inject, we fall back to the native automation utility that is
/// commonly installed on Linux/macOS. Nothing is ever sent over the network.
pub fn type_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }
    match enigo_type(text) {
        Ok(()) => Ok(()),
        Err(primary) => {
            tracing::warn!(error = %primary, "enigo injection failed; trying platform fallback");
            platform_fallback(text).map_err(|fallback| {
                format!("Could not type into the focused app. enigo: {primary}. fallback: {fallback}")
            })
        }
    }
}

fn enigo_type(text: &str) -> Result<(), String> {
    use enigo::{Enigo, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default()).map_err(|error| format!("init enigo: {error}"))?;
    enigo.text(text).map_err(|error| format!("enigo text: {error}"))
}

pub fn undo_last_injection() -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default()).map_err(|error| format!("init enigo: {error}"))?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|error| format!("enigo ctrl press: {error}"))?;
    let result = enigo.key(Key::Unicode('z'), Direction::Click);
    let _ = enigo.key(Key::Control, Direction::Release);
    result.map_err(|error| format!("enigo undo: {error}"))
}

#[cfg(target_os = "linux")]
fn platform_fallback(text: &str) -> Result<(), String> {
    let status = Command::new("xdotool")
        .args(["type", "--clearmodifiers", "--delay", "8", text])
        .status()
        .map_err(|error| format!("xdotool not available: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("xdotool exited with {status}"))
    }
}

#[cfg(target_os = "macos")]
fn platform_fallback(text: &str) -> Result<(), String> {
    let status = Command::new("cliclick")
        .arg(format!("t:{text}"))
        .status()
        .map_err(|error| format!("cliclick not available: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cliclick exited with {status}"))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn platform_fallback(_text: &str) -> Result<(), String> {
    Err("No native keyboard injection fallback is available on this platform.".into())
}
