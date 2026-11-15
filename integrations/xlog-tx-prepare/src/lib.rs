// SPDX-License-Identifier: GPL-3.0-only
//! Unwired local content preparation. This grants no TX, timing or route authority.
//! No Engine, fallback encoder, device, scheduler, transport or runtime registration.
use modes::{slot_over_samples_12k, tx_mode, wsjt77, ModeKind};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    F32Le,
    /// Deliberately unsupported: no implicit quantization is performed.
    S16Le,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Format {
    pub sample_rate: u32,
    pub channels: u16,
    pub encoding: Encoding,
}

/// Explicit preparation bounds, not owner policy or commissioning defaults.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_message_bytes: usize,
    pub max_frames: usize,
    pub max_bytes: usize,
    pub min_offset_hz: f32,
    pub max_offset_hz: f32,
    pub max_abs_sample: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    UnsupportedMode,
    UnsupportedFormat,
    InvalidLimits,
    InvalidMessage,
    UnsupportedMessageForm,
    RefusedMessage,
    RequiresStatefulEncoder,
    InvalidTones,
    InvalidOffset,
    SizeLimit,
    InvalidWaveform,
    AmplitudeLimit,
}

/// Exact locally prepared bytes. Private fields prevent digest/byte divergence.
/// No operation ID, lease, period timestamp, authority, or on-air receipt exists.
#[derive(Clone, Debug)]
pub struct PreparedPeriod {
    mode: ModeKind,
    message: String,
    format: Format,
    offset_hz: f32,
    frames: usize,
    waveform_frames: usize,
    pcm: Vec<u8>,
    pcm_sha256: [u8; 32],
    content_sha256: [u8; 32],
}

impl PreparedPeriod {
    pub fn mode(&self) -> ModeKind {
        self.mode
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn format(&self) -> Format {
        self.format
    }
    pub fn offset_hz(&self) -> f32 {
        self.offset_hz
    }
    pub fn frames(&self) -> usize {
        self.frames
    }
    pub fn waveform_frames(&self) -> usize {
        self.waveform_frames
    }
    pub fn pcm(&self) -> &[u8] {
        &self.pcm
    }
    pub fn pcm_sha256(&self) -> &[u8; 32] {
        &self.pcm_sha256
    }
    pub fn content_sha256(&self) -> &[u8; 32] {
        &self.content_sha256
    }
}

pub fn prepare(
    mode: ModeKind,
    message: &str,
    format: Format,
    offset_hz: f32,
    limits: Limits,
) -> Result<PreparedPeriod, Error> {
    // The reused wave generators have FIXED samples/symbol even when fsample
    // changes. Only 12 kHz preserves the actual FT8/FT4 symbol timing here.
    let (frames, tone_count, tone_max) = match mode {
        ModeKind::Ft8 => (180_000usize, 79usize, 7),
        ModeKind::Ft4 => (90_000usize, 103usize, 3),
        _ => return Err(Error::UnsupportedMode),
    };
    if format.sample_rate != 12_000 || format.channels != 1 || format.encoding != Encoding::F32Le {
        return Err(Error::UnsupportedFormat);
    }
    if limits.max_message_bytes == 0
        || limits.max_frames == 0
        || limits.max_bytes == 0
        || !limits.min_offset_hz.is_finite()
        || !limits.max_offset_hz.is_finite()
        || limits.min_offset_hz < 0.0
        || limits.max_offset_hz < limits.min_offset_hz
        || !limits.max_abs_sample.is_finite()
        || !(0.0..=1.0).contains(&limits.max_abs_sample)
        || limits.max_abs_sample == 0.0
    {
        return Err(Error::InvalidLimits);
    }
    // This only guards the C string boundary. The packer ALSO truncates free
    // text/individual tokens at 13 characters; supported_message below excludes
    // those paths rather than treating packer's successful verdict as fidelity.
    if message.is_empty()
        || message.len() > 37
        || message.len() > limits.max_message_bytes
        || message.trim() != message
        || message.contains("  ")
        || !message
            .bytes()
            .all(|b| (0x20..=0x7e).contains(&b) && !b.is_ascii_lowercase())
    {
        return Err(Error::InvalidMessage);
    }
    supported_message(message)?;
    let spacing = if mode == ModeKind::Ft8 {
        12_000.0 / 1920.0
    } else {
        12_000.0 / 576.0
    };
    if !offset_hz.is_finite()
        || offset_hz < limits.min_offset_hz
        || offset_hz > limits.max_offset_hz
        || offset_hz + tone_max as f32 * spacing >= format.sample_rate as f32 / 2.0
    {
        return Err(Error::InvalidOffset);
    }
    let bytes = frames.checked_mul(4).ok_or(Error::SizeLimit)?;
    if frames > limits.max_frames || bytes > limits.max_bytes {
        return Err(Error::SizeLimit);
    }
    let refused = match mode {
        ModeKind::Ft8 => wsjt77::ft8_refusal(message),
        ModeKind::Ft4 => wsjt77::ft4_refusal(message),
        _ => unreachable!(),
    };
    if refused.is_some() {
        return Err(Error::RefusedMessage);
    }
    let tones = match mode {
        ModeKind::Ft8 => wsjt77::ft8_tones(message),
        ModeKind::Ft4 => wsjt77::ft4_tones(message),
        _ => unreachable!(),
    }
    .ok_or(Error::RequiresStatefulEncoder)?;
    if tones.len() != tone_count
        || tones.iter().any(|t| !(0..=tone_max).contains(t))
        || tones.iter().all(|t| *t == 0)
    {
        return Err(Error::InvalidTones);
    }
    // gen_wave is pure for these two selected implementations; encode() is NOT
    // called (it may enqueue encode_later or invoke stateful vendored fallback).
    let wave = tx_mode(mode)
        .ok_or(Error::UnsupportedMode)?
        .gen_wave(&tones, 12_000.0, offset_hz);
    finish(mode, message, format, offset_hz, limits, frames, wave)
}

/// Deliberately narrow first slice: literal, standard type-1 exchanges only.
/// No free text, compound/hash calls, contest fields, telemetry, directed CQ,
/// normalization, or attempt to implement a second WSJT message packer.
fn supported_message(message: &str) -> Result<(), Error> {
    let words: Vec<_> = message.split(' ').collect();
    let [first, second, third] = words.as_slice() else {
        return Err(Error::UnsupportedMessageForm);
    };
    let base_call = |call: &str| {
        let bytes = call.as_bytes();
        if !(3..=6).contains(&bytes.len()) || !bytes.iter().all(u8::is_ascii_alphanumeric) {
            return false;
        }
        let Some(area) = bytes.iter().rposition(u8::is_ascii_digit) else {
            return false;
        };
        (1..=2).contains(&area)
            && bytes[..area].iter().any(u8::is_ascii_uppercase)
            && (1..=3).contains(&(bytes.len() - area - 1))
            && bytes[area + 1..].iter().all(u8::is_ascii_uppercase)
            && wsjt77::pack::chkcall_ok(call)
    };
    let grid = |text: &str| {
        let b = text.as_bytes();
        b.len() == 4
            && (b'A'..=b'R').contains(&b[0])
            && (b'A'..=b'R').contains(&b[1])
            && b[2].is_ascii_digit()
            && b[3].is_ascii_digit()
    };
    let report = |text: &str| {
        let text = text.strip_prefix('R').unwrap_or(text);
        let b = text.as_bytes();
        if b.len() != 3 || !matches!(b[0], b'+' | b'-') || !b[1..].iter().all(u8::is_ascii_digit) {
            return false;
        }
        // Exclude alternate spellings and WSJT's extended negative-report
        // remapping from this initial explicitly supported subset.
        let value = i16::from(b[1] - b'0') * 10 + i16::from(b[2] - b'0');
        if b[0] == b'-' {
            (1..=30).contains(&value)
        } else {
            value <= 49
        }
    };
    let supported = base_call(second)
        && if *first == "CQ" {
            grid(third)
        } else {
            base_call(first)
                && (grid(third) || report(third) || matches!(*third, "RRR" | "RR73" | "73"))
        };
    if !supported {
        return Err(Error::UnsupportedMessageForm);
    }
    let mut padded = [b' '; 37];
    padded[..message.len()].copy_from_slice(message.as_bytes());
    let packed = wsjt77::pack::pack(&padded)
        .map_err(|_| Error::RequiresStatefulEncoder)?
        .ok_or(Error::RefusedMessage)?;
    // Public pack returns numerical bits. Type-1 is i3=001, not free-text
    // i3/n3=000/000. Fail closed if upstream changes selection for our grammar.
    if packed[74..] != [0, 0, 1] {
        return Err(Error::UnsupportedMessageForm);
    }
    Ok(())
}

fn finish(
    mode: ModeKind,
    message: &str,
    format: Format,
    offset_hz: f32,
    limits: Limits,
    frames: usize,
    wave: Vec<f32>,
) -> Result<PreparedPeriod, Error> {
    let (lead, signal) = slot_over_samples_12k(mode).ok_or(Error::UnsupportedMode)?;
    let minimum = lead.checked_add(signal).ok_or(Error::SizeLimit)?;
    if wave.len() < minimum
        || wave.len() > frames
        || !wave.iter().any(|v| *v != 0.0)
        || wave.iter().any(|v| !v.is_finite())
        || wave[..lead].iter().any(|v| *v != 0.0)
    {
        return Err(Error::InvalidWaveform);
    }
    if wave.iter().any(|v| v.abs() > limits.max_abs_sample) {
        return Err(Error::AmplitudeLimit);
    }
    let waveform_frames = wave.len();
    let size = frames.checked_mul(4).ok_or(Error::SizeLimit)?;
    if frames > limits.max_frames || size > limits.max_bytes {
        return Err(Error::SizeLimit);
    }
    let mut pcm = Vec::with_capacity(size);
    for sample in wave {
        pcm.extend_from_slice(&sample.to_le_bytes());
    }
    // Intentional trailing period silence only; original samples/lead-in stay
    // byte-identical. Never clamp, normalize, resample or truncate to fit.
    pcm.resize(size, 0);
    let pcm_sha256: [u8; 32] = Sha256::digest(&pcm).into();
    let mut identity = Sha256::new();
    identity.update(b"xnexus-local-period-v1\0");
    identity.update([if mode == ModeKind::Ft8 { 8 } else { 4 }]);
    identity.update(format.sample_rate.to_le_bytes());
    identity.update(format.channels.to_le_bytes());
    identity.update(b"f32le\0");
    identity.update(offset_hz.to_le_bytes());
    identity.update((message.len() as u32).to_le_bytes());
    identity.update(message.as_bytes());
    identity.update((pcm.len() as u64).to_le_bytes());
    identity.update(&pcm);
    Ok(PreparedPeriod {
        mode,
        message: message.into(),
        format,
        offset_hz,
        frames,
        waveform_frames,
        pcm,
        pcm_sha256,
        content_sha256: identity.finalize().into(),
    })
}

#[cfg(test)]
mod tests;
