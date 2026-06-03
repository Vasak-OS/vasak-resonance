use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use rodio::buffer::SamplesBuffer;
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
    // cached decoded PCM samples for instant seeking
    cached_samples: Option<(Vec<f32>, u16, u32)>,
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
            current_metadata: None,
            cover_cache_by_path: HashMap::new(),
            dominant_color_cache_by_path: HashMap::new(),
            current_duration: None,
            started_at: None,
            paused_position: Duration::from_secs(0),
            is_paused: false,
            volume: 1.0,
            cached_samples: None,
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
        let (all_samples, channels, sample_rate) = Self::decode_file(&canonical_path)?;
        let total_dur = (all_samples.len() as u64) / (sample_rate as u64 * channels as u64);
        self.cached_samples = Some((all_samples.clone(), channels, sample_rate));

        let target = seek_to.unwrap_or(0).min(total_dur);
        let skip_samples = target * sample_rate as u64 * channels as u64;
        let offset = (skip_samples as usize).min(all_samples.len());

        let source = SamplesBuffer::new(channels, sample_rate, all_samples[offset..].to_vec());
        let new_sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("No se pudo crear sink: {e}"))?;
        new_sink.set_volume(self.volume);
        new_sink.append(source);
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
        self.current_duration = Some(Duration::from_secs(total_dur));
        self.started_at = Some(Instant::now());
        self.paused_position = Duration::from_secs(target);
        self.is_paused = false;

        Ok(())
    }

    fn decode_file(path: &Path) -> Result<(Vec<f32>, u16, u32), String> {
        let output = std::process::Command::new("ffmpeg")
            .args([
                "-v", "quiet",
                "-i", &path.to_string_lossy(),
                "-f", "wav",
                "pipe:1",
            ])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    "ffmpeg no está instalado. Instálelo para reproducir este formato (ej: sudo apt install ffmpeg)".to_string()
                } else {
                    format!("Error al ejecutar ffmpeg: {e}")
                }
            })?;

        if !output.status.success() || output.stdout.is_empty() {
            return Err(
                "ffmpeg no pudo decodificar el archivo. Puede estar corrupto o usar un códec no soportado."
                    .to_string(),
            );
        }

        let cursor = std::io::Cursor::new(output.stdout);
        let decoder = rodio::Decoder::new(cursor)
            .map_err(|e| format!("Error al leer salida de ffmpeg: {e}"))?;

        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();
        let samples: Vec<f32> = decoder.convert_samples().collect();

        if samples.is_empty() {
            return Err("No se decodificaron muestras de audio desde ffmpeg".to_string());
        }

        Ok((samples, channels, sample_rate))
    }

    fn play_stream_blocking(&mut self, url: &str, station_name: &str) -> Result<(), String> {
        self.streaming_active.store(true, Ordering::SeqCst);

        let mut child = std::process::Command::new("ffmpeg")
            .args(["-v", "quiet", "-i", url, "-f", "wav", "-acodec", "pcm_f32le", "-"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    "ffmpeg no está instalado. Instálelo para reproducir radio (ej: sudo apt install ffmpeg)".to_string()
                } else {
                    format!("Error al ejecutar ffmpeg: {e}")
                }
            })?;

        let stdout = child.stdout.take()
            .ok_or_else(|| "No se pudo obtener stdout de ffmpeg".to_string())?;
        let mut reader = std::io::BufReader::new(stdout);

        let (channels, sample_rate) = Self::read_wav_header(&mut reader)?;

        let (pcm_tx, pcm_rx) = mpsc::channel::<Vec<f32>>();

        let decoder_handle = thread::spawn(move || {
            let mut buf = [0u8; 65536];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let samples: Vec<f32> = buf[..n / 4 * 4]
                            .chunks_exact(4)
                            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                            .collect();
                        if pcm_tx.send(samples).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Rodio source for streaming PCM
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
                }
                let v = self.current[self.pos];
                self.pos += 1;
                Some(v)
            }
        }

        impl Source for StreamSource {
            fn current_frame_len(&self) -> Option<usize> { None }
            fn channels(&self) -> u16 { self.channels }
            fn sample_rate(&self) -> u32 { self.sample_rate }
            fn total_duration(&self) -> Option<Duration> { None }
        }

        let rodio_src = StreamSource {
            rx: pcm_rx,
            current: Vec::new(),
            pos: 0,
            channels,
            sample_rate,
        };

        let new_sink = Sink::try_new(&self.stream_handle).map_err(|e| format!("No se pudo crear sink: {e}"))?;
        new_sink.set_volume(self.volume);
        new_sink.append(rodio_src);
        new_sink.play();

        self.sink.stop();
        self.sink = new_sink;
        self.ffmpeg_process = Some(child);
        self.current_path = None;
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

        self.stream_writer_handle = None;
        self.stream_decoder_handle = Some(decoder_handle);

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
        self.cached_samples = None;
        self.started_at = None;
        self.paused_position = Duration::from_secs(0);
        self.is_paused = false;
        // Stop streaming threads if active
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
        Ok(())
    }

    fn seek(&mut self, second: u64) -> Result<(), String> {
        let (samples, channels, sample_rate) = self
            .cached_samples
            .as_ref()
            .ok_or_else(|| "No hay datos de audio en caché".to_string())?;

        let total_secs = samples.len() as u64 / (*sample_rate as u64 * *channels as u64);
        let target = second.min(total_secs);
        let skip_samples = target * *sample_rate as u64 * *channels as u64;
        let offset = (skip_samples as usize).min(samples.len());

        let source = SamplesBuffer::new(*channels, *sample_rate, samples[offset..].to_vec());
        let new_sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("No se pudo crear sink: {e}"))?;
        new_sink.set_volume(self.volume);
        new_sink.append(source);
        new_sink.play();

        if self.is_paused {
            new_sink.pause();
            self.started_at = None;
        } else {
            self.started_at = Some(Instant::now());
        }

        self.sink.stop();
        self.sink = new_sink;
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
                seek_to,
                respond_to,
            }) => {
                let _ = respond_to.send(manager.play_file(file_path, seek_to));
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
