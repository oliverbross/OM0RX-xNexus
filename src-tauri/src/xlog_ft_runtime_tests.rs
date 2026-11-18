use super::*;
use crate::xlog_decode::{CompleteFrame, Decoder};
use tempo_core::modes::Decode;
use xnexus_xlog_rx::{ClockProvenance, TimestampBasis};

fn context(mode: DecoderMode) -> Context {
    let owner = OwnerIdentity {
        server_instance: "server".into(),
        workspace_id: "workspace".into(),
        capture_id: "capture".into(),
        epoch: "epoch".into(),
        radio_id: "radio".into(),
        receiver_id: "A".into(),
        mapping_generation: 1,
        device_id: "device".into(),
        slice: "A".into(),
        dax_channel: 1,
        frequency_hz: 14_074_000,
        radio_mode: "DIGU".into(),
        sample_rate: 12_000,
        source_channels: 1,
        selected_channel: 0,
    };
    Context {
        connection_token: "connection".into(),
        station_identity_epoch: "saved-station".into(),
        transmit_context: "independently-commissioned-tx".into(),
        clock_epoch: "clock".into(),
        receive: DecodeContext {
            stream_id: owner.capture_id.clone(),
            epoch: owner.epoch.clone(),
            mapping_generation: 1,
            generation: 1,
            radio_id: owner.radio_id.clone(),
            receiver_id: owner.receiver_id.clone(),
            mode,
            dial_hz: owner.frequency_hz,
        },
        owner,
    }
}
fn settings() -> Settings {
    Settings {
        call: "K1ABC".into(),
        grid: "FN42".into(),
        tx_even: true,
        auto_sequence: true,
        cq_call_cap: Some(3),
        call_cap: Some(3),
        cq_pause: Duration::ZERO,
        watchdog: Duration::from_secs(300),
        disable_after_73: true,
    }
}
fn runtime(mode: DecoderMode) -> (Coordinator, Instant) {
    let now = Instant::now();
    (
        Coordinator::new(
            context(mode),
            settings(),
            now + Duration::from_secs(600),
            now,
        )
        .unwrap(),
        now,
    )
}
fn clock() -> ClockProvenance {
    ClockProvenance {
        basis: TimestampBasis::WindowsEndpointCapture,
        host_mapping_uncertainty_us: 10,
        calibration_uncertainty_us: 10,
        utc_sync_accuracy_us: None,
        remote_rf_latency_us: None,
        sample_alignment_offset_ns: 0,
        resampler_center_offset_ns: 0,
    }
}
fn period(c: &Context, start: u64, text: &str) -> NativeDecodedPeriod {
    NativeDecodedPeriod {
        context: c.receive.clone(),
        owner: c.owner.clone(),
        host_period_start_unix_us: start,
        frame_time_ms: 1000,
        clock: clock(),
        rows: vec![Decode {
            message: text.into(),
            sync: 1.0,
            snr: -10,
            dt: 0.0,
            freq: 1500.0,
            nap: 0,
            qual: 1.0,
            rv: None,
            mode: Some(c.mode()),
            raw: None,
            unchecked: false,
        }],
    }
}
enum Fixture {
    Ack,
    Start,
    NoStart,
    Released,
    Unknown,
    WrongTime,
}
impl OwnerEvents for Fixture {
    fn state(&self, op: &Operation, _: Instant) -> Result<OwnerState, Error> {
        Ok(match self {
            Self::Ack => OwnerState::Pending,
            Self::Start => OwnerState::Started(Started {
                event_id: "actual-start-1".into(),
                started_us: op.key.slot_start_us,
            }),
            Self::NoStart => OwnerState::NoStart,
            Self::Released => OwnerState::ObservedReleased,
            Self::Unknown => OwnerState::Unknown,
            Self::WrongTime => OwnerState::Started(Started {
                event_id: "wrong-time".into(),
                started_us: op.key.latest_start_us + 1,
            }),
        })
    }
}
fn plan_bind(c: &mut Coordinator, slot: u64, now: Instant) -> Plan {
    let p = c.plan(slot, slot + 500_000, slot - 100_000, now).unwrap();
    c.bind(
        &p,
        Operation {
            key: p.key.clone(),
            id: format!("op-{}", p.key.intent),
            body_digest: [1; 32],
        },
        now,
    )
    .unwrap();
    p
}
fn sent(c: &mut Coordinator, slot: u64, now: Instant) -> String {
    let p = plan_bind(c, slot, now);
    c.owner_event(&Fixture::Start, now).unwrap();
    c.owner_event(&Fixture::Released, now).unwrap();
    p.text
}

#[test]
fn initial_state_and_settings_changes_never_arm() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    assert!(!c.view().unwrap().armed);
    c.apply(Intent::Parity(false), n).unwrap();
    c.apply(Intent::AutoSequence(false), n).unwrap();
    assert!(!c.view().unwrap().armed);
    assert_eq!(
        c.plan(15_000_000, 15_500_000, 15_000_000, n).err(),
        Some(Error::Disarmed)
    );
}
#[test]
fn cq_ack_is_not_a_start_and_actual_start_counts_exactly_once() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.apply(Intent::Cq, n).unwrap();
    let p = plan_bind(&mut c, 30_000_000, n);
    assert_eq!(p.text(), "CQ K1ABC FN42");
    c.owner_event(&Fixture::Ack, n).unwrap();
    assert_eq!(c.view().unwrap().tx_count, 0);
    assert!(!c.view().unwrap().keyed);
    c.owner_event(&Fixture::Start, n).unwrap();
    c.owner_event(&Fixture::Start, n).unwrap();
    assert_eq!(c.view().unwrap().tx_count, 1);
    assert!(c.view().unwrap().keyed);
}
#[test]
fn complete_original_cq_exchange_in_both_modes() {
    for mode in [DecoderMode::Ft8, DecoderMode::Ft4] {
        let (mut c, n) = runtime(mode);
        let d = c.context.period_us();
        c.apply(Intent::Cq, n).unwrap();
        assert_eq!(sent(&mut c, 2 * d, n), "CQ K1ABC FN42");
        c.observe(&period(&c.context, 3 * d, "K1ABC W9XYZ EN37"), n)
            .unwrap();
        assert_eq!(sent(&mut c, 4 * d, n), "W9XYZ K1ABC -10");
        c.observe(&period(&c.context, 5 * d, "K1ABC W9XYZ R-12"), n)
            .unwrap();
        assert_eq!(sent(&mut c, 6 * d, n), "W9XYZ K1ABC RR73");
        c.observe(&period(&c.context, 7 * d, "K1ABC W9XYZ 73"), n)
            .unwrap();
        assert_eq!(c.view().unwrap().state, State::CallingCq);
        assert_eq!(sent(&mut c, 8 * d, n), "CQ K1ABC FN42");
    }
}
#[test]
fn work_context_resume_skip_tx1_and_final73_once() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.apply(
        Intent::Work {
            call: "W9XYZ".into(),
            heard: Some((Msg::parse("CQ W9XYZ EN37"), -12)),
            prefer_rrr: false,
            skip_tx1: true,
        },
        n,
    )
    .unwrap();
    assert_eq!(sent(&mut c, 30_000_000, n), "W9XYZ K1ABC -12");
    c.observe(&period(&c.context, 45_000_000, "K1ABC W9XYZ R-09"), n)
        .unwrap();
    assert_eq!(sent(&mut c, 60_000_000, n), "W9XYZ K1ABC RR73");
    assert!(!c.view().unwrap().armed);
    c.observe(&period(&c.context, 75_000_000, "K1ABC W9XYZ 73"), n)
        .unwrap();
    assert!(c.view().unwrap().done);
}
#[test]
fn manual_auto_off_keeps_message_until_operator_override() {
    let (mut c, n) = runtime(DecoderMode::Ft4);
    c.apply(Intent::Cq, n).unwrap();
    c.apply(Intent::AutoSequence(false), n).unwrap();
    c.observe(&period(&c.context, 7_500_000, "K1ABC W9XYZ EN37"), n)
        .unwrap();
    assert_eq!(c.view().unwrap().state, State::CallingCq);
    c.apply(Intent::Override("W9XYZ K1ABC -10".into()), n)
        .unwrap();
    assert_eq!(sent(&mut c, 15_000_000, n), "W9XYZ K1ABC -10");
}
#[test]
fn busy_decode_period_is_consumed_and_never_replayed() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.apply(Intent::Cq, n).unwrap();
    plan_bind(&mut c, 30_000_000, n);
    let rows = period(&c.context, 45_000_000, "K1ABC W9XYZ EN37");
    assert_eq!(c.observe(&rows, n), Err(Error::Busy));
    c.owner_event(&Fixture::Start, n).unwrap();
    c.owner_event(&Fixture::Released, n).unwrap();
    assert_eq!(c.observe(&rows, n), Err(Error::OldPeriod));
    assert_eq!(c.view().unwrap().state, State::CallingCq);
}
#[test]
fn parity_and_same_slot_fence_survive_operator_restart() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.apply(Intent::Cq, n).unwrap();
    assert_eq!(
        c.plan(15_000_000, 15_500_000, 15_000_000, n).err(),
        Some(Error::WrongParity)
    );
    sent(&mut c, 30_000_000, n);
    c.apply(Intent::Stop, n).unwrap();
    c.apply(Intent::Cq, n).unwrap();
    assert_eq!(
        c.plan(30_000_000, 30_500_000, 30_000_000, n).err(),
        Some(Error::SlotUsed)
    );
    c.apply(Intent::Parity(false), n).unwrap();
    assert!(c.plan(45_000_000, 45_500_000, 45_000_000, n).is_ok());
}
#[test]
fn independent_stop_wins_while_owner_verifier_runs() {
    struct Stops(StopHandle);
    impl OwnerEvents for Stops {
        fn state(&self, op: &Operation, _: Instant) -> Result<OwnerState, Error> {
            self.0.stop()?;
            Ok(OwnerState::Started(Started {
                event_id: "start".into(),
                started_us: op.key.slot_start_us,
            }))
        }
    }
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.apply(Intent::Cq, n).unwrap();
    plan_bind(&mut c, 30_000_000, n);
    let stop = c.stop_handle();
    let before = stop.release().unwrap().unwrap();
    assert_eq!(c.owner_event(&Stops(stop.clone()), n), Err(Error::Disarmed));
    assert_eq!(c.view().unwrap().tx_count, 0);
    assert_eq!(stop.release().unwrap().unwrap().as_ref(), before.as_ref());
    assert_eq!(c.apply(Intent::Cq, n), Err(Error::Busy));
    c.owner_event(&Fixture::Released, n).unwrap();
    c.apply(Intent::Cq, n).unwrap();
    assert!(c.view().unwrap().armed);
}
#[test]
fn wrong_time_start_is_not_accounted() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.apply(Intent::Cq, n).unwrap();
    plan_bind(&mut c, 30_000_000, n);
    assert_eq!(
        c.owner_event(&Fixture::WrongTime, n),
        Err(Error::WrongOperation)
    );
    assert_eq!(c.view().unwrap().tx_count, 0);
}
#[test]
fn context_changes_disarm_and_retain_exact_release() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.apply(Intent::Cq, n).unwrap();
    plan_bind(&mut c, 30_000_000, n);
    let old = c.stop.release().unwrap().unwrap();
    let mut changed = c.context.clone();
    changed.station_identity_epoch = "replacement".into();
    assert_eq!(
        c.refresh(&changed, n + Duration::from_secs(60), n),
        Err(Error::WrongContext)
    );
    assert!(!c.view().unwrap().armed);
    assert_eq!(c.stop.release().unwrap().unwrap().as_ref(), old.as_ref());
    assert!(c.owner_event(&Fixture::Start, n).is_err());
    assert_eq!(c.view().unwrap().tx_count, 0);
}
#[test]
fn no_start_and_unknown_are_distinct_and_neither_advance() {
    for proof in [Fixture::NoStart, Fixture::Unknown] {
        let (mut c, n) = runtime(DecoderMode::Ft4);
        c.apply(Intent::Cq, n).unwrap();
        plan_bind(&mut c, 15_000_000, n);
        c.owner_event(&proof, n).unwrap();
        let v = c.view().unwrap();
        assert_eq!(v.tx_count, 0);
        assert!(!v.armed);
        assert_eq!(v.release.is_some(), matches!(proof, Fixture::Unknown));
    }
}
#[test]
fn renewed_context_cannot_extend_frozen_preparation_deadline() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.apply(Intent::Cq, n).unwrap();
    let p = c.plan(30_000_000, 30_500_000, 30_000_000, n).unwrap();
    c.refresh(&c.context.clone(), n + Duration::from_secs(900), n)
        .unwrap();
    assert_eq!(
        c.bind(
            &p,
            Operation {
                key: p.key.clone(),
                id: "late".into(),
                body_digest: [1; 32]
            },
            n + Duration::from_secs(1)
        ),
        Err(Error::Expired)
    );
    assert!(!c.view().unwrap().armed);
}
#[test]
fn delayed_but_current_actual_start_can_arrive_after_start_window() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.apply(Intent::Cq, n).unwrap();
    plan_bind(&mut c, 30_000_000, n);
    c.owner_event(&Fixture::Start, n + Duration::from_secs(1))
        .unwrap();
    assert_eq!(c.view().unwrap().tx_count, 1);
}
#[test]
fn watchdog_stops_without_forgetting_release_obligation() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.apply(Intent::Cq, n).unwrap();
    plan_bind(&mut c, 30_000_000, n);
    assert_eq!(
        c.owner_event(&Fixture::Ack, n + Duration::from_secs(301)),
        Err(Error::Expired)
    );
    assert!(c.view().unwrap().release.is_some());
    assert!(!c.view().unwrap().armed);
}
#[test]
fn cq_budget_pause_and_resend_use_original_station_counters() {
    let (mut c, n) = runtime(DecoderMode::Ft4);
    c.settings.cq_call_cap = Some(1);
    c.settings.cq_pause = Duration::from_secs(3);
    c.apply(Intent::Cq, n).unwrap();
    sent(&mut c, 15_000_000, n);
    assert_eq!(
        c.plan(30_000_000, 30_500_000, 30_000_000, n).err(),
        Some(Error::BudgetSpent)
    );
    assert!(c
        .plan(
            45_000_000,
            45_500_000,
            45_000_000,
            n + Duration::from_secs(4)
        )
        .is_ok());
    c.retire(Halt::Stop).unwrap();
    c.apply(Intent::Resend, n + Duration::from_secs(5)).unwrap();
    assert_eq!(c.view().unwrap().tx_count, 0);
}
#[test]
fn preparation_and_message_changes_cannot_mutate_frozen_intent() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.apply(Intent::Cq, n).unwrap();
    let p = plan_bind(&mut c, 30_000_000, n);
    assert_eq!(
        c.apply(Intent::FreeText("TNX 73".into()), n),
        Err(Error::Busy)
    );
    assert_eq!(p.text(), "CQ K1ABC FN42");
    c.owner_event(&Fixture::Start, n).unwrap();
    c.owner_event(&Fixture::Released, n).unwrap();
    c.apply(Intent::FreeText("TNX 73".into()), n).unwrap();
    assert_eq!(sent(&mut c, 60_000_000, n), "TNX 73");
}
#[test]
fn actual_generated_pcm_decodes_into_original_station_in_both_modes() {
    for mode in [DecoderMode::Ft8, DecoderMode::Ft4] {
        let (mut c, n) = runtime(mode);
        c.apply(Intent::Cq, n).unwrap();
        let prepared = xlog_ft_encode::prepare(
            c.context.mode(),
            "K1ABC W9XYZ EN37",
            1500.0,
            Instant::now() + Duration::from_secs(20),
        )
        .unwrap();
        let frame = CompleteFrame {
            context: c.context.receive.clone(),
            owner: c.context.owner.clone(),
            host_period_start_unix_us: 2 * c.context.period_us(),
            frame_time_ms: 1000,
            clock: clock(),
            sample_rate: 12_000,
            channels: 1,
            samples: prepared
                .samples()
                .iter()
                .take(mode.frame_samples())
                .map(|x| (*x * 1000.0) as i16)
                .collect(),
        };
        let mut decoder = Decoder::default();
        let actual = decoder.decode_native(frame).unwrap();
        assert!(actual.rows.iter().any(|r| r.message.contains("W9XYZ")));
        c.observe(&actual, n).unwrap();
        assert_eq!(c.view().unwrap().state, State::AwaitRoger);
    }
}
#[test]
fn source_pcm_matches_existing_standard_preparer_and_full_period() {
    use xnexus_tx_prepare::{prepare, Encoding, Format, Limits};
    for mode in [ModeKind::Ft8, ModeKind::Ft4] {
        let p = xlog_ft_encode::prepare(
            mode,
            "CQ K1ABC FN42",
            1500.0,
            Instant::now() + Duration::from_secs(20),
        )
        .unwrap();
        let old = prepare(
            mode,
            "CQ K1ABC FN42",
            Format {
                sample_rate: 12_000,
                channels: 1,
                encoding: Encoding::F32Le,
            },
            1500.0,
            Limits {
                max_message_bytes: 37,
                max_frames: 180_000,
                max_bytes: 720_000,
                min_offset_hz: 100.0,
                max_offset_hz: 5000.0,
                max_abs_sample: 1.0,
            },
        )
        .unwrap();
        let bytes: Vec<u8> = p.samples().iter().flat_map(|v| v.to_le_bytes()).collect();
        assert_eq!(bytes, old.pcm());
        assert_eq!(p.waveform_frames(), old.waveform_frames());
        assert_eq!(p.mode(), mode);
        assert_eq!(p.text(), old.message());
        assert_eq!(p.offset_hz(), 1500.0);
    }
}
#[test]
fn original_free_text_and_compound_forms_encode_without_new_packer() {
    for mode in [ModeKind::Ft8, ModeKind::Ft4] {
        for text in ["TNX 73", "CQ K1ABC/P FN42", "W9XYZ K1ABC/P -10"] {
            let p = xlog_ft_encode::prepare(
                mode,
                text,
                1500.0,
                Instant::now() + Duration::from_secs(20),
            )
            .unwrap();
            assert_eq!(p.text(), text);
            assert!(p.samples().iter().any(|v| *v != 0.0));
        }
    }
}
#[test]
fn invalid_text_offset_and_expired_preparation_never_produce_pcm() {
    for text in [
        "THIS MESSAGE IS LONG FREE TEXT",
        "lower case",
        "TNX\n73",
        "",
        "W9XYZ K1ABC TNX 73",
        "<...> W9XYZ -10",
        "@1500",
        "5*3",
    ] {
        assert!(
            xlog_ft_encode::prepare(
                ModeKind::Ft8,
                text,
                1500.0,
                Instant::now() + Duration::from_secs(20)
            )
            .is_err(),
            "{text}"
        );
    }
    assert!(xlog_ft_encode::prepare(
        ModeKind::Ft4,
        "TNX 73",
        f32::NAN,
        Instant::now() + Duration::from_secs(20)
    )
    .is_err());
    assert!(matches!(
        xlog_ft_encode::prepare(
            ModeKind::Ft8,
            "TNX 73",
            1500.0,
            Instant::now() - Duration::from_secs(1)
        ),
        Err(xlog_ft_encode::Error::Expired)
    ));
}

#[test]
fn live_context_heartbeat_renews_admission_but_not_immutable_slot_deadline() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.authority_deadline = n + Duration::from_millis(900);
    c.apply(Intent::Cq, n).unwrap();
    let p = c.plan(30_000_000, 30_050_000, 25_000_000, n).unwrap();
    c.refresh(
        &c.context.clone(),
        n + Duration::from_secs(3),
        n + Duration::from_millis(800),
    )
    .unwrap();
    c.bind(
        &p,
        Operation {
            key: p.key.clone(),
            id: "bound".into(),
            body_digest: [9; 32],
        },
        n + Duration::from_secs(2),
    )
    .unwrap();
    c.refresh(
        &c.context.clone(),
        n + Duration::from_secs(99),
        n + Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(p.deadline, n + Duration::from_millis(5050));
    assert_eq!(
        c.bind(
            &p,
            Operation {
                key: p.key.clone(),
                id: "bound".into(),
                body_digest: [9; 32]
            },
            n + Duration::from_secs(6)
        ),
        Err(Error::Expired)
    );
}
#[test]
fn directed_cq_and_manual_target_keep_original_message_and_grid() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.apply(Intent::CqDirected("DX".into()), n).unwrap();
    assert_eq!(
        c.view().unwrap().pending_text.as_deref(),
        Some("CQ DX K1ABC FN42")
    );
    c.manual_target("W9XYZ", Some("EN50"), n).unwrap();
    assert_eq!(c.view().unwrap().dx_call.as_deref(), Some("W9XYZ"));
    assert_eq!(c.view().unwrap().dx_grid.as_deref(), Some("EN50"));
    assert_eq!(
        c.manual_target("W9XYZ", Some("bad"), n),
        Err(Error::Invalid)
    );
}

#[test]
fn future_disarm_before_start_preserves_correlated_start_and_release_without_rearming() {
    let (mut c, n) = runtime(DecoderMode::Ft4);
    c.apply(Intent::Cq, n).unwrap();
    plan_bind(&mut c, 15_000_000, n);
    c.apply(Intent::Enable(false), n).unwrap();
    assert!(!c.view().unwrap().armed);
    assert!(c.stop.release().unwrap().is_some());
    c.owner_event(&Fixture::Start, n).unwrap();
    c.owner_event(&Fixture::Start, n).unwrap();
    assert_eq!(c.view().unwrap().tx_count, 1);
    assert!(!c.view().unwrap().armed);
    c.owner_event(&Fixture::Released, n).unwrap();
    assert!(matches!(
        c.plan(30_000_000, 30_050_000, 29_000_000, n),
        Err(Error::Disarmed)
    ));
}
#[test]
fn future_disarm_after_start_finishes_but_explicit_stop_retires_pending_state() {
    let (mut c, n) = runtime(DecoderMode::Ft8);
    c.apply(Intent::Cq, n).unwrap();
    plan_bind(&mut c, 30_000_000, n);
    c.owner_event(&Fixture::Start, n).unwrap();
    c.apply(Intent::Enable(false), n).unwrap();
    assert!(c.pending.is_some());
    assert_eq!(c.view().unwrap().tx_count, 1);
    c.owner_event(&Fixture::Released, n).unwrap();
    assert!(!c.view().unwrap().armed);
    c.apply(Intent::Enable(true), n).unwrap();
    plan_bind(&mut c, 60_000_000, n);
    c.apply(Intent::Stop, n).unwrap();
    assert!(c.pending.is_none());
    assert!(c.owner_event(&Fixture::Start, n).is_err());
}
