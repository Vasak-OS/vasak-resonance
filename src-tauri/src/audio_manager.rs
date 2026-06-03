use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use rodio::buffer::SamplesBuffer;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Condvar, Mutex};
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
            streaming_active: Arc::new(AtomicBool::new(false)),
        })
    }

    fn play_file(&mut self, file_path: String, seek_to: Option<u64>) -> Result<(), String> {
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
        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();

        // Decode and cache all PCM samples for instant seeking
        let all_samples: Vec<f32> = decoder.convert_samples().collect();
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

    fn play_stream_blocking(&mut self, url: &str, station_name: &str) -> Result<(), String> {
        // New approach: streaming decode using Symphonia + a blocking shared buffer.
        use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
        use symphonia::core::codecs::DecoderOptions;
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;
        use symphonia::default::{get_codecs, get_probe};

        let client = reqwest::blocking::Client::new();
        let mut resp = client
            .get(url)
            .header("User-Agent", "vasak-resonance/1.0")
            .send()
            .map_err(|e| format!("Error connecting to stream: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Stream returned status: {}", resp.status()));
        }

        let buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let condvar = Arc::new(Condvar::new());
        let eof = Arc::new(AtomicBool::new(false));
        self.streaming_active.store(true, Ordering::SeqCst);

        // Writer thread
        let buffer_writer = Arc::clone(&buffer);
        let condvar_writer = Arc::clone(&condvar);
        let eof_writer = Arc::clone(&eof);
        let mut resp_for_writer = resp;
        let writer_handle = thread::spawn(move || {
            let mut tmp = [0u8; 8192];
            loop {
                match resp_for_writer.read(&mut tmp) {
                    Ok(0) => {
                        eof_writer.store(true, Ordering::SeqCst);
                        condvar_writer.notify_all();
                        break;
                    }
                    Ok(n) => {
                        {
                            let mut guard = buffer_writer.lock().unwrap();
                            guard.extend_from_slice(&tmp[..n]);
                        }
                        condvar_writer.notify_all();
                    }
                    Err(_) => {
                        eof_writer.store(true, Ordering::SeqCst);
                        condvar_writer.notify_all();
                        break;
                    }
                }
            }
        });

        // StreamingSource that implements Read + Seek
        struct StreamingSource {
            buf: Arc<Mutex<Vec<u8>>>,
            condvar: Arc<Condvar>,
            eof: Arc<AtomicBool>,
            pos: u64,
        }

        impl Read for StreamingSource {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                let mut guard = self.buf.lock().unwrap();
                loop {
                    let available = guard.len() as i64 - self.pos as i64;
                    if available > 0 {
                        let n = std::cmp::min(available as usize, out.len());
                        let start = self.pos as usize;
                        out[..n].copy_from_slice(&guard[start..start + n]);
                        self.pos += n as u64;
                        return Ok(n);
                    }
                    if self.eof.load(Ordering::SeqCst) {
                        return Ok(0);
                    }
                    guard = self.condvar.wait(guard).unwrap();
                }
            }
        }

        impl Seek for StreamingSource {
            fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
                let mut guard = self.buf.lock().unwrap();
                match pos {
                    SeekFrom::Start(off) => {
                        while guard.len() as u64 <= off && !self.eof.load(Ordering::SeqCst) {
                            guard = self.condvar.wait(guard).unwrap();
                        }
                        let max = guard.len() as u64;
                        self.pos = std::cmp::min(off, max);
                        Ok(self.pos)
                    }
                    SeekFrom::Current(off) => {
                        let target = if off < 0 {
                            self.pos.saturating_sub((-off) as u64)
                        } else {
                            self.pos.saturating_add(off as u64)
                        };
                        while guard.len() as u64 <= target && !self.eof.load(Ordering::SeqCst) {
                            guard = self.condvar.wait(guard).unwrap();
                        }
                        let max = guard.len() as u64;
                        self.pos = std::cmp::min(target, max);
                        Ok(self.pos)
                    }
                    SeekFrom::End(off) => {
                        while !self.eof.load(Ordering::SeqCst) {
                            guard = self.condvar.wait(guard).unwrap();
                        }
                        let len = guard.len() as i64;
                        let target = len + off;
                        let target_u = if target < 0 { 0 } else { target as u64 };
                        self.pos = std::cmp::min(target_u, guard.len() as u64);
                        Ok(self.pos)
                    }
                }
            }
        }

        let streaming_source = StreamingSource {
            buf: Arc::clone(&buffer),
            condvar: Arc::clone(&condvar),
            eof: Arc::clone(&eof),
            pos: 0,
        };

        use symphonia::core::io::ReadOnlySource;
        let mss = MediaSourceStream::new(Box::new(ReadOnlySource::new(streaming_source)), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = url.split('.').last() {
            hint.with_extension(ext);
        }

        let probed = get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| format!("Error probing stream format: {}", e))?;
        let mut format = probed.format;

        let track = format
            .tracks()
            .get(0)
            .ok_or_else(|| "No se encontró pista de audio en el stream".to_string())?;

        // Clone codec params to create decoder inside the decoder thread and to
        // read channels/sample_rate here for rodio source.
        let codec_params = track.codec_params.clone();
        let channels = codec_params.channels.as_ref().map(|c| c.count() as u16).unwrap_or(2);
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);

        // Channel for PCM frames
        let (pcm_tx, pcm_rx) = mpsc::channel::<Vec<f32>>();

        use symphonia::core::audio::Signal;
        // Decoder thread: create decoder inside the thread using cloned codec_params
        let decoder_handle = thread::spawn(move || {
            let mut decoder = match get_codecs().make(&codec_params, &DecoderOptions::default()) {
                Ok(d) => d,
                Err(_) => return,
            };
            loop {
                match format.next_packet() {
                    Ok(packet) => match decoder.decode(&packet) {
                        Ok(audio_buf) => {
                            let frames = audio_buf.frames();
                            let spec = audio_buf.spec().clone();
                            let mut sb = SampleBuffer::<f32>::new(frames as u64, spec);
                            sb.copy_interleaved_ref(audio_buf);
                            let out = sb.samples().to_vec();
                            let _ = pcm_tx.send(out);
                        }
                        Err(_) => break,
                    },
                    Err(_) => break,
                }
            }
        });

        // Rodio source
        struct RodioSymphoniaSource {
            rx: mpsc::Receiver<Vec<f32>>,
            current: Vec<f32>,
            pos: usize,
            channels: u16,
            sample_rate: u32,
        }

        impl Iterator for RodioSymphoniaSource {
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

        impl Source for RodioSymphoniaSource {
            fn current_frame_len(&self) -> Option<usize> { None }
            fn channels(&self) -> u16 { self.channels }
            fn sample_rate(&self) -> u32 { self.sample_rate }
            fn total_duration(&self) -> Option<Duration> { None }
        }

        

        let rodio_src = RodioSymphoniaSource {
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

        self.stream_writer_handle = Some(writer_handle);
        self.stream_decoder_handle = Some(decoder_handle);

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
        self.cached_samples = None;
        self.started_at = None;
        self.paused_position = Duration::from_secs(0);
        self.is_paused = false;
        // Stop streaming threads if active
        self.streaming_active.store(false, Ordering::SeqCst);
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
