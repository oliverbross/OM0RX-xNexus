//! Immutable final 48 kHz mono FT output. No device, route or TX authority.
//! FIR kernel/table provenance: codex/offline-final-output 81aaf90c718a0f1ce14902717739a76ea3dea8d0.
use crate::xlog_ft_encode::Prepared;
use sha2::{Digest, Sha256};
use std::time::Instant;
use tempo_core::modes::ModeKind;
#[path = "xlog_ft_output/fir.rs"]
mod fir;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Mode,
    Size,
    Coefficients,
    Offset,
    Sample,
    Amplitude,
    Empty,
    Tail,
    Gain,
    Expired,
}
/// All bytes are held privately after rendering. The SDK uses manifest() inside
/// its exact operation descriptor and hashes prefix + raw descriptor + pcm().
pub(crate) struct FinalPeriod {
    pcm: Box<[u8]>,
    manifest: Box<[u8]>,
    pcm_sha256: [u8; 32],
    content_sha256: [u8; 32],
    support: (usize, usize),
}
impl FinalPeriod {
    pub(crate) fn pcm(&self) -> &[u8] {
        &self.pcm
    }
    pub(crate) fn manifest(&self) -> &[u8] {
        &self.manifest
    }
    pub(crate) fn pcm_sha256(&self) -> &[u8; 32] {
        &self.pcm_sha256
    }
    pub(crate) fn content_sha256(&self) -> &[u8; 32] {
        &self.content_sha256
    }
    pub(crate) fn support(&self) -> (usize, usize) {
        self.support
    }
}
/// The ONLY public input is the actual private upstream encoder result. Gain is
/// applied once to source f32 before FIR; no implicit normalization or clipping.
/// Call on the bounded preparation lane, never under Stop/RX/connection locks.
pub(crate) fn prepare(
    source: &Prepared,
    gain: f32,
    deadline: Instant,
) -> Result<FinalPeriod, Error> {
    if Instant::now() >= deadline {
        return Err(Error::Expired);
    }
    if !gain.is_finite() || gain <= 0.0 || gain > 0.8 {
        return Err(Error::Gain);
    }
    let (name, tag, frames, duration, highest) = match source.mode() {
        ModeKind::Ft8 => ("FT8", 8u8, 720_000usize, 15_000_000u64, 7.0 * 6.25),
        ModeKind::Ft4 => (
            "FT4",
            4u8,
            360_000usize,
            7_500_000u64,
            3.0 * 12000.0 / 576.0,
        ),
        _ => return Err(Error::Mode),
    };
    if source.samples().len() != frames / 4 || source.waveform_frames() > frames / 4 {
        return Err(Error::Size);
    }
    if !source.offset_hz().is_finite()
        || source.offset_hz() < 100.0
        || source.offset_hz() + highest > 5000.0
    {
        return Err(Error::Offset);
    }
    let gained: Vec<f32> = source.samples().iter().map(|x| *x * gain).collect();
    let filter = fir::CandidateFilter::fixed_candidate()?;
    let support = fir::support(&gained, frames)?;
    let pcm = render(&filter, &gained, frames, deadline)?;
    let pcm_sha256: [u8; 32] = Sha256::digest(&pcm).into();
    // Wire content identity. Complete operation identity additionally binds all
    // manifest/filter/support/gain fields in the exact raw descriptor bytes.
    let mut h = Sha256::new();
    h.update(b"xnexus-final-period-v2\0");
    h.update([tag]);
    h.update(48000u32.to_le_bytes());
    h.update(1u16.to_le_bytes());
    h.update(b"f32le\0");
    h.update(source.offset_hz().to_bits().to_le_bytes());
    h.update((source.text().len() as u32).to_le_bytes());
    h.update(source.text().as_bytes());
    h.update((pcm.len() as u64).to_le_bytes());
    h.update(&pcm);
    let content_sha256: [u8; 32] = h.finalize().into();
    let manifest = serde_json::to_vec(&serde_json::json!({
        "schema_version":2,"mode":name,"message_form":"wsjt77","message":source.text(),
        "format":{"sample_rate_hz":48000,"channels":1,"encoding":"f32le"},
        "offset_hz_f32_bits":source.offset_hz().to_bits(),"frames":frames,
        "waveform_frames":support.1,"pcm_byte_count":pcm.len(),"period_duration_us":duration,
        "pcm_sha256":hex(&pcm_sha256),"content_sha256":hex(&content_sha256),
        "filter_id":fir::CANDIDATE_ID,"coefficients_sha256":hex(&filter.sha256()),
        "group_delay_frames":256,"support_start_frame":support.0,"gain_f32_bits":gain.to_bits()
    }))
    .map_err(|_| Error::Size)?
    .into_boxed_slice();
    if Instant::now() >= deadline {
        return Err(Error::Expired);
    }
    Ok(FinalPeriod {
        pcm: pcm.into_boxed_slice(),
        manifest,
        pcm_sha256,
        content_sha256,
        support,
    })
}
fn render(
    filter: &fir::CandidateFilter,
    input: &[f32],
    frames: usize,
    deadline: Instant,
) -> Result<Vec<u8>, Error> {
    fir::support(input, frames)?;
    let mut pcm = Vec::with_capacity(frames.checked_mul(4).ok_or(Error::Size)?);
    for frame in 0..frames {
        if frame % 4096 == 0 && Instant::now() >= deadline {
            return Err(Error::Expired);
        }
        let sample = filter.sample(input, frame) as f32;
        if !sample.is_finite() {
            return Err(Error::Sample);
        }
        if sample.abs() > 0.95 {
            return Err(Error::Amplitude);
        }
        pcm.extend_from_slice(&sample.to_le_bytes());
    }
    Ok(pcm)
}
fn hex(v: &[u8; 32]) -> String {
    v.iter().map(|x| format!("{x:02x}")).collect()
}
#[cfg(test)]
#[path = "xlog_ft_output_tests.rs"]
mod tests;
