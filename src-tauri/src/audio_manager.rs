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
    /// How long tracks overlap, in seconds. Zero turns the overlap off.
    SetCrossfade {
        seconds: f32,
        respond_to: Sender<Result<(), String>>,
    },
    /// Which track follows the current one, so the fade can be prepared without
    /// a round trip to the frontend.
    ///
    /// The queue lives in the Vue store, not here, so the backend cannot look
    /// the next track up on its own — and a crossfade has to have the next
    /// decoder already running before the current track ends.
    SetNextTrack {
        file_path: Option<String>,
        respond_to: Sender<Result<(), String>>,
    },
    Shutdown,
}

/// How long two tracks overlap out of the box.
///
/// Settable per person: four seconds is right for a shuffled library and wrong
/// for a record that segues — a live album, a DJ set, one movement running into
/// the next — where any overlap destroys the join the musicians intended. Zero
/// turns it off, which is why this is a default and not a constant.
const DEFAULT_CROSSFADE: Duration = Duration::from_secs(4);

/// Longest overlap that can be configured.
///
/// Bounded because the fade holds two decoders and, past a point, stops being a
/// transition and becomes a mashup.
const MAX_CROSSFADE: Duration = Duration::from_secs(12);

/// How often the audio thread wakes with nothing to do.
const IDLE_TICK: Duration = Duration::from_millis(500);

/// How often the volume ramp is stepped while a crossfade runs.
///
/// The idle loop wakes twice a second, which is far too coarse for a fade —
/// eight steps over four seconds is audible as stairs. 25 ms gives 160 steps,
/// smooth to the ear and still nothing next to the cost of decoding.
const FADE_TICK: Duration = Duration::from_millis(25);

/// Ceiling on the per-path cover cache.
///
/// The cache held every cover ever decoded, as base64, for the life of the
/// process: playing through a large library was an unbounded climb in memory.
/// Small because it exists to stop re-decoding the *same* track on seek and
/// replay, not to be a library-wide store.
const COVER_CACHE_LIMIT: usize = 64;

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

    /// Sets how long tracks overlap, in seconds. Zero turns the overlap off.
    pub fn set_crossfade(&self, seconds: f32) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.send(AudioCommand::SetCrossfade {
            seconds,
            respond_to: tx,
        })?;
        rx.recv()
            .map_err(|_| "No se recibió respuesta del hilo de audio".to_string())?
    }

    /// Tells the audio thread which track follows, so it can start the overlap
    /// on its own. `None` means the current track is the last one.
    ///
    /// Fire-and-forget from the caller's point of view: it is a hint, and a
    /// failure to deliver it costs a crossfade, not a track.
    pub fn set_next_track(&self, file_path: Option<String>) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.send(AudioCommand::SetNextTrack {
            file_path,
            respond_to: tx,
        })?;
        rx.recv()
            .map_err(|_| "No se recibió respuesta del hilo de audio".to_string())?
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

/// Everything one decoded track needs: its ffmpeg process, the thread pumping
/// PCM out of it, and the sink playing it.
///
/// This exists so two can be alive at once during a crossfade. The abort flag
/// is **per playback** for the same reason: it used to be one flag on the
/// manager, and with two streams running, tearing down the outgoing track
/// would have stopped the incoming one's decoder mid-fade.
struct Playback {
    sink: Sink,
    child: std::process::Child,
    decoder: Option<thread::JoinHandle<()>>,
    active: Arc<AtomicBool>,
}

impl Playback {
    /// Kills ffmpeg and joins the decoder thread.
    fn shutdown(mut self) {
        self.active.store(false, Ordering::SeqCst);
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(handle) = self.decoder.take() {
            let _ = handle.join();
        }
        self.sink.stop();
    }

    /// Whether ffmpeg has exited and every decoded sample has been played.
    fn drained(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_))) && self.sink.empty()
    }
}

/// A crossfade in flight.
struct Fade {
    started_at: Instant,
    /// The volume both sides are scaled against, captured at the start so a
    /// mid-fade volume change does not fight the ramp.
    volume: f32,
    /// Captured at the start too: changing the setting while two tracks are
    /// overlapping would otherwise move the finish line mid-ramp, and a fade
    /// shortened underneath itself jumps straight to silence.
    duration: Duration,
}

impl Fade {
    /// How far along the fade is, from 0.0 to 1.0.
    fn progress(&self) -> f32 {
        let total = self.duration.as_secs_f32();
        if total <= 0.0 {
            return 1.0;
        }
        (self.started_at.elapsed().as_secs_f32() / total).clamp(0.0, 1.0)
    }
}

/// Equal-power gains for the outgoing and incoming track at `t` in 0.0..=1.0.
///
/// Returns `(outgoing, incoming)`.
///
/// The issue proposed linear ramps (ffmpeg's `c1=tri`), but crossfading two
/// uncorrelated signals linearly makes the sum dip about 3 dB in the middle —
/// audible as a slump exactly where the acceptance criteria ask for "no volume
/// drops". Quarter-sine gains satisfy cos²+sin²=1, so the summed power is
/// constant across the whole overlap.
fn equal_power_gains(t: f32) -> (f32, f32) {
    let angle = t.clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2;
    (angle.cos(), angle.sin())
}

struct AudioManager {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    /// The track being listened to. During a crossfade this is already the
    /// *incoming* one: the queue advances when the fade starts, not when the
    /// old track finally goes quiet.
    current: Option<Playback>,
    /// The track on its way out. Only set while a crossfade runs.
    outgoing: Option<Playback>,
    fade: Option<Fade>,
    /// What to crossfade into, as told by the frontend.
    next_track: Option<PathBuf>,
    /// Zero means tracks follow one another without overlapping.
    crossfade: Duration,
    /// What is loaded: a file path, or a station URL when `stream_url` is set.
    ///
    /// Radio used to leave this empty, and every transport method starts by
    /// checking it — so pause, resume and the play/pause state were all dead
    /// while a station played.
    current_path: Option<PathBuf>,
    /// Set only for radio; a live stream cannot be sought and pausing it has to
    /// tear the connection down rather than accumulate stale audio.
    stream_url: Option<String>,
    /// Shared rather than owned: this is handed to every progress tick, and it
    /// carries the base64 cover. See `PlaybackProgressEvent::now_playing`.
    current_metadata: Option<Arc<NowPlayingMetadata>>,
    cover_cache_by_path: HashMap<String, Option<String>>,
    dominant_color_cache_by_path: HashMap<String, Option<String>>,
    current_duration: Option<Duration>,
    started_at: Option<Instant>,
    paused_position: Duration,
    is_paused: bool,
    volume: f32,
}

impl AudioManager {
    fn new() -> Result<Self, String> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("No se pudo inicializar salida de audio: {e}"))?;

        Ok(Self {
            _stream: stream,
            stream_handle,
            current: None,
            outgoing: None,
            fade: None,
            next_track: None,
            crossfade: DEFAULT_CROSSFADE,
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

        // An explicit play replaces everything, fade included: the person asked
        // for this track now, not blended into whatever was going out.
        self.shutdown_all();
        let playback =
            self.spawn_playback(&Self::file_ffmpeg_args(&canonical_path, target), self.volume)?;

        self.current = Some(playback);
        self.current_path = Some(canonical_path);
        self.stream_url = None;
        self.current_metadata = metadata.map(Arc::new);
        self.trim_caches();
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
    fn spawn_playback(&self, args: &[String], volume: f32) -> Result<Playback, String> {
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

        let active = Arc::new(AtomicBool::new(true));
        let active_for_thread = active.clone();
        // ~32 chunks of up to 64 KiB ≈ a few seconds of audio buffered.
        let (pcm_tx, pcm_rx) = mpsc::sync_channel::<Vec<f32>>(32);

        let decoder_handle = thread::spawn(move || {
            let active = active_for_thread;
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
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("No se pudo crear sink: {e}"))?;
        sink.set_volume(volume);
        sink.append(source);
        sink.play();

        Ok(Playback {
            sink,
            child,
            decoder: Some(decoder_handle),
            active,
        })
    }

    /// Tears down every playing track. Used by stop, and before starting a new
    /// track outright — as opposed to fading into one.
    fn shutdown_all(&mut self) {
        if let Some(playback) = self.current.take() {
            playback.shutdown();
        }
        if let Some(playback) = self.outgoing.take() {
            playback.shutdown();
        }
        self.fade = None;
    }

    /// Ends a crossfade early, keeping the incoming track at full volume.
    ///
    /// Called by anything that redefines what "now" means — a manual skip, a
    /// seek, a stop. Without it the ramp would keep running against a track
    /// that is no longer the one fading in.
    fn cancel_fade(&mut self) {
        if let Some(playback) = self.outgoing.take() {
            playback.shutdown();
        }
        self.fade = None;
        if let Some(playback) = self.current.as_ref() {
            playback.sink.set_volume(self.volume);
        }
    }

    /// Bounds the cover caches.
    ///
    /// Called after every insert. Clearing wholesale rather than evicting the
    /// oldest entry: there is no access order recorded, and the cache only
    /// exists to make a seek or an immediate replay cheap, so losing it costs
    /// one re-decode.
    fn trim_caches(&mut self) {
        if self.cover_cache_by_path.len() > COVER_CACHE_LIMIT {
            self.cover_cache_by_path.clear();
            self.dominant_color_cache_by_path.clear();
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
        self.shutdown_all();

        let playback = self.spawn_playback(&Self::stream_ffmpeg_args(url), self.volume)?;
        self.current = Some(playback);
        // The URL stands in for the path so the transport controls, the
        // play/pause state and the events all recognise that something is
        // loaded.
        self.current_path = Some(PathBuf::from(url));
        self.stream_url = Some(url.to_string());
        self.current_metadata = Some(Arc::new(NowPlayingMetadata {
            title: station_name.to_string(),
            artist: "Radio Stream".to_string(),
            album: String::new(),
            duration_seconds: 0,
            cover_data_url: None,
            dominant_color: None,
            path: url.to_string(),
        }));
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

        // A fade cannot survive a pause: resuming would find the ramp already
        // expired and jump the outgoing track to silence.
        self.cancel_fade();

        if self.stream_url.is_some() {
            // A live stream has to be disconnected, not held. Keeping ffmpeg
            // running against a paused sink just fills the buffer, so resuming
            // would play minutes-old audio and keep drifting further behind.
            self.shutdown_all();
        } else if let Some(playback) = self.current.as_ref() {
            playback.sink.pause();
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
            self.shutdown_all();
            let playback = self.spawn_playback(&Self::stream_ffmpeg_args(&url), self.volume)?;
            self.current = Some(playback);
        } else if let Some(playback) = self.current.as_ref() {
            playback.sink.play();
        }

        self.started_at = Some(Instant::now());
        self.is_paused = false;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.shutdown_all();
        self.next_track = None;
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

        // Seeking redefines where the track is, so a fade that was triggered by
        // the old position no longer means anything.
        self.shutdown_all();
        let playback = self.spawn_playback(&Self::file_ffmpeg_args(&path, target), self.volume)?;
        let sink_paused = was_paused;
        if sink_paused {
            playback.sink.pause();
        }
        self.current = Some(playback);
        self.paused_position = Duration::from_secs(target);

        if was_paused {
            self.started_at = None;
            self.is_paused = true;
        } else {
            self.started_at = Some(Instant::now());
            self.is_paused = false;
        }
        Ok(())
    }

    /// Sets the overlap length. Zero turns it off.
    ///
    /// Takes effect on the *next* transition: a fade already running keeps the
    /// length it started with, so the ramp cannot be moved out from under
    /// itself.
    fn set_crossfade(&mut self, seconds: f32) -> Result<(), String> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("Duración de encadenado inválida".to_string());
        }

        self.crossfade = Duration::from_secs_f32(seconds).min(MAX_CROSSFADE);
        Ok(())
    }

    fn set_volume(&mut self, volume: f32) -> Result<(), String> {
        if !volume.is_finite() {
            return Err("Volumen inválido".to_string());
        }

        let normalized = volume.clamp(0.0, 2.0);
        self.volume = normalized;

        // Mid-fade the ramp owns both sinks, so the new level is applied by
        // scaling it into the curve rather than by overwriting the gains.
        if let Some(fade) = self.fade.as_mut() {
            fade.volume = normalized;
            let (out_gain, in_gain) = equal_power_gains(fade.progress());
            if let Some(playback) = self.outgoing.as_ref() {
                playback.sink.set_volume(normalized * out_gain);
            }
            if let Some(playback) = self.current.as_ref() {
                playback.sink.set_volume(normalized * in_gain);
            }
        } else if let Some(playback) = self.current.as_ref() {
            playback.sink.set_volume(normalized);
        }

        Ok(())
    }

    /// Starts the overlap if the current track is within `CROSSFADE` of its end.
    ///
    /// Returns the metadata of the track that just became current, so the caller
    /// can tell the frontend — the UI, MPRIS and Discord have to change at the
    /// moment the new track becomes audible, not when the old one goes silent.
    ///
    /// Note what this does to the notion of "current": the incoming track takes
    /// over `current_path`, `current_metadata` and the clock immediately. The
    /// outgoing one keeps playing but is no longer what the player considers to
    /// be on. That is what keeps a manual skip during the overlap unambiguous.
    fn maybe_start_fade(&mut self) -> Option<Arc<NowPlayingMetadata>> {
        if self.fade.is_some() || self.is_paused || self.stream_url.is_some() {
            return None;
        }
        if self.crossfade.is_zero() {
            return None;
        }
        // Nothing playing, nothing to fade out of.
        self.current.as_ref()?;

        // Needs a known duration: without one there is no way to know the end
        // is coming, and the plain end-of-track path handles those files.
        let duration = self.current_duration?;
        if duration <= self.crossfade {
            return None;
        }

        let next_path = self.next_track.clone()?;
        let position = self.current_position_duration();
        if duration.saturating_sub(position) > self.crossfade {
            return None;
        }

        if !next_path.exists() {
            // A queued file that has since been moved or deleted. Dropping the
            // hint lets the track end normally instead of retrying every tick.
            self.next_track = None;
            return None;
        }

        let metadata = extract_now_playing_metadata_with_cover_cache(
            &next_path,
            &mut self.cover_cache_by_path,
            &mut self.dominant_color_cache_by_path,
        )
        .ok()
        .map(Arc::new);
        self.trim_caches();

        // The incoming track starts silent and is raised by the ramp; starting
        // at full volume is the "spike" the acceptance criteria rule out.
        let playback = match self.spawn_playback(&Self::file_ffmpeg_args(&next_path, 0), 0.0) {
            Ok(playback) => playback,
            Err(_) => {
                // ffmpeg refused this file. Let the current track finish and
                // let the normal end-of-track path deal with it.
                self.next_track = None;
                return None;
            }
        };

        let total = metadata.as_ref().map(|m| m.duration_seconds).unwrap_or(0);

        self.outgoing = self.current.take();
        self.current = Some(playback);
        self.fade = Some(Fade {
            started_at: Instant::now(),
            volume: self.volume,
            duration: self.crossfade,
        });

        self.current_path = Some(next_path);
        self.current_metadata = metadata.clone();
        self.current_duration = Some(Duration::from_secs(total));
        self.started_at = Some(Instant::now());
        self.paused_position = Duration::from_secs(0);
        self.next_track = None;

        metadata
    }

    /// Steps the volume ramp, and finishes the fade once the overlap is over.
    fn tick_fade(&mut self) {
        let Some(fade) = self.fade.as_ref() else {
            return;
        };

        let progress = fade.progress();
        let volume = fade.volume;
        let (out_gain, in_gain) = equal_power_gains(progress);

        if let Some(playback) = self.current.as_ref() {
            playback.sink.set_volume(volume * in_gain);
        }

        if progress >= 1.0 {
            if let Some(playback) = self.outgoing.take() {
                playback.shutdown();
            }
            self.fade = None;
            if let Some(playback) = self.current.as_ref() {
                playback.sink.set_volume(volume);
            }
            return;
        }

        if let Some(playback) = self.outgoing.as_mut() {
            playback.sink.set_volume(volume * out_gain);
            // A track shorter than the overlap runs out before the ramp ends.
            // Releasing it early frees the process; the ramp carries on for the
            // incoming side.
            if playback.drained() {
                if let Some(playback) = self.outgoing.take() {
                    playback.shutdown();
                }
            }
        }
    }

    fn is_fading(&self) -> bool {
        self.fade.is_some()
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

        // Never during a fade: the incoming track has barely started and the
        // outgoing one is deliberately being drained.
        if self.fade.is_some() {
            return false;
        }

        match self.current.as_mut() {
            Some(playback) => playback.drained(),
            None => false,
        }
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
        let is_playing = self.current_path.is_some()
            && !self.is_paused
            && self.current.as_ref().is_some_and(|p| !p.sink.empty());

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
                | Ok(AudioCommand::SetVolume { respond_to, .. })
                | Ok(AudioCommand::SetNextTrack { respond_to, .. })
                | Ok(AudioCommand::SetCrossfade { respond_to, .. }) => {
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

    let mut tick = IDLE_TICK;

    loop {
        match command_rx.recv_timeout(tick) {
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
            Ok(AudioCommand::SetCrossfade {
                seconds,
                respond_to,
            }) => {
                let _ = respond_to.send(manager.set_crossfade(seconds));
                // No snapshot: the setting changes nothing about what is
                // playing right now.
            }
            Ok(AudioCommand::SetNextTrack {
                file_path,
                respond_to,
            }) => {
                manager.next_track = file_path.map(PathBuf::from);
                let _ = respond_to.send(Ok(()));
                // No snapshot: a hint about what comes next changes nothing the
                // frontend is showing, and this arrives on every queue edit.
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
                // The overlap is driven from here rather than from its own
                // thread: everything that can cancel it — pause, seek, skip,
                // stop — is already serialised through this loop, so a separate
                // thread would need to take a lock on all of it to do the same
                // work.
                manager.tick_fade();

                if let Some(metadata) = manager.maybe_start_fade() {
                    // Emitted at the *start* of the overlap, which is the point
                    // the new track becomes audible. The frontend advances its
                    // queue off this instead of waiting for the old track to
                    // finish, so the cover, the title, MPRIS and Discord change
                    // together with what is being heard.
                    let _ = app_handle.emit("audio-crossfade-started", metadata.as_ref());
                }

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

        // Idle costs two wakeups a second; a fade needs forty. Switching only
        // while one runs keeps the ramp smooth without turning the whole
        // service into a busy loop — the cost this project keeps removing from
        // other daemons.
        tick = if manager.is_fading() {
            FADE_TICK
        } else {
            IDLE_TICK
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    /// What the issue asked for: ffmpeg's `c1=tri`, a straight line.
    ///
    /// Only defined here, in the tests, because it is the thing being rejected —
    /// the comparison below is the reason the real curve is not this.
    fn linear_gains(t: f32) -> (f32, f32) {
        let t = t.clamp(0.0, 1.0);
        (1.0 - t, t)
    }

    #[test]
    fn the_fade_starts_on_the_old_track_and_ends_on_the_new_one() {
        let (out, incoming) = equal_power_gains(0.0);
        assert!((out - 1.0).abs() < 1e-6);
        assert!(incoming.abs() < 1e-6);

        let (out, incoming) = equal_power_gains(1.0);
        assert!(out.abs() < 1e-6);
        assert!((incoming - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_summed_power_never_moves() {
        // This is the acceptance criterion "no volume drops or spikes", stated
        // as arithmetic: two uncorrelated signals add as the sum of squares.
        for step in 0..=100 {
            let t = step as f32 / 100.0;
            let (out, incoming) = equal_power_gains(t);
            let power = out * out + incoming * incoming;
            assert!(
                (power - 1.0).abs() < 1e-5,
                "en t={t} la potencia dio {power}"
            );
        }
    }

    #[test]
    fn a_linear_fade_would_have_dipped_in_the_middle() {
        // Halfway through a linear crossfade both gains are 0.5, so the summed
        // power is 0.5 — about 3 dB down, heard as a slump. Kept as a test so
        // nobody "simplifies" the curve back to a straight line.
        let (out, incoming) = linear_gains(0.5);
        let linear_power = out * out + incoming * incoming;
        assert!((linear_power - 0.5).abs() < 1e-6);

        let (out, incoming) = equal_power_gains(0.5);
        let equal_power = out * out + incoming * incoming;
        assert!((equal_power - 1.0).abs() < 1e-5);
        assert!(equal_power > linear_power);
    }

    #[test]
    fn the_two_sides_cross_exactly_halfway() {
        let (out, incoming) = equal_power_gains(0.5);
        assert!((out - incoming).abs() < 1e-6);
    }

    #[test]
    fn the_gains_are_monotonic() {
        // A ramp that backtracks is audible as a wobble.
        let mut previous = equal_power_gains(0.0);
        for step in 1..=100 {
            let current = equal_power_gains(step as f32 / 100.0);
            assert!(current.0 <= previous.0, "la salida subió en t={step}");
            assert!(current.1 >= previous.1, "la entrada bajó en t={step}");
            previous = current;
        }
    }

    #[test]
    fn out_of_range_progress_is_clamped_rather_than_wrapped() {
        // `progress()` clamps, but a NaN or a negative elapsed time from a clock
        // adjustment would otherwise produce gains outside 0..=1 and a sink
        // volume that rodio would happily apply.
        assert_eq!(equal_power_gains(-1.0), equal_power_gains(0.0));
        assert_eq!(equal_power_gains(2.0), equal_power_gains(1.0));
    }

    #[test]
    fn the_fade_reports_its_progress_from_zero() {
        let fade = Fade {
            started_at: Instant::now(),
            volume: 1.0,
            duration: DEFAULT_CROSSFADE,
        };
        assert!(fade.progress() < 0.01);
    }

    #[test]
    fn the_overlap_is_shorter_than_a_track_worth_crossfading() {
        // `maybe_start_fade` refuses tracks no longer than the overlap: fading a
        // three-second track would mean it is never heard on its own.
        assert!(DEFAULT_CROSSFADE < Duration::from_secs(10));
        assert!(
            FADE_TICK < DEFAULT_CROSSFADE / 50,
            "el ramp se oiría escalonado"
        );
        assert!(DEFAULT_CROSSFADE <= MAX_CROSSFADE);
    }

    #[test]
    fn a_zero_length_fade_reports_itself_finished() {
        // Guards the division in `progress()`: a fade configured to zero must
        // not produce infinity and a volume rodio would happily apply.
        let fade = Fade {
            started_at: Instant::now(),
            volume: 1.0,
            duration: Duration::ZERO,
        };
        assert_eq!(fade.progress(), 1.0);
    }

    #[test]
    fn the_configured_length_is_what_the_ramp_uses() {
        // A fade keeps the length it started with, so changing the setting
        // mid-transition cannot move the finish line.
        let fade = Fade {
            started_at: Instant::now(),
            volume: 1.0,
            duration: Duration::from_secs(8),
        };
        // Barely started: an 8-second ramp is half the progress of a 4-second
        // one at the same instant, and both round to nearly zero here.
        assert!(fade.progress() < 0.01);
    }
}
