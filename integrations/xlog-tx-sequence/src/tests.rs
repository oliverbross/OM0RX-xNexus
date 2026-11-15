use super::*;
use xnexus_tx_prepare::Encoding;

fn context(mode: ModeKind) -> Context {
    Context {
        owner_session: "fake-owner-session".into(),
        receive_origin: "fake-capture/mapping/receiver/dial".into(),
        transmit_context: "independent-fake-TX-route".into(),
        clock_epoch: "fake-clock".into(),
        mode,
    }
}
fn settings(call: &str) -> OperatorSettings {
    OperatorSettings {
        mycall: call.into(),
        mygrid: "FN42".into(),
        cq_call_cap: Some(2),
        call_cap: Some(2),
    }
}
fn format() -> Format {
    Format {
        sample_rate: 12000,
        channels: 1,
        encoding: Encoding::F32Le,
    }
}
fn limits() -> Limits {
    Limits {
        max_message_bytes: 37,
        max_frames: 180000,
        max_bytes: 720000,
        min_offset_hz: 0.0,
        max_offset_hz: 5000.0,
        max_abs_sample: 1.0,
    }
}
fn start(mode: ModeKind) -> Adapter {
    let mut a = Adapter::new();
    a.begin(context(mode), settings("K1ABC"), OperatorIntent::Cq)
        .unwrap();
    a
}
fn prepare_at(a: &mut Adapter, slot: u64) -> PreparedIntent {
    a.prepare_slot(slot, slot + 100_000, format(), 1000.0, limits())
        .unwrap()
}
fn operation(p: &PreparedIntent) -> Arc<OfflineOperation> {
    // Deliberately opaque stand-ins: NOT claimed to be owner-valid manifests.
    Arc::new(
        OfflineOperation::from_offline_fixture(
            p,
            "fake-operation".into(),
            b"exact fake owner body\n".to_vec(),
            b"exact fake manifest\n".to_vec(),
            p.period().pcm().to_vec(),
        )
        .unwrap(),
    )
}
fn bind(a: &mut Adapter, p: &PreparedIntent) -> Arc<OfflineOperation> {
    let op = operation(p);
    a.bind_offline(op.clone()).unwrap();
    op
}
fn fake_start(a: &mut Adapter, op: &Arc<OfflineOperation>) {
    let t = op.correlation().slot_start_us;
    a.fake_actual_start(FakeActualStart {
        operation: op.clone(),
        started_us: t,
        observed_us: t,
    })
    .unwrap();
}
fn decoded(mode: ModeKind, text: &str) -> Decode {
    // Explicit generated test values, never promoted as actual DSP measurements.
    Decode {
        message: text.into(),
        sync: 1.0,
        snr: -10,
        dt: 0.0,
        freq: 1000.0,
        nap: 0,
        qual: 1.0,
        rv: None,
        mode: Some(mode),
        raw: None,
    }
}
fn batch(a: &Adapter, period: u64, text: &str) -> CompleteDecodeBatch {
    let c = a.context.clone().unwrap();
    CompleteDecodeBatch {
        epoch: a.epoch,
        context: c.clone(),
        period_start_us: period,
        rows: vec![decoded(c.mode, text)],
    }
}
fn sent(a: &mut Adapter, slot: u64) -> PreparedIntent {
    let p = prepare_at(a, slot);
    let op = bind(a, &p);
    fake_start(a, &op);
    a.acknowledge_fake_release(&op).unwrap();
    p
}

#[test]
fn new_process_is_disarmed_and_does_not_recover_operations() {
    let mut a = Adapter::new();
    assert!(!a.view().armed);
    assert!(a.release_identity().is_none());
    assert_eq!(
        a.prepare_slot(15_000_000, 15_100_000, format(), 1000.0, limits())
            .unwrap_err(),
        Error::Disarmed
    );
}
#[test]
fn preparation_starting_permit_ack_do_not_advance_station() {
    let mut a = start(ModeKind::Ft8);
    let before = a.view();
    let p = prepare_at(&mut a, 15_000_000);
    let op = bind(&mut a, &p);
    for event in [NonStart::Starting, NonStart::Permit, NonStart::Ack] {
        a.non_start(&op, event).unwrap();
    }
    let view = a.view();
    assert_eq!(view.tx_count, before.tx_count);
    assert!(!view.keyed);
    assert_eq!(view.pending_text, before.pending_text);
    assert_eq!(p.period().pcm().len(), 720000);
    assert_eq!(p.period().message(), p.message().to_text());
}
#[test]
fn fake_start_applies_frozen_station_once_and_requires_exact_release() {
    let mut a = start(ModeKind::Ft8);
    let p = prepare_at(&mut a, 15_000_000);
    let op = bind(&mut a, &p);
    fake_start(&mut a, &op);
    assert_eq!(a.view().tx_count, 1);
    assert!(a.view().keyed);
    assert_eq!(
        a.fake_actual_start(FakeActualStart {
            operation: op.clone(),
            started_us: 15_000_000,
            observed_us: 15_000_000
        }),
        Err(Error::WrongOperation)
    );
    assert_eq!(a.view().tx_count, 1);
    assert_eq!(
        a.prepare_slot(45_000_000, 45_100_000, format(), 1000.0, limits())
            .unwrap_err(),
        Error::ReleaseOutstanding
    );
    let mut wrong = (*op).clone();
    wrong.owner_body.push(b' ');
    assert_eq!(
        a.acknowledge_fake_release(&wrong),
        Err(Error::WrongOperation)
    );
    assert_eq!(a.release_identity().unwrap(), op);
    a.acknowledge_fake_release(&op).unwrap();
}
#[test]
fn busy_period_is_permanently_dropped_not_replayed_after_start() {
    let mut a = start(ModeKind::Ft8);
    let p = prepare_at(&mut a, 15_000_000);
    let op = bind(&mut a, &p);
    let before = a.view().pending_text;
    let input = batch(&a, 30_000_000, "K1ABC W9XYZ EN37");
    assert_eq!(a.observe(input), Err(Error::BusyDropped));
    fake_start(&mut a, &op);
    a.acknowledge_fake_release(&op).unwrap();
    assert_eq!(
        a.observe(batch(&a, 30_000_000, "K1ABC W9XYZ EN37")),
        Err(Error::DuplicatePeriod)
    );
    assert_eq!(a.view().pending_text, before);
    assert_eq!(a.view().tx_count, 1);
    a.observe(batch(&a, 60_000_000, "K1ABC W9XYZ EN37"))
        .unwrap();
    assert_eq!(a.view().state, Some(State::AwaitRoger));
}
#[test]
fn duplicate_conflicting_and_older_periods_never_observe_twice() {
    let mut a = start(ModeKind::Ft8);
    a.observe(batch(&a, 15_000_000, "K1ABC W9XYZ EN37"))
        .unwrap();
    let before = a.view();
    assert_eq!(
        a.observe(batch(&a, 15_000_000, "K1ABC W9XYZ EN37")),
        Err(Error::DuplicatePeriod)
    );
    assert_eq!(
        a.observe(batch(&a, 15_000_000, "K1ABC W9XYZ R-10")),
        Err(Error::ConflictingPeriod)
    );
    a.observe(batch(&a, 45_000_000, "CQ K2DEF FN31")).unwrap();
    assert_eq!(
        a.observe(batch(&a, 30_000_000, "K1ABC W9XYZ R-10")),
        Err(Error::OldPeriod)
    );
    assert_eq!(a.view().state, before.state);
    assert_eq!(a.view().pending_text, before.pending_text);
}
#[test]
fn stopped_pending_retains_release_and_blocks_replacement() {
    let mut a = start(ModeKind::Ft8);
    let p = prepare_at(&mut a, 15_000_000);
    let op = bind(&mut a, &p);
    assert_eq!(a.halt(Halt::Stop).unwrap(), op);
    assert_eq!(
        a.begin(
            context(ModeKind::Ft8),
            settings("W9XYZ"),
            OperatorIntent::Cq
        ),
        Err(Error::ReleaseOutstanding)
    );
    assert_eq!(a.release_identity().unwrap(), op);
    assert!(!a.view().keyed);
    assert_eq!(
        a.fake_actual_start(FakeActualStart {
            operation: op.clone(),
            started_us: 15_000_000,
            observed_us: 15_000_000
        }),
        Err(Error::Disarmed)
    );
    a.acknowledge_fake_release(&op).unwrap();
    a.begin(
        context(ModeKind::Ft8),
        settings("W9XYZ"),
        OperatorIntent::Cq,
    )
    .unwrap();
    assert_eq!(
        a.prepare_slot(15_000_000, 15_100_000, format(), 1000.0, limits())
            .unwrap_err(),
        Error::SlotUsed
    );
}
#[test]
fn stop_after_start_preserves_last_started_identity_and_count() {
    let mut a = start(ModeKind::Ft4);
    let p = prepare_at(&mut a, 7_500_000);
    let op = bind(&mut a, &p);
    fake_start(&mut a, &op);
    assert_eq!(a.halt(Halt::Stop).unwrap(), op);
    assert_eq!(a.view().tx_count, 1);
    assert_eq!(a.halt(Halt::ContextRetired).unwrap(), op);
}
#[test]
fn refusal_and_unknown_halt_without_count_or_automatic_retry() {
    for unknown in [false, true] {
        let mut a = start(ModeKind::Ft8);
        let p = prepare_at(&mut a, 15_000_000);
        let op = bind(&mut a, &p);
        assert_eq!(a.refuse_or_unknown(&op, unknown).unwrap(), op);
        assert_eq!(a.view().tx_count, 0);
        assert!(!a.view().armed);
        assert!(!a.view().keyed);
        a.acknowledge_fake_release(&op).unwrap();
        assert_eq!(
            a.prepare_slot(45_000_000, 45_100_000, format(), 1000.0, limits())
                .unwrap_err(),
            Error::Disarmed
        );
    }
}
#[test]
fn late_fake_start_halts_and_preserves_exact_old_operation() {
    let mut a = start(ModeKind::Ft8);
    let p = prepare_at(&mut a, 15_000_000);
    let op = bind(&mut a, &p);
    assert_eq!(
        a.fake_actual_start(FakeActualStart {
            operation: op.clone(),
            started_us: 15_000_000,
            observed_us: 15_100_001
        }),
        Err(Error::Expired)
    );
    assert_eq!(a.release_identity().unwrap(), op);
    assert_eq!(a.view().tx_count, 0);
    assert!(!a.view().armed);
}
#[test]
fn explicit_expiry_before_binding_also_disarms_without_catchup() {
    let mut a = start(ModeKind::Ft8);
    prepare_at(&mut a, 15_000_000);
    assert!(a.expire(15_100_001).is_none());
    assert!(!a.view().armed);
    assert_eq!(a.view().tx_count, 0);
}
#[test]
fn exact_body_manifest_pcm_and_coordinates_are_all_compared() {
    let mut a = start(ModeKind::Ft8);
    let p = prepare_at(&mut a, 15_000_000);
    let op = bind(&mut a, &p);
    for field in 0..8 {
        let mut wrong = (*op).clone();
        match field {
            0 => wrong.owner_body.push(b' '),
            1 => wrong.manifest.push(b' '),
            2 => wrong.pcm[0] ^= 1,
            3 => wrong.operation_id.push('x'),
            4 => wrong.correlation.intent += 1,
            5 => wrong.correlation.context.clock_epoch.push('x'),
            6 => wrong.correlation.epoch.qso += 1,
            _ => wrong.correlation.latest_start_us += 1,
        }
        assert_eq!(
            a.fake_actual_start(FakeActualStart {
                operation: Arc::new(wrong),
                started_us: 15_000_000,
                observed_us: 15_000_000
            }),
            Err(Error::WrongOperation)
        );
        assert_eq!(a.view().tx_count, 0);
    }
    fake_start(&mut a, &op);
    assert_eq!(a.view().tx_count, 1);
}
#[test]
fn a_b_a_context_and_qso_epochs_never_resurrect_old_input() {
    let mut a = start(ModeKind::Ft8);
    let old = batch(&a, 15_000_000, "K1ABC W9XYZ EN37");
    let first = a.view().epoch;
    let mut b = context(ModeKind::Ft8);
    b.receive_origin.push('B');
    a.begin(b, settings("K1ABC"), OperatorIntent::Cq).unwrap();
    a.begin(
        context(ModeKind::Ft8),
        settings("K1ABC"),
        OperatorIntent::Cq,
    )
    .unwrap();
    assert!(a.view().epoch.context > first.context);
    assert!(a.view().epoch.qso > first.qso);
    assert_eq!(a.observe(old), Err(Error::WrongContext));
}
#[test]
fn unsupported_mode_and_malformed_batches_fail_closed() {
    let mut a = start(ModeKind::Ft8);
    let other = ModeKind::Fst4 {
        period_s: 15,
        wspr: false,
    };
    assert_eq!(period_us(other), Err(Error::UnsupportedMode));
    for case in 0..7 {
        let mut input = batch(&a, 15_000_000, "K1ABC W9XYZ EN37");
        match case {
            0 => input.rows[0].mode = Some(ModeKind::Ft4),
            1 => input.rows[0].dt = f32::NAN,
            2 => input.rows[0].nap = 1,
            3 => input.rows[0].message = "A".repeat(MAX_TEXT + 1),
            4 => input.period_start_us += 1,
            5 => input.rows = vec![input.rows[0].clone(); MAX_ROWS + 1],
            _ => input.rows[0].rv = Some(0),
        }
        assert_eq!(a.observe(input), Err(Error::InvalidInput));
    }
    assert!(a.seen.is_none());
    assert_eq!(a.view().state, Some(State::CallingCq));
}
#[test]
fn preparation_refusal_consumes_slot_without_after_tx() {
    let mut a = start(ModeKind::Ft8);
    let bad = Format {
        encoding: Encoding::S16Le,
        ..format()
    };
    assert!(matches!(
        a.prepare_slot(15_000_000, 15_100_000, bad, 1000.0, limits()),
        Err(Error::Preparation(_))
    ));
    assert!(!a.view().armed);
    assert!(!a.view().keyed);
    a.begin(
        context(ModeKind::Ft8),
        settings("K1ABC"),
        OperatorIntent::Cq,
    )
    .unwrap();
    assert_eq!(
        a.prepare_slot(15_000_000, 15_100_000, format(), 1000.0, limits())
            .unwrap_err(),
        Error::SlotUsed
    );
}
#[test]
fn mixed_mode_reset_cannot_overlap_reserved_ft8_period() {
    let mut a = start(ModeKind::Ft8);
    prepare_at(&mut a, 15_000_000);
    a.halt(Halt::Stop);
    a.begin(
        context(ModeKind::Ft4),
        settings("K1ABC"),
        OperatorIntent::Cq,
    )
    .unwrap();
    assert_eq!(
        a.prepare_slot(22_500_000, 22_600_000, format(), 1000.0, limits())
            .unwrap_err(),
        Error::SlotUsed
    );
    prepare_at(&mut a, 30_000_000);
}
#[test]
fn cq_and_directed_budgets_are_actual_station_policy() {
    let mut cq = start(ModeKind::Ft8);
    sent(&mut cq, 15_000_000);
    sent(&mut cq, 45_000_000);
    assert_eq!(
        cq.prepare_slot(75_000_000, 75_100_000, format(), 1000.0, limits())
            .unwrap_err(),
        Error::BudgetSpent
    );
    let mut directed = start(ModeKind::Ft8);
    directed
        .observe(batch(&directed, 15_000_000, "K1ABC W9XYZ EN37"))
        .unwrap();
    sent(&mut directed, 30_000_000);
    sent(&mut directed, 60_000_000);
    assert_eq!(
        directed
            .prepare_slot(90_000_000, 90_100_000, format(), 1000.0, limits())
            .unwrap_err(),
        Error::BudgetSpent
    );
}
#[test]
fn explicitly_selected_await_report_is_not_silently_call_capped() {
    let mut a = Adapter::new();
    a.begin(
        context(ModeKind::Ft8),
        settings("K1ABC"),
        OperatorIntent::Call {
            dxcall: "W9XYZ".into(),
            context: None,
            prefer_rrr: false,
            skip_tx1: false,
        },
    )
    .unwrap();
    for i in 1..=3 {
        sent(&mut a, i * 30_000_000);
    }
    assert_eq!(a.view().state, Some(State::AwaitReport));
    assert_eq!(a.view().tx_count, 3);
}
#[test]
fn rr73_is_once_but_rrr_stays_pending() {
    for prefer_rrr in [false, true] {
        let mut a = Adapter::new();
        let msg = Msg::parse("K1ABC W9XYZ R-10");
        a.begin(
            context(ModeKind::Ft8),
            settings("K1ABC"),
            OperatorIntent::Call {
                dxcall: "W9XYZ".into(),
                context: Some((msg, -10)),
                prefer_rrr,
                skip_tx1: false,
            },
        )
        .unwrap();
        let p = sent(&mut a, 15_000_000);
        assert_eq!(a.view().state, Some(State::Confirming));
        if prefer_rrr {
            assert!(p.period().message().ends_with(" RRR"));
            assert!(a.view().pending_text.is_some());
            assert!(!a.view().closing_sent);
        } else {
            assert!(p.period().message().ends_with(" RR73"));
            assert!(a.view().pending_text.is_none());
            assert!(a.view().closing_sent);
        }
    }
}
#[test]
fn actual_station_two_party_exchanges_for_ft8_and_ft4() {
    for mode in [ModeKind::Ft8, ModeKind::Ft4] {
        let duration = period_us(mode).unwrap();
        let mut a = start(mode);
        let mut b = Adapter::new();
        b.begin(
            context(mode),
            settings("W9XYZ"),
            OperatorIntent::Call {
                dxcall: "K1ABC".into(),
                context: Some((Msg::parse("CQ K1ABC FN42"), -10)),
                prefer_rrr: false,
                skip_tx1: false,
            },
        )
        .unwrap();
        let mut transcript = Vec::new();
        for slot in 1..=8 {
            let (tx, rx) = if slot % 2 == 1 {
                (&mut a, &mut b)
            } else {
                (&mut b, &mut a)
            };
            if tx.station.as_ref().unwrap().outgoing_rv().is_none() {
                continue;
            }
            let p = sent(tx, slot * duration);
            transcript.push(p.period().message().to_string());
            rx.observe(batch(rx, slot * duration, p.period().message()))
                .unwrap();
        }
        assert!(
            transcript.iter().any(|t| t.ends_with(" RR73")),
            "{transcript:?}"
        );
        assert!(
            transcript.iter().any(|t| t.ends_with(" 73")),
            "{transcript:?}"
        );
        assert!(a.view().done && b.view().done, "{transcript:?}");
        assert_eq!(
            transcript.iter().filter(|t| t.ends_with(" RR73")).count(),
            1
        );
        assert_eq!(transcript.iter().filter(|t| t.ends_with(" 73")).count(), 1);
    }
}
#[test]
fn counters_and_event_bound_halt_without_wrap() {
    let mut a = start(ModeKind::Ft8);
    a.intent_counter = u64::MAX;
    assert_eq!(
        a.prepare_slot(15_000_000, 15_100_000, format(), 1000.0, limits())
            .unwrap_err(),
        Error::Exhausted
    );
    assert!(!a.view().armed);
    a.epoch.qso = u64::MAX;
    assert_eq!(
        a.begin(
            context(ModeKind::Ft8),
            settings("K1ABC"),
            OperatorIntent::Cq
        ),
        Err(Error::Exhausted)
    );
    let mut b = start(ModeKind::Ft8);
    let p = prepare_at(&mut b, 15_000_000);
    let op = bind(&mut b, &p);
    b.session_events = MAX_SESSION_EVENTS;
    assert_eq!(b.non_start(&op, NonStart::Ack), Err(Error::Exhausted));
    assert_eq!(b.release_identity().unwrap(), op);
}
#[test]
fn transcript_is_drained_and_diagnostic_storage_bounded() {
    let mut a = start(ModeKind::Ft8);
    a.station.as_mut().unwrap().transcript = vec!["x".repeat(2048); 100];
    a.diagnostics();
    assert!(a.station.as_ref().unwrap().transcript.is_empty());
    assert_eq!(a.diagnostics.len(), MAX_DIAGNOSTICS);
    assert!(a.diagnostics.iter().all(|line| line.len() <= 1024));
}
#[test]
fn offline_constructor_preserves_raw_bytes_and_refuses_size_or_pcm_change() {
    let mut a = start(ModeKind::Ft8);
    let p = prepare_at(&mut a, 15_000_000);
    let op = operation(&p);
    assert_eq!(op.owner_body(), b"exact fake owner body\n");
    assert_eq!(op.manifest(), b"exact fake manifest\n");
    let mut wrong = p.period().pcm().to_vec();
    wrong[0] ^= 1;
    assert!(
        OfflineOperation::from_offline_fixture(&p, "op".into(), vec![1], vec![1], wrong).is_err()
    );
    assert!(OfflineOperation::from_offline_fixture(
        &p,
        "op".into(),
        vec![1; MAX_BODY + 1],
        vec![1],
        p.period().pcm().to_vec()
    )
    .is_err());
    assert!(OfflineOperation::from_offline_fixture(
        &p,
        "op".into(),
        vec![1],
        vec![1; MAX_MANIFEST + 1],
        p.period().pcm().to_vec()
    )
    .is_err());
}
