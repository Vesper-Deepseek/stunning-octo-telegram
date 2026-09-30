//! Offline voice transcription using whisper.cpp through the maintained whispercpp binding.
//!
//! Android records 16 kHz mono PCM16 WAV files. This module accepts that
//! format directly and also handles mono/stereo WAVs with other sample rates
//! by downmixing and linearly resampling before Whisper inference.

use std::{fs::File, io::Read, path::Path, sync::Arc};

use whispercpp::{Context, ContextParams, Params, SamplingStrategy};

/// Maximum audio length handed to Whisper in one request. Longer recordings
/// would balloon the mel-spectrogram allocation and OOM-kill the process on
/// mid-range phones.
const MAX_AUDIO_SECONDS: usize = 120;
const SAMPLE_RATE_HZ: usize = 16_000;

/// Approximate peak RSS Whisper needs for a GGML model, derived from the
/// weights size stored in the file header. Must stay in sync with
/// MemoryGuard.estimatedWhisperPeakMb on the Kotlin side so the two
/// pre-flight checks agree (weights x4 + 160 MB activation overhead).
fn estimated_whisper_bytes(model_bytes: u64) -> u64 {
    let weight_mb = model_bytes / (1024 * 1024);
    (weight_mb.saturating_mul(4).saturating_add(160)).saturating_mul(1024 * 1024)
}

/// Free memory reported by the kernel (`MemAvailable` from /proc/meminfo).
/// Returns None where procfs is unavailable so callers fail open.
fn mem_available_kb() -> Option<u64> {
    let text = {
        let mut buf = String::new();
        File::open("/proc/meminfo")
            .ok()
            .and_then(|mut f| f.read_to_string(&mut buf).ok().map(|_| buf))
    }?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            return rest
                .trim_end()
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok());
        }
    }
    None
}

/// Verify the GGML Whisper weights before touching native code: correct
/// magic, sane size, readable file. ggml_log.cpp calls abort() (an
/// uncatchable SIGABRT) when it reads a truncated or foreign file, so this
/// pre-flight check is what prevents the immediate force-close after a
/// corrupted download.
fn verify_ggml_model(path: &Path) -> Result<u64, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("model stat: {e}"))?;
    if !meta.is_file() || meta.len() < 128 {
        return Err("Whisper model file is missing or too small to be valid".into());
    }
    let mut header = [0u8; 8];
    File::open(path)
        .map_err(|e| format!("model open: {e}"))?
        .read_exact(&mut header)
        .map_err(|e| format!("model header read: {e}"))?;
    // whisper.cpp writes GGML_FILE_MAGIC = 0x67676d6c in native
    // integer order; the on-disk little-endian bytes are "lmgg".
    // The official ggml-tiny.en-q5_1.bin uses this magic.
    const GGML_FILE_MAGIC: u32 = 0x67676d6c;
    let magic = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    if magic != GGML_FILE_MAGIC {
        return Err(format!(
            "Whisper model is not a GGML file (magic 0x{magic:08x}); re-download the model"
        ));
    }
    Ok(meta.len())
}

/// Transcribe a local WAV file using a local Whisper GGML model.
pub fn transcribe_wav<P: AsRef<Path>, M: AsRef<Path>>(
    wav_path: P,
    model_path: M,
) -> Result<String, String> {
    let model_bytes = verify_ggml_model(model_path.as_ref())?;

    // OOM pre-flight: refuse to start inference the kernel cannot back,
    // instead of letting ggml's operator new raise std::bad_alloc which
    // crosses the C boundary and kills the process.
    if let Some(avail_kb) = mem_available_kb() {
        let avail_bytes = avail_kb.saturating_mul(1024);
        let needed = estimated_whisper_bytes(model_bytes);
        if avail_bytes < needed {
            return Err(format!(
                "insufficient memory for offline transcription: need ~{} MB, {} MB available",
                needed / (1024 * 1024),
                avail_bytes / (1024 * 1024)
            ));
        }
    }

    let mut samples = read_wav_16k_mono(wav_path)?;
    let max_samples = MAX_AUDIO_SECONDS * SAMPLE_RATE_HZ;
    if samples.len() > max_samples {
        samples.truncate(max_samples);
    }
    if samples.is_empty() {
        return Ok(String::new());
    }

    let context = Arc::new(
        Context::new(
            model_path.as_ref(),
            ContextParams::new().with_use_gpu(false),
        )
        .map_err(|e| format!("whisper model init: {e}"))?,
    );

    let mut state = context
        .create_state()
        .map_err(|e| format!("whisper state init: {e}"))?;

    let mut params = Params::new(SamplingStrategy::Greedy { best_of: 1 });
    // Whisper's mel-spectrogram and KV buffers scale with thread count; on
    // memory-constrained phones cap at 2 threads to lower peak RSS instead
    // of letting ggml OOM-abort the process.
    let whisper_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);
    let mem_available_mb = mem_available_kb().map(|kb| kb / 1024).unwrap_or(u64::MAX);
    let n_threads = if mem_available_mb < 3_072 { 1 } else { whisper_threads.min(2) };
    params
        .set_language("auto")
        .map_err(|e| format!("whisper language setup: {e}"))?;
    params
        .set_n_threads(n_threads as i32)
        .set_no_context(true)
        .set_suppress_blank(true)
        .set_suppress_nst(true)
        .set_temperature(0.0)
        .set_temperature_inc(0.0)
        .set_no_speech_thold(0.6)
        .silence_print_toggles();

    state
        .full(&params, &samples)
        .map_err(|e| format!("whisper decode: {e}"))?;

    let mut transcript = String::new();
    for i in 0..state.n_segments() {
        let segment = state
            .segment(i)
            .ok_or_else(|| format!("whisper segment {i} unavailable"))?;
        let text = segment.text().map_err(|e| format!("whisper text: {e}"))?;
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        if !transcript.is_empty() {
            transcript.push('\n');
        }
        transcript.push_str(text);
    }

    Ok(transcript.trim().to_string())
}

fn read_wav_16k_mono<P: AsRef<Path>>(path: P) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| format!("WAV open: {e}"))?;
    let spec = reader.spec();
    if spec.channels == 0 {
        return Err("WAV contains zero channels".into());
    }

    let mut mono = Vec::new();
    match spec.sample_format {
        hound::SampleFormat::Int => {
            let max =
                ((1_i64 << (spec.bits_per_sample.saturating_sub(1).min(31))) - 1).max(1) as f32;
            let mut frame = Vec::with_capacity(spec.channels as usize);
            for sample in reader.samples::<i32>() {
                let value = sample.map_err(|e| format!("WAV sample: {e}"))? as f32 / max;
                frame.push(value.clamp(-1.0, 1.0));
                if frame.len() == spec.channels as usize {
                    mono.push(frame.iter().copied().sum::<f32>() / frame.len() as f32);
                    frame.clear();
                }
            }
        }
        hound::SampleFormat::Float => {
            let mut frame = Vec::with_capacity(spec.channels as usize);
            for sample in reader.samples::<f32>() {
                frame.push(
                    sample
                        .map_err(|e| format!("WAV sample: {e}"))?
                        .clamp(-1.0, 1.0),
                );
                if frame.len() == spec.channels as usize {
                    mono.push(frame.iter().copied().sum::<f32>() / frame.len() as f32);
                    frame.clear();
                }
            }
        }
    }

    if spec.sample_rate == 16_000 {
        return Ok(mono);
    }
    Ok(resample_linear(&mono, spec.sample_rate, 16_000))
}

fn resample_linear(input: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if input.is_empty() || source_rate == target_rate {
        return input.to_vec();
    }

    let out_len = ((input.len() as u64 * target_rate as u64) / source_rate as u64) as usize;
    let mut out = Vec::with_capacity(out_len.max(1));

    for i in 0..out_len {
        let pos = i as f64 * source_rate as f64 / target_rate as f64;
        let idx = pos.floor() as usize;
        let frac = (pos - idx as f64) as f32;
        let a = input.get(idx).copied().unwrap_or(0.0);
        let b = input.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_keeps_empty_empty() {
        assert!(resample_linear(&[], 48_000, 16_000).is_empty());
    }

    #[test]
    fn resample_changes_length_predictably() {
        let src = vec![0.0; 48_000];
        let out = resample_linear(&src, 48_000, 16_000);
        assert_eq!(out.len(), 16_000);
    }
}
