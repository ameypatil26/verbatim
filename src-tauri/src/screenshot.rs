use std::io::Cursor;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use base64::Engine;
use tauri::{AppHandle, Emitter, Manager};

use crate::models::{CaptureResult, OcrResult};
use crate::AppState;

/// Open the native region-selection UI, read the resulting image from the
/// clipboard, OCR it, copy the text, and return the result to the UI.
pub async fn capture_and_ocr(app: AppHandle) -> Result<CaptureResult, String> {
    let png = tokio::task::spawn_blocking(capture_region_png)
        .await
        .map_err(|error| format!("screenshot task: {error}"))??;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&png);
    let data_url = format!("data:image/png;base64,{encoded}");

    let state = app.state::<AppState>();
    let settings = state.db.get_settings();
    let part = reqwest::multipart::Part::bytes(png)
        .file_name("region.png")
        .mime_str("image/png")
        .map_err(|error| format!("image part: {error}"))?;
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("masks", "[]");
    let response = state
        .http
        .post(format!("{}/ocr", settings.ocr_url.trim_end_matches('/')))
        .multipart(form)
        .send()
        .await
        .map_err(|error| format!("OCR service unavailable: {error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("OCR failed ({status}): {body}"));
    }
    let ocr: OcrResult = response
        .json()
        .await
        .map_err(|error| format!("OCR response: {error}"))?;

    let text = ocr.text.clone();
    let text_for_clipboard = text.clone();
    let _ = tokio::task::spawn_blocking(move || {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text_for_clipboard);
        }
    })
    .await;

    let result = CaptureResult {
        ocr,
        image_data_url: data_url,
    };
    let _ = app.emit("ocr:result", &result);
    tracing::info!(characters = text.chars().count(), "screenshot OCR complete");
    Ok(result)
}

fn capture_region_png() -> Result<Vec<u8>, String> {
    let before = read_clipboard_image_hash();
    let mut child = launch_region_selector()?;
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if Instant::now() > deadline {
            stop_child(&mut child);
            return Err("Region capture timed out. Select an area with the screenshot tool and try again.".into());
        }
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(image) = clipboard.get_image() {
                let hash = image_hash(image.width, image.height, &image.bytes);
                if before.as_ref() != Some(&hash) {
                    stop_child(&mut child);
                    let width = u32::try_from(image.width).map_err(|_| "invalid screenshot width")?;
                    let height = u32::try_from(image.height).map_err(|_| "invalid screenshot height")?;
                    let rgba = image::RgbaImage::from_raw(width, height, image.bytes.into_owned())
                        .ok_or_else(|| "Screenshot pixel buffer was invalid.".to_string())?;
                    let mut png = Cursor::new(Vec::new());
                    image::DynamicImage::ImageRgba8(rgba)
                        .write_to(&mut png, image::ImageFormat::Png)
                        .map_err(|error| format!("encode screenshot: {error}"))?;
                    return Ok(png.into_inner());
                }
            }
        }
        thread::sleep(Duration::from_millis(180));
    }
}

fn image_hash(width: usize, height: usize, bytes: &[u8]) -> u64 {
    let mut hash = 1469598103934665603u64;
    for value in bytes.iter().step_by(97) {
        hash ^= u64::from(*value);
        hash = hash.wrapping_mul(1099511628211);
    }
    hash ^= width as u64;
    hash ^= (height as u64) << 32;
    hash
}

fn read_clipboard_image_hash() -> Option<u64> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    let image = clipboard.get_image().ok()?;
    Some(image_hash(image.width, image.height, &image.bytes))
}

fn launch_region_selector() -> Result<Child, String> {
    #[cfg(target_os = "windows")]
    {
        return Command::new("cmd")
            .args(["/C", "start", "", "ms-screenclip:"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Could not open Windows Snipping Tool: {error}"));
    }
    #[cfg(target_os = "macos")]
    {
        return Command::new("screencapture")
            .args(["-i", "-c"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Could not run screencapture: {error}"));
    }
    #[cfg(target_os = "linux")]
    {
        let candidates: Vec<(&str, Vec<&str>)> = vec![
            ("gnome-screenshot", vec!["-a", "-c"]),
            ("spectacle", vec!["-r", "-b", "-n", "-c"]),
            ("scrot", vec!["-s", "-o", "/tmp/verbatim-region.png"]),
        ];
        for (program, args) in candidates {
            if which::which(program).is_ok() {
                return Command::new(program)
                    .args(args)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|error| format!("Could not run {program}: {error}"));
            }
        }
        return Err("No region capture tool found. Install gnome-screenshot, spectacle, or scrot.".into());
    }
    #[allow(unreachable_code)]
    Err("Region capture is not supported on this platform.".into())
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
