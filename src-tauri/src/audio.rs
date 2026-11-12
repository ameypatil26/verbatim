use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use tauri::{AppHandle, Emitter};

pub struct RecordingSession {
    stop: Arc<AtomicBool>,
    join: JoinHandle<Result<PathBuf, String>>,
    source: String,
}

pub struct AudioEngine {
    session: Mutex<Option<RecordingSession>>,
    last_text: Mutex<String>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
            last_text: Mutex::new(String::new()),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.session.lock().map(|guard| guard.is_some()).unwrap_or(false)
    }

    pub fn last_text(&self) -> String {
        self.last_text.lock().map(|value| value.clone()).unwrap_or_default()
    }

    pub fn set_last_text(&self, value: &str) {
        if let Ok(mut current) = self.last_text.lock() {
            *current = value.to_string();
        }
    }

    pub fn start(&self, source: &str, app: AppHandle) -> Result<(), String> {
        let mut guard = self.session.lock().map_err(|_| "audio lock poisoned".to_string())?;
        if guard.is_some() {
            return Err("A dictation recording is already in progress.".into());
        }

        let normalized = if source == "system" { "system" } else { "microphone" };
        let stop = Arc::new(AtomicBool::new(false));
        let level = Arc::new(AtomicU32::new(0));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();
        let source_owned = normalized.to_string();
        let stop_for_thread = Arc::clone(&stop);
        let level_for_thread = Arc::clone(&level);
        let app_for_thread = app.clone();

        let join = thread::Builder::new()
            .name("verbatim-audio-capture".into())
            .spawn(move || capture_loop(source_owned, stop_for_thread, level_for_thread, app_for_thread, ready_tx))
            .map_err(|error| format!("start audio thread: {error}"))?;

        match ready_rx.recv_timeout(Duration::from_secs(12)) {
            Ok(Ok(_sample_rate)) => {
                *guard = Some(RecordingSession {
                    stop,
                    join,
                    source: normalized.to_string(),
                });
                Ok(())
            }
            Ok(Err(error)) => {
                stop.store(true, Ordering::SeqCst);
                let _ = join.join();
                Err(error)
            }
            Err(error) => {
                stop.store(true, Ordering::SeqCst);
                Err(format!("Audio device did not start in time: {error}"))
            }
        }
    }

    pub fn stop(&self) -> Result<(PathBuf, String), String> {
        let mut guard = self.session.lock().map_err(|_| "audio lock poisoned".to_string())?;
        let session = guard.take().ok_or_else(|| "No dictation recording is in progress.".to_string())?;
        session.stop.store(true, Ordering::SeqCst);
        let source = session.source;
        match session.join.join() {
            Ok(Ok(path)) => Ok((path, source)),
            Ok(Err(error)) => Err(error),
            Err(_) => Err("The audio capture thread panicked.".into()),
        }
    }
}

fn capture_loop(
    source: String,
    stop: Arc<AtomicBool>,
    level: Arc<AtomicU32>,
    app: AppHandle,
    ready_tx: mpsc::Sender<Result<u32, String>>,
) -> Result<PathBuf, String> {
    let host = cpal::default_host();
    let device = match select_device(&host, &source) {
        Ok(device) => device,
        Err(error) => {
            let _ = ready_tx.send(Err(error.clone()));
            return Err(error);
        }
    };
    let supported = match device.default_input_config() {
        Ok(config) => config,
        Err(error) => {
            let message = format!("No supported input configuration: {error}");
            let _ = ready_tx.send(Err(message.clone()));
            return Err(message);
        }
    };
    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.config();
    let sample_rate = config.sample_rate.0;
    let channels = config.channels as usize;
    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(sample_rate as usize * 30)));
    let stream_error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let samples_cb = Arc::clone(&samples);
    let level_cb = Arc::clone(&level);
    let error_cb = Arc::clone(&stream_error);
    let err_fn = move |error: cpal::StreamError| {
        if let Ok(mut current) = error_cb.lock() {
            *current = Some(error.to_string());
        }
    };

    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                push_f32(&samples_cb, &level_cb, data, channels);
            },
            err_fn,
            None,
        ),
        SampleFormat::I16 => {
            let samples_cb = Arc::clone(&samples);
            let level_cb = Arc::clone(&level);
            let error_cb = Arc::clone(&stream_error);
            let err_fn = move |error: cpal::StreamError| {
                if let Ok(mut current) = error_cb.lock() {
                    *current = Some(error.to_string());
                }
            };
            device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    let converted: Vec<f32> = data.iter().map(|sample| *sample as f32 / 32768.0).collect();
                    push_f32(&samples_cb, &level_cb, &converted, channels);
                },
                err_fn,
                None,
            )
        }
        SampleFormat::U16 => {
            let samples_cb = Arc::clone(&samples);
            let level_cb = Arc::clone(&level);
            let error_cb = Arc::clone(&stream_error);
            let err_fn = move |error: cpal::StreamError| {
                if let Ok(mut current) = error_cb.lock() {
                    *current = Some(error.to_string());
                }
            };
            device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    let converted: Vec<f32> = data
                        .iter()
                        .map(|sample| (*sample as f32 - 32768.0) / 32768.0)
                        .collect();
                    push_f32(&samples_cb, &level_cb, &converted, channels);
                },
                err_fn,
                None,
            )
        }
        other => {
            let message = format!("Unsupported input sample format: {other:?}");
            let _ = ready_tx.send(Err(message.clone()));
            return Err(message);
        }
    };

    let stream = match stream {
        Ok(stream) => stream,
        Err(error) => {
            let message = format!("Could not open audio input: {error}");
            let _ = ready_tx.send(Err(message.clone()));
            return Err(message);
        }
    };
    if let Err(error) = stream.play() {
        let message = format!("Could not start audio input: {error}");
        let _ = ready_tx.send(Err(message.clone()));
        return Err(message);
    }
    let _ = ready_tx.send(Ok(sample_rate));

    let mut last_emit = Instant::now();
    while !stop.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(35));
        if last_emit.elapsed() >= Duration::from_millis(55) {
            let level_value = f32::from_bits(level.load(Ordering::Relaxed));
            let _ = app.emit("dictation:level", level_value);
            last_emit = Instant::now();
        }
    }
    drop(stream);

    if let Ok(error) = stream_error.lock() {
        if let Some(message) = error.as_ref() {
            return Err(format!("Audio stream failed: {message}"));
        }
    }

    let dir = crate::logging::data_dir().join("tmp");
    std::fs::create_dir_all(&dir).map_err(|error| format!("create temp dir: {error}"))?;
    let path = dir.join(format!("dictation-{}.wav", uuid::Uuid::new_v4()));
    let collected = samples.lock().map(|value| value.clone()).unwrap_or_default();
    write_wav(&path, &collected, sample_rate)?;
    Ok(path)
}

fn push_f32(samples: &Arc<Mutex<Vec<f32>>>, level: &Arc<AtomicU32>, data: &[f32], channels: usize) {
    if channels == 0 {
        return;
    }
    let mut sum_squares = 0.0f64;
    let mut count = 0usize;
    if let Ok(mut buffer) = samples.lock() {
        for frame in data.chunks(channels) {
            let mono = frame.iter().copied().sum::<f32>() / channels as f32;
            buffer.push(mono);
            sum_squares += (mono as f64) * (mono as f64);
            count += 1;
        }
    }
    let rms = if count > 0 {
        (sum_squares / count as f64).sqrt() as f32
    } else {
        0.0
    };
    let scaled = (rms * 5.5).clamp(0.02, 1.0);
    level.store(scaled.to_bits(), Ordering::Relaxed);
}

fn write_wav(path: &PathBuf, samples: &[f32], sample_rate: u32) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|error| format!("create wav: {error}"))?;
    for sample in samples {
        let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(value).map_err(|error| format!("write wav: {error}"))?;
    }
    writer.finalize().map_err(|error| format!("finalize wav: {error}"))
}

fn select_device(host: &cpal::Host, source: &str) -> Result<cpal::Device, String> {
    if source == "system" {
        #[cfg(target_os = "macos")]
        {
            return Err(
                "System audio capture is not enabled on macOS yet. Choose Microphone, or use a loopback audio device."
                    .into(),
            );
        }
        let devices = host
            .input_devices()
            .map_err(|error| format!("Enumerate audio inputs: {error}"))?;
        for device in devices {
            let name = device.name().unwrap_or_default().to_lowercase();
            let looks_like_loopback = name.contains("loopback")
                || name.contains("stereo mix")
                || name.contains("monitor")
                || name.contains("what u hear")
                || name.contains("cable output");
            if looks_like_loopback && device.default_input_config().is_ok() {
                tracing::info!(device = %name, "using monitor/loopback input for system audio");
                return Ok(device);
            }
        }
        return Err(
            "No system-audio monitor input was found. On Windows enable “Stereo Mix”, or use a virtual loopback device. On Linux select a PulseAudio monitor source."
                .into(),
        );
    }
    host.default_input_device()
        .ok_or_else(|| "No microphone was found. Connect an input device and try again.".to_string())
}
