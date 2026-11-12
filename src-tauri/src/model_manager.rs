use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::models::{
    ModelDownloadProgress, ModelStatus, ModelsReport, SetupModel, SetupStatus,
};
use crate::AppState;

pub const ASR_MODEL: &str = "sensevoice";
pub const OCR_MODEL: &str = "rapidocr";
pub const LLM_MODEL: &str = "qwen2.5-coder:7b";

const ASR_FILES: &[&str] = &[
    "model.pt",
    "config.yaml",
    "tokens.json",
    "am.mvn",
    "configuration.json",
];

const ASR_BASES: &[&str] = &[
    "https://huggingface.co/iic/SenseVoiceSmall/resolve/main",
    "https://huggingface.co/FunAudioLLM/SenseVoiceSmall/resolve/main",
];

const LLM_ESTIMATE_MB: f64 = 4700.0;
const OCR_ESTIMATE_MB: f64 = 15.0;

pub fn models_dir() -> PathBuf {
    crate::logging::data_dir().join("models")
}

pub fn asr_dir() -> PathBuf {
    models_dir().join("asr")
}

pub fn ocr_dir() -> PathBuf {
    models_dir().join("ocr")
}

fn emit_progress(app: &AppHandle, model: &str, downloaded_mb: f64, total_mb: f64, status: &str) {
    let payload = ModelDownloadProgress {
        model: model.to_string(),
        downloaded_mb,
        total_mb,
        status: status.to_string(),
    };
    let _ = app.emit("model-download-progress", payload);
}

pub fn check(app: &AppHandle) -> ModelsReport {
    let asr_ready = ASR_FILES
        .iter()
        .all(|file| asr_dir().join(file).exists())
        && fs::metadata(asr_dir().join("model.pt"))
            .map(|meta| meta.len() > 1_000_000)
            .unwrap_or(false);
    let ocr_ready = ocr_dir().join(".installed").exists();
    let (ollama_installed, llm_ready) = ollama_status();

    let models = vec![
        ModelStatus {
            model: ASR_MODEL.into(),
            name: "SenseVoice ASR".into(),
            ready: asr_ready,
            size_bytes: dir_size(&asr_dir()),
            detail: if asr_ready {
                "Installed — ~228 MB speech model".into()
            } else {
                "Speech recognition · ~228 MB".into()
            },
        },
        ModelStatus {
            model: OCR_MODEL.into(),
            name: "RapidOCR ONNX".into(),
            ready: ocr_ready,
            size_bytes: dir_size(&ocr_dir()),
            detail: if ocr_ready {
                "Installed — detection, recognition, classification".into()
            } else {
                "OCR models · ~15 MB".into()
            },
        },
        ModelStatus {
            model: LLM_MODEL.into(),
            name: "Qwen 2.5 Coder 7B".into(),
            ready: llm_ready,
            size_bytes: 0,
            detail: if llm_ready {
                "Installed in Ollama".into()
            } else if ollama_installed {
                "Not pulled yet · ~4.7 GB".into()
            } else {
                "Ollama not found. Install Ollama, then pull this model.".into()
            },
        },
    ];
    let ready = models.iter().all(|model| model.ready);
    let message = if !ollama_installed {
        Some("Ollama was not found on PATH. Install it from ollama.com, then reopen Verbatim.".into())
    } else if !ready {
        Some("Required models are missing. Download them to enable local processing.".into())
    } else {
        None
    };
    ModelsReport {
        models,
        ready,
        ollama_installed,
        data_dir: crate::logging::data_dir().display().to_string(),
        message,
    }
}

pub fn setup_status(app: &AppHandle) -> SetupStatus {
    let report = check(app);
    let model_list = report
        .models
        .into_iter()
        .map(|model| {
            let key = match model.model.as_str() {
                ASR_MODEL => "asr",
                OCR_MODEL => "ocr",
                _ => "llm",
            };
            SetupModel {
                key: key.into(),
                label: model.name,
                detail: model.detail,
                installed: model.ready,
                loading: false,
                size_bytes: model.size_bytes,
                version: None,
                error: if model.model == LLM_MODEL && !report.ollama_installed {
                    Some("Ollama binary not found".into())
                } else {
                    None
                },
            }
        })
        .collect();
    SetupStatus {
        ready: report.ready,
        data_dir: report.data_dir,
        models: model_list,
        ollama_reachable: report.ollama_installed,
        message: report.message,
    }
}

fn ollama_status() -> (bool, bool) {
    let Some(program) = which::which("ollama").ok() else {
        return (false, false);
    };
    let mut command = std::process::Command::new(program);
    command.args(["list"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let output = match command.output() {
        Ok(output) => output,
        Err(_) => return (true, false),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let ready = stdout.lines().any(|line| {
        let name = line.split_whitespace().next().unwrap_or_default();
        name == LLM_MODEL || name == format!("{LLM_MODEL}:latest")
    });
    (true, ready)
}

pub async fn download(app: AppHandle, model: &str) -> Result<(), String> {
    match model {
        ASR_MODEL => download_sensevoice(&app).await,
        OCR_MODEL => download_rapidocr(&app).await,
        LLM_MODEL => download_ollama(&app).await,
        _ => Err(format!("Unknown model: {model}")),
    }
}

async fn download_sensevoice(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let client = state.http.clone();
    let dir = asr_dir();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|error| format!("Create ASR model dir: {error}"))?;
    emit_progress(app, ASR_MODEL, 0.0, 0.0, "downloading");

    // Resolve every file and its real Content-Length before starting, so the
    // progress bar is based on the server's actual size rather than a guess.
    let mut resolved: Vec<(String, u64, PathBuf)> = Vec::new();
    for file in ASR_FILES {
        let mut found: Option<(String, u64)> = None;
        for base in ASR_BASES {
            let url = format!("{base}/{file}");
            match client.head(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    let size = response.content_length().unwrap_or(0);
                    found = Some((url, size));
                    break;
                }
                _ => continue,
            }
        }
        let Some((url, size)) = found else {
            let error = format!(
                "Could not find {file} on Hugging Face. Check your connection, then retry the download."
            );
            emit_progress(app, ASR_MODEL, 0.0, 0.0, "error");
            return Err(error);
        };
        resolved.push((url, size, dir.join(file)));
    }
    let total_bytes: u64 = resolved.iter().map(|(_, size, _)| *size).sum();
    let total_mb = bytes_to_mb(total_bytes);
    let mut completed_bytes = 0u64;
    let mut last_emit = Instant::now();

    for (url, expected_size, destination) in resolved {
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|error| format!("Download {url}: {error}"))?;
        if !response.status().is_success() {
            emit_progress(app, ASR_MODEL, bytes_to_mb(completed_bytes), total_mb, "error");
            return Err(format!("Download {url}: HTTP {}", response.status()));
        }
        let response_size = response.content_length().unwrap_or(expected_size);
        let part_path = destination.with_extension(format!(
            "{}.part",
            destination.extension().and_then(|value| value.to_str()).unwrap_or("bin")
        ));
        {
            use tokio::io::AsyncWriteExt;
            let mut file = tokio::fs::File::create(&part_path)
                .await
                .map_err(|error| format!("Create {}: {error}", part_path.display()))?;
            let mut stream = response.bytes_stream();
            let mut file_bytes = 0u64;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|error| format!("Read model download: {error}"))?;
                file.write_all(&chunk)
                    .await
                    .map_err(|error| format!("Write model file: {error}"))?;
                file_bytes += chunk.len() as u64;
                if last_emit.elapsed() >= Duration::from_millis(220) {
                    let downloaded = bytes_to_mb(completed_bytes + file_bytes);
                    emit_progress(app, ASR_MODEL, downloaded, total_mb, "downloading");
                    last_emit = Instant::now();
                }
            }
            file.flush()
                .await
                .map_err(|error| format!("Flush model file: {error}"))?;
            if response_size > 0 && file_bytes != response_size {
                let _ = tokio::fs::remove_file(&part_path).await;
                emit_progress(app, ASR_MODEL, bytes_to_mb(completed_bytes), total_mb, "error");
                return Err(format!(
                    "Incomplete download for {}: received {file_bytes} of {response_size} bytes",
                    destination.display()
                ));
            }
            tokio::fs::rename(&part_path, &destination)
                .await
                .map_err(|error| format!("Finalize model file: {error}"))?;
            completed_bytes += file_bytes;
            emit_progress(app, ASR_MODEL, bytes_to_mb(completed_bytes), total_mb, "downloading");
        }
    }
    fs::write(dir.join(".installed"), "sensevoice\n").map_err(|error| format!("Write model marker: {error}"))?;
    emit_progress(app, ASR_MODEL, bytes_to_mb(completed_bytes), total_mb, "done");
    tracing::info!(bytes = completed_bytes, "SenseVoice downloaded");
    Ok(())
}

async fn download_rapidocr(app: &AppHandle) -> Result<(), String> {
    emit_progress(app, OCR_MODEL, 0.0, OCR_ESTIMATE_MB, "downloading");
    let project = project_root();
    let script = project.join("services/ocr/download_models.py");
    if !script.exists() {
        emit_progress(app, OCR_MODEL, 0.0, OCR_ESTIMATE_MB, "error");
        return Err("services/ocr/download_models.py is missing. Reinstall Verbatim.".into());
    }
    let (python, python_path) = script_python(&project);
    let python_for_task = python.clone();
    let script_for_task = script.clone();
    let python_path_for_task = python_path.clone();
    let app_for_task = app.clone();
    let result = tokio::task::spawn_blocking(move || {
        use std::process::Command;
        let mut command = Command::new(&python_for_task);
        command
            .arg(&script_for_task)
            .current_dir(project)
            .env("PYTHONUNBUFFERED", "1");
        if let Some(path) = python_path_for_task {
            let mut paths = vec![path];
            if let Some(existing) = std::env::var_os("PYTHONPATH") {
                paths.extend(std::env::split_paths(&existing));
            }
            if let Ok(joined) = std::env::join_paths(paths) {
                command.env("PYTHONPATH", joined);
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let output = command
            .output()
            .map_err(|error| format!("Run OCR model preparation: {error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    })
    .await
    .map_err(|error| format!("OCR model task: {error}"))?;
    if let Err(error) = result {
        emit_progress(&app_for_task, OCR_MODEL, 0.0, OCR_ESTIMATE_MB, "error");
        return Err(error);
    }
    let size = dir_size(&ocr_dir());
    let total = if size > 0 {
        bytes_to_mb(size).max(OCR_ESTIMATE_MB)
    } else {
        OCR_ESTIMATE_MB
    };
    emit_progress(app, OCR_MODEL, total, total, "done");
    tracing::info!("RapidOCR models prepared");
    Ok(())
}

async fn download_ollama(app: &AppHandle) -> Result<(), String> {
    let Some(program) = which::which("ollama").ok() else {
        emit_progress(app, LLM_MODEL, 0.0, LLM_ESTIMATE_MB, "error");
        return Err("Ollama is not installed. Install it from https://ollama.com, then try again.".into());
    };
    emit_progress(app, LLM_MODEL, 0.0, LLM_ESTIMATE_MB, "downloading");
    let mut command = tokio::process::Command::new(program);
    command
        .args(["pull", LLM_MODEL])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("Start ollama pull: {error}"))?;
    let stdout = child.stdout.take().ok_or_else(|| "ollama stdout unavailable".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "ollama stderr unavailable".to_string())?;
    let mut stdout_lines = BufReader::new(stdout).lines();
    let mut stderr_lines = BufReader::new(stderr).lines();
    let mut last_emit = Instant::now();

    loop {
        tokio::select! {
            line = stdout_lines.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if let Some(percent) = parse_percent(&line) {
                            if last_emit.elapsed() >= Duration::from_millis(220) {
                                let downloaded = LLM_ESTIMATE_MB * percent / 100.0;
                                emit_progress(app, LLM_MODEL, downloaded, LLM_ESTIMATE_MB, "downloading");
                                last_emit = Instant::now();
                            }
                        }
                    }
                    _ => break,
                }
            }
            line = stderr_lines.next_line() => {
                if let Ok(Some(line)) = line {
                    if let Some(percent) = parse_percent(&line) {
                        let downloaded = LLM_ESTIMATE_MB * percent / 100.0;
                        emit_progress(app, LLM_MODEL, downloaded, LLM_ESTIMATE_MB, "downloading");
                    }
                }
            }
            status = child.wait() => {
                match status {
                    Ok(status) if status.success() => {
                        emit_progress(app, LLM_MODEL, LLM_ESTIMATE_MB, LLM_ESTIMATE_MB, "done");
                        return Ok(());
                    }
                    Ok(status) => {
                        emit_progress(app, LLM_MODEL, 0.0, LLM_ESTIMATE_MB, "error");
                        return Err(format!("ollama pull exited with {status}"));
                    }
                    Err(error) => {
                        emit_progress(app, LLM_MODEL, 0.0, LLM_ESTIMATE_MB, "error");
                        return Err(format!("ollama pull failed: {error}"));
                    }
                }
            }
        }
    }
    let status = child.wait().await.map_err(|error| format!("ollama wait: {error}"))?;
    if status.success() {
        emit_progress(app, LLM_MODEL, LLM_ESTIMATE_MB, LLM_ESTIMATE_MB, "done");
        Ok(())
    } else {
        emit_progress(app, LLM_MODEL, 0.0, LLM_ESTIMATE_MB, "error");
        Err(format!("ollama pull exited with {status}"))
    }
}

pub fn delete(app: &AppHandle, model: &str) -> Result<(), String> {
    match model {
        ASR_MODEL => remove_dir_if_exists(&asr_dir()),
        OCR_MODEL => remove_dir_if_exists(&ocr_dir()),
        LLM_MODEL => {
            let program = which::which("ollama").map_err(|_| "Ollama is not installed.".to_string())?;
            let mut command = std::process::Command::new(program);
            command.args(["rm", LLM_MODEL]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x0800_0000);
            }
            let output = command.output().map_err(|error| format!("ollama rm: {error}"))?;
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
        _ => Err(format!("Unknown model: {model}")),
    }
}

fn remove_dir_if_exists(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|error| format!("Remove {}: {error}", path.display()))?;
    }
    Ok(())
}

fn parse_percent(line: &str) -> Option<f64> {
    let index = line.find('%')?;
    let start = line[..index]
        .rfind(|character: char| !character.is_ascii_digit() && character != '.')
        .map(|position| position + 1)
        .unwrap_or(0);
    line[start..index].trim().parse::<f64>().ok().map(|value| value.clamp(0.0, 100.0))
}

fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    let mut total = 0u64;
    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            total += dir_size(&entry.path());
        } else {
            total += metadata.len();
        }
    }
    total
}

fn bytes_to_mb(value: u64) -> f64 {
    value as f64 / (1024.0 * 1024.0)
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn script_python(project: &Path) -> (String, Option<PathBuf>) {
    if let Ok(explicit) = std::env::var("VERBATIM_PYTHON") {
        return (explicit, None);
    }
    let venv = if cfg!(windows) {
        project.join(".venv/Scripts/python.exe")
    } else {
        project.join(".venv/bin/python")
    };
    if venv.exists() {
        return (venv.display().to_string(), None);
    }
    let pydeps = project.join(".pydeps");
    let executable = which::which("python")
        .or_else(|_| which::which("python3"))
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "python".into());
    (executable, if pydeps.exists() { Some(pydeps) } else { None })
}
