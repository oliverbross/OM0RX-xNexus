// SPDX-License-Identifier: GPL-3.0-only
//! SOURCE-ONLY offline candidate. No application membership, backend or TX authority.
//! Coefficient admission below checks identity/structure, NOT spectral qualification.
//! A future owner must independently admit the producer and reproduce final bytes.
use modes::ModeKind;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use xnexus_tx_prepare::{Encoding, PreparedPeriod};

pub mod fir;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Manifest,
    SourceMismatch,
    UnsupportedMode,
    Format,
    Size,
    Coefficients,
    Offset,
    Policy,
    Sample,
    Amplitude,
    Empty,
    Tail,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFormat {
    sample_rate_hz: u32,
    channels: u16,
    encoding: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceManifest {
    schema_version: u32,
    mode: String,
    message_form: String,
    message: String,
    format: SourceFormat,
    offset_hz_f32_bits: u32,
    frames: u32,
    waveform_frames: u32,
    pcm_byte_count: u32,
    period_duration_us: u64,
    pcm_sha256: String,
    content_sha256: String,
}

/// Borrows an actual privately constructed preparer result. Raw PCM cannot be
/// supplied through this boundary. This is local source correspondence, not trust.
pub struct BoundSource<'a> {
    period: &'a PreparedPeriod,
    raw_manifest: Box<[u8]>,
    whole_body_sha256: [u8; 32],
}
impl<'a> BoundSource<'a> {
    pub fn bind(period: &'a PreparedPeriod, raw_manifest: &[u8]) -> Result<Self, Error> {
        if raw_manifest.is_empty() || raw_manifest.len() > 4096 {
            return Err(Error::Size);
        }
        let manifest: SourceManifest =
            serde_json::from_slice(raw_manifest).map_err(|_| Error::Manifest)?;
        let (mode, frames, duration) = match period.mode() {
            ModeKind::Ft8 => ("FT8", 180_000, 15_000_000),
            ModeKind::Ft4 => ("FT4", 90_000, 7_500_000),
            _ => return Err(Error::UnsupportedMode),
        };
        if period.format().sample_rate != 12_000
            || period.format().channels != 1
            || period.format().encoding != Encoding::F32Le
        {
            return Err(Error::Format);
        }
        if manifest.schema_version != 1
            || manifest.mode != mode
            || manifest.message_form != "standard_type1"
            || manifest.message != period.message()
            || manifest.format.sample_rate_hz != 12_000
            || manifest.format.channels != 1
            || manifest.format.encoding != "f32le"
            || manifest.offset_hz_f32_bits != period.offset_hz().to_bits()
            || manifest.frames as usize != period.frames()
            || period.frames() != frames
            || manifest.waveform_frames as usize != period.waveform_frames()
            || manifest.pcm_byte_count as usize != period.pcm().len()
            || period.pcm().len() != frames * 4
            || manifest.period_duration_us != duration
            || manifest.pcm_sha256 != hex(period.pcm_sha256())
            || manifest.content_sha256 != hex(period.content_sha256())
        {
            return Err(Error::SourceMismatch);
        }
        let whole_body_sha256 = body_hash(
            b"xnexus-prepared-corpus-body-v1\0",
            raw_manifest,
            period.pcm(),
        );
        Ok(Self {
            period,
            raw_manifest: raw_manifest.into(),
            whole_body_sha256,
        })
    }
    pub fn raw_manifest(&self) -> &[u8] {
        &self.raw_manifest
    }
    pub fn source_pcm(&self) -> &[u8] {
        self.period.pcm()
    }
    pub fn whole_body_sha256(&self) -> &[u8; 32] {
        &self.whole_body_sha256
    }
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|v| format!("{v:02x}")).collect()
}
fn body_hash(domain: &[u8], raw: &[u8], pcm: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(domain);
    h.update((raw.len() as u64).to_le_bytes());
    h.update(raw);
    h.update((pcm.len() as u64).to_le_bytes());
    h.update(pcm);
    h.finalize().into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Source { gain_f32_bits: u32 },
    Silence,
}

/// No defaults: ordered channel slots are explicit fixture coordinates, not
/// interpreted DAX semantics. Both Source slots require an explicit duplication
/// declaration; matching gains alone never implies that declaration.
pub struct OutputPolicy {
    pub channels: [Channel; 2],
    pub source_to_both_explicit: bool,
    pub route_policy_identity: String,
    pub max_abs_sample: f32,
    pub max_frames: usize,
    pub max_bytes: usize,
}

/// Internal candidate result, deliberately NOT Serialize. The final wire schema
/// and owner digest domain require separate review before approval/journaling.
pub struct CandidateOutput {
    pcm: Box<[u8]>,
    raw_manifest: Box<[u8]>,
    whole_body_sha256: [u8; 32],
    source_raw_manifest: Box<[u8]>,
    source_pcm: Box<[u8]>,
    frames: usize,
    support_start: usize,
    support_end_exclusive: usize,
    group_delay_frames: usize,
    source_body_sha256: [u8; 32],
    coefficient_sha256: [u8; 32],
    pcm_sha256: [u8; 32],
    policy: OutputPolicy,
}
impl CandidateOutput {
    pub fn pcm(&self) -> &[u8] {
        &self.pcm
    }
    pub fn raw_manifest(&self) -> &[u8] {
        &self.raw_manifest
    }
    pub fn whole_body_sha256(&self) -> &[u8; 32] {
        &self.whole_body_sha256
    }
    pub fn source_raw_manifest(&self) -> &[u8] {
        &self.source_raw_manifest
    }
    pub fn source_pcm(&self) -> &[u8] {
        &self.source_pcm
    }
    pub fn frames(&self) -> usize {
        self.frames
    }
    pub fn support(&self) -> (usize, usize) {
        (self.support_start, self.support_end_exclusive)
    }
    pub fn group_delay_frames(&self) -> usize {
        self.group_delay_frames
    }
    pub fn source_body_sha256(&self) -> &[u8; 32] {
        &self.source_body_sha256
    }
    pub fn coefficient_sha256(&self) -> &[u8; 32] {
        &self.coefficient_sha256
    }
    pub fn pcm_sha256(&self) -> &[u8; 32] {
        &self.pcm_sha256
    }
    pub fn policy(&self) -> &OutputPolicy {
        &self.policy
    }
}

pub fn prepare_candidate(
    source: &BoundSource<'_>,
    filter: &fir::CandidateFilter,
    policy: OutputPolicy,
) -> Result<CandidateOutput, Error> {
    let spacing = match source.period.mode() {
        ModeKind::Ft8 => 7.0 * (12_000.0 / 1920.0),
        ModeKind::Ft4 => 3.0 * (12_000.0 / 576.0),
        _ => return Err(Error::UnsupportedMode),
    };
    // This only bounds tone centres inside the proposed passband. Actual GFSK
    // spectrum must pass the independent spectral gate; this is NOT that proof.
    let offset = source.period.offset_hz();
    if !offset.is_finite() || offset < 0.0 || offset + spacing > 5200.0 {
        return Err(Error::Offset);
    }
    if policy.route_policy_identity.is_empty()
        || policy.route_policy_identity.len() > 512
        || !policy
            .route_policy_identity
            .bytes()
            .all(|b| (0x21..=0x7e).contains(&b))
        || !policy.max_abs_sample.is_finite()
        || policy.max_abs_sample <= 0.0
        || policy.max_abs_sample > 1.0
        || policy.max_frames == 0
        || policy.max_frames > 720_000
        || policy.max_bytes == 0
        || policy.max_bytes > 5_760_000
    {
        return Err(Error::Policy);
    }
    let mut gains = [None; 2];
    for (i, channel) in policy.channels.iter().enumerate() {
        if let Channel::Source { gain_f32_bits } = channel {
            let gain = f32::from_bits(*gain_f32_bits);
            if !gain.is_finite() || gain <= 0.0 || gain > 1.0 {
                return Err(Error::Policy);
            }
            gains[i] = Some(gain);
        }
    }
    let count = gains.iter().filter(|g| g.is_some()).count();
    if count == 0 || (count == 2) != policy.source_to_both_explicit {
        return Err(Error::Policy);
    }
    let frames = source.period.frames().checked_mul(4).ok_or(Error::Size)?;
    let bytes = frames.checked_mul(8).ok_or(Error::Size)?;
    if frames > 720_000
        || frames > policy.max_frames
        || bytes > 5_760_000
        || bytes > policy.max_bytes
    {
        return Err(Error::Size);
    }
    let samples: Vec<f32> = source
        .period
        .pcm()
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    let (start, end) = fir::support(&samples, frames)?;
    let mut pcm = Vec::with_capacity(bytes);
    let mut nonzero = false;
    for m in 0..frames {
        let sample = filter.sample(&samples, m);
        if !sample.is_finite() {
            return Err(Error::Sample);
        }
        for gain in gains {
            let output = match gain {
                Some(g) => (sample * f64::from(g)) as f32,
                None => 0.0,
            };
            if !output.is_finite() {
                return Err(Error::Sample);
            }
            if output.abs() > policy.max_abs_sample {
                return Err(Error::Amplitude);
            }
            nonzero |= output != 0.0;
            pcm.extend_from_slice(&output.to_le_bytes());
        }
    }
    if !nonzero {
        return Err(Error::Empty);
    }
    let pcm_sha256: [u8; 32] = Sha256::digest(&pcm).into();
    // This defines only an OFFLINE artifact, not a live owner request/Body. Exact
    // serialized bytes are retained; nobody reconstructs them when hashing.
    let channels = policy.channels.map(|channel| match channel {
        Channel::Source { gain_f32_bits } => {
            serde_json::json!({"kind":"source","gain_f32_bits":gain_f32_bits})
        }
        Channel::Silence => serde_json::json!({"kind":"silence"}),
    });
    let manifest = serde_json::json!({
        "schema_version":1,"artifact_kind":"offline_final_output_candidate",
        "source":{
            "body_domain":"xnexus-prepared-corpus-body-v1",
            "body_sha256":hex(&source.whole_body_sha256),
            "raw_manifest_bytes":source.raw_manifest().len(),"pcm_bytes":source.source_pcm().len()
        },
        "transform":{
            "algorithm":"causal-polyphase4-513-ascending-f64-v1",
            "candidate_id":fir::CANDIDATE_ID,"coefficient_sha256":hex(&filter.sha256()),
            "coefficient_encoding":"f64le","coefficient_count":513,
            "arithmetic":"ascending-source-f64-multiply-then-add;channel-gain-f64;round-f32",
            "causal_offset_frames":0,"group_delay_output_frames":256
        },
        "format":{"sample_rate_hz":48000,"channels":2,"encoding":"f32le","layout":"interleaved"},
        "channel_map":channels,"source_to_both_explicit":policy.source_to_both_explicit,
        "route_policy_description":policy.route_policy_identity,
        "preparation_limits":{"max_abs_sample_f32_bits":policy.max_abs_sample.to_bits(),
            "max_frames":policy.max_frames,"max_bytes":policy.max_bytes},
        "frames":frames,"pcm_byte_count":bytes,
        "period_duration_us":if source.period.mode()==ModeKind::Ft8 {15000000} else {7500000},
        "conservative_support":{"start_frame":start,"end_frame_exclusive":end,
            "derivation":"actual-source-nonzero-plus-full-coefficient-extent"},
        "pcm_sha256":hex(&pcm_sha256)
    });
    let raw_manifest = serde_json::to_vec(&manifest).map_err(|_| Error::Manifest)?;
    if raw_manifest.len() > 8192 {
        return Err(Error::Size);
    }
    let whole_body_sha256 = body_hash(
        b"xnexus-offline-final-output-body-v1\0",
        &raw_manifest,
        &pcm,
    );
    Ok(CandidateOutput {
        pcm: pcm.into(),
        raw_manifest: raw_manifest.into(),
        whole_body_sha256,
        source_raw_manifest: source.raw_manifest().into(),
        source_pcm: source.source_pcm().into(),
        frames,
        support_start: start,
        support_end_exclusive: end,
        group_delay_frames: 256,
        source_body_sha256: source.whole_body_sha256,
        coefficient_sha256: filter.sha256(),
        pcm_sha256,
        policy,
    })
}

#[cfg(test)]
mod tests;
