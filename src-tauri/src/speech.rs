//! Dusty's own voice: small neural text-to-speech models, fully on-device.
//!
//! Three voices to choose from (downloaded once, only if the user wants them):
//! - Supertonic 3: lightning fast, natural, 31 languages incl. Italian (MIT).
//! - Kokoro: the most natural English of the small models (Apache-2.0).
//! - Kitten nano: tiny (~30 MB) and quick, English (Apache-2.0).
//!
//! All run through sherpa-onnx on the CPU. Speech is only ever a short reply
//! to a voice query and is off by default.

use crate::voice;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use serde::Serialize;
use sherpa_onnx::{
    GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsKittenModelConfig, OfflineTtsKokoroModelConfig, OfflineTtsModelConfig,
    OfflineTtsSupertonicModelConfig,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const BASE: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/";

pub struct TtsInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub blurb: &'static str,
    pub archive: &'static str,
    pub dir: &'static str,
    pub mb: u32,
    pub languages: &'static str,
    /// Speaker used unless the user picks another one.
    pub default_speaker: i32,
}

pub const TTS_MODELS: &[TtsInfo] = &[
    TtsInfo {
        id: "supertonic",
        name: "Supertonic 3",
        blurb: "Lightning fast and natural. Speaks Italian too.",
        archive: "sherpa-onnx-supertonic-3-tts-int8-2026-05-11.tar.bz2",
        dir: "sherpa-onnx-supertonic-3-tts-int8-2026-05-11",
        mb: 129,
        languages: "31 languages",
        default_speaker: 6,
    },
    TtsInfo {
        id: "kokoro",
        name: "Kokoro",
        blurb: "The warmest English voice. Takes a few seconds to answer.",
        archive: "kokoro-int8-multi-lang-v1_0.tar.bz2",
        dir: "kokoro-int8-multi-lang-v1_0",
        mb: 132,
        languages: "English",
        default_speaker: 3,
    },
    TtsInfo {
        id: "kitten",
        name: "Kitten nano",
        blurb: "Tiny and quick. Good on older PCs.",
        archive: "kitten-nano-en-v0_8-int8.tar.bz2",
        dir: "kitten-nano-en-v0_8-int8",
        mb: 31,
        languages: "English",
        default_speaker: 0,
    },
];

pub fn info(id: &str) -> Option<&'static TtsInfo> {
    TTS_MODELS.iter().find(|m| m.id == id)
}

#[derive(Default)]
pub struct SpeechState {
    engine: Mutex<Option<(String, Arc<OfflineTts>)>>,
    pub downloading: AtomicBool,
    stop: AtomicBool,
    speaking: AtomicBool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TtsModel {
    pub id: String,
    pub name: String,
    pub blurb: String,
    pub size_mb: u32,
    pub languages: String,
    pub installed: bool,
}

fn dir_of(data: &Path, m: &TtsInfo) -> PathBuf {
    voice::models_dir(data).join(m.dir)
}

fn installed(data: &Path, m: &TtsInfo) -> bool {
    let d = dir_of(data, m);
    d.is_dir() && std::fs::read_dir(&d).map(|r| r.flatten().any(|e| e.path().extension().is_some_and(|x| x == "onnx"))).unwrap_or(false)
}

pub fn models(data: &Path) -> Vec<TtsModel> {
    TTS_MODELS
        .iter()
        .map(|m| TtsModel {
            id: m.id.into(),
            name: m.name.into(),
            blurb: m.blurb.into(),
            size_mb: m.mb,
            languages: m.languages.into(),
            installed: installed(data, m),
        })
        .collect()
}

/// Downloads a voice, with "tts-download" progress events (0..1).
pub fn download(app: &AppHandle, data: &Path, state: &SpeechState, id: &str) -> Result<(), String> {
    let m = info(id).ok_or("unknown voice")?;
    if state.downloading.swap(true, Ordering::SeqCst) {
        return Err("already downloading".into());
    }
    let r = voice::fetch_archive(app, &voice::models_dir(data), &format!("{BASE}{}", m.archive), m.mb, "tts-download")
        .and_then(|_| if installed(data, m) { Ok(()) } else { Err("the voice archive looked different than expected".into()) });
    state.downloading.store(false, Ordering::SeqCst);
    r
}

fn path(d: &Path, name: &str) -> Option<String> {
    let p = d.join(name);
    p.exists().then(|| p.to_string_lossy().into_owned())
}

fn load(data: &Path, m: &TtsInfo) -> Result<OfflineTts, String> {
    if !installed(data, m) {
        return Err("this voice isn't downloaded yet".into());
    }
    let d = dir_of(data, m);
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 4) as i32;
    let mut model = OfflineTtsModelConfig { num_threads: threads, ..Default::default() };
    match m.id {
        "supertonic" => {
            model.supertonic = OfflineTtsSupertonicModelConfig {
                duration_predictor: Some(voice::find(&d, "duration_predictor")?),
                text_encoder: Some(voice::find(&d, "text_encoder")?),
                vector_estimator: Some(voice::find(&d, "vector_estimator")?),
                vocoder: Some(voice::find(&d, "vocoder")?),
                tts_json: path(&d, "tts.json"),
                unicode_indexer: path(&d, "unicode_indexer.bin"),
                voice_style: path(&d, "voice.bin"),
            };
        }
        "kokoro" => {
            let lexicons: Vec<String> = ["lexicon-us-en.txt", "lexicon-zh.txt"].iter().filter_map(|n| path(&d, n)).collect();
            model.kokoro = OfflineTtsKokoroModelConfig {
                model: Some(voice::find(&d, "model")?),
                voices: path(&d, "voices.bin"),
                tokens: path(&d, "tokens.txt"),
                data_dir: path(&d, "espeak-ng-data"),
                dict_dir: path(&d, "dict"),
                lexicon: (!lexicons.is_empty()).then(|| lexicons.join(",")),
                length_scale: 1.0,
                lang: None,
            };
        }
        _ => {
            model.kitten = OfflineTtsKittenModelConfig {
                model: Some(voice::find(&d, "model")?),
                voices: path(&d, "voices.bin"),
                tokens: path(&d, "tokens.txt"),
                data_dir: path(&d, "espeak-ng-data"),
                length_scale: 1.0,
            };
        }
    }
    OfflineTts::create(&OfflineTtsConfig { model, max_num_sentences: 2, ..Default::default() }).ok_or_else(|| "couldn't load the voice".to_string())
}

fn engine(data: &Path, state: &SpeechState, id: &str) -> Result<Arc<OfflineTts>, String> {
    let mut g = state.engine.lock();
    if let Some((cur, e)) = g.as_ref() {
        if cur == id {
            return Ok(e.clone());
        }
    }
    let m = info(id).ok_or("unknown voice")?;
    let e = Arc::new(load(data, m)?);
    *g = Some((id.to_string(), e.clone()));
    Ok(e)
}

/// Renders `text` to mono samples. `lang` is a 2-letter code ("en", "it"…).
pub fn synthesize(data: &Path, state: &SpeechState, id: &str, speaker: i32, text: &str, lang: &str) -> Result<(Vec<f32>, u32), String> {
    let m = info(id).ok_or("unknown voice")?;
    let tts = engine(data, state, id)?;
    let sid = if speaker >= 0 && speaker < tts.num_speakers().max(1) { speaker } else { m.default_speaker.min(tts.num_speakers().max(1) - 1) };
    let mut cfg = GenerationConfig { sid, speed: 1.05, ..Default::default() };
    if m.id == "supertonic" {
        cfg.num_steps = 6;
        cfg.extra = Some(HashMap::from([("lang".to_string(), serde_json::json!(lang))]));
    }
    let audio = tts.generate_with_config(text, &cfg, None::<fn(&[f32], f32) -> bool>).ok_or("couldn't speak that")?;
    Ok((audio.samples().to_vec(), audio.sample_rate() as u32))
}

/// Speaks `text` on the default output device, off the UI thread. Emits
/// "dusty-speaking" (true/false). A new call interrupts the previous one.
pub fn say(app: AppHandle, text: String, lang: String) {
    let state = app.state::<crate::state::AppState>();
    let (id, speaker) = {
        let s = state.settings.read();
        (s.tts_model.clone(), s.tts_speaker)
    };
    stop(&state.speech);
    let data = state.data_dir.clone();
    std::thread::spawn(move || {
        let st = app.state::<crate::state::AppState>();
        // Wait for a previous utterance to wind down.
        for _ in 0..50 {
            if !st.speech.speaking.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        st.speech.stop.store(false, Ordering::SeqCst);
        st.speech.speaking.store(true, Ordering::SeqCst);
        let _ = app.emit("dusty-speaking", true);
        let r = synthesize(&data, &st.speech, &id, speaker, &text, &lang).and_then(|(pcm, rate)| play(&st.speech, &pcm, rate));
        if let Err(e) = r {
            eprintln!("speech: {e}");
        }
        st.speech.speaking.store(false, Ordering::SeqCst);
        let _ = app.emit("dusty-speaking", false);
    });
}

pub fn stop(state: &SpeechState) {
    state.stop.store(true, Ordering::SeqCst);
}

fn play(state: &SpeechState, pcm: &[f32], rate: u32) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("no speakers found")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let out_rate = supported.sample_rate();
    let channels = supported.channels() as usize;
    let config: cpal::StreamConfig = supported.clone().into();
    let samples = Arc::new(voice::resample(pcm, rate, out_rate));
    let total = samples.len();
    let pos = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let err = |e: cpal::Error| eprintln!("speaker error: {e}");
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let (samples, pos) = (samples.clone(), pos.clone());
            device.build_output_stream(
                config,
                move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    for frame in out.chunks_mut(channels) {
                        let i = pos.fetch_add(1, Ordering::Relaxed);
                        let v = samples.get(i).copied().unwrap_or(0.0);
                        frame.iter_mut().for_each(|s| *s = v);
                    }
                },
                err,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let (samples, pos) = (samples.clone(), pos.clone());
            device.build_output_stream(
                config,
                move |out: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    for frame in out.chunks_mut(channels) {
                        let i = pos.fetch_add(1, Ordering::Relaxed);
                        let v = (samples.get(i).copied().unwrap_or(0.0).clamp(-1.0, 1.0) * 32767.0) as i16;
                        frame.iter_mut().for_each(|s| *s = v);
                    }
                },
                err,
                None,
            )
        }
        other => return Err(format!("unsupported speaker format {other:?}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    while pos.load(Ordering::Relaxed) < total + out_rate as usize / 10 {
        if state.stop.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthesizes with every downloaded voice and transcribes the result with the
    /// speech recognizer, so quality is checked end to end without ears:
    /// `cargo test --release real_tts -- --ignored --nocapture`
    /// (set DUSTY_TTS_DIR to a folder holding the unpacked models).
    #[test]
    #[ignore]
    fn real_tts() {
        let models = PathBuf::from(std::env::var("DUSTY_TTS_DIR").expect("DUSTY_TTS_DIR"));
        let data = models.parent().unwrap().to_path_buf();
        assert!(models.ends_with("models"), "DUSTY_TTS_DIR must be a folder called `models`");
        let state = SpeechState::default();
        for m in TTS_MODELS {
            if !installed(&data, m) {
                println!("{}: not downloaded", m.id);
                continue;
            }
            let t = std::time::Instant::now();
            let tts = engine(&data, &state, m.id).unwrap();
            let load = t.elapsed();
            let cases: &[(&str, &str)] = if m.id == "supertonic" {
                &[("en", "I found three invoices from March. The top one is in Downloads."), ("it", "Ho trovato tre fatture di marzo.")]
            } else {
                &[("en", "I found three invoices from March. The top one is in Downloads.")]
            };
            for (lang, text) in cases {
                let t = std::time::Instant::now();
                let (pcm, rate) = synthesize(&data, &state, m.id, -1, text, lang).unwrap();
                let took = t.elapsed();
                let secs = pcm.len() as f32 / rate as f32;
                let wav = std::env::temp_dir().join(format!("dusty-tts-{}-{lang}.wav", m.id));
                sherpa_onnx::write(&wav.to_string_lossy(), &pcm, rate as i32);
                println!(
                    "{} [{lang}] speakers={} load {:?}, synth {:?} for {:.1}s audio (RTF {:.2}) -> {}",
                    m.id,
                    tts.num_speakers(),
                    load,
                    took,
                    secs,
                    took.as_secs_f32() / secs,
                    wav.display()
                );
            }
        }
    }
}
