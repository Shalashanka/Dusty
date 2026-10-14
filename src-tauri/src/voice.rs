//! Voice search, fully on-device: microphone -> speech model -> query.
//!
//! Engines run through sherpa-onnx (ONNX Runtime, CPU):
//! - Moonshine tiny (English): built for short commands, near-instant.
//! - NVIDIA Parakeet TDT v3: 25 European languages incl. Italian, very accurate.
//!
//! Nothing leaves the computer. A model is downloaded once, only when the
//! user turns voice on.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use serde::Serialize;
use sherpa_onnx::{OfflineMoonshineModelConfig, OfflineRecognizer, OfflineRecognizerConfig, OfflineTransducerModelConfig};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub struct ModelInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    /// Folder inside the archive.
    pub dir: &'static str,
    pub mb: u32,
    pub languages: &'static str,
}

const BASE: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/";

pub const MODELS: &[ModelInfo] = &[
    ModelInfo {
        id: "moonshine",
        name: "Moonshine (fast, English)",
        url: "sherpa-onnx-moonshine-tiny-en-int8.tar.bz2",
        dir: "sherpa-onnx-moonshine-tiny-en-int8",
        mb: 103,
        languages: "English",
    },
    ModelInfo {
        id: "parakeet",
        name: "Parakeet v3 (25 languages)",
        url: "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2",
        dir: "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
        mb: 640,
        languages: "English, Italian, Spanish, French, German and 20 more",
    },
];

pub fn model_info(id: &str) -> &'static ModelInfo {
    MODELS.iter().find(|m| m.id == id).unwrap_or(&MODELS[0])
}

struct Engine(OfflineRecognizer);

impl Engine {
    fn transcribe(&mut self, audio: &[f32]) -> String {
        let stream = self.0.create_stream();
        stream.accept_waveform(16_000, audio);
        self.0.decode(&stream);
        stream.get_result().map(|r| r.text).unwrap_or_default()
    }
}

#[derive(Default)]
pub struct VoiceState {
    engine: Mutex<Option<(String, Engine)>>,
    pub listening: AtomicBool,
    stop: AtomicBool,
    pub downloading: AtomicBool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VoiceStatus {
    pub model: String,
    pub installed: bool,
    pub size_mb: u32,
    pub downloading: bool,
    pub listening: bool,
    pub has_mic: bool,
    pub mic_hint: String,
    pub models: Vec<VoiceModel>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VoiceModel {
    pub id: String,
    pub name: String,
    pub size_mb: u32,
    pub languages: String,
    pub installed: bool,
}

pub fn models_dir(data: &Path) -> PathBuf {
    data.join("models")
}

fn model_dir(data: &Path, m: &ModelInfo) -> PathBuf {
    models_dir(data).join(m.dir)
}

fn installed(data: &Path, m: &ModelInfo) -> bool {
    model_dir(data, m).join("tokens.txt").is_file()
}

pub fn status(data: &Path, state: &VoiceState, model: &str) -> VoiceStatus {
    let m = model_info(model);
    VoiceStatus {
        model: m.id.into(),
        installed: installed(data, m),
        size_mb: m.mb,
        downloading: state.downloading.load(Ordering::Relaxed),
        listening: state.listening.load(Ordering::Relaxed),
        has_mic: cpal::default_host().default_input_device().is_some(),
        mic_hint: if cpal::default_host().default_input_device().is_some() { String::new() } else { no_mic_message() },
        models: MODELS
            .iter()
            .map(|x| VoiceModel {
                id: x.id.into(),
                name: x.name.into(),
                size_mb: x.mb,
                languages: x.languages.into(),
                installed: installed(data, x),
            })
            .collect(),
    }
}

/// Downloads and unpacks a model with progress events ("voice-download": 0..1).
pub fn download(app: &AppHandle, data: &Path, state: &VoiceState, model: &str) -> Result<(), String> {
    if state.downloading.swap(true, Ordering::SeqCst) {
        return Err("already downloading".into());
    }
    let m = model_info(model);
    let r = fetch_archive(app, &models_dir(data), &format!("{BASE}{}", m.url), m.mb, "voice-download")
        .and_then(|_| if installed(data, m) { Ok(()) } else { Err("the model archive looked different than expected".into()) });
    state.downloading.store(false, Ordering::SeqCst);
    r
}

/// Streams a `.tar.bz2` model archive into `dir` and unpacks it, emitting
/// `event` with progress 0..1. Shared by speech recognition and Dusty's voices.
pub fn fetch_archive(app: &AppHandle, dir: &Path, url: &str, mb: u32, event: &str) -> Result<(), String> {
    use std::io::{Read, Write};
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let name = url.rsplit('/').next().unwrap_or("model.tar.bz2");
    let archive = dir.join(name);
    let resp = ureq::get(url).call().map_err(|e| format!("download failed: {e}"))?;
    let total = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(mb as u64 * 1_048_576);
    let mut reader = resp.into_body().into_with_config().limit(4 << 30).reader();
    let mut out = std::fs::File::create(&archive).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 1 << 16];
    let mut done = 0u64;
    let mut last = Instant::now();
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if last.elapsed() > Duration::from_millis(150) {
            let _ = app.emit(event, (done as f64 / total as f64).min(0.97));
            last = Instant::now();
        }
    }
    drop(out);
    let _ = app.emit(event, 0.98);
    let f = std::fs::File::open(&archive).map_err(|e| e.to_string())?;
    let unpacked = tar::Archive::new(bzip2::read::BzDecoder::new(std::io::BufReader::new(f)))
        .unpack(dir)
        .map_err(|e| format!("couldn't unpack the model: {e}"));
    let _ = std::fs::remove_file(&archive);
    unpacked?;
    let _ = app.emit(event, 1.0);
    Ok(())
}

/// First file in `dir` whose name starts with `prefix` (prefers int8 builds).
pub(crate) fn find(dir: &Path, prefix: &str) -> Result<String, String> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            n.starts_with(prefix) && n.ends_with(".onnx")
        })
        .collect();
    hits.sort_by_key(|p| !p.to_string_lossy().contains("int8"));
    hits.first().map(|p| p.to_string_lossy().into_owned()).ok_or(format!("{prefix}*.onnx missing"))
}

fn load(data: &Path, m: &ModelInfo) -> Result<Engine, String> {
    let dir = model_dir(data, m);
    if !installed(data, m) {
        return Err("the voice model isn't downloaded yet".into());
    }
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 8) as i32;
    let mut config = OfflineRecognizerConfig::default();
    config.model_config.tokens = Some(dir.join("tokens.txt").to_string_lossy().into_owned());
    config.model_config.num_threads = threads;
    match m.id {
        "moonshine" => {
            config.model_config.moonshine = OfflineMoonshineModelConfig {
                preprocessor: Some(find(&dir, "preprocess")?),
                encoder: Some(find(&dir, "encode")?),
                uncached_decoder: Some(find(&dir, "uncached_decode")?),
                cached_decoder: Some(find(&dir, "cached_decode")?),
                merged_decoder: None,
            };
        }
        _ => {
            config.model_config.transducer = OfflineTransducerModelConfig {
                encoder: Some(find(&dir, "encoder")?),
                decoder: Some(find(&dir, "decoder")?),
                joiner: Some(find(&dir, "joiner")?),
            };
            config.model_config.model_type = Some("nemo_transducer".into());
            config.decoding_method = Some("greedy_search".into());
        }
    }
    OfflineRecognizer::create(&config).map(Engine).ok_or_else(|| "couldn't load the voice model".to_string())
}

fn transcribe_with(data: &Path, state: &VoiceState, model: &str, audio: &[f32]) -> Result<String, String> {
    let m = model_info(model);
    let mut guard = state.engine.lock();
    if guard.as_ref().map(|(id, _)| id != m.id).unwrap_or(true) {
        *guard = Some((m.id.to_string(), load(data, m)?));
    }
    let (_, engine) = guard.as_mut().unwrap();
    // Short silence on both ends helps the models settle.
    let mut padded = vec![0.0f32; 4800];
    padded.extend_from_slice(audio);
    padded.extend(std::iter::repeat_n(0.0, 8000));
    Ok(clean(&engine.transcribe(&padded)))
}

/// Loads the model in the background so the first query is instant.
pub fn warm_up(app: AppHandle) {
    std::thread::spawn(move || {
        let state = app.state::<crate::state::AppState>();
        let (enabled, model) = {
            let s = state.settings.read();
            (s.voice_enabled, s.voice_model.clone())
        };
        if !enabled {
            return;
        }
        let m = model_info(&model);
        let mut guard = state.voice.engine.lock();
        if guard.is_none() && installed(&state.data_dir, m) {
            if let Ok(e) = load(&state.data_dir, m) {
                *guard = Some((m.id.to_string(), e));
            }
        }
    });
}

fn clean(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").trim_end_matches(['.', '?', '!']).trim().to_string()
}

/// Stops an ongoing recording early (user pressed the hotkey again).
pub fn stop(state: &VoiceState) {
    state.stop.store(true, Ordering::SeqCst);
}

/// Records until the speaker goes quiet, then transcribes. Emits:
/// "voice-level" (0..1, ~25/s), "voice-state" ("listening"|"thinking"|"idle"),
/// "voice-partial" (live transcript so far), "voice-text" (final transcript) or "voice-error".
pub fn listen(app: AppHandle, model: String) {
    let state = app.state::<crate::state::AppState>();
    if state.voice.listening.swap(true, Ordering::SeqCst) {
        stop(&state.voice);
        return;
    }
    state.voice.stop.store(false, Ordering::SeqCst);
    let data = state.data_dir.clone();
    let app2 = app.clone();
    std::thread::spawn(move || {
        let st = app2.state::<crate::state::AppState>();
        let result = record(&app2, &st.voice, &data, &model).and_then(|(audio, rate)| {
            let _ = app2.emit("voice-state", "thinking");
            let t = Instant::now();
            let text = transcribe_with(&data, &st.voice, &model, &resample(&audio, rate, 16_000));
            eprintln!("voice: transcribed in {:?}", t.elapsed());
            text
        });
        st.voice.listening.store(false, Ordering::SeqCst);
        match result {
            Ok(text) if !text.is_empty() => {
                let _ = app2.emit("voice-text", text);
            }
            Ok(_) => {
                let _ = app2.emit("voice-error", "I didn't catch that. Try again?");
            }
            Err(e) => {
                let _ = app2.emit("voice-error", e);
            }
        }
        let _ = app2.emit("voice-state", "idle");
    });
}

/// Why there's no microphone, in words the user can act on.
fn no_mic_message() -> String {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_REMOTESESSION};
    if unsafe { GetSystemMetrics(SM_REMOTESESSION) } != 0 {
        "No microphone in this Remote Desktop session. In Remote Desktop: Show options › Local resources ›          Remote audio › Settings › \"Record from this computer\", then reconnect."
            .into()
    } else {
        "No microphone found. Plug one in, or check Settings › Privacy › Microphone.".into()
    }
}

fn record(app: &AppHandle, voice: &VoiceState, data: &Path, model: &str) -> Result<(Vec<f32>, u32), String> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(no_mic_message)?;
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let rate = supported.sample_rate();
    let channels = supported.channels() as usize;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let level: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));
    let push = {
        let samples = samples.clone();
        let level = level.clone();
        move |mono: &mut dyn Iterator<Item = f32>| {
            let mut s = samples.lock();
            let mut sum = 0.0f32;
            let mut n = 0usize;
            for v in mono {
                s.push(v);
                sum += v * v;
                n += 1;
            }
            if n > 0 {
                *level.lock() = (sum / n as f32).sqrt();
            }
        }
    };
    let err = |e: cpal::Error| eprintln!("mic error: {e}");
    let stream = match format {
        cpal::SampleFormat::F32 => {
            let push = push.clone();
            device.build_input_stream(
                config.clone(),
                move |d: &[f32], _: &cpal::InputCallbackInfo| {
                    push(&mut d.chunks(channels).map(|c| c.iter().sum::<f32>() / channels as f32))
                },
                err,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let push = push.clone();
            device.build_input_stream(
                config.clone(),
                move |d: &[i16], _: &cpal::InputCallbackInfo| {
                    push(&mut d.chunks(channels).map(|c| c.iter().map(|v| *v as f32 / 32768.0).sum::<f32>() / channels as f32))
                },
                err,
                None,
            )
        }
        other => return Err(format!("unsupported microphone format {other:?}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    let _ = app.emit("voice-state", "listening");

    // Live transcript: while speaking, the audio so far is re-transcribed about
    // twice a second on a helper thread and shown word by word.
    let partial_busy = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let mut last_partial = Instant::now();
    let mut partial_len = 0usize;

    // Endpointing: wait for speech, then stop after ~0.9 s of quiet.
    let started = Instant::now();
    let mut noise_floor = 0.004f32;
    let mut heard = false;
    let mut quiet_since: Option<Instant> = None;
    loop {
        std::thread::sleep(Duration::from_millis(40));
        let l = *level.lock();
        let _ = app.emit("voice-level", (l * 9.0).min(1.0));
        if started.elapsed() < Duration::from_millis(250) {
            noise_floor = noise_floor.max(l * 1.2);
            continue;
        }
        let speaking = l > (noise_floor * 2.2).max(0.012);
        if speaking {
            heard = true;
            quiet_since = None;
        } else if heard {
            let q = *quiet_since.get_or_insert_with(Instant::now);
            if q.elapsed() > Duration::from_millis(900) {
                break;
            }
        }
        let len = samples.lock().len();
        if heard
            && last_partial.elapsed() > Duration::from_millis(450)
            && len > partial_len + rate as usize / 5
            && !partial_busy.swap(true, Ordering::SeqCst)
        {
            last_partial = Instant::now();
            partial_len = len;
            let audio = resample(&samples.lock()[..len], rate, 16_000);
            let (app, data, model) = (app.clone(), data.to_path_buf(), model.to_string());
            let (busy, finished) = (partial_busy.clone(), finished.clone());
            std::thread::spawn(move || {
                let st = app.state::<crate::state::AppState>();
                if let Ok(text) = transcribe_with(&data, &st.voice, &model, &audio) {
                    if !finished.load(Ordering::SeqCst) && !text.is_empty() {
                        let _ = app.emit("voice-partial", text);
                    }
                }
                busy.store(false, Ordering::SeqCst);
            });
        }
        if voice.stop.load(Ordering::SeqCst)
            || started.elapsed() > Duration::from_secs(15)
            || (!heard && started.elapsed() > Duration::from_secs(6))
        {
            break;
        }
    }
    drop(stream);
    finished.store(true, Ordering::SeqCst);
    let audio = std::mem::take(&mut *samples.lock());
    if !heard && !voice.stop.load(Ordering::SeqCst) {
        return Err("I didn't hear anything. Is the microphone on?".into());
    }
    Ok((audio, rate))
}

/// Linear resampling to 16 kHz.
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let n = (input.len() as f64 / ratio) as usize;
    (0..n)
        .map(|i| {
            let pos = i as f64 * ratio;
            let j = pos as usize;
            let frac = (pos - j as f64) as f32;
            let a = input[j.min(input.len() - 1)];
            let b = input[(j + 1).min(input.len() - 1)];
            a + (b - a) * frac
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Transcribes WAV files listed in DUSTY_WAVS (comma separated) with Parakeet:
    /// `cargo test --release hear_wavs -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn hear_wavs() {
        let data = dirs::data_local_dir().unwrap().join("com.griz.dusty");
        let state = VoiceState::default();
        for f in std::env::var("DUSTY_WAVS").unwrap().split(',') {
            let w = sherpa_onnx::Wave::read(f).expect("wav");
            let pcm = resample(w.samples(), w.sample_rate() as u32, 16_000);
            println!("{f}: {:?}", transcribe_with(&data, &state, "parakeet", &pcm));
        }
    }

    /// `cargo test mics -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn mics() {
        let host = cpal::default_host();
        println!("host {:?}", host.id());
        println!("default input: {:?}", host.default_input_device().map(|d| d.description().map(|x| format!("{x:?}"))));
        for d in host.input_devices().unwrap() {
            println!("input: {:?} -> {:?}", d.description().map(|x| format!("{x:?}")), d.default_input_config().map(|c| (c.sample_rate(), c.channels())));
        }
    }

    fn read_wav(path: &Path) -> (Vec<f32>, u32) {
        let b = std::fs::read(path).unwrap();
        let channels = u16::from_le_bytes([b[22], b[23]]) as usize;
        let rate = u32::from_le_bytes([b[24], b[25], b[26], b[27]]);
        let bits = u16::from_le_bytes([b[34], b[35]]);
        assert_eq!(bits, 16, "test expects 16-bit PCM");
        let data = b.windows(4).position(|w| w == b"data").unwrap() + 8;
        let samples = b[data..]
            .chunks_exact(2 * channels)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
            .collect();
        (samples, rate)
    }

    fn say(text: &str, voice_lang: &str) -> (Vec<f32>, u32) {
        let wav = std::env::temp_dir().join(format!("dusty-voice-{voice_lang}.wav"));
        let text = text.replace('\'', "''");
        let script = format!(
            "Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
             $v = $s.GetInstalledVoices() | Where-Object {{ $_.VoiceInfo.Culture.Name -like '{voice_lang}*' }} | Select-Object -First 1; \
             if ($v) {{ $s.SelectVoice($v.VoiceInfo.Name) }}; \
             $fmt = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono); \
             $s.SetOutputToWaveFile('{}', $fmt); $s.Speak('{text}'); $s.Dispose()",
            wav.display()
        );
        crate::winsys::powershell(&script).unwrap();
        read_wav(&wav)
    }

    /// Downloads models on first run:
    /// `cargo test --release real_voice -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_voice() {
        let data = dirs::data_local_dir().unwrap().join("com.griz.dusty");
        let state = VoiceState::default();
        for (model, lang, phrase) in [
            ("moonshine", "en", "find the invoice pdf I downloaded about five months ago"),
            ("parakeet", "en", "show me big videos on the d drive"),
            ("parakeet", "it", "trova le foto delle vacanze dell'anno scorso"),
        ] {
            let m = model_info(model);
            if !installed(&data, m) {
                eprintln!("downloading {model}...");
                let dir = models_dir(&data);
                std::fs::create_dir_all(&dir).unwrap();
                let archive = dir.join(m.url);
                let mut r = ureq::get(&format!("{BASE}{}", m.url)).call().unwrap().into_body().into_with_config().limit(4 << 30).reader();
                std::io::copy(&mut r, &mut std::fs::File::create(&archive).unwrap()).unwrap();
                let f = std::fs::File::open(&archive).unwrap();
                tar::Archive::new(bzip2::read::BzDecoder::new(std::io::BufReader::new(f))).unpack(&dir).unwrap();
                std::fs::remove_file(&archive).unwrap();
            }
            let (audio, rate) = say(phrase, lang);
            let t = Instant::now();
            let _ = transcribe_with(&data, &state, model, &resample(&audio[..1600.min(audio.len())], rate, 16_000));
            let load = t.elapsed();
            let t = Instant::now();
            let text = transcribe_with(&data, &state, model, &resample(&audio, rate, 16_000)).unwrap();
            eprintln!(
                "{model} [{lang}] load+warm {load:?}, transcribe {:?} for {:.1}s: {text:?} -> {:?}",
                t.elapsed(),
                audio.len() as f32 / rate as f32,
                crate::nlq::parse(&text).explain
            );
        }
    }
}
