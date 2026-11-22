//! OM0RX-xNexus automation, through the engine: Answer callers and the supervised autopilot.
//!
//! What must hold (the accepted proposal, 2026-11-20): with both features off nothing changes;
//! Answer callers works a caller only after our closing over is on the air, never while TX is off,
//! and stops after its chain limit; the autopilot never arms TX, stops on every stop, and turns TX
//! off when presence is not confirmed.

use super::tests::dec_snr;
use super::*;

const ME: &str = "OM0RX";
const DX: &str = "7P8RU";
const CALLER: &str = "K1ABC";

fn engine() -> Engine {
    let mut e = Engine::new(ME, "JN88", 0);
    e.set_tier(Tier::Ft8);
    e.set_frequency(14.074, "20m", "USB");
    e.settings.auto_log = true;
    e.settings.prompt_to_log = false;
    e.settings.disable_tx_after_73 = true;
    e
}

fn tx_now(e: &Engine) -> String {
    e.snapshot()
        .qso
        .as_ref()
        .and_then(|q| q.tx_now.clone())
        .unwrap_or_default()
}

/// Transmit once on whichever of the two slots is ours; the text that went out.
fn key_once(e: &mut Engine, s: u64) -> String {
    let text = tx_now(e);
    for slot in [s, s + 1] {
        if !e.poll_tx(slot).is_empty() {
            return text;
        }
    }
    panic!("nothing keyed at slot {s} or {} (queued: {text:?})", s + 1)
}

/// An S&P contact with DX up to their RR73, with CALLER calling us beside it. Returns the next slot.
fn contact_with_a_caller(e: &mut Engine) -> u64 {
    e.ingest_decodes_for_test(&[dec_snr("CQ 7P8RU KG30", -12)], 11);
    e.work_station(DX, None, Some("CQ 7P8RU KG30"), Some(-12), Some(1500.0))
        .expect("the contact starts");
    key_once(e, 12);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX 7P8RU -15", -12)], 14);
    key_once(e, 15);
    e.ingest_decodes_for_test(
        &[
            dec_snr("OM0RX 7P8RU RR73", -12),
            dec_snr("OM0RX K1ABC FN42", -5),
        ],
        17,
    );
    assert!(
        tx_now(e).contains("73"),
        "premise: our 73 is queued, got {:?}",
        tx_now(e)
    );
    18
}

#[test]
fn with_answer_callers_off_tx_goes_off_after_the_73_as_today() {
    let mut e = engine();
    let s = contact_with_a_caller(&mut e);
    key_once(&mut e, s);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -5)], s + 2);
    assert_eq!(
        e.automation_dx().as_deref(),
        Some(DX),
        "the caller is not answered"
    );
    assert!(
        e.pending_tx_disable || !e.tx_enabled,
        "TX goes off after the 73"
    );
}

#[test]
fn answer_callers_works_the_caller_after_our_73_is_sent() {
    let mut e = engine();
    e.settings.automation.answer_callers = true;
    let s = contact_with_a_caller(&mut e);
    // The caller is not answered before our 73 to the DX goes out.
    assert_eq!(e.automation_dx().as_deref(), Some(DX));
    let sent = key_once(&mut e, s);
    assert!(sent.contains("73"), "our 73 went out first: {sent:?}");
    assert!(!e.pending_tx_disable, "TX is held for the next contact");
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -5)], s + 2);
    assert_eq!(
        e.automation_dx().as_deref(),
        Some(CALLER),
        "the caller is answered"
    );
    assert!(e.tx_enabled, "TX stays on for the caller");
    assert!(
        tx_now(&e).starts_with("K1ABC OM0RX"),
        "we answer K1ABC: {:?}",
        tx_now(&e)
    );
}

#[test]
fn answer_callers_never_arms_transmit() {
    let mut e = engine();
    e.settings.automation.answer_callers = true;
    let s = contact_with_a_caller(&mut e);
    key_once(&mut e, s);
    e.set_tx_enabled(false);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -5)], s + 2);
    assert_eq!(
        e.automation_dx().as_deref(),
        Some(DX),
        "nobody is answered with TX off"
    );
    assert!(!e.tx_enabled);
}

#[test]
fn a_held_disable_with_no_caller_left_turns_tx_off() {
    let mut e = engine();
    e.settings.automation.answer_callers = true;
    let s = contact_with_a_caller(&mut e);
    key_once(&mut e, s);
    // The caller is not heard again for longer than it stays fresh.
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], s + 10);
    assert_eq!(e.automation_dx().as_deref(), Some(DX));
    assert!(
        !e.tx_enabled,
        "with no caller to answer, TX goes off as it would have"
    );
}

#[test]
fn the_chain_stops_at_its_limit() {
    let mut e = engine();
    e.settings.automation.answer_callers = true;
    e.settings.automation.answer_chain_limit = 1;
    let s = contact_with_a_caller(&mut e);
    key_once(&mut e, s);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -5)], s + 2);
    assert_eq!(e.automation_dx().as_deref(), Some(CALLER));
    assert_eq!(e.automation_dto().chain, 1);
    // K1ABC's contact runs to its end with a second caller waiting: not answered.
    key_once(&mut e, s + 3);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC R-07", -5)], s + 5);
    key_once(&mut e, s + 6);
    e.ingest_decodes_for_test(
        &[
            dec_snr("OM0RX K1ABC 73", -5),
            dec_snr("OM0RX W1AW FN31", -9),
        ],
        s + 8,
    );
    e.ingest_decodes_for_test(&[dec_snr("OM0RX W1AW FN31", -9)], s + 10);
    assert_ne!(
        e.automation_dx().as_deref(),
        Some("W1AW"),
        "the chain limit holds"
    );
}

#[test]
fn an_unanswered_first_call_gives_way_to_a_caller() {
    let mut e = engine();
    e.settings.automation.answer_callers = true;
    e.settings.automation.answer_takeover_slots = 2;
    e.ingest_decodes_for_test(&[dec_snr("CQ 7P8RU KG30", -12)], 11);
    e.work_station(DX, None, Some("CQ 7P8RU KG30"), Some(-12), Some(1500.0))
        .unwrap();
    key_once(&mut e, 12);
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 14);
    assert_eq!(
        e.automation_dx().as_deref(),
        Some(DX),
        "one call: still calling the DX"
    );
    key_once(&mut e, 15);
    // K1ABC is heard for the first time in this very slot: answered from it, as a click would be.
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -5)], 17);
    assert_eq!(
        e.automation_dx().as_deref(),
        Some(CALLER),
        "two unanswered calls: the caller takes over"
    );
}

#[test]
fn a_caller_does_not_interrupt_a_contact_under_way() {
    let mut e = engine();
    e.settings.automation.answer_callers = true;
    e.ingest_decodes_for_test(&[dec_snr("CQ 7P8RU KG30", -12)], 11);
    e.work_station(DX, None, Some("CQ 7P8RU KG30"), Some(-12), Some(1500.0))
        .unwrap();
    key_once(&mut e, 12);
    e.ingest_decodes_for_test(
        &[
            dec_snr("OM0RX 7P8RU -15", -12),
            dec_snr("OM0RX K1ABC FN42", -5),
        ],
        14,
    );
    assert_eq!(
        e.automation_dx().as_deref(),
        Some(DX),
        "the DX answered: we stay with them"
    );
}

#[test]
fn the_autopilot_needs_settings_and_transmit_on() {
    let mut e = engine();
    assert!(e.set_autopilot(true).is_err(), "off in Settings");
    e.settings.automation.autopilot_enabled = true;
    e.set_tx_enabled(false);
    assert!(e.set_autopilot(true).is_err(), "TX off");
    e.set_tx_enabled(true);
    assert!(e.set_autopilot(true).is_ok());
    assert!(e.automation_dto().autopilot_on);
}

#[test]
fn stop_tx_switches_the_autopilot_off() {
    let mut e = engine();
    e.settings.automation.autopilot_enabled = true;
    e.set_tx_enabled(true);
    e.set_autopilot(true).unwrap();
    e.halt_tx();
    assert!(!e.automation_dto().autopilot_on);
    assert!(!e.tx_enabled);
}

#[test]
fn an_unconfirmed_presence_turns_transmit_and_the_autopilot_off() {
    let mut e = engine();
    e.settings.automation.autopilot_enabled = true;
    e.settings.automation.presence_check_min = 2;
    e.set_tx_enabled(true);
    e.set_autopilot(true).unwrap();
    e.automation_backdate_presence(121);
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 30);
    assert!(!e.automation_dto().autopilot_on);
    assert!(!e.tx_enabled, "TX off when presence is not confirmed");
}

#[test]
fn a_confirmed_presence_keeps_it_running() {
    let mut e = engine();
    e.settings.automation.autopilot_enabled = true;
    e.settings.automation.presence_check_min = 2;
    e.set_tx_enabled(true);
    e.set_autopilot(true).unwrap();
    e.automation_backdate_presence(100);
    e.autopilot_confirm_presence();
    assert!(e.automation_dto().presence_remaining_secs > 100);
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 30);
    assert!(e.automation_dto().autopilot_on);
}

#[test]
fn the_autopilot_listens_then_calls_a_new_station() {
    let mut e = engine();
    e.settings.automation.autopilot_enabled = true;
    // A schedule that keeps 20 m FT8 all day, so nothing moves.
    e.settings.automation.schedule = vec![crate::automation::AutopilotWindow {
        start: "00:00".into(),
        bands: vec!["20m".into()],
        mode: "FT8".into(),
    }];
    e.set_tx_enabled(true);
    e.set_autopilot(true).unwrap();
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 30);
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 32);
    assert_eq!(e.automation_dx(), None, "it listens first");
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 34);
    assert_eq!(
        e.automation_dx().as_deref(),
        Some("N0ABC"),
        "then calls a new call"
    );
    assert!(e.tx_enabled);
}

#[test]
fn the_autopilot_skips_a_directed_cq_for_someone_else() {
    let mut e = engine();
    e.settings.automation.autopilot_enabled = true;
    e.settings.automation.schedule = vec![crate::automation::AutopilotWindow {
        start: "00:00".into(),
        bands: vec!["20m".into()],
        mode: "FT8".into(),
    }];
    e.settings.automation.listen_slots = 1;
    e.set_tx_enabled(true);
    e.set_autopilot(true).unwrap();
    e.ingest_decodes_for_test(&[dec_snr("CQ NA N0ABC EM48", -3)], 30);
    e.ingest_decodes_for_test(&[dec_snr("CQ NA N0ABC EM48", -3)], 32);
    assert_eq!(e.automation_dx(), None);
}

/// 20 m and 17 m all day, listening one slot: the smallest schedule that can move.
fn two_band_pilot() -> Engine {
    let mut e = engine();
    e.settings.automation.autopilot_enabled = true;
    e.settings.automation.schedule = vec![crate::automation::AutopilotWindow {
        start: "00:00".into(),
        bands: vec!["20m".into(), "17m".into()],
        mode: "FT8".into(),
    }];
    e.settings.automation.listen_slots = 1;
    e.set_tx_enabled(true);
    e.set_autopilot(true).unwrap();
    e
}

#[test]
fn a_quiet_band_is_left_once_everything_is_finished() {
    let mut e = two_band_pilot();
    e.automation_backdate_band(10_000);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[], 32);
    assert_eq!(
        e.settings.band, "17m",
        "positive control: a quiet band is left"
    );
}

#[test]
fn the_band_never_changes_while_the_partner_is_still_sending() {
    let mut e = two_band_pilot();
    // One call per step, so the exchange stalls the moment the partner misses our report.
    e.settings.directed_max_calls = Some(1);
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 30);
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 32);
    assert_eq!(
        e.automation_dx().as_deref(),
        Some("N0ABC"),
        "premise: calling"
    );
    key_once(&mut e, 33);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX N0ABC -10", -3)], 35);
    key_once(&mut e, 36);
    // The partner did not copy our R-report and repeats theirs; our step is spent (stalled).
    for s in [38, 40] {
        e.automation_backdate_band(10_000);
        e.ingest_decodes_for_test(&[dec_snr("OM0RX N0ABC -10", -3)], s);
        assert_eq!(e.settings.band, "20m", "slot {s}: mid-QSO, the band stays");
    }
    // Two of the partner's periods of silence: the contact is over, the band may change.
    for s in [42, 44] {
        e.automation_backdate_band(10_000);
        e.ingest_decodes_for_test(&[], s);
    }
    assert_eq!(
        e.settings.band, "17m",
        "after the partner went quiet, the band moves"
    );
}

#[test]
fn a_station_calling_us_holds_the_band_even_with_answer_callers_off() {
    let mut e = two_band_pilot();
    for s in [30, 32, 34] {
        e.automation_backdate_band(10_000);
        e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -5)], s);
        assert_eq!(e.settings.band, "20m", "slot {s}: somebody is calling us");
    }
}

/// Field report 2026-11-22: ten minutes of only ordinary calls made the move due while the last
/// partner was still finishing, the move was held, and the same pass started another ordinary call
/// (BA3KY → RU4LM) — which holds the band again, so the move kept slipping. A due move that is held
/// waits; nothing ordinary starts in its place, and the band moves once the hold lapses.
#[test]
fn a_held_band_move_is_not_put_off_by_another_ordinary_call() {
    let mut e = two_band_pilot();
    for s in [30, 32] {
        e.automation_backdate_band(10_000);
        e.ingest_decodes_for_test(
            &[
                dec_snr("CQ N0ABC EM48", -3),
                dec_snr("OM0RX K1ABC FN42", -5),
            ],
            s,
        );
        assert_eq!(
            e.settings.band, "20m",
            "slot {s}: K1ABC calling holds the band"
        );
        assert_eq!(
            e.automation_dx(),
            None,
            "slot {s}: the move is due, so no ordinary call starts in its place"
        );
    }
    for s in [34, 36, 38, 40] {
        e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], s);
        if e.settings.band == "17m" {
            break;
        }
        assert_eq!(e.automation_dx(), None, "slot {s}: still waiting to move");
    }
    assert_eq!(e.settings.band, "17m", "the hold lapsed: the band moved");
}

/// CONTROL for the test above: with the ordinary dwell not yet spent, the same held band still
/// lets the autopilot call an ordinary new station.
#[test]
fn an_ordinary_call_still_starts_on_a_held_band_before_the_dwell_is_spent() {
    let mut e = two_band_pilot();
    for s in [30, 32] {
        e.ingest_decodes_for_test(
            &[
                dec_snr("CQ N0ABC EM48", -3),
                dec_snr("OM0RX K1ABC FN42", -5),
            ],
            s,
        );
    }
    assert_eq!(e.automation_dx().as_deref(), Some("N0ABC"));
    assert_eq!(e.settings.band, "20m");
}

#[test]
fn the_band_never_changes_during_a_cq_run() {
    let mut e = two_band_pilot();
    e.cq_running = true;
    e.automation_backdate_band(10_000);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[], 32);
    assert_eq!(e.settings.band, "20m");
}

fn dxped_spot(call: &str, band: &str, freq: f64) -> crate::automation::FeedEntry {
    crate::automation::FeedEntry {
        call: call.into(),
        band: band.into(),
        mode: Some("FT8".into()),
        freq_mhz: Some(freq),
        dxpedition: true,
        new_slot: true,
        ..Default::default()
    }
}

/// Field report 2026-11-22: the autopilot followed a 5U4JSF spot (a new band slot) to 20 m and,
/// while listening for it, called SA7SVR, an ordinary new call that could keep it busy for two
/// minutes just when 5U4JSF appears. A spot chase listens for its station and calls nothing
/// ordinary; the chased station itself is called as soon as it is heard.
#[test]
fn a_spot_chase_waits_for_its_station_instead_of_calling_ordinary_ones() {
    let mut e = two_band_pilot();
    e.autopilot_feed(vec![crate::automation::FeedEntry {
        call: "5U4JSF".into(),
        band: "17m".into(),
        mode: Some("FT8".into()),
        freq_mhz: Some(18.101),
        new_slot: true,
        ..Default::default()
    }]);
    // The first slot is the listening one; the spot is weighed from the next.
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[], 32);
    assert_eq!(e.settings.band, "17m", "premise: the spot is chased");
    for s in [34, 36, 38] {
        e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], s);
        assert_eq!(
            e.automation_dx(),
            None,
            "slot {s}: chasing 5U4JSF, N0ABC is not called"
        );
    }
    e.ingest_decodes_for_test(
        &[dec_snr("CQ N0ABC EM48", -3), dec_snr("CQ 5U4JSF JK73", -15)],
        40,
    );
    assert_eq!(
        e.automation_dx().as_deref(),
        Some("5U4JSF"),
        "the chased station is called the moment it is heard"
    );
}

fn dec_at(msg: &str, snr: i32, freq: f32) -> modes::Decode {
    let mut d = dec_snr(msg, snr);
    d.freq = freq;
    d
}

#[test]
fn a_dxpedition_on_an_unscheduled_band_is_hunted_on_its_own_frequency() {
    let mut e = two_band_pilot();
    e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
    e.ingest_decodes_for_test(&[], 30);
    assert_eq!(
        e.settings.band, "40m",
        "40 m is in no window, but a DXpedition is"
    );
    assert!(
        (e.settings.dial_mhz - 7.056).abs() < 1e-6,
        "its own frequency, not the 7.074 channel: {}",
        e.settings.dial_mhz
    );
    assert_eq!(e.automation_dto().status, "moving");
    // Heard above 1000 Hz (no Fox/Hound): called like a double-click.
    e.ingest_decodes_for_test(&[dec_snr("CQ 3Y0K JD15", -14)], 32);
    assert_eq!(e.automation_dx().as_deref(), Some("3Y0K"));
    assert_eq!(e.settings.special_op, crate::settings::SpecialOp::None);
}

#[test]
fn a_dxpedition_hunt_waits_for_the_contact_in_progress() {
    let mut e = two_band_pilot();
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 30);
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 32);
    assert_eq!(
        e.automation_dx().as_deref(),
        Some("N0ABC"),
        "premise: calling"
    );
    key_once(&mut e, 33);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX N0ABC -10", -3)], 35);
    e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
    key_once(&mut e, 36);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX N0ABC -10", -3)], 38);
    assert_eq!(e.settings.band, "20m", "mid-QSO: the DXpedition waits");
}

#[test]
fn a_fox_switches_the_hunt_to_hound_and_stop_tx_puts_it_back() {
    let mut e = two_band_pilot();
    e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[dec_at("CQ 3Y0K JD15", -14, 600.0)], 32);
    assert_eq!(e.settings.special_op, crate::settings::SpecialOp::Hound);
    e.halt_tx();
    assert_eq!(
        e.settings.special_op,
        crate::settings::SpecialOp::None,
        "the operator's own setting is back"
    );
    assert!(!e.automation_dto().autopilot_on);
}

#[test]
fn a_hunt_returns_where_it_came_from_when_the_spot_goes() {
    let mut e = two_band_pilot();
    e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
    e.ingest_decodes_for_test(&[], 30);
    assert_eq!(e.settings.band, "40m", "premise: hunting");
    e.autopilot_feed(vec![]);
    e.ingest_decodes_for_test(&[], 32);
    assert_eq!(e.settings.band, "20m", "back to the band it left");
    assert!((e.settings.dial_mhz - 14.074).abs() < 1e-6);
}

#[test]
fn a_needed_state_is_called_only_when_switched_on_and_wanted() {
    let mut e = engine();
    e.settings.automation.autopilot_enabled = true;
    e.settings.automation.schedule = vec![crate::automation::AutopilotWindow {
        start: "00:00".into(),
        bands: vec!["20m".into()],
        mode: "FT8".into(),
    }];
    e.settings.automation.listen_slots = 1;
    // Nothing else would call it: every other tier off.
    e.settings.automation.tiers = crate::automation::AutopilotTiers {
        atno: false,
        dxpedition: false,
        dxcc_unconfirmed: false,
        dxcc_new_slot: false,
        us_state: false,
        reference: false,
        new_call: false,
    };
    e.set_tx_enabled(true);
    e.set_autopilot(true).unwrap();
    let vt = crate::automation::FeedEntry {
        call: "W1VT".into(),
        band: "20m".into(),
        mode: Some("FT8".into()),
        new_state: true,
        state: Some("VT".into()),
        ..Default::default()
    };
    e.autopilot_feed(vec![vt]);
    for s in [30, 32] {
        e.ingest_decodes_for_test(&[dec_snr("CQ W1VT FN33", -8)], s);
    }
    assert_eq!(e.automation_dx(), None, "the state tier is off");
    e.settings.automation.tiers.us_state = true;
    e.settings.automation.wanted_states = vec!["ND".into()];
    e.ingest_decodes_for_test(&[dec_snr("CQ W1VT FN33", -8)], 34);
    assert_eq!(e.automation_dx(), None, "VT is not on the wanted list");
    e.settings.automation.wanted_states = vec!["VT".into()];
    e.ingest_decodes_for_test(&[dec_snr("CQ W1VT FN33", -8)], 36);
    assert_eq!(
        e.automation_dx().as_deref(),
        Some("W1VT"),
        "wanted and needed"
    );
}

#[test]
fn a_lapsed_presence_stops_answer_callers_too() {
    // REVIEW N-03: Answer callers ran before the autopilot and read neither presence nor
    // the hourly cap, so contacts kept starting after the presence check lapsed.
    let mut e = engine();
    e.settings.automation.answer_callers = true;
    e.settings.automation.autopilot_enabled = true;
    e.settings.automation.presence_check_min = 2;
    let s = contact_with_a_caller(&mut e);
    e.set_autopilot(true).unwrap();
    e.automation_backdate_presence(121);
    key_once(&mut e, s);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -5)], s + 2);
    assert_ne!(
        e.automation_dx().as_deref(),
        Some(CALLER),
        "no caller is answered once presence lapsed"
    );
    assert!(!e.tx_enabled, "TX goes off");
}

#[test]
fn a_dxpedition_spot_off_its_band_or_at_the_edge_is_not_hunted() {
    // REVIEW N-02: a mistyped spot (20 m, 14.349 MHz) put our signal above the 20 m edge,
    // and a spot whose frequency is on another band than its label tuned there anyway.
    for spot in [
        dxped_spot("3Y0K", "20m", 14.349),
        dxped_spot("3Y0K", "40m", 14.090),
    ] {
        let mut e = two_band_pilot();
        e.autopilot_feed(vec![spot]);
        e.ingest_decodes_for_test(&[], 30);
        assert_eq!(e.settings.band, "20m", "the station stays where it was");
        assert!(
            (e.settings.dial_mhz - 14.074).abs() < 1e-6,
            "no retune: {}",
            e.settings.dial_mhz
        );
    }
}

#[test]
fn an_autopilot_band_change_re_arms_without_releasing_the_broker_hold() {
    // REVIEW N-01: the re-arm after the autopilot's own retune cleared the CAT-broker hold,
    // so a logger or JTAlert on the broker could key after an unattended band change.
    let mut e = two_band_pilot();
    e.automation_backdate_band(10_000);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[], 32);
    assert_eq!(e.settings.band, "17m", "positive control: the band changed");
    assert!(e.tx_enabled, "the autopilot keeps its own arm");
    assert!(
        e.broker_context_hold,
        "a foreign broker key still waits for the operator after the move"
    );
}

#[test]
fn a_station_just_worked_is_never_answered_again_from_its_old_call() {
    // Field report 2026-11-22: SP4NKJ called us while we finished EW4M, so Answer callers
    // queued it. We then worked SP4NKJ; once our RR73 was out, the stale queue entry
    // "answered" SP4NKJ again with a fresh report, after the contact was already made.
    let mut e = engine();
    e.settings.automation.answer_callers = true;
    contact_with_a_caller(&mut e); // K1ABC queued at slot 17 while we work 7P8RU
    assert!(
        e.automation_dto().callers.iter().any(|c| c == CALLER),
        "premise: K1ABC is queued"
    );
    // The operator works K1ABC directly (any path other than the queue).
    e.work_station(
        CALLER,
        None,
        Some("OM0RX K1ABC FN42"),
        Some(-5),
        Some(1200.0),
    )
    .expect("K1ABC's contact starts");
    let sent = key_once(&mut e, 18);
    assert!(sent.starts_with("K1ABC OM0RX"), "our report: {sent:?}");
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC R-07", -5)], 19);
    let sent = key_once(&mut e, 20);
    assert!(sent.contains("RR73"), "our RR73: {sent:?}");
    // Still inside the caller's freshness window: the old entry must not restart the contact.
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC 73", -5)], 20);
    assert!(
        !e.automation_dto().callers.iter().any(|c| c == CALLER),
        "the station being worked is not a caller: {:?}",
        e.automation_dto().callers
    );
    let next = tx_now(&e);
    assert!(
        !next.starts_with("K1ABC OM0RX -") && !next.starts_with("K1ABC OM0RX R"),
        "no fresh report to a station already worked: {next:?}"
    );
}

/// The autopilot calling N0ABC on 20 m with a one-minute watchdog and a ten-minute target
/// budget, its first over keyed. Returns the next slot.
fn pilot_calling_past_the_watchdog(e: &mut Engine) -> u64 {
    e.settings.tx_watchdog_min = 1;
    e.settings.automation.autopilot_enabled = true;
    e.settings.automation.target_budget_secs = 600;
    e.settings.automation.listen_slots = 1;
    e.settings.automation.schedule = vec![crate::automation::AutopilotWindow {
        start: "00:00".into(),
        bands: vec!["20m".into()],
        mode: "FT8".into(),
    }];
    e.set_tx_enabled(true);
    e.set_autopilot(true).unwrap();
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 30);
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 32);
    assert_eq!(e.automation_dx().as_deref(), Some("N0ABC"), "premise");
    key_once(e, 33);
    // More than the watchdog's minute of unanswered calling has passed.
    e.tx_watchdog_start = Some(now_unix_secs().saturating_sub(61));
    35
}

#[test]
fn the_watchdog_does_not_end_a_contact_the_autopilot_still_keeps() {
    // Operator 2026-11-22: "make sure the TX watchdog can't just stop it for no reason". The
    // contact is inside its own time budget, so the watchdog clock restarts with each step.
    let mut e = engine();
    let s = pilot_calling_past_the_watchdog(&mut e);
    e.ingest_decodes_for_test(&[dec_snr("CQ W9XYZ EN52", -3)], s - 1);
    let sent = key_once(&mut e, s);
    assert!(sent.starts_with("N0ABC OM0RX"), "still calling: {sent:?}");
    assert!(e.tx_enabled && e.automation_dto().autopilot_on);
}

#[test]
fn the_watchdog_still_trips_when_the_autopilot_step_stops_running() {
    // Control: with no decode fold (a wedged decoder, a dead loop) nothing restarts the clock,
    // and the watchdog ends it exactly as before.
    let mut e = engine();
    let s = pilot_calling_past_the_watchdog(&mut e);
    for slot in [s, s + 1] {
        let _ = e.poll_tx(slot);
    }
    assert!(!e.tx_enabled, "the watchdog tripped");
}

#[test]
fn a_dxpedition_is_called_for_at_least_ten_minutes() {
    let s = crate::automation::AutomationSettings {
        rare_budget_secs: 300,
        ..Default::default()
    };
    assert_eq!(s.sanitized().rare_budget_secs, 600);
}
