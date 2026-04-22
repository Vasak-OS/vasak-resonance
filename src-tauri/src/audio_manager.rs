use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use crate::audio::extract_now_playing_metadata_with_cover_cache;
use crate::structs::{NowPlayingMetadata, PlaybackProgressEvent};

enum AudioCommand {
    PlayFile {
        file_path: String,
        respond_to: Sender<Result<(), String>>,
    },
    Pause {
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
}

#[derive(Clone)]
pub struct AudioState {
    command_tx: Arc<Mutex<Sender<AudioCommand>>>,
}

impl AudioState {
    pub fn new(app_handle: AppHandle) -> Self {
        let (command_tx, command_rx) = mpsc::channel::<AudioCommand>();

        thread::spawn(move || {
            run_audio_loop(app_handle, command_rx);
        });

        Self {
            command_tx: Arc::new(Mutex::new(command_tx)),
        }
    }

    pub fn play_file(&self, file_path: String) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.send(AudioCommand::PlayFile {
            file_path,
            respond_to: tx,
        })?;
        rx.recv()
            .map_err(|_| "No se recibió respuesta del hilo de audio".to_string())?
    }

    pub fn pause(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.send(AudioCommand::Pause { respond_to: tx })?;
        rx.recv()
            .map_err(|_| "No se recibió respuesta del hilo de audio".to_string())?
    }

    pub fn resume(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.send(AudioCommand::Resume { respond_to: tx })?;
        rx.recv()
            .map_err(|_| "No se recibió respuesta del hilo de audio".to_string())?
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

    fn send(&self, command: AudioCommand) -> Result<(), String> {
        let command_tx = self
            .command_tx
            .lock()
            .map_err(|_| "No se pudo acceder al canal de audio".to_string())?;

        command_tx
            .send(command)
            .map_err(|_| "No se pudo enviar comando al hilo de audio".to_string())
    }
}

struct AudioManager {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Sink,
    current_path: Option<PathBuf>,
    current_metadata: Option<NowPlayingMetadata>,
    cover_cache_by_path: HashMap<String, Option<String>>,
    current_duration: Option<Duration>,
    started_at: Option<Instant>,
    paused_position: Duration,
    is_paused: bool,
    volume: f32,
}

impl AudioManager {
    fn new() -> Result<Self, String> {
        let (stream, stream_handle) =
            OutputStream::try_default().map_err(|e| format!("No se pudo inicializar salida de audio: {e}"))?;

        let sink =
            Sink::try_new(&stream_handle).map_err(|e| format!("No se pudo crear sink de audio: {e}"))?;

        Ok(Self {
            _stream: stream,
            stream_handle,
            sink,
            current_path: None,
            current_metadata: None,
            cover_cache_by_path: HashMap::new(),
            current_duration: None,
            started_at: None,
            paused_position: Duration::from_secs(0),
            is_paused: false,
            volume: 1.0,
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

        let new_sink =
            Sink::try_new(&self.stream_handle).map_err(|e| format!("No se pudo crear sink: {e}"))?;
        new_sink.set_volume(self.volume);
        new_sink.append(decoder);
        new_sink.play();

        self.sink.stop();
        self.sink = new_sink;
        self.current_path = Some(canonical_path);
        self.current_metadata = self.current_path.as_ref().and_then(|path| {
            extract_now_playing_metadata_with_cover_cache(path, &mut self.cover_cache_by_path).ok()
        });
        self.current_duration = duration;
        self.started_at = Some(Instant::now());
        self.paused_position = Duration::from_secs(0);
        self.is_paused = false;

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

        let file = File::open(&path).map_err(|e| format!("No se pudo abrir archivo de audio: {e}"))?;
        let decoder =
            Decoder::new(BufReader::new(file)).map_err(|e| format!("No se pudo decodificar audio: {e}"))?;
        let duration = decoder.total_duration();

        let new_sink =
            Sink::try_new(&self.stream_handle).map_err(|e| format!("No se pudo crear sink: {e}"))?;
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

fn run_audio_loop(app_handle: AppHandle, command_rx: Receiver<AudioCommand>) {
    let mut manager = match AudioManager::new() {
        Ok(manager) => manager,
        Err(error) => {
            loop {
                match command_rx.recv() {
                    Ok(AudioCommand::PlayFile { respond_to, .. })
                    | Ok(AudioCommand::Pause { respond_to })
                    | Ok(AudioCommand::Resume { respond_to })
                    | Ok(AudioCommand::Seek { respond_to, .. })
                    | Ok(AudioCommand::SetVolume { respond_to, .. }) => {
                        let _ = respond_to.send(Err(error.clone()));
                    }
                    Err(_) => return,
                }
            }
        }
    };

    loop {
        match command_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(AudioCommand::PlayFile {
                file_path,
                respond_to,
            }) => {
                let _ = respond_to.send(manager.play_file(file_path));
                let _ = app_handle.emit("audio-playback-progress", manager.progress_snapshot());
            }
            Ok(AudioCommand::Pause { respond_to }) => {
                let _ = respond_to.send(manager.pause());
                let _ = app_handle.emit("audio-playback-progress", manager.progress_snapshot());
            }
            Ok(AudioCommand::Resume { respond_to }) => {
                let _ = respond_to.send(manager.resume());
                let _ = app_handle.emit("audio-playback-progress", manager.progress_snapshot());
            }
            Ok(AudioCommand::Seek { second, respond_to }) => {
                let _ = respond_to.send(manager.seek(second));
                let _ = app_handle.emit("audio-playback-progress", manager.progress_snapshot());
            }
            Ok(AudioCommand::SetVolume { volume, respond_to }) => {
                let _ = respond_to.send(manager.set_volume(volume));
                let _ = app_handle.emit("audio-playback-progress", manager.progress_snapshot());
            }
            Err(RecvTimeoutError::Timeout) => {
                let _ = app_handle.emit("audio-playback-progress", manager.progress_snapshot());
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}
