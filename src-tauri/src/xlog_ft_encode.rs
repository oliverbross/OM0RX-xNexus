//! Original FT8/FT4 encoders behind a bounded native preparation boundary.
//! This is 12 kHz source PCM, NOT a commissioned output format or TX permission.
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
use tempo_core::modes::{self, ModeKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Mode,
    Text,
    TruncatedFreeText,
    Refused,
    Offset,
    Waveform,
    Busy,
    Expired,
}
static PREPARING: AtomicBool = AtomicBool::new(false);
struct Admission;
impl Drop for Admission {
    fn drop(&mut self) {
        PREPARING.store(false, Ordering::Release);
    }
}

/// Private samples cannot be replaced after preparation. Final output conversion
/// and the owner's complete-body binding happen separately, before approval.
pub(crate) struct Prepared {
    mode: ModeKind,
    text: String,
    offset_hz: f32,
    samples: Vec<f32>,
    waveform_frames: usize,
}
impl Prepared {
    pub(crate) fn mode(&self) -> ModeKind {
        self.mode
    }
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
    pub(crate) fn offset_hz(&self) -> f32 {
        self.offset_hz
    }
    pub(crate) fn samples(&self) -> &[f32] {
        &self.samples
    }
    pub(crate) fn waveform_frames(&self) -> usize {
        self.waveform_frames
    }
}

pub(crate) fn validate_text(mode: ModeKind, text: &str) -> Result<Option<Vec<i32>>, Error> {
    if !matches!(mode, ModeKind::Ft8 | ModeKind::Ft4) {
        return Err(Error::Mode);
    }
    if text.is_empty()
        || text.len() > 37
        || text.trim() != text
        || text.contains("  ")
        || text.contains("<...>")
        || !text
            .bytes()
            .all(|b| b" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ+-./?<>;#$".contains(&b))
        || text.split(' ').any(|word| word.len() > 13)
    {
        return Err(Error::Text);
    }
    let refused = match mode {
        ModeKind::Ft8 => modes::wsjt77::ft8_refusal(text),
        _ => modes::wsjt77::ft4_refusal(text),
    };
    if refused.is_some() {
        return Err(Error::Refused);
    }
    let mut message = [b' '; 37];
    message[..text.len()].copy_from_slice(text.as_bytes());
    let bits = match modes::wsjt77::pack::pack(&message) {
        Ok(Some(bits)) => bits,
        Ok(None) => return Err(Error::Refused),
        Err(_) => return Ok(None), // Actual upstream memo/FFI encoder, off-thread below.
    };
    // The upstream packer falls back to 13-character free text. Never label a
    // truncated/character-substituted fallback as the operator's complete text.
    if bits[71..].iter().all(|bit| *bit == 0)
        && (text.len() > 13
            || !text
                .bytes()
                .all(|b| b" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ+-./?".contains(&b)))
    {
        return Err(Error::TruncatedFreeText);
    }
    let tones = match mode {
        ModeKind::Ft8 => modes::wsjt77::ft8_tones(text),
        _ => modes::wsjt77::ft4_tones(text),
    };
    let Some(tones) = tones else {
        return Ok(None);
    };
    let (count, max) = if mode == ModeKind::Ft8 {
        (79, 7)
    } else {
        (103, 3)
    };
    if tones.len() != count
        || tones.iter().any(|t| !(0..=max).contains(t))
        || tones.iter().all(|t| *t == 0)
    {
        return Err(Error::Refused);
    }
    Ok(Some(tones))
}

/// Call ONLY on the bounded preparation worker outside all connection/Stop/RX
/// locks. At most one invocation can enter the upstream encoder. On timeout the
/// caller retires intent; a still-running FFI retains admission until it exits.
/// No thread kill, replacement encoder queue, or late result retry is permitted.
pub(crate) fn prepare(
    mode: ModeKind,
    text: &str,
    offset_hz: f32,
    deadline: Instant,
) -> Result<Prepared, Error> {
    if Instant::now() >= deadline {
        return Err(Error::Expired);
    }
    PREPARING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| Error::Busy)?;
    let _admission = Admission;
    let tones = match validate_text(mode, text)? {
        Some(tones) => tones,
        None => upstream_fallback(mode, text)?,
    };
    if Instant::now() >= deadline {
        return Err(Error::Expired);
    }
    let (tone_count, tone_max) = if mode == ModeKind::Ft8 {
        (79, 7)
    } else {
        (103, 3)
    };
    if tones.len() != tone_count
        || tones.iter().any(|t| !(0..=tone_max).contains(t))
        || tones.iter().all(|t| *t == 0)
    {
        return Err(Error::Refused);
    }
    let (frames, spacing, high) = if mode == ModeKind::Ft8 {
        (180_000, 6.25, 7.0)
    } else {
        (90_000, 12_000.0 / 576.0, 3.0)
    };
    if !offset_hz.is_finite() || offset_hz < 100.0 || offset_hz + spacing * high > 5000.0 {
        return Err(Error::Offset);
    }
    let mut samples = modes::tx_mode(mode)
        .ok_or(Error::Mode)?
        .gen_wave(&tones, 12_000.0, offset_hz);
    if samples.is_empty()
        || samples.len() > frames
        || samples.iter().any(|x| !x.is_finite() || x.abs() > 1.0)
    {
        return Err(Error::Waveform);
    }
    let waveform_frames = samples.len();
    samples.resize(frames, 0.0);
    if Instant::now() >= deadline {
        return Err(Error::Expired);
    }
    Ok(Prepared {
        mode,
        text: text.into(),
        offset_hz,
        samples,
        waveform_frames,
    })
}

/// Existing Mode implementation owns memo updates and vendored FFI. This helper
/// is called only inside prepare's single admission and deadline fence.
fn upstream_fallback(mode: ModeKind, text: &str) -> Result<Vec<i32>, Error> {
    Ok(modes::tx_mode(mode).ok_or(Error::Mode)?.encode(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_upstream_fallback_matches_tones_for_explicit_call_and_free_text() {
        for mode in [ModeKind::Ft8, ModeKind::Ft4] {
            for text in ["CQ K1ABC FN42", "TNX 73", "W9XYZ/M <K1ABC> 73"] {
                let expected = validate_text(mode, text)
                    .unwrap()
                    .expect("explicit known calls are pure controls");
                assert_eq!(upstream_fallback(mode, text).unwrap(), expected);
            }
        }
    }
}
