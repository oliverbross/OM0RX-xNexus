use super::*;
use xnexus_tx_prepare::{prepare, Format, Limits};

fn period(mode: ModeKind, offset: f32) -> PreparedPeriod {
    prepare(
        mode,
        "CQ K1ABC FN42",
        Format {
            sample_rate: 12000,
            channels: 1,
            encoding: Encoding::F32Le,
        },
        offset,
        Limits {
            max_message_bytes: 37,
            max_frames: 180000,
            max_bytes: 720000,
            min_offset_hz: 0.0,
            max_offset_hz: 5900.0,
            max_abs_sample: 1.0,
        },
    )
    .unwrap()
}
fn raw(p: &PreparedPeriod) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"schema_version":1,
        "mode":if p.mode()==ModeKind::Ft8 {"FT8"} else {"FT4"}, "message_form":"standard_type1",
        "message":p.message(),"format":{"sample_rate_hz":12000,"channels":1,"encoding":"f32le"},
        "offset_hz_f32_bits":p.offset_hz().to_bits(),"frames":p.frames(),"waveform_frames":p.waveform_frames(),
        "pcm_byte_count":p.pcm().len(),"period_duration_us":if p.mode()==ModeKind::Ft8 {15000000} else {7500000},
        "pcm_sha256":hex(p.pcm_sha256()),"content_sha256":hex(p.content_sha256())})).unwrap()
}
fn policy() -> OutputPolicy {
    OutputPolicy {
        channels: [
            Channel::Source {
                gain_f32_bits: 0.125f32.to_bits(),
            },
            Channel::Silence,
        ],
        source_to_both_explicit: false,
        route_policy_identity: "OFFLINE-FIXTURE-NOT-COMMISSIONED".into(),
        max_abs_sample: 1.0,
        max_frames: 720000,
        max_bytes: 5760000,
    }
}
fn hash_bits(bits: &[u64; 513]) -> [u8; 32] {
    let mut h = Sha256::new();
    for b in bits {
        h.update(b.to_le_bytes());
    }
    h.finalize().into()
}
fn fixture_bits() -> [u64; 513] {
    // Deliberately NOT a qualified reconstruction filter. Numerical/guard tests
    // only; independent analyzer must reject this exact unity-impulse control.
    let mut bits = [0; 513];
    bits[256] = 4f64.to_bits();
    bits
}
fn fixture_filter() -> fir::CandidateFilter {
    let b = fixture_bits();
    fir::CandidateFilter::from_bits(b, hash_bits(&b)).unwrap()
}

#[test]
fn actual_prepared_source_and_all_manifest_fields_match() {
    let p = period(ModeKind::Ft8, 1000.0);
    let bytes = raw(&p);
    let source = BoundSource::bind(&p, &bytes).unwrap();
    assert_eq!(source.source_pcm(), p.pcm());
    assert_eq!(source.raw_manifest(), bytes);
    for key in [
        "schema_version",
        "mode",
        "message_form",
        "message",
        "format",
        "offset_hz_f32_bits",
        "frames",
        "waveform_frames",
        "pcm_byte_count",
        "period_duration_us",
        "pcm_sha256",
        "content_sha256",
    ] {
        let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        v[key] = serde_json::Value::Null;
        assert!(
            BoundSource::bind(&p, &serde_json::to_vec(&v).unwrap()).is_err(),
            "missing {key}"
        );
    }
    let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["waveform_frames"] = 1.into();
    assert!(matches!(
        BoundSource::bind(&p, &serde_json::to_vec(&v).unwrap()),
        Err(Error::SourceMismatch)
    ));
}
#[test]
fn unknown_duplicate_nested_and_oversize_manifests_refused() {
    let p = period(ModeKind::Ft8, 1000.0);
    let bytes = raw(&p);
    let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["format"]["extra"] = true.into();
    assert!(matches!(
        BoundSource::bind(&p, &serde_json::to_vec(&v).unwrap()),
        Err(Error::Manifest)
    ));
    let duplicate =
        String::from_utf8(bytes.clone())
            .unwrap()
            .replacen('{', "{\"schema_version\":1,", 1);
    assert!(matches!(
        BoundSource::bind(&p, duplicate.as_bytes()),
        Err(Error::Manifest)
    ));
    assert!(matches!(
        BoundSource::bind(&p, &vec![b' '; 4097]),
        Err(Error::Size)
    ));
}
#[test]
fn whitespace_is_retained_and_changes_source_whole_body_identity() {
    let p = period(ModeKind::Ft8, 1000.0);
    let bytes = raw(&p);
    let mut spaced = bytes.clone();
    spaced.push(b'\n');
    let a = BoundSource::bind(&p, &bytes).unwrap();
    let b = BoundSource::bind(&p, &spaced).unwrap();
    assert_eq!(a.source_pcm(), b.source_pcm());
    assert_ne!(a.whole_body_sha256(), b.whole_body_sha256());
    assert_eq!(b.raw_manifest(), spaced);
    let filter = fir::CandidateFilter::fixed_candidate().unwrap();
    let first = prepare_candidate(&a, &filter, policy()).unwrap();
    let second = prepare_candidate(&b, &filter, policy()).unwrap();
    assert_eq!(first.pcm(), second.pcm());
    assert_ne!(first.whole_body_sha256(), second.whole_body_sha256());
}
#[test]
fn different_actual_prepared_offset_cannot_bind_old_manifest() {
    let a = period(ModeKind::Ft8, 1000.0);
    let b = period(ModeKind::Ft8, 1001.0);
    assert!(matches!(
        BoundSource::bind(&b, &raw(&a)),
        Err(Error::SourceMismatch)
    ));
}
#[test]
fn conservative_support_uses_actual_nonzero_and_entire_tail() {
    let mut x = [0f32; 200];
    x[10] = 1.0;
    x[20] = -0.1;
    assert_eq!(fir::support(&x, 800).unwrap(), (40, 593));
    x[199] = 0.1;
    assert_eq!(fir::support(&x, 800), Err(Error::Tail));
}
#[test]
fn nonfinite_and_empty_input_refused_not_thresholded() {
    assert_eq!(fir::support(&[0.0, -0.0], 520), Err(Error::Empty));
    assert_eq!(fir::support(&[f32::NAN], 520), Err(Error::Sample));
    assert_eq!(fir::support(&[f32::INFINITY], 520), Err(Error::Sample));
    assert_eq!(fir::support(&[f32::from_bits(1)], 513).unwrap(), (0, 513));
}
#[test]
fn coefficient_identity_symmetry_finite_and_gain_are_enforced() {
    let fixed = fir::CandidateFilter::fixed_candidate().unwrap();
    assert_eq!(
        hex(&fixed.sha256()),
        "b3b266e2346da0272cdb1910943a6ba8020d17a56e55a7b0eb0f12cd2c73c764"
    );
    let good = fixture_bits();
    assert!(fir::CandidateFilter::from_bits(good, [0; 32]).is_err());
    let mut bad = good;
    bad[0] = 1f64.to_bits();
    assert!(fir::CandidateFilter::from_bits(bad, hash_bits(&bad)).is_err());
    bad = good;
    bad[256] = f64::NAN.to_bits();
    assert!(fir::CandidateFilter::from_bits(bad, hash_bits(&bad)).is_err());
    bad = good;
    bad[256] = 3f64.to_bits();
    assert!(fir::CandidateFilter::from_bits(bad, hash_bits(&bad)).is_err());
}
#[test]
fn ascending_polyphase_matches_independent_scatter_all_phases() {
    let mut b = [0u64; 513];
    b[252] = 0.5f64.to_bits();
    b[253] = 0.5f64.to_bits();
    b[254] = 0.5f64.to_bits();
    b[255] = 0.25f64.to_bits();
    b[256] = 0.5f64.to_bits();
    for n in 0..256 {
        b[512 - n] = b[n];
    }
    let filter = fir::CandidateFilter::from_bits(b, hash_bits(&b)).unwrap();
    let x = [0.3f32, -0.5, 1.0, -0.125, 0.0];
    let mut reference = vec![0f64; x.len() * 4 + 512];
    for (n, s) in x.iter().enumerate() {
        for (k, bits) in b.iter().enumerate() {
            reference[4 * n + k] += f64::from(*s) * f64::from_bits(*bits);
        }
    }
    for (m, want) in reference.iter().enumerate() {
        assert_eq!(filter.sample(&x, m).to_bits(), want.to_bits(), "frame {m}");
    }
}
#[test]
fn both_modes_exact_frames_bytes_channel_silence_and_hash() {
    for (mode, frames, bytes) in [
        (ModeKind::Ft8, 720000, 5760000),
        (ModeKind::Ft4, 360000, 2880000),
    ] {
        let p = period(mode, 1000.0);
        let s = BoundSource::bind(&p, &raw(&p)).unwrap();
        let out = prepare_candidate(
            &s,
            &fir::CandidateFilter::fixed_candidate().unwrap(),
            policy(),
        )
        .unwrap();
        assert_eq!(out.frames(), frames);
        assert_eq!(out.pcm().len(), bytes);
        assert_eq!(out.group_delay_frames(), 256);
        assert!(out.pcm().chunks_exact(8).all(|b| b[4..] == [0; 4]));
        assert_eq!(
            *out.pcm_sha256(),
            <[u8; 32]>::from(Sha256::digest(out.pcm()))
        );
        assert_eq!(out.source_body_sha256(), s.whole_body_sha256());
        assert_eq!(out.source_raw_manifest(), s.raw_manifest());
        assert_eq!(out.source_pcm(), p.pcm());
        assert_eq!(
            *out.whole_body_sha256(),
            body_hash(
                b"xnexus-offline-final-output-body-v1\0",
                out.raw_manifest(),
                out.pcm()
            )
        );
        let v: serde_json::Value = serde_json::from_slice(out.raw_manifest()).unwrap();
        assert_eq!(v["format"]["sample_rate_hz"], 48000);
        assert_eq!(v["transform"]["group_delay_output_frames"], 256);
        assert_eq!(v["artifact_kind"], "offline_final_output_candidate");
        assert_eq!(v["channel_map"][1]["kind"], "silence");
        assert_eq!(v["source"]["body_sha256"], hex(s.whole_body_sha256()));
        let (start, end) = out.support();
        assert!(out.pcm()[..start * 8].iter().all(|b| *b == 0));
        assert!(out.pcm()[end * 8..].iter().all(|b| *b == 0));
    }
}
#[test]
fn duplicated_source_requires_explicit_policy_and_silence_is_not_a_default() {
    let p = period(ModeKind::Ft4, 1000.0);
    let s = BoundSource::bind(&p, &raw(&p)).unwrap();
    let f = fixture_filter();
    let mut policy = policy();
    policy.channels[1] = policy.channels[0];
    assert!(matches!(
        prepare_candidate(&s, &f, policy),
        Err(Error::Policy)
    ));
    let mut policy = super::tests::policy();
    policy.channels = [Channel::Silence; 2];
    assert!(matches!(
        prepare_candidate(&s, &f, policy),
        Err(Error::Policy)
    ));
    let mut policy = super::tests::policy();
    policy.channels[1] = policy.channels[0];
    policy.source_to_both_explicit = true;
    let out = prepare_candidate(&s, &f, policy).unwrap();
    assert!(out.pcm().chunks_exact(8).all(|b| b[..4] == b[4..]));
}
#[test]
fn invalid_gains_peak_and_capacity_refuse_without_clamp() {
    let p = period(ModeKind::Ft4, 1000.0);
    let s = BoundSource::bind(&p, &raw(&p)).unwrap();
    let f = fixture_filter();
    for gain in [f32::NAN, f32::INFINITY, -0.1, 0.0, 1.01] {
        let mut pol = policy();
        pol.channels[0] = Channel::Source {
            gain_f32_bits: gain.to_bits(),
        };
        assert!(matches!(prepare_candidate(&s, &f, pol), Err(Error::Policy)));
    }
    let mut pol = policy();
    pol.channels[0] = Channel::Source {
        gain_f32_bits: 1f32.to_bits(),
    };
    assert!(matches!(
        prepare_candidate(&s, &f, pol),
        Err(Error::Amplitude)
    ));
    let mut pol = policy();
    pol.max_bytes = 2880000 - 1;
    assert!(matches!(prepare_candidate(&s, &f, pol), Err(Error::Size)));
}
#[test]
fn source_accepted_v1_but_tone_outside_new_passband_is_refused() {
    let p = period(ModeKind::Ft4, 5190.0);
    let s = BoundSource::bind(&p, &raw(&p)).unwrap();
    assert!(matches!(
        prepare_candidate(&s, &fixture_filter(), policy()),
        Err(Error::Offset)
    ));
    let p = period(ModeKind::Ft4, 5137.5);
    let s = BoundSource::bind(&p, &raw(&p)).unwrap();
    assert!(prepare_candidate(
        &s,
        &fir::CandidateFilter::fixed_candidate().unwrap(),
        policy()
    )
    .is_ok());
}
