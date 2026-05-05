use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use crate::audio::extract_now_playing_metadata_with_cover_cache;
use crate::structs::{NowPlayingMetadata, PlaybackProgressEvent};

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeStatus {
    has_track: bool,
    is_paused: bool,
}

enum AudioCommand {
    PlayFile {
        file_path: String,
        respond_to: Sender<Result<(), String>>,
    },
    PlayStream {
        url: String,
        station_name: String,
        respond_to: Sender<Result<(), String>>,
    },
    Pause {
        respond_to: Sender<Result<(), String>>,
    },
    Stop {
        respond_to: Sender<Result<(), String>>,
    },
    Resume {
        respond_to: Sender<Result<(), String>>,
    },
    Seek {
        second: u64,
        respond_to: Sender<Result<(), String>>,
    },
    SetVolume {
        volume: f32,
        respond_to: Sender<Result<(), String>>,
    },
    Shutdown,
}

#[derive(Clone)]
pub struct AudioState {
    command_tx: Arc<Mutex<Sender<AudioCommand>>>,
    runtime_status: Arc<Mutex<RuntimeStatus>>,
    playback_snapshot: Arc<Mutex<PlaybackProgressEvent>>,
}

impl AudioState {
    pub fn new(app_handle: AppHandle) -> Self {
        let (command_tx, command_rx) = mpsc::channel::<AudioCommand>();
        let playback_snapshot = Arc::new(Mutex::new(PlaybackProgressEvent::default()));
        let playback_snapshot_for_thread = Arc::clone(&playback_snapshot);

        thread::spawn(move || {
            run_audio_loop(app_handle, command_rx, playback_snapshot_for_thread);
        });

        Self {
            command_tx: Arc::new(Mutex::new(command_tx)),
            runtime_status: Arc::new(Mutex::new(RuntimeStatus::default())),
            playback_snapshot,
        }
    }

    pub fn play_file(&self, file_path: String) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.send(AudioCommand::PlayFile {
            file_path,
            respond_to: tx,
        })?;
        let response = rx
            .recv()
            .map_err(|_| "No se recibió respuesta del hilo de audio".to_string())?;

        if response.is_ok() {
            self.update_runtime_status(true, false)?;
        }

        response
    }

    pub fn play_stream(&self, url: String, station_name: String) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.send(AudioCommand::PlayStream {
            url,
            station_name,
            respond_to: tx,
        })?;
        let response = rx
            .recv()
            .map_err(|_| "No se recibió respuesta del hilo de audio".to_string())?;

        if response.is_ok() {
            self.update_runtime_status(true, false)?;
        }

        response
    }

    pub fn pause(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.send(AudioCommand::Pause { respond_to: tx })?;
        let response = rx
            .recv()
            .map_err(|_| "No se recibió respuesta del hilo de audio".to_string())?;

        if response.is_ok() {
            self.update_runtime_status(true, true)?;
        }

        response
    }

    pub fn resume(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.send(AudioCommand::Resume { respond_to: tx })?;
        let response = rx
            .recv()
            .map_err(|_| "No se recibió respuesta del hilo de audio".to_string())?;

        if response.is_ok() {
            self.update_runtime_status(true, false)?;
        }

        response
    }

    pub fn stop(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.send(AudioCommand::Stop { respond_to: tx })?;
        let response = rx
            .recv()
            .map_err(|_| "No se recibió respuesta del hilo de audio".to_string())?;

        if response.is_ok() {
            self.update_runtime_status(false, false)?;
        }

        response
    }

    pub fn shutdown(&self) -> Result<(), String> {
        self.send(AudioCommand::Shutdown)?;
        Ok(())
    }

    pub fn seek(&self, second: u64) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.send(AudioCommand::Seek {
            second,
            respond_to: tx,
        })?;
        rx.recv()
            .map_err(|_| "No se recibió respuesta del hilo de audio".to_string())?
    }

    pub fn set_volume(&self, volume: f32) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.send(AudioCommand::SetVolume {
            volume,
            respond_to: tx,
        })?;
        rx.recv()
            .map_err(|_| "No se recibió respuesta del hilo de audio".to_string())?
    }

    pub fn play_pause_toggle(&self) -> Result<(), String> {
        let status = self
            .runtime_status
            .lock()
            .map_err(|_| "No se pudo leer estado de reproducción".to_string())?
            .to_owned();

        if !status.has_track {
            return Err("No hay ninguna canción cargada".to_string());
        }

        if status.is_paused {
            self.resume()
        } else {
            self.pause()
        }
    }

    pub fn play(&self) -> Result<(), String> {
        match self.playback_status()? {
            "Paused" => self.resume(),
            _ => Ok(()),
        }
    }

    pub fn playback_status(&self) -> Result<&'static str, String> {
        let status = self
            .runtime_status
            .lock()
            .map_err(|_| "No se pudo leer estado de reproducción".to_string())?
            .to_owned();

        if !status.has_track {
            Ok("Stopped")
        } else if status.is_paused {
            Ok("Paused")
        } else {
            Ok("Playing")
        }
    }

    pub fn playback_snapshot(&self) -> Result<PlaybackProgressEvent, String> {
        self.playback_snapshot
            .lock()
            .map_err(|_| "No se pudo leer el snapshot de reproducción".to_string())
            .map(|snapshot| snapshot.clone())
    }

    fn send(&self, command: AudioCommand) -> Result<(), String> {
        let command_tx = self
            .command_tx
            .lock()
            .map_err(|_| "No se pudo acceder al canal de audio".to_string())?;

        command_tx
            .send(command)
            .map_err(|_| "No se pudo enviar comando al hilo de audio".to_string())
    }

    fn update_runtime_status(&self, has_track: bool, is_paused: bool) -> Result<(), String> {
        let mut status = self
            .runtime_status
            .lock()
            .map_err(|_| "No se pudo actualizar estado de reproducción".to_string())?;
        status.has_track = has_track;
        status.is_paused = is_paused;
        Ok(())
    }
}

struct AudioManager {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Sink,
    current_path: Option<PathBuf>,
    current_metadata: Option<NowPlayingMetadata>,
    cover_cache_by_path: HashMap<String, Option<String>>,
    dominant_color_cache_by_path: HashMap<String, Option<String>>,
    current_duration: Option<Duration>,
    started_at: Option<Instant>,
    paused_position: Duration,
    is_paused: bool,
    volume: f32,
    stream_tempfile: Option<PathBuf>,
    _stream_writer_running: bool,
}

impl AudioManager {
    fn new() -> Result<Self, String> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("No se pudo inicializar salida de audio: {e}"))?;

        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| format!("No se pudo crear sink de audio: {e}"))?;

        Ok(Self {
            _stream: stream,
            stream_handle,
            sink,
            current_path: None,
            current_metadata: None,
            cover_cache_by_path: HashMap::new(),
            dominant_color_cache_by_path: HashMap::new(),
            current_duration: None,
            started_at: None,
            paused_position: Duration::from_secs(0),
            is_paused: false,
            volume: 1.0,
            stream_tempfile: None,
            _stream_writer_running: false,
        })
    }

    fn play_file(&mut self, file_path: String) -> Result<(), String> {
        let path = PathBuf::from(file_path);
        if !path.exists() || !path.is_file() {
            return Err("El archivo no existe o no es válido".to_string());
        }

        let canonical_path = std::fs::canonicalize(&path).unwrap_or(path);
        let file = File::open(&canonical_path)
            .map_err(|e| format!("No se pudo abrir archivo de audio: {e}"))?;
        let decoder = Decoder::new(BufReader::new(file))
            .map_err(|e| format!("No se pudo decodificar audio: {e}"))?;
        let duration = decoder.total_duration();

        let new_sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("No se pudo crear sink: {e}"))?;
        new_sink.set_volume(self.volume);
        new_sink.append(decoder);
        new_sink.play();

        self.sink.stop();
        self.sink = new_sink;
        self.current_path = Some(canonical_path);
        self.current_metadata = self.current_path.as_ref().and_then(|path| {
            extract_now_playing_metadata_with_cover_cache(
                path,
                &mut self.cover_cache_by_path,
                &mut self.dominant_color_cache_by_path,
            )
            .ok()
        });
        self.current_duration = duration;
        self.started_at = Some(Instant::now());
        self.paused_position = Duration::from_secs(0);
        self.is_paused = false;

        Ok(())
    }

    fn play_stream_blocking(&mut self, url: &str, station_name: &str) -> Result<(), String> {
        // Buffer a small prefix of the stream to a temp file, then open it for decoding
        // and continue writing while decoding. This is a pragmatic approach for
        // streaming formats that require Seek at decode initialization.
        let client = reqwest::blocking::Client::new();
        let mut resp = client
            .get(url)
            .header("User-Agent", "vasak-resonance/1.0")
            .send()
            .map_err(|e| format!("Error connecting to stream: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Stream returned status: {}", resp.status()));
        }

        // Create a named temp file so we can open it twice (writer + reader)
        let mut named = tempfile::NamedTempFile::new().map_err(|e| format!("Tempfile error: {}", e))?;
        let path = named.path().to_path_buf();

        // Write an initial buffer (e.g., 256KB) to allow decoder detection
        const PRIMER_BYTES: usize = 256 * 1024;
        let mut written: usize = 0;
        while written < PRIMER_BYTES {
            let mut buf = [0u8; 8192];
            let n = resp.read(&mut buf).map_err(|e| format!("Stream read error: {}", e))?;
            if n == 0 {
                break;
            }
            named
                .write_all(&buf[..n])
                .map_err(|e| format!("Tempfile write error: {}", e))?;
            written += n;
        }

        // Flush and reopen for reading
        named.flush().map_err(|e| format!("Tempfile flush error: {}", e))?;
        let reader_file = std::fs::File::open(&path).map_err(|e| format!("Open tempfile for read error: {}", e))?;
        let mut buf_reader = std::io::BufReader::new(reader_file);

        let decoder = Decoder::new(buf_reader).map_err(|e| format!("No se pudo decodificar stream de audio: {e}"))?;
        let duration = decoder.total_duration();

        // Continue writing the rest of the stream in a detached thread so decoding can continue
        let mut writer = named.reopen().map_err(|e| format!("Reopen tempfile for write error: {}", e))?;
        std::thread::spawn(move || {
            let mut resp = resp; // moved into closure
            let mut buf = [0u8; 8192];
            loop {
                match resp.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let _ = writer.write_all(&buf[..n]);
                        let _ = writer.flush();
                    }
                }
            }
        });

        let new_sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("No se pudo crear sink: {e}"))?;
        new_sink.set_volume(self.volume);
        new_sink.append(decoder);
        new_sink.play();

        self.sink.stop();
        self.sink = new_sink;
        self.current_path = None;
        self.current_metadata = Some(NowPlayingMetadata {
            title: station_name.to_string(),
            artist: "Radio Stream".to_string(),
            album: String::new(),
            duration_seconds: duration.as_ref().map(|d| d.as_secs()).unwrap_or(0),
            cover_data_url: None,
            dominant_color: None,
            path: url.to_string(),
        });
        self.current_duration = duration;
        self.started_at = Some(Instant::now());
        self.paused_position = Duration::from_secs(0);
        self.is_paused = false;

        // Remember tempfile path so we can clean up later
        self.stream_tempfile = Some(path);

        Ok(())
    }

    fn pause(&mut self) -> Result<(), String> {
        if self.current_path.is_none() {
            return Err("No hay ninguna canción cargada".to_string());
        }

        if self.is_paused {
            return Ok(());
        }

        self.paused_position = self.current_position_duration();
        self.started_at = None;
        self.is_paused = true;
        self.sink.pause();
        Ok(())
    }

    fn resume(&mut self) -> Result<(), String> {
        if self.current_path.is_none() {
            return Err("No hay ninguna canción cargada".to_string());
        }

        if !self.is_paused {
            return Ok(());
        }

        self.started_at = Some(Instant::now());
        self.is_paused = false;
        self.sink.play();
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.sink.stop();
        self.current_path = None;
        self.current_metadata = None;
        self.current_duration = None;
        self.started_at = None;
        self.paused_position = Duration::from_secs(0);
        self.is_paused = false;
        // Clean up any temporary stream file
        if let Some(path) = &self.stream_tempfile {
            let _ = std::fs::remove_file(path);
            self.stream_tempfile = None;
        }
        Ok(())
    }

    fn seek(&mut self, second: u64) -> Result<(), String> {
        let path = self
            .current_path
            .clone()
            .ok_or_else(|| "No hay ninguna canción cargada".to_string())?;

        let target = if let Some(duration) = self.current_duration {
            second.min(duration.as_secs())
        } else {
            second
        };

        let file =
            File::open(&path).map_err(|e| format!("No se pudo abrir archivo de audio: {e}"))?;
        let decoder = Decoder::new(BufReader::new(file))
            .map_err(|e| format!("No se pudo decodificar audio: {e}"))?;
        let duration = decoder.total_duration();

        let new_sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("No se pudo crear sink: {e}"))?;
        new_sink.set_volume(self.volume);

        if target > 0 {
            new_sink.append(decoder.skip_duration(Duration::from_secs(target)));
        } else {
            new_sink.append(decoder);
        }

        if self.is_paused {
            new_sink.pause();
            self.started_at = None;
        } else {
            new_sink.play();
            self.started_at = Some(Instant::now());
        }

        self.sink.stop();
        self.sink = new_sink;
        self.current_duration = duration;
        self.paused_position = Duration::from_secs(target);

        Ok(())
    }

    fn set_volume(&mut self, volume: f32) -> Result<(), String> {
        if !volume.is_finite() {
            return Err("Volumen inválido".to_string());
        }

        let normalized = volume.clamp(0.0, 2.0);
        self.volume = normalized;
        self.sink.set_volume(normalized);
        Ok(())
    }

    fn current_position_duration(&self) -> Duration {
        let mut position = self.paused_position;

        if !self.is_paused {
            if let Some(started_at) = self.started_at {
                position += started_at.elapsed();
            }
        }

        if let Some(duration) = self.current_duration {
            position = position.min(duration);
        }

        position
    }

    fn progress_snapshot(&self) -> PlaybackProgressEvent {
        let position = self.current_position_duration();
        let duration_seconds = self.current_duration.map(|duration| duration.as_secs());
        let is_playing = self.current_path.is_some() && !self.is_paused && !self.sink.empty();

        PlaybackProgressEvent {
            path: self
                .current_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            position_seconds: position.as_secs(),
            duration_seconds,
            is_playing,
            is_paused: self.is_paused,
            volume: self.volume,
            now_playing: self.current_metadata.clone(),
        }
    }
}

fn run_audio_loop(
    app_handle: AppHandle,
    command_rx: Receiver<AudioCommand>,
    playback_snapshot: Arc<Mutex<PlaybackProgressEvent>>,
) {
    let mut manager = match AudioManager::new() {
        Ok(manager) => manager,
        Err(error) => loop {
            match command_rx.recv() {
                Ok(AudioCommand::PlayFile { respond_to, .. })
                | Ok(AudioCommand::PlayStream { respond_to, .. })
                | Ok(AudioCommand::Pause { respond_to })
                | Ok(AudioCommand::Stop { respond_to })
                | Ok(AudioCommand::Resume { respond_to })
                | Ok(AudioCommand::Seek { respond_to, .. })
                | Ok(AudioCommand::SetVolume { respond_to, .. }) => {
                    let _ = respond_to.send(Err(error.clone()));
                }
                Ok(AudioCommand::Shutdown) => return,
                Err(_) => return,
            }
        },
    };

    // No separate Tokio runtime needed for blocking stream playback.

    loop {
        match command_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(AudioCommand::PlayFile {
                file_path,
                respond_to,
            }) => {
                let _ = respond_to.send(manager.play_file(file_path));
                publish_snapshot(&app_handle, &playback_snapshot, manager.progress_snapshot());
            }
            Ok(AudioCommand::PlayStream {
                url,
                station_name,
                respond_to,
            }) => {
                // Use blocking playback on the audio thread to stream without downloading entire data.
                let result = manager.play_stream_blocking(&url, &station_name);
                let _ = respond_to.send(result);
                publish_snapshot(&app_handle, &playback_snapshot, manager.progress_snapshot());
            }
            Ok(AudioCommand::Pause { respond_to }) => {
                let _ = respond_to.send(manager.pause());
                publish_snapshot(&app_handle, &playback_snapshot, manager.progress_snapshot());
            }
            Ok(AudioCommand::Stop { respond_to }) => {
                let _ = respond_to.send(manager.stop());
                publish_snapshot(&app_handle, &playback_snapshot, manager.progress_snapshot());
            }
            Ok(AudioCommand::Resume { respond_to }) => {
                let _ = respond_to.send(manager.resume());
                publish_snapshot(&app_handle, &playback_snapshot, manager.progress_snapshot());
            }
            Ok(AudioCommand::Seek { second, respond_to }) => {
                let _ = respond_to.send(manager.seek(second));
                publish_snapshot(&app_handle, &playback_snapshot, manager.progress_snapshot());
            }
            Ok(AudioCommand::SetVolume { volume, respond_to }) => {
                let _ = respond_to.send(manager.set_volume(volume));
                publish_snapshot(&app_handle, &playback_snapshot, manager.progress_snapshot());
            }
            Ok(AudioCommand::Shutdown) => {
                let _ = manager.stop();
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                publish_snapshot(&app_handle, &playback_snapshot, manager.progress_snapshot());
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn publish_snapshot(
    app_handle: &AppHandle,
    playback_snapshot: &Arc<Mutex<PlaybackProgressEvent>>,
    snapshot: PlaybackProgressEvent,
) {
    if let Ok(mut shared_snapshot) = playback_snapshot.lock() {
        *shared_snapshot = snapshot.clone();
    }

    let _ = app_handle.emit("audio-playback-progress", snapshot);
}
