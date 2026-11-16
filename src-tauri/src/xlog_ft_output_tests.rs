use super::*;
use crate::xlog_ft_encode;
use std::time::Duration;
mod fixed {
    include!("xlog_ft_output/coefficients.rs");
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(60)
}
fn source(mode: ModeKind, text: &str, hz: f32) -> Prepared {
    xlog_ft_encode::prepare(mode, text, hz, deadline()).unwrap()
}
fn samples(p: &FinalPeriod) -> Vec<f32> {
    p.pcm()
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
        .collect()
}
#[test]
fn actual_modes_offsets_complete_tail_and_exact_wire_identity() {
    for mode in [ModeKind::Ft8, ModeKind::Ft4] {
        for hz in [100.0, 1000.0, 4500.0] {
            let s = source(mode, "CQ K1ABC FN42", hz);
            let p = prepare(&s, 0.5, deadline()).unwrap();
            let m: serde_json::Value = serde_json::from_slice(p.manifest()).unwrap();
            let expected = if mode == ModeKind::Ft8 {
                720000
            } else {
                360000
            };
            assert_eq!(p.pcm().len(), expected * 4);
            assert_eq!(m["frames"], expected);
            assert_eq!(m["message"], s.text());
            assert_eq!(m["schema_version"], 2);
            assert_eq!(m["gain_f32_bits"], 0.5f32.to_bits());
            assert_eq!(m["group_delay_frames"], 256);
            let first = s.samples().iter().position(|x| *x * 0.5 != 0.0).unwrap();
            let last = s.samples().iter().rposition(|x| *x * 0.5 != 0.0).unwrap();
            assert_eq!(p.support(), (first * 4, last * 4 + 513));
            assert_eq!(m["waveform_frames"], last * 4 + 513);
            let v = samples(&p);
            assert!(v[..p.support().0].iter().all(|x| x.to_bits() == 0));
            assert!(v[p.support().1..].iter().all(|x| x.to_bits() == 0));
            assert!(v.iter().all(|x| x.is_finite() && x.abs() <= 0.95));
            assert_eq!(*p.pcm_sha256(), <[u8; 32]>::from(Sha256::digest(p.pcm())));
            // Independently assemble the frozen wire content preimage.
            let mut bytes = b"xnexus-final-period-v2\0".to_vec();
            bytes.push(if mode == ModeKind::Ft8 { 8 } else { 4 });
            bytes.extend(48000u32.to_le_bytes());
            bytes.extend(1u16.to_le_bytes());
            bytes.extend(b"f32le\0");
            bytes.extend(hz.to_bits().to_le_bytes());
            bytes.extend((s.text().len() as u32).to_le_bytes());
            bytes.extend(s.text().as_bytes());
            bytes.extend((p.pcm().len() as u64).to_le_bytes());
            bytes.extend(p.pcm());
            assert_eq!(
                *p.content_sha256(),
                <[u8; 32]>::from(Sha256::digest(&bytes))
            );
            let original = Sha256::digest(&bytes);
            *bytes.last_mut().unwrap() ^= 1;
            assert_ne!(original, Sha256::digest(&bytes));
        }
    }
}
#[test]
fn actual_free_text_and_compound_are_preserved_at_final_output() {
    for mode in [ModeKind::Ft8, ModeKind::Ft4] {
        for text in ["TNX 73", "W9XYZ/M <K1ABC> 73"] {
            let s = source(mode, text, 1000.0);
            let p = prepare(&s, 0.5, deadline()).unwrap();
            let m: serde_json::Value = serde_json::from_slice(p.manifest()).unwrap();
            assert_eq!(m["message"], text);
            assert_eq!(m["message_form"], "wsjt77");
            assert!(samples(&p).iter().any(|x| x.abs() > 0.1));
        }
    }
}
#[test]
fn independent_zero_insert_convolution_matches_actual_ft4_full_period() {
    let s = source(ModeKind::Ft4, "CQ K1ABC FN42", 4500.0);
    let p = prepare(&s, 0.5, deadline()).unwrap();
    // Separate source-driven accumulation, ascending n, full convolution tail.
    let mut reference = vec![0.0f64; 360000];
    for (n, x) in s.samples().iter().enumerate() {
        let input = f64::from(*x * 0.5);
        for (k, bits) in fixed::COEFFICIENT_BITS.iter().enumerate() {
            let i = n * 4 + k;
            if i < reference.len() {
                reference[i] += input * f64::from_bits(*bits);
            }
        }
    }
    for (expected, actual) in reference.iter().zip(p.pcm().chunks_exact(4)) {
        assert_eq!((*expected as f32).to_le_bytes(), actual);
    }
}
#[test]
fn fixed_filter_passband_and_image_rejection_with_negative_control() {
    fn db(hz: f64, bits: &[u64]) -> f64 {
        let mut re = 0.0;
        let mut im = 0.0;
        for (k, b) in bits.iter().enumerate() {
            let w = std::f64::consts::TAU * hz * k as f64 / 48000.0;
            let c = f64::from_bits(*b) / 4.0;
            re += c * w.cos();
            im -= c * w.sin();
        }
        20.0 * (re * re + im * im).sqrt().log10()
    }
    for hz in (0..=5200).step_by(10) {
        assert!(db(hz as f64, &fixed::COEFFICIENT_BITS).abs() <= 0.01);
    }
    for hz in (6800..=24000).step_by(10) {
        assert!(db(hz as f64, &fixed::COEFFICIENT_BITS) <= -80.0);
    }
    let mut corrupt = fixed::COEFFICIENT_BITS;
    corrupt[256] = 4.0f64.to_bits();
    assert!(db(12000.0, &corrupt) > -80.0);
    let filter = fir::CandidateFilter::fixed_candidate().unwrap();
    assert_eq!(
        hex(&filter.sha256()),
        "b3b266e2346da0272cdb1910943a6ba8020d17a56e55a7b0eb0f12cd2c73c764"
    );
}
#[test]
fn no_tail_trimming_clipping_nonfinite_or_empty_acceptance() {
    let f = fir::CandidateFilter::fixed_candidate().unwrap();
    assert_eq!(fir::support(&[0.0, 1.0], 8), Err(Error::Tail));
    assert_eq!(fir::support(&[0.0, 0.0], 600), Err(Error::Empty));
    assert_eq!(fir::support(&[f32::NAN], 600), Err(Error::Sample));
    assert_eq!(fir::support(&[f32::INFINITY], 600), Err(Error::Sample));
    assert_eq!(render(&f, &[100.0], 600, deadline()), Err(Error::Amplitude));
    assert_eq!(render(&f, &[1.0], 600, Instant::now()), Err(Error::Expired));
    let p = render(&f, &[0.0, 0.5, 0.0], 600, deadline()).unwrap();
    assert!(p[..16].iter().all(|b| *b == 0));
    assert!(p[517 * 4..].iter().all(|b| *b == 0));
}
#[test]
fn gain_deadline_and_offset_admission_are_explicit() {
    let s = source(ModeKind::Ft8, "CQ K1ABC FN42", 1000.0);
    for g in [0.0, -0.1, 0.8001, f32::NAN, f32::INFINITY] {
        assert_eq!(prepare(&s, g, deadline()).err(), Some(Error::Gain));
    }
    assert_eq!(prepare(&s, 0.5, Instant::now()).err(), Some(Error::Expired));
    let low = prepare(&s, 0.25, deadline()).unwrap();
    let high = prepare(&s, 0.8, deadline()).unwrap();
    assert_ne!(low.pcm_sha256(), high.pcm_sha256());
    assert_ne!(low.content_sha256(), high.content_sha256());
    for mode in [ModeKind::Ft8, ModeKind::Ft4] {
        for offset in [99.0, 5000.0, f32::NAN] {
            assert!(xlog_ft_encode::prepare(mode, "CQ K1ABC FN42", offset, deadline()).is_err());
        }
    }
}
