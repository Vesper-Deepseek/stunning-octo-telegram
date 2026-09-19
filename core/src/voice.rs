//! Offline voice transcription using whisper.cpp through the maintained whispercpp binding.
//!
//! Android records 16 kHz mono PCM16 WAV files. This module accepts that
//! format directly and also handles mono/stereo WAVs with other sample rates
//! by downmixing and linearly resampling before Whisper inference.

use std::{path::Path, sync::Arc};

use whispercpp::{Context, ContextParams, Params, SamplingStrategy};

/// Transcribe a local WAV file using a local Whisper GGML model.
pub fn transcribe_wav<P: AsRef<Path>, M: AsRef<Path>>(
    wav_path: P,
    model_path: M,
) -> Result<String, String> {
    let samples = read_wav_16k_mono(wav_path)?;
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
    params
        .set_language("auto")
        .map_err(|e| format!("whisper language setup: {e}"))?;
    params
        .set_n_threads(
            std::thread::available_parallelism()
                .map(|n| n.get().min(4) as i32)
                .unwrap_or(2),
        )
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
