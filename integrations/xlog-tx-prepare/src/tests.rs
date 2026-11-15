use super::*;

fn format() -> Format {
    Format {
        sample_rate: 12_000,
        channels: 1,
        encoding: Encoding::F32Le,
    }
}
// Disposable test policy only. These values confer no route approval.
fn limits() -> Limits {
    Limits {
        max_message_bytes: 37,
        max_frames: 180_000,
        max_bytes: 720_000,
        min_offset_hz: 200.0,
        max_offset_hz: 2900.0,
        max_abs_sample: 1.0,
    }
}
fn samples(p: &PreparedPeriod) -> Vec<f32> {
    p.pcm()
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
        .collect()
}

#[test]
fn actual_nexus_content_matches_independent_vendored_reference_without_extra_lead_in() {
    // The same explicit CQ/reply/report/closing forms used by the upstream
    // wsjt77 golden oracle. Fortran calls exist ONLY in this offline test.
    for mode in [ModeKind::Ft8, ModeKind::Ft4] {
        for message in [
            "CQ K1ABC FN42",
            "K1ABC W9XYZ EN37",
            "K1ABC W9XYZ -10",
            "K1ABC W9XYZ R-10",
            "K1ABC W9XYZ RR73",
        ] {
            let (pure, reference, bare, lead, period) = match mode {
                ModeKind::Ft8 => {
                    let reference = ft8::encode(message);
                    let wave = ft8::gen_wave(&reference, 12_000.0, 1500.0);
                    (
                        wsjt77::ft8_tones(message).unwrap(),
                        reference,
                        wave,
                        6000,
                        180_000,
                    )
                }
                ModeKind::Ft4 => {
                    let reference = ft4::encode(message);
                    let wave = ft4::gen_wave(&reference, 12_000.0, 1500.0);
                    (
                        wsjt77::ft4_tones(message).unwrap(),
                        reference,
                        wave,
                        3600,
                        90_000,
                    )
                }
                _ => unreachable!(),
            };
            assert_eq!(pure, reference, "actual golden tones {mode:?} {message}");
            let prepared = prepare(mode, message, format(), 1500.0, limits()).unwrap();
            let actual = samples(&prepared);
            assert_eq!(prepared.frames(), period);
            assert_eq!(actual.len(), period);
            assert_eq!(prepared.waveform_frames(), lead + bare.len());
            assert!(actual[..lead].iter().all(|v| v.to_bits() == 0));
            // Existing upstream golden waveform tolerance for libm/Fortran ramp
            // rounding. The adapter itself never alters a generated sample.
            for (a, b) in actual[lead..lead + bare.len()].iter().zip(&bare) {
                assert!((a - b).abs() <= 1e-6, "reference waveform difference");
            }
            assert!(actual[lead + bare.len()..].iter().all(|v| v.to_bits() == 0));
            let slot_wave = tx_mode(mode).unwrap().gen_wave(&pure, 12_000.0, 1500.0);
            for (sample, raw) in slot_wave.iter().zip(prepared.pcm().chunks_exact(4)) {
                assert_eq!(
                    sample.to_le_bytes(),
                    raw,
                    "no clipping/resampling/normalization"
                );
            }
        }
    }
}

#[test]
fn rejects_truncation_nul_unicode_whitespace_and_known_bad_frames() {
    for mode in [ModeKind::Ft8, ModeKind::Ft4] {
        for message in [
            "",
            " CQ K1ABC FN42",
            "CQ K1ABC FN42 ",
            "CQ K1ABC\0 FN42",
            "CQ K1ABC\nFN42",
            "CQ K1ABC ÉM12",
            "CQ K1ABC FN42 trailing text beyond thirty seven bytes",
            "cq K1ABC FN42",
            "CQ  K1ABC FN42",
        ] {
            assert!(matches!(
                prepare(mode, message, format(), 1500.0, limits()),
                Err(Error::InvalidMessage)
            ));
        }
        for message in [
            "CQ K1ABC R FN42",
            "CQ K1ABC RRR",
            "CQ K1ABC -10",
            "CQ <K1ABC> FN42",
            "@@@",
        ] {
            assert!(matches!(
                prepare(mode, message, format(), 1500.0, limits()),
                Err(Error::UnsupportedMessageForm)
            ));
        }
        // This unresolved hash requires process-global state; no fallback.
        assert!(prepare(mode, "<...> W9XYZ -10", format(), 1500.0, limits()).is_err());
    }
}

#[test]
fn successful_packing_does_not_prove_text_fidelity_and_adapter_excludes_lossy_forms() {
    let pack = |text: &str| {
        let mut padded = [b' '; 37];
        padded[..text.len()].copy_from_slice(text.as_bytes());
        wsjt77::pack::pack(&padded).unwrap()
    };
    // Actual pure packer behavior, not an invented encoder fixture: a valid
    // nonempty frame can discard the suffix or map unsupported chars to spaces.
    assert!(pack("HELLOTHEREABC").is_some());
    assert_eq!(pack("HELLOTHEREABC"), pack("HELLOTHEREABCDEF"));
    // Internal substitution preserves right-alignment; a trailing @ does not.
    assert_eq!(pack("HELLO@WORLD"), pack("HELLO WORLD"));
    assert!(wsjt77::ft8_refusal("@@@").is_some()); // all spaces => bad message
    assert!(wsjt77::ft4_refusal("@@@").is_some());
    for mode in [ModeKind::Ft8, ModeKind::Ft4] {
        for text in [
            "HELLOTHEREABCDEF",
            "HELLO@",
            "HELLO",
            "K1ABCDEFGHIJKLM W9XYZ -10",
            "K1ABC/P W9XYZ -10",
            "CQ DX K1ABC FN42",
            "K1ABC W9XYZ -00",
            "K1ABC W9XYZ -31",
            "K1ABC W9XYZ +50",
            "K1ABC W9XYZ +1",
            "K1ABC W9XYZ +001",
            "K1ABC W9XYZ 1,2",
            "K1ABC W9XYZ R-00",
        ] {
            assert!(text.len() <= 37);
            assert!(
                matches!(
                    prepare(mode, text, format(), 1500.0, limits()),
                    Err(Error::UnsupportedMessageForm)
                ),
                "must not silently alter {text}"
            );
        }
    }
    for text in [
        "CQ K1ABC FN42",
        "K1ABC W9XYZ EN37",
        "K1ABC W9XYZ -30",
        "K1ABC W9XYZ +00",
        "K1ABC W9XYZ +49",
        "K1ABC W9XYZ R-30",
        "K1ABC W9XYZ R+00",
        "K1ABC W9XYZ R+49",
        "K1ABC W9XYZ RRR",
        "K1ABC W9XYZ RR73",
        "K1ABC W9XYZ 73",
    ] {
        assert_eq!(
            supported_message(text),
            Ok(()),
            "supported exact form {text}"
        );
    }
}

#[test]
fn caller_limits_and_unsupported_formats_fail_before_any_waveform_generation() {
    for bad in [
        Format {
            sample_rate: 48_000,
            ..format()
        },
        Format {
            sample_rate: 0,
            ..format()
        },
        Format {
            channels: 2,
            ..format()
        },
        Format {
            encoding: Encoding::S16Le,
            ..format()
        },
    ] {
        assert!(matches!(
            prepare(ModeKind::Ft8, "CQ K1ABC FN42", bad, 1500.0, limits()),
            Err(Error::UnsupportedFormat)
        ));
    }
    assert!(matches!(
        prepare(ModeKind::Wspr, "CQ K1ABC FN42", format(), 1500.0, limits()),
        Err(Error::UnsupportedMode)
    ));
    for policy in [
        Limits {
            max_frames: 179_999,
            ..limits()
        },
        Limits {
            max_bytes: 719_999,
            ..limits()
        },
    ] {
        assert!(matches!(
            prepare(ModeKind::Ft8, "CQ K1ABC FN42", format(), 1500.0, policy),
            Err(Error::SizeLimit)
        ));
    }
    for offset in [f32::NAN, f32::INFINITY, -1.0, 199.0, 2901.0] {
        assert!(matches!(
            prepare(ModeKind::Ft8, "CQ K1ABC FN42", format(), offset, limits()),
            Err(Error::InvalidOffset)
        ));
    }
    assert!(matches!(
        prepare(
            ModeKind::Ft8,
            "CQ K1ABC FN42",
            format(),
            1500.0,
            Limits {
                max_abs_sample: f32::NAN,
                ..limits()
            }
        ),
        Err(Error::InvalidLimits)
    ));
    assert!(matches!(
        prepare(
            ModeKind::Ft8,
            "CQ K1ABC FN42",
            format(),
            1500.0,
            Limits {
                max_abs_sample: 0.01,
                ..limits()
            }
        ),
        Err(Error::AmplitudeLimit)
    ));
}

#[test]
fn rejects_nonfinite_incomplete_overflowing_or_shifted_generator_output_without_repair() {
    let mode = ModeKind::Ft4;
    let tones = wsjt77::ft4_tones("CQ K1ABC FN42").unwrap();
    let good = tx_mode(mode).unwrap().gen_wave(&tones, 12_000.0, 1500.0);
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut bad = good.clone();
        bad[4000] = value;
        assert!(matches!(
            finish(
                mode,
                "CQ K1ABC FN42",
                format(),
                1500.0,
                limits(),
                90_000,
                bad
            ),
            Err(Error::InvalidWaveform)
        ));
    }
    let mut shifted = good.clone();
    shifted[0] = 0.1;
    for bad in [
        Vec::new(),
        vec![0.0; good.len()],
        good[..4000].to_vec(),
        vec![0.1; 90_001],
        shifted,
    ] {
        assert!(matches!(
            finish(
                mode,
                "CQ K1ABC FN42",
                format(),
                1500.0,
                limits(),
                90_000,
                bad
            ),
            Err(Error::InvalidWaveform)
        ));
    }
    assert!(matches!(
        finish(
            mode,
            "CQ K1ABC FN42",
            format(),
            1500.0,
            Limits {
                max_frames: usize::MAX,
                max_bytes: usize::MAX,
                ..limits()
            },
            usize::MAX,
            good
        ),
        Err(Error::SizeLimit)
    ));
}

#[test]
fn immutable_digests_bind_final_padded_bytes_and_local_content_identity() {
    let a = prepare(ModeKind::Ft4, "CQ K1ABC FN42", format(), 1500.0, limits()).unwrap();
    let b = prepare(ModeKind::Ft4, "CQ K1ABC FN42", format(), 1500.0, limits()).unwrap();
    assert_eq!(a.pcm(), b.pcm());
    assert_eq!(a.pcm_sha256(), b.pcm_sha256());
    assert_eq!(a.content_sha256(), b.content_sha256());
    let independently_hashed: [u8; 32] = Sha256::digest(a.pcm()).into();
    assert_eq!(a.pcm_sha256(), &independently_hashed);
    let prefix_hash: [u8; 32] = Sha256::digest(&a.pcm()[..a.waveform_frames() * 4]).into();
    assert_ne!(
        a.pcm_sha256(),
        &prefix_hash,
        "digest must include deliberate trailing silence"
    );
    let offset = prepare(ModeKind::Ft4, "CQ K1ABC FN42", format(), 1510.0, limits()).unwrap();
    let text = prepare(ModeKind::Ft4, "CQ W9XYZ EN37", format(), 1500.0, limits()).unwrap();
    assert_ne!(a.content_sha256(), offset.content_sha256());
    assert_ne!(a.content_sha256(), text.content_sha256());
    let mut caller_copy = a.pcm().to_vec();
    caller_copy[0] ^= 1;
    assert_ne!(caller_copy.as_slice(), a.pcm());
    assert_eq!(a.pcm_sha256(), &independently_hashed);
}
