use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
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
        seek_to: Option<u64>,
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

    pub fn play_file(&self, file_path: String, seek_to: Option<u64>) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.send(AudioCommand::PlayFile {
            file_path,
            seek_to,
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

/// Rodio source backed by a channel of PCM (f32) chunks produced by a decoder
/// thread. Used for all playback (local files and radio) so audio is streamed,
/// never fully decoded into memory.
struct StreamSource {
    rx: mpsc::Receiver<Vec<f32>>,
    current: Vec<f32>,
    pos: usize,
    channels: u16,
    sample_rate: u32,
}

impl Iterator for StreamSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.pos >= self.current.len() {
            match self.rx.recv() {
                Ok(v) => {
                    self.current = v;
                    self.pos = 0;
                }
                Err(_) => return None,
            }
            if self.current.is_empty() {
                return None;
            }
        }
        let v = self.current[self.pos];
        self.pos += 1;
        Some(v)
    }
}

impl Source for StreamSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        self.channels
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

struct AudioManager {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Sink,
    /// What is loaded: a file path, or a station URL when `stream_url` is set.
    ///
    /// Radio used to leave this empty, and every transport method starts by
    /// checking it — so pause, resume and the play/pause state were all dead
    /// while a station played.
    current_path: Option<PathBuf>,
    /// Set only for radio; a live stream cannot be sought and pausing it has to
    /// tear the connection down rather than accumulate stale audio.
    stream_url: Option<String>,
    current_metadata: Option<NowPlayingMetadata>,
    cover_cache_by_path: HashMap<String, Option<String>>,
    dominant_color_cache_by_path: HashMap<String, Option<String>>,
    current_duration: Option<Duration>,
    started_at: Option<Instant>,
    paused_position: Duration,
    is_paused: bool,
    volume: f32,
    // streaming management
    stream_writer_handle: Option<thread::JoinHandle<()>>,
    stream_decoder_handle: Option<thread::JoinHandle<()>>,
    ffmpeg_process: Option<std::process::Child>,
    streaming_active: Arc<AtomicBool>,
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
            stream_url: None,
            current_metadata: None,
            cover_cache_by_path: HashMap::new(),
            dominant_color_cache_by_path: HashMap::new(),
            current_duration: None,
            started_at: None,
            paused_position: Duration::from_secs(0),
            is_paused: false,
            volume: 1.0,
            stream_writer_handle: None,
            stream_decoder_handle: None,
            ffmpeg_process: None,
            streaming_active: Arc::new(AtomicBool::new(false)),
        })
    }

    fn play_file(&mut self, file_path: String, seek_to: Option<u64>) -> Result<(), String> {
        let path = PathBuf::from(file_path);
        if !path.exists() || !path.is_file() {
            return Err("El archivo no existe o no es válido".to_string());
        }
        let canonical_path = std::fs::canonicalize(&path).unwrap_or(path);

        // Duration/metadata come from tags — the file is streamed, not decoded
        // into memory.
        let metadata = extract_now_playing_metadata_with_cover_cache(
            &canonical_path,
            &mut self.cover_cache_by_path,
            &mut self.dominant_color_cache_by_path,
        )
        .ok();
        let total_dur = metadata.as_ref().map(|m| m.duration_seconds).unwrap_or(0);
        let target = if total_dur > 0 {
            seek_to.unwrap_or(0).min(total_dur)
        } else {
            seek_to.unwrap_or(0)
        };

        self.stop_stream_child();
        let (child, decoder_handle) =
            self.play_ffmpeg_stream(&Self::file_ffmpeg_args(&canonical_path, target))?;

        self.ffmpeg_process = Some(child);
        self.stream_decoder_handle = Some(decoder_handle);
        self.current_path = Some(canonical_path);
        self.stream_url = None;
        self.current_metadata = metadata;
        self.current_duration = Some(Duration::from_secs(total_dur));
        self.started_at = Some(Instant::now());
        self.paused_position = Duration::from_secs(target);
        self.is_paused = false;
        Ok(())
    }

    /// ffmpeg args to stream a local file to stdout as PCM f32 WAV, seeking to
    /// `seek_secs` first (fast input seeking).
    fn file_ffmpeg_args(path: &Path, seek_secs: u64) -> Vec<String> {
        let mut args: Vec<String> = vec!["-v".into(), "quiet".into()];
        if seek_secs > 0 {
            args.push("-ss".into());
            args.push(seek_secs.to_string());
        }
        args.push("-i".into());
        args.push(path.to_string_lossy().to_string());
        for a in ["-f", "wav", "-acodec", "pcm_f32le", "pipe:1"] {
            args.push(a.to_string());
        }
        args
    }

    /// Spawn ffmpeg with `args`, stream its PCM output through a decoder thread
    /// into a fresh sink, and swap it in. A bounded channel provides backpressure
    /// so ffmpeg is paced to playback speed instead of buffering the whole file;
    /// the `streaming_active` flag makes the decoder abortable. Returns the child
    /// and the decoder thread handle.
    fn play_ffmpeg_stream(
        &mut self,
        args: &[String],
    ) -> Result<(std::process::Child, thread::JoinHandle<()>), String> {
        let mut child = std::process::Command::new("ffmpeg")
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    "ffmpeg no está instalado. Instálelo para reproducir audio (ej: sudo pacman -S ffmpeg)".to_string()
                } else {
                    format!("Error al ejecutar ffmpeg: {e}")
                }
            })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "No se pudo obtener stdout de ffmpeg".to_string())?;
        let mut reader = std::io::BufReader::new(stdout);
        let (channels, sample_rate) = Self::read_wav_header(&mut reader)?;

        self.streaming_active.store(true, Ordering::SeqCst);
        let active = self.streaming_active.clone();
        // ~32 chunks of up to 64 KiB ≈ a few seconds of audio buffered.
        let (pcm_tx, pcm_rx) = mpsc::sync_channel::<Vec<f32>>(32);

        let decoder_handle = thread::spawn(move || {
            let mut buf = [0u8; 65536];
            'read: loop {
                if !active.load(Ordering::SeqCst) {
                    break;
                }
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let samples: Vec<f32> = buf[..n / 4 * 4]
                            .chunks_exact(4)
                            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                            .collect();
                        // Backpressure with an abort check so stop() never blocks.
                        let mut pending = samples;
                        loop {
                            match pcm_tx.try_send(pending) {
                                Ok(()) => break,
                                Err(mpsc::TrySendError::Full(s)) => {
                                    if !active.load(Ordering::SeqCst) {
                                        break 'read;
                                    }
                                    pending = s;
                                    thread::sleep(Duration::from_millis(5));
                                }
                                Err(mpsc::TrySendError::Disconnected(_)) => break 'read,
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let source = StreamSource {
            rx: pcm_rx,
            current: Vec::new(),
            pos: 0,
            channels,
            sample_rate,
        };
        let new_sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("No se pudo crear sink: {e}"))?;
        new_sink.set_volume(self.volume);
        new_sink.append(source);
        new_sink.play();

        self.sink.stop();
        self.sink = new_sink;
        Ok((child, decoder_handle))
    }

    /// Kill the current ffmpeg process (if any) and join the stream threads.
    fn stop_stream_child(&mut self) {
        self.streaming_active.store(false, Ordering::SeqCst);
        if let Some(mut child) = self.ffmpeg_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(handle) = self.stream_writer_handle.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.stream_decoder_handle.take() {
            let _ = handle.join();
        }
    }


    /// ffmpeg args to pull a live stream to stdout as PCM f32 WAV.
    fn stream_ffmpeg_args(url: &str) -> Vec<String> {
        ["-v", "quiet", "-i", url, "-f", "wav", "-acodec", "pcm_f32le", "pipe:1"]
            .iter()
            .map(|arg| arg.to_string())
            .collect()
    }

    fn play_stream_blocking(&mut self, url: &str, station_name: &str) -> Result<(), String> {
        self.stop_stream_child();

        let (child, decoder_handle) = self.play_ffmpeg_stream(&Self::stream_ffmpeg_args(url))?;

        self.ffmpeg_process = Some(child);
        self.stream_decoder_handle = Some(decoder_handle);
        // The URL stands in for the path so the transport controls, the
        // play/pause state and the events all recognise that something is
        // loaded.
        self.current_path = Some(PathBuf::from(url));
        self.stream_url = Some(url.to_string());
        self.current_metadata = Some(NowPlayingMetadata {
            title: station_name.to_string(),
            artist: "Radio Stream".to_string(),
            album: String::new(),
            duration_seconds: 0,
            cover_data_url: None,
            dominant_color: None,
            path: url.to_string(),
        });
        self.current_duration = None;
        self.started_at = Some(Instant::now());
        self.paused_position = Duration::from_secs(0);
        self.is_paused = false;
        Ok(())
    }

    fn read_wav_header<R: std::io::Read>(reader: &mut std::io::BufReader<R>) -> Result<(u16, u32), String> {
        let mut riff = [0u8; 12];
        reader.read_exact(&mut riff).map_err(|e| format!("Error leyendo header WAV: {e}"))?;

        if &riff[..4] != b"RIFF" || &riff[8..12] != b"WAVE" {
            return Err("ffmpeg no produjo un WAV válido".to_string());
        }

        let mut channels = 2u16;
        let mut sample_rate = 44100u32;

        loop {
            let mut chunk_id = [0u8; 4];
            let mut chunk_size_raw = [0u8; 4];

            reader.read_exact(&mut chunk_id).map_err(|e| format!("Error leyendo chunk WAV: {e}"))?;
            reader.read_exact(&mut chunk_size_raw).map_err(|e| format!("Error leyendo tamaño chunk: {e}"))?;

            let chunk_size = u32::from_le_bytes(chunk_size_raw) as usize;

            match &chunk_id {
                b"fmt " => {
                    let mut fmt_data = vec![0u8; chunk_size.max(16)];
                    reader.read_exact(&mut fmt_data[..chunk_size.min(16)])
                        .map_err(|e| format!("Error leyendo fmt chunk: {e}"))?;
                    if chunk_size > 16 {
                        let mut skip = vec![0u8; chunk_size - 16];
                        reader.read_exact(&mut skip).map_err(|e| format!("Error saltando fmt extra: {e}"))?;
                    }

                    channels = u16::from_le_bytes([fmt_data[2], fmt_data[3]]);
                    sample_rate = u32::from_le_bytes([fmt_data[4], fmt_data[5], fmt_data[6], fmt_data[7]]);
                }
                b"data" => {
                    return Ok((channels, sample_rate));
                }
                _ => {
                    let mut skip = vec![0u8; chunk_size];
                    if chunk_size > 0 {
                        let _ = reader.read_exact(&mut skip);
                    }
                }
            }
        }
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

        if self.stream_url.is_some() {
            // A live stream has to be disconnected, not held. Keeping ffmpeg
            // running against a paused sink just fills the buffer, so resuming
            // would play minutes-old audio and keep drifting further behind.
            self.stop_stream_child();
            self.sink.stop();
            self.sink = Sink::try_new(&self.stream_handle)
                .map_err(|e| format!("No se pudo crear sink de audio: {e}"))?;
            self.sink.set_volume(self.volume);
            self.sink.pause();
        } else {
            self.sink.pause();
        }

        Ok(())
    }

    fn resume(&mut self) -> Result<(), String> {
        if self.current_path.is_none() {
            return Err("No hay ninguna canción cargada".to_string());
        }

        if !self.is_paused {
            return Ok(());
        }

        // Reconnect rather than resume: the stream was torn down on pause, and
        // a station is live anyway — the listener wants what is on air now.
        if let Some(url) = self.stream_url.clone() {
            self.stop_stream_child();
            let (child, decoder_handle) =
                self.play_ffmpeg_stream(&Self::stream_ffmpeg_args(&url))?;
            self.ffmpeg_process = Some(child);
            self.stream_decoder_handle = Some(decoder_handle);
        } else {
            self.sink.play();
        }

        self.started_at = Some(Instant::now());
        self.is_paused = false;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.stop_stream_child();
        self.sink.stop();
        self.current_path = None;
        self.stream_url = None;
        self.current_metadata = None;
        self.current_duration = None;
        self.started_at = None;
        self.paused_position = Duration::from_secs(0);
        self.is_paused = false;
        Ok(())
    }

    fn seek(&mut self, second: u64) -> Result<(), String> {
        if self.stream_url.is_some() {
            return Err("No se puede avanzar ni retroceder en una radio en vivo".to_string());
        }

        // Streaming: re-spawn ffmpeg from the target offset (fast input seek).
        let path = self
            .current_path
            .clone()
            .ok_or_else(|| "No hay ninguna canción cargada".to_string())?;

        let target = match self.current_duration {
            Some(dur) if dur.as_secs() > 0 => second.min(dur.as_secs()),
            _ => second,
        };
        let was_paused = self.is_paused;

        self.stop_stream_child();
        let (child, decoder_handle) =
            self.play_ffmpeg_stream(&Self::file_ffmpeg_args(&path, target))?;
        self.ffmpeg_process = Some(child);
        self.stream_decoder_handle = Some(decoder_handle);
        self.paused_position = Duration::from_secs(target);

        if was_paused {
            self.sink.pause();
            self.started_at = None;
            self.is_paused = true;
        } else {
            self.started_at = Some(Instant::now());
            self.is_paused = false;
        }
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

    /// Whether the loaded track has played to its end.
    ///
    /// ffmpeg having exited *and* the sink having drained means every decoded
    /// sample has been heard. Checking the sink alone would fire at the moment
    /// playback starts, before the first buffer arrives.
    ///
    /// The frontend used to infer this from the position reaching the duration,
    /// which never happened for a file whose tags carry no duration — those
    /// tracks ended and the player simply sat there instead of moving on.
    fn track_finished(&mut self) -> bool {
        if self.current_path.is_none() || self.is_paused || self.stream_url.is_some() {
            return false;
        }

        let ffmpeg_exited = match self.ffmpeg_process.as_mut() {
            Some(child) => matches!(child.try_wait(), Ok(Some(_))),
            None => false,
        };

        ffmpeg_exited && self.sink.empty()
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

    // Which track's metadata the frontend already has.
    let mut last_published_track: Option<String> = None;

    loop {
        match command_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(AudioCommand::PlayFile {
                file_path,
                seek_to,
                respond_to,
            }) => {
                let _ = respond_to.send(manager.play_file(file_path, seek_to));
                publish_snapshot(
                    &app_handle,
                    &playback_snapshot,
                    &mut last_published_track,
                    manager.progress_snapshot(),
                );
            }
            Ok(AudioCommand::PlayStream {
                url,
                station_name,
                respond_to,
            }) => {
                // Use blocking playback on the audio thread to stream without downloading entire data.
                let result = manager.play_stream_blocking(&url, &station_name);
                let _ = respond_to.send(result);
                publish_snapshot(
                    &app_handle,
                    &playback_snapshot,
                    &mut last_published_track,
                    manager.progress_snapshot(),
                );
            }
            Ok(AudioCommand::Pause { respond_to }) => {
                let _ = respond_to.send(manager.pause());
                publish_snapshot(
                    &app_handle,
                    &playback_snapshot,
                    &mut last_published_track,
                    manager.progress_snapshot(),
                );
            }
            Ok(AudioCommand::Stop { respond_to }) => {
                let _ = respond_to.send(manager.stop());
                publish_snapshot(
                    &app_handle,
                    &playback_snapshot,
                    &mut last_published_track,
                    manager.progress_snapshot(),
                );
            }
            Ok(AudioCommand::Resume { respond_to }) => {
                let _ = respond_to.send(manager.resume());
                publish_snapshot(
                    &app_handle,
                    &playback_snapshot,
                    &mut last_published_track,
                    manager.progress_snapshot(),
                );
            }
            Ok(AudioCommand::Seek { second, respond_to }) => {
                let _ = respond_to.send(manager.seek(second));
                publish_snapshot(
                    &app_handle,
                    &playback_snapshot,
                    &mut last_published_track,
                    manager.progress_snapshot(),
                );
            }
            Ok(AudioCommand::SetVolume { volume, respond_to }) => {
                let _ = respond_to.send(manager.set_volume(volume));
                publish_snapshot(
                    &app_handle,
                    &playback_snapshot,
                    &mut last_published_track,
                    manager.progress_snapshot(),
                );
            }
            Ok(AudioCommand::Shutdown) => {
                let _ = manager.stop();
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                // Announce the end of a track before publishing, so the
                // frontend gets the finished path rather than an already
                // cleared state.
                if manager.track_finished() {
                    let finished = manager
                        .current_path
                        .as_ref()
                        .map(|path| path.to_string_lossy().to_string());
                    let _ = manager.stop();
                    let _ = app_handle.emit("audio-track-finished", finished);
                }

                publish_snapshot(
                    &app_handle,
                    &playback_snapshot,
                    &mut last_published_track,
                    manager.progress_snapshot(),
                );
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

/// Identifies the track a snapshot is describing, for deciding whether the
/// heavy metadata has to travel again.
fn now_playing_identity(snapshot: &PlaybackProgressEvent) -> Option<String> {
    snapshot
        .now_playing
        .as_ref()
        .map(|metadata| metadata.path.clone())
}

fn publish_snapshot(
    app_handle: &AppHandle,
    playback_snapshot: &Arc<Mutex<PlaybackProgressEvent>>,
    last_published_track: &mut Option<String>,
    snapshot: PlaybackProgressEvent,
) {
    // The shared snapshot always keeps the full metadata: it is what
    // `get_playback_state` and the MPRIS bridge read.
    if let Ok(mut shared_snapshot) = playback_snapshot.lock() {
        *shared_snapshot = snapshot.clone();
    }

    // Strip the metadata from the twice-a-second tick unless the track changed.
    //
    // `now_playing` carries the album art as a base64 data URL — routinely
    // hundreds of kilobytes. Sending it on every tick meant serialising and
    // pushing the cover across the IPC bridge twice per second for as long as
    // music played, which the receiving side then re-parsed and wrote to disk.
    // The cover only ever changes when the track does.
    let mut event = snapshot;
    let identity = now_playing_identity(&event);
    if identity.is_some() && identity == *last_published_track {
        event.now_playing = None;
    } else {
        *last_published_track = identity;
    }

    let _ = app_handle.emit("audio-playback-progress", event);
}
