use std::fs::{self, File};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::models::ServiceRuntimeStatus;

pub const ASR_PORT: u16 = 8001;
pub const OCR_PORT: u16 = 8002;

struct ManagedService {
    name: String,
    script: String,
    port: u16,
    child: Mutex<Option<Child>>,
    state: Mutex<String>,
    message: Mutex<Option<String>>,
    retries: AtomicU8,
}

impl ManagedService {
    fn new(name: &str, script: &str, port: u16) -> Self {
        Self {
            name: name.to_string(),
            script: script.to_string(),
            port,
            child: Mutex::new(None),
            state: Mutex::new("stopped".into()),
            message: Mutex::new(None),
            retries: AtomicU8::new(0),
        }
    }

    fn status(&self) -> ServiceRuntimeStatus {
        let pid = self
            .child
            .lock()
            .ok()
            .and_then(|mut child| child.as_mut().and_then(|process| Some(process.id())));
        ServiceRuntimeStatus {
            name: self.name.clone(),
            state: self.state.lock().map(|value| value.clone()).unwrap_or_else(|_| "failed".into()),
            pid,
            retries: self.retries.load(Ordering::Relaxed),
            message: self.message.lock().ok().and_then(|value| value.clone()),
        }
    }
}

pub struct ServiceManager {
    app: AppHandle,
    services_dir: PathBuf,
    log_dir: PathBuf,
    entries: Vec<Arc<ManagedService>>,
    shutdown: AtomicBool,
    monitor_started: AtomicBool,
}

impl ServiceManager {
    pub fn new(app: AppHandle) -> Result<Arc<Self>, String> {
        let services_dir = locate_services_dir(&app)
            .ok_or_else(|| "Could not locate the services/ directory. Reinstall Verbatim.".to_string())?;
        let log_dir = crate::logging::log_dir();
        let manager = Arc::new(Self {
            app,
            services_dir,
            log_dir,
            entries: vec![
                Arc::new(ManagedService::new("asr", "asr/main.py", ASR_PORT)),
                Arc::new(ManagedService::new("ocr", "ocr/main.py", OCR_PORT)),
            ],
            shutdown: AtomicBool::new(false),
            monitor_started: AtomicBool::new(false),
        });
        Ok(manager)
    }

    pub fn start(self: &Arc<Self>) {
        for entry in &self.entries {
            if port_open(entry.port) {
                self.set_state(entry, "external", Some(format!("Already running on 127.0.0.1:{}", entry.port)));
                continue;
            }
            if let Err(error) = self.spawn(entry) {
                tracing::error!(service = %entry.name, error = %error, "service spawn failed");
                self.set_state(entry, "failed", Some(error));
            }
        }
        if !self.monitor_started.swap(true, Ordering::SeqCst) {
            let manager = Arc::clone(self);
            thread::spawn(move || manager.monitor());
        }
        self.emit_status();
    }

    fn monitor(self: Arc<Self>) {
        loop {
            if self.shutdown.load(Ordering::SeqCst) {
                break;
            }
            for entry in &self.entries {
                let exited = {
                    let Ok(mut guard) = entry.child.lock() else {
                        continue;
                    };
                    match guard.as_mut() {
                        Some(child) => match child.try_wait() {
                            Ok(Some(status)) => {
                                *guard = None;
                                Some(Ok(status.code()))
                            }
                            Ok(None) => None,
                            Err(error) => {
                                *guard = None;
                                Some(Err(error.to_string()))
                            }
                        },
                        None => None,
                    }
                };
                let Some(result) = exited else {
                    continue;
                };
                if self.shutdown.load(Ordering::SeqCst) {
                    break;
                }
                let detail = match result {
                    Ok(code) => format!("Service exited with code {:?}", code),
                    Err(error) => format!("Service process error: {error}"),
                };
                tracing::warn!(service = %entry.name, detail = %detail, "local service stopped");
                if entry.retries.load(Ordering::SeqCst) >= 3 {
                    self.set_state(entry, "failed", Some(format!("{detail}. Check logs at {}", self.log_dir.display())));
                    continue;
                }
                entry.retries.fetch_add(1, Ordering::SeqCst);
                self.set_state(entry, "starting", Some(format!("{detail}; restarting")));
                thread::sleep(Duration::from_secs(1));
                if self.shutdown.load(Ordering::SeqCst) {
                    break;
                }
                if let Err(error) = self.spawn(entry) {
                    self.set_state(entry, "failed", Some(format!("Restart failed: {error}")));
                }
            }
            self.emit_status();
            thread::sleep(Duration::from_secs(2));
        }
    }

    fn spawn(self: &Arc<Self>, entry: &Arc<ManagedService>) -> Result<(), String> {
        let script = self.services_dir.join(&entry.script);
        if !script.exists() {
            return Err(format!("Missing Python service: {}", script.display()));
        }
        if port_open(entry.port) {
            self.set_state(entry, "external", Some(format!("Running on 127.0.0.1:{}", entry.port)));
            return Ok(());
        }

        let python = python_runtime(&self.services_dir);
        fs::create_dir_all(&self.log_dir).map_err(|error| format!("create log dir: {error}"))?;
        let log_path = self.log_dir.join(format!("{}-service.log", entry.name));
        let stdout = File::options()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|error| format!("open {}: {error}", log_path.display()))?;
        let stderr = stdout.try_clone().map_err(|error| format!("clone service log: {error}"))?;

        let mut command = Command::new(&python.executable);
        command
            .arg(&script)
            .current_dir(script.parent().unwrap_or(&self.services_dir))
            .env("VERBATIM_DATA_DIR", crate::logging::data_dir())
            .env("VERBATIM_LOG_DIR", &self.log_dir)
            .env("PYTHONUNBUFFERED", "1")
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        if let Some(path) = python.python_path {
            let existing = std::env::var_os("PYTHONPATH").unwrap_or_default();
            let mut paths = vec![path];
            if !existing.is_empty() {
                paths.extend(std::env::split_paths(&existing));
            }
            if let Ok(joined) = std::env::join_paths(paths) {
                command.env("PYTHONPATH", joined);
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let child = command
            .spawn()
            .map_err(|error| format!("start {} with {}: {error}", entry.name, python.executable))?;
        tracing::info!(service = %entry.name, pid = child.id(), "started local service");
        *entry.child.lock().map_err(|_| "service lock poisoned".to_string())? = Some(child);
        self.set_state(entry, "starting", Some(format!("Waiting for 127.0.0.1:{}", entry.port)));
        self.emit_status();

        // Give the process a moment, then report whether the port came up. The
        // monitor thread keeps watching after this initial health check.
        let manager = Arc::clone(self);
        let entry_clone = Arc::clone(entry);
        let port = entry.port;
        let delay = Duration::from_millis(150);
        thread::spawn(move || {
            for attempt in 0..40 {
                thread::sleep(delay);
                if port_open(port) {
                    manager.set_state(&entry_clone, "running", Some(format!("Listening on 127.0.0.1:{port}")));
                    manager.emit_status();
                    return;
                }
                if attempt == 39 {
                    // Do not mark failed here; the process may still be
                    // importing torch/FunASR. The monitor will notice if it dies.
                    manager.set_state(
                        &entry_clone,
                        "starting",
                        Some("Service is still importing its model stack".into()),
                    );
                    manager.emit_status();
                }
            }
        });
        Ok(())
    }

    fn set_state(&self, entry: &Arc<ManagedService>, state: &str, message: Option<String>) {
        if let Ok(mut current) = entry.state.lock() {
            *current = state.to_string();
        }
        if let Ok(mut current) = entry.message.lock() {
            *current = message;
        }
    }

    fn emit_status(&self) {
        let statuses: Vec<ServiceRuntimeStatus> = self.entries.iter().map(|entry| entry.status()).collect();
        let _ = self.app.emit("service:status", statuses);
    }

    pub fn statuses(&self) -> Vec<ServiceRuntimeStatus> {
        self.entries.iter().map(|entry| entry.status()).collect()
    }

    pub fn restart(self: &Arc<Self>, name: &str) -> Result<(), String> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .ok_or_else(|| format!("Unknown service: {name}"))?;
        if let Ok(mut guard) = entry.child.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        entry.retries.store(0, Ordering::SeqCst);
        self.set_state(entry, "starting", Some("Restart requested".into()));
        self.emit_status();
        self.spawn(entry)?;
        Ok(())
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        for entry in &self.entries {
            if let Ok(mut guard) = entry.child.lock() {
                if let Some(mut child) = guard.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
        tracing::info!("local services stopped");
    }
}

struct PythonRuntime {
    executable: String,
    python_path: Option<PathBuf>,
}

fn locate_services_dir(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(value) = std::env::var("VERBATIM_SERVICES_DIR") {
        let path = PathBuf::from(value);
        if path.join("asr/main.py").exists() {
            return Some(path);
        }
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent()?.join("services");
    if dev.join("asr/main.py").exists() {
        return Some(dev);
    }
    app.path()
        .resolve("services", tauri::path::BaseDirectory::Resource)
        .ok()
        .filter(|path| path.join("asr/main.py").exists())
}

fn python_runtime(services_dir: &Path) -> PythonRuntime {
    if let Ok(explicit) = std::env::var("VERBATIM_PYTHON") {
        return PythonRuntime {
            executable: explicit,
            python_path: None,
        };
    }
    let project = services_dir.parent().unwrap_or(services_dir);
    let venv = if cfg!(windows) {
        project.join(".venv/Scripts/python.exe")
    } else {
        project.join(".venv/bin/python")
    };
    if venv.exists() {
        return PythonRuntime {
            executable: venv.display().to_string(),
            python_path: None,
        };
    }
    if project.join(".pydeps").exists() {
        let executable = which::which("python")
            .or_else(|_| which::which("python3"))
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "python".into());
        return PythonRuntime {
            executable,
            python_path: Some(project.join(".pydeps")),
        };
    }
    let executable = which::which("python3")
        .or_else(|_| which::which("python"))
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "python".into());
    PythonRuntime {
        executable,
        python_path: None,
    }
}

pub fn port_open(port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&address, Duration::from_millis(350)).is_ok()
}
