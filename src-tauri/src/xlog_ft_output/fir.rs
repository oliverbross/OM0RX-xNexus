// SPDX-License-Identifier: GPL-3.0-only
use super::Error;
use sha2::{Digest, Sha256};
pub const TAPS: usize = 513;
pub const CANDIDATE_ID: &str = "kaiser-513-fc6000-beta10-x4-candidate1";
mod fixed {
    include!("coefficients.rs");
}

/// Offline candidate only. The reviewed generator will produce fixed f64 bits
/// plus their SHA-256. Supplying a hash here is NOT spectral/owner admission.
pub struct CandidateFilter {
    coefficients: [f64; TAPS],
    sha256: [u8; 32],
}
impl CandidateFilter {
    /// The only external constructor: exact numerically measured candidate bits.
    /// Numerical coefficient qualification is NOT waveform/owner qualification.
    pub fn fixed_candidate() -> Result<Self, Error> {
        Self::from_bits(fixed::COEFFICIENT_BITS, fixed::COEFFICIENT_SHA256)
    }
    pub(crate) fn from_bits(bits: [u64; TAPS], expected_sha256: [u8; 32]) -> Result<Self, Error> {
        let mut h = Sha256::new();
        for bit in bits {
            h.update(bit.to_le_bytes());
        }
        let sha256: [u8; 32] = h.finalize().into();
        if sha256 != expected_sha256 {
            return Err(Error::Coefficients);
        }
        let coefficients = bits.map(f64::from_bits);
        if coefficients.iter().any(|c| !c.is_finite())
            || bits.iter().zip(bits.iter().rev()).any(|(a, b)| a != b)
            || (coefficients.iter().sum::<f64>() - 4.0).abs() > 1e-12
        {
            return Err(Error::Coefficients);
        }
        Ok(Self {
            coefficients,
            sha256,
        })
    }
    pub fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
    /// Ascending source index, distinct f64 multiply then add, no mul_add.
    /// Compiler flags permitting reassociation/contraction are not admissible.
    pub fn sample(&self, input: &[f32], output_frame: usize) -> f64 {
        let first = output_frame.saturating_sub(TAPS - 1).div_ceil(4);
        let end = (output_frame / 4 + 1).min(input.len());
        let mut sum = 0.0f64;
        for n in first..end {
            let product = f64::from(input[n]) * self.coefficients[output_frame - 4 * n];
            sum += product;
        }
        sum
    }
}

/// Conservative FIR extent from actual sample values, not declared waveform
/// length or a caller span. Zero-extension is mathematical boundary handling;
/// any nonzero source whose FULL coefficient tail cannot fit is refused.
pub fn support(input: &[f32], output_frames: usize) -> Result<(usize, usize), Error> {
    if input.iter().any(|v| !v.is_finite()) {
        return Err(Error::Sample);
    }
    let first = input.iter().position(|v| *v != 0.0).ok_or(Error::Empty)?;
    let last = input.iter().rposition(|v| *v != 0.0).ok_or(Error::Empty)?;
    let start = first.checked_mul(4).ok_or(Error::Size)?;
    let end = last
        .checked_mul(4)
        .and_then(|v| v.checked_add(TAPS))
        .ok_or(Error::Size)?;
    if end > output_frames {
        return Err(Error::Tail);
    }
    Ok((start, end))
}
