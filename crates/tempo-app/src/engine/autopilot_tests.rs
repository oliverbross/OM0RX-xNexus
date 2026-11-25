//! OM0RX-xNexus automation, through the engine: Answer callers and the supervised autopilot.
//!
//! What must hold (the accepted proposal, 2026-11-20): with both features off nothing changes;
//! Answer callers works a caller only after our closing over is on the air, never while TX is off,
//! and stops after its chain limit; the autopilot never arms TX, stops on every stop, and turns TX
//! off when presence is not confirmed.

use super::autopilot::mode_name_for_test;
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

/// Live run 2026-11-22 (xlog.23): the operator called VK9XY (a DXpedition in the feed) from its
/// pileup, and after two unanswered calls Answer callers dropped it for N5EI, who was calling us,
/// twice. A DXpedition is a long call by nature: a caller never takes over from one.
#[test]
fn a_caller_never_takes_over_from_a_dxpedition() {
    let mut e = engine();
    e.settings.automation.answer_callers = true;
    e.settings.automation.answer_takeover_slots = 2;
    e.autopilot_feed(vec![dxped_spot(DX, "20m", 14.074)]);
    e.ingest_decodes_for_test(&[dec_snr("CQ 7P8RU KG30", -12)], 11);
    e.work_station(DX, None, Some("CQ 7P8RU KG30"), Some(-12), Some(1500.0))
        .unwrap();
    key_once(&mut e, 12);
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 14);
    key_once(&mut e, 15);
    for s in [17, 20, 23] {
        e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -5)], s);
        assert_eq!(
            e.automation_dx().as_deref(),
            Some(DX),
            "slot {s}: still calling the DXpedition"
        );
        key_once(&mut e, s + 1);
    }
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

/// 20 m and 17 m all day, listening one slot: the smallest schedule that can move. The new-grid
/// rule is off: in a test's empty log every grid is new, which would make every station needed.
fn two_band_pilot() -> Engine {
    let mut e = engine();
    e.settings.automation.autopilot_enabled = true;
    e.settings.automation.tiers.grid = false;
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

/// Field report 2026-11-22 (xlog.23): the autopilot took OM0RX to 60 m at 5.3715 MHz, the US
/// channel of the raw band table, and decoded nothing there for eight periods. A station whose call
/// is not a US one works 60 m FT8 at 5.357 (#175), the dial its own 60 m button tunes; 5.3715 +
/// audio is outside the WRC-15 segment most of the world is licensed for.
#[test]
fn the_autopilot_tunes_60m_where_the_stations_own_band_button_does() {
    let mut e = two_band_pilot();
    e.settings.automation.schedule[0].bands = vec!["20m".into(), "60m".into()];
    e.automation_backdate_band(10_000);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[], 32);
    assert_eq!(e.settings.band, "60m", "premise: the quiet band is left");
    let button = e
        .band_plan()
        .into_iter()
        .find(|c| c.band == "60m")
        .map(|c| c.dial_mhz)
        .unwrap();
    assert!(
        (button - crate::bandplan::SIXTY_M_FT8_WORLDWIDE_MHZ).abs() < 1e-9,
        "premise: a non-US station's 60 m button is the worldwide dial, got {button}"
    );
    assert!(
        (e.settings.dial_mhz - button).abs() < 1e-9,
        "the autopilot tuned {} MHz, its own 60 m button tunes {button}",
        e.settings.dial_mhz
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

/// Live run 2026-11-22 (xlog.23): VK9XY's CQ came in the early fold of a slot that was the
/// autopilot's listening slot; the boundary fold of the same slot (which carries only what the early
/// one did not) held other stations, so the DXpedition was never ranked and a US-state spot was
/// chased instead. Everything heard in the slot counts, whichever fold brought it.
#[test]
fn a_station_from_the_early_fold_still_counts_in_the_boundary_fold() {
    let mut e = two_band_pilot();
    e.autopilot_feed(vec![dxped_spot("3Y0K", "20m", 14.074)]);
    // Early fold: the listening slot.
    // Heard at 1200 Hz: above the Fox range, so not left alone as a Fox would be.
    e.ingest_decodes_for_test(&[dec_at("CQ 3Y0K JD15", -14, 1200.0)], 30);
    assert_eq!(e.automation_dx(), None, "premise: the first slot listens");
    // Boundary fold of the same slot: nothing new in it.
    e.ingest_decodes_for_test(&[], 30);
    assert_eq!(
        e.automation_dx().as_deref(),
        Some("3Y0K"),
        "the DXpedition heard earlier in this slot is called"
    );
}

fn state_spot(call: &str, band: &str, freq: f64) -> crate::automation::FeedEntry {
    crate::automation::FeedEntry {
        call: call.into(),
        band: band.into(),
        mode: Some("FT8".into()),
        freq_mhz: Some(freq),
        new_state: true,
        state: Some("MA".into()),
        ..Default::default()
    }
}

/// The dial each band of [`two_band_pilot`] sits on, and the other band.
fn other_band(e: &Engine) -> (&'static str, f64) {
    if e.settings.band == "20m" {
        ("17m", 18.101)
    } else {
        ("20m", 14.075)
    }
}

/// Live run 2026-11-22 (xlog.23): from 22:39 the autopilot chased five US-state spots back to back
/// (N7ILO, K0JWI, AJ6T, K6AFW, K9TD), five minutes each, 40 s on a band between two, and made no
/// contact in 22 minutes. After a chase that never heard its station, a lower rule's spot waits
/// its rest so the schedule gets its turn. On 2026-11-23 DXCC slots did the same for two and a half
/// hours, so they wait too, but half as long; an ATNO is still chased at once.
#[test]
fn after_an_unheard_chase_a_spot_waits_its_rest_and_an_atno_does_not() {
    let mut e = two_band_pilot();
    e.settings.automation.tiers.us_state = true;
    e.autopilot_feed(vec![state_spot("N7ILO", "17m", 18.101)]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[], 32);
    assert_eq!(e.settings.band, "17m", "premise: the state spot is chased");
    // Five minutes without N7ILO: given up.
    e.automation_backdate_band(400);
    // The first slot on the new band only listens.
    e.ingest_decodes_for_test(&[], 34);
    e.ingest_decodes_for_test(&[], 36);
    assert!(
        e.automation_dto()
            .log
            .iter()
            .any(|l| l.contains("N7ILO not heard")),
        "premise: the chase was given up"
    );
    let (band, freq) = other_band(&e);
    let here = e.settings.band.clone();
    e.autopilot_feed(vec![state_spot("K0JWI", band, freq)]);
    for s in [38, 40, 42] {
        e.ingest_decodes_for_test(&[], s);
        assert_eq!(e.settings.band, here, "slot {s}: K0JWI waits out the rest");
    }
    let slot_spot = |new_entity: bool| crate::automation::FeedEntry {
        call: "5U4JSF".into(),
        band: band.into(),
        mode: Some("FT8".into()),
        freq_mhz: Some(freq),
        new_slot: true,
        new_entity,
        ..Default::default()
    };
    e.autopilot_feed(vec![slot_spot(false)]);
    e.ingest_decodes_for_test(&[], 44);
    assert_eq!(e.settings.band, here, "a DXCC slot waits too");
    e.autopilot_feed(vec![slot_spot(true)]);
    e.ingest_decodes_for_test(&[], 46);
    assert_eq!(e.settings.band, band, "an ATNO is chased during the rest");
}

#[test]
fn the_chase_rest_ends() {
    let mut e = two_band_pilot();
    e.settings.automation.tiers.us_state = true;
    e.autopilot_feed(vec![state_spot("N7ILO", "17m", 18.101)]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[], 32);
    e.automation_backdate_band(400);
    // The first slot on the new band only listens.
    e.ingest_decodes_for_test(&[], 34);
    e.ingest_decodes_for_test(&[], 36);
    let (band, freq) = other_band(&e);
    e.autopilot_feed(vec![state_spot("K0JWI", band, freq)]);
    e.automation_backdate_band(700);
    // A band change in the give-up pass makes this slot a listening one.
    e.ingest_decodes_for_test(&[], 38);
    e.ingest_decodes_for_test(&[], 40);
    assert_eq!(
        e.settings.band, band,
        "after the rest a state spot is chased again"
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

/// Operator, 2026-11-22: "it should never call on the DXpedition frequency but a minimum 1200
/// higher … usually a DXpedition should be within 500 Hz … if not, the software needs to retune to
/// fit the DXpedition under 500". Heard low, it is called 1200 Hz above with the dial left alone;
/// heard at 1500 Hz, the dial moves up 1100 Hz so it sits at 400, and it is called from 1600.
#[test]
fn a_dxpedition_is_called_1200_hz_above_its_own_frequency() {
    let mut low = two_band_pilot();
    low.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
    low.ingest_decodes_for_test(&[], 30);
    low.ingest_decodes_for_test(&[dec_at("CQ 3Y0K JD15", -14, 450.0)], 32);
    assert_eq!(
        low.automation_dx().as_deref(),
        Some("3Y0K"),
        "premise: called"
    );
    assert!(
        (low.settings.dial_mhz - 7.056).abs() < 1e-9,
        "under 500 Hz: no retune"
    );
    // At least 1200 Hz above it: the offset already in use stays when it is far enough up.
    assert!(
        (1650.0..=2500.0).contains(&low.tx_offset_hz),
        "called at least 1200 Hz above it: {}",
        low.tx_offset_hz
    );

    let mut e = two_band_pilot();
    e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[dec_at("CQ 3Y0K JD15", -14, 1500.0)], 32);
    assert_eq!(
        e.automation_dx().as_deref(),
        Some("3Y0K"),
        "premise: called"
    );
    assert!(
        (e.settings.dial_mhz - 7.0571).abs() < 1e-9,
        "the dial moved up 1100 Hz: {}",
        e.settings.dial_mhz
    );
    assert!(
        (e.tx_offset_hz - 1600.0).abs() < 0.5,
        "called 1200 Hz above where it now sits (400 Hz): {}",
        e.tx_offset_hz
    );
    assert!(e.tx_enabled, "the retune put transmit back");
    assert_eq!(e.settings.band, "40m");
    // CONTROL: an ordinary station is still called as a double-click calls it, on its frequency.
    let mut o = two_band_pilot();
    o.settings.automation.schedule[0].bands = vec!["20m".into()];
    for s in [30, 32] {
        o.ingest_decodes_for_test(&[dec_at("CQ N0ABC EM48", -3, 1500.0)], s);
    }
    assert_eq!(
        o.automation_dx().as_deref(),
        Some("N0ABC"),
        "premise: called"
    );
    assert!(
        (o.tx_offset_hz - 1500.0).abs() < 0.5,
        "an ordinary call is unchanged: {}",
        o.tx_offset_hz
    );
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

/// Live run 2026-11-25: VK9XY worked as Hound ended transmit, and the autopilot with it.
#[test]
fn a_hound_contact_ended_leaves_transmit_and_the_autopilot_on() {
    let mut e = two_band_pilot();
    e.settings.disable_tx_after_73 = true;
    e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[dec_at("CQ 3Y0K JD15", -14, 600.0)], 32);
    assert_eq!(e.settings.special_op, crate::settings::SpecialOp::Hound);
    key_once(&mut e, 33);
    e.ingest_decodes_for_test(&[dec_at("OM0RX 3Y0K -10", -14, 600.0)], 34);
    let over = key_once(&mut e, 35);
    assert!(
        over.contains(" R-"),
        "premise: our report goes back: {over}"
    );
    e.ingest_decodes_for_test(&[dec_at("OM0RX 3Y0K RR73", -14, 600.0)], 36);
    assert!(
        matches!(&e.mode, Mode::Qso { station, .. } if station.state == QsoState::Done),
        "premise: the contact is made"
    );
    assert!(!e.take_pending_tx_disable(), "transmit stays on");
    assert!(e.tx_enabled && e.automation_dto().autopilot_on);
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
        most_wanted: false,
        grid: false,
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
fn a_dxpedition_is_called_for_the_operators_minutes() {
    let mut s = crate::automation::AutomationSettings::default();
    assert_eq!(s.dxped_call_secs(), 600, "ten by default");
    s.dxped_call_mins = 25;
    assert_eq!(s.clone().sanitized().dxped_call_secs(), 1500);
    s.dxped_call_mins = 0;
    assert_eq!(s.sanitized().dxped_call_mins, 1);

    // A heard DXpedition is called that long, counted from when it was first heard.
    let mut e = two_band_pilot();
    e.settings.automation.dxped_call_mins = 25;
    e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
    e.ingest_decodes_for_test(&[], 30);
    e.automation_backdate_band(280);
    e.ingest_decodes_for_test(&[dec_at("CQ 3Y0K JD15", -14, 406.0)], 32);
    assert_eq!(e.automation_dx().as_deref(), Some("3Y0K"), "premise");
    e.automation_abandon_for_test();
    e.automation_backdate_band(1_300);
    e.ingest_decodes_for_test(&[], 36);
    assert_eq!(e.settings.band, "40m", "under 25 minutes");
    e.automation_backdate_band(250);
    e.ingest_decodes_for_test(&[], 38);
    assert_eq!(e.settings.band, "20m", "25 minutes");
}

#[test]
fn a_garbled_spot_is_no_station_to_chase() {
    // 2026-11-23, live: "E34XQXUYE9H" was chased for five minutes as a new entity, and
    // "89EARO/J8Z5" as a band slot.
    for call in ["E34XQXUYE9H", "89EARO/J8Z5", "KH6XX.W0"] {
        let mut e = two_band_pilot();
        e.autopilot_feed(vec![crate::automation::FeedEntry {
            new_entity: true,
            ..state_spot(call, "17m", 18.101)
        }]);
        e.ingest_decodes_for_test(&[], 30);
        e.ingest_decodes_for_test(&[], 32);
        assert_eq!(e.settings.band, "20m", "{call} is no callsign");
    }
}

/// A pilot on one band all day (no schedule move), listening one slot, with Answer callers on and
/// its takeover setting at "never".
fn answering_pilot() -> Engine {
    let mut e = engine();
    e.settings.automation.autopilot_enabled = true;
    e.settings.automation.answer_callers = true;
    e.settings.automation.answer_takeover_slots = 0;
    e.settings.automation.tiers.grid = false;
    e.settings.automation.schedule = vec![crate::automation::AutopilotWindow {
        start: "00:00".into(),
        bands: vec!["20m".into()],
        mode: "FT8".into(),
    }];
    e.settings.automation.listen_slots = 1;
    e.set_tx_enabled(true);
    e.set_autopilot(true).unwrap();
    e
}

/// Key our next over from slot `s` on; the slot it went out in.
fn over(e: &mut Engine, s: u64) -> u64 {
    (s..s + 3)
        .find(|slot| !e.poll_tx(*slot).is_empty())
        .expect("an over goes out")
}

/// The operator double-clicks `call`'s CQ heard in slot 11.
fn pick(e: &mut Engine, call: &str) {
    let cq = format!("CQ {call} JN88");
    e.ingest_decodes_for_test(&[dec_snr(&cq, -12)], 11);
    e.work_station(call, None, Some(&cq), Some(-12), Some(1500.0))
        .unwrap();
}

/// Field report 2026-11-23: the operator double-clicked DO8JB, who left nine calls unanswered, and
/// N9ATF, calling us meanwhile, was not answered. "If no ongoing QSO is happening and someone is
/// calling us, we answer."
#[test]
fn a_caller_is_answered_at_once_while_our_call_is_unanswered() {
    let mut e = answering_pilot();
    pick(&mut e, "DO8JB");
    let s = over(&mut e, 12);
    e.ingest_decodes_for_test(&[], s + 1);
    assert_eq!(e.automation_dx().as_deref(), Some("DO8JB"));
    let s = over(&mut e, s + 2);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -24)], s + 1);
    assert_eq!(
        e.automation_dx().as_deref(),
        Some(CALLER),
        "K1ABC is answered with the takeover setting at never"
    );
    assert!(tx_now(&e).starts_with("K1ABC OM0RX"), "{:?}", tx_now(&e));
}

/// A station that has addressed us is a contact under way: a second caller does not cut it off.
#[test]
fn a_second_caller_does_not_cut_off_the_one_we_answer() {
    let mut e = answering_pilot();
    pick(&mut e, "DO8JB");
    let s = over(&mut e, 12);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -24)], s + 1);
    assert_eq!(e.automation_dx().as_deref(), Some(CALLER), "premise");
    let s = over(&mut e, s + 2);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX W1AW FN31", -5)], s + 1);
    assert_eq!(e.automation_dx().as_deref(), Some(CALLER));
}

#[test]
fn the_operators_call_to_a_dxpedition_is_kept_from_a_caller_with_the_autopilot_on() {
    let mut e = answering_pilot();
    e.autopilot_feed(vec![dxped_spot(DX, "20m", 14.074)]);
    pick(&mut e, DX);
    let mut s = 10;
    for _ in 0..6 {
        s = over(&mut e, s + 2);
        e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -5)], s + 1);
        assert_eq!(e.automation_dx().as_deref(), Some(DX), "slot {}", s + 1);
    }
}

/// "Just 4 calls, no answer move on": the operator's own pick too, and the fifth is never sent.
#[test]
fn four_unanswered_calls_then_the_autopilot_moves_on() {
    let mut e = answering_pilot();
    pick(&mut e, "DO8JB");
    let mut s = 10;
    for _ in 0..4 {
        s = over(&mut e, s + 2);
        e.ingest_decodes_for_test(&[], s + 1);
        assert_eq!(
            e.automation_dx().as_deref(),
            Some("DO8JB"),
            "slot {}: the last call's answer is still waited for",
            s + 1
        );
    }
    assert_eq!(e.automation_dto().calls_made, Some(4));
    assert!(e.poll_tx(s + 2).is_empty(), "no fifth call");
    e.ingest_decodes_for_test(&[], s + 2);
    assert_eq!(e.automation_dx(), None, "moved on");
    assert!(e.tx_enabled && e.automation_dto().autopilot_on);
}

/// "Receive an answer later: take it": the station that left four calls unanswered calls us.
#[test]
fn a_late_answer_is_taken() {
    let mut e = answering_pilot();
    pick(&mut e, "DO8JB");
    let mut s = 10;
    for _ in 0..4 {
        s = over(&mut e, s + 2);
        e.ingest_decodes_for_test(&[], s + 1);
    }
    e.ingest_decodes_for_test(&[], s + 2);
    assert_eq!(e.automation_dx(), None, "premise: given up");
    e.ingest_decodes_for_test(&[dec_snr("OM0RX DO8JB -11", -9)], s + 3);
    assert_eq!(e.automation_dx().as_deref(), Some("DO8JB"));
    assert!(tx_now(&e).contains("R-"), "{:?}", tx_now(&e));
}

/// The answer to the fourth call, in the boundary fold of the next slot, is taken.
#[test]
fn the_answer_to_the_last_call_is_taken() {
    let mut e = answering_pilot();
    pick(&mut e, "DO8JB");
    let mut s = 10;
    for _ in 0..3 {
        s = over(&mut e, s + 2);
        e.ingest_decodes_for_test(&[], s + 1);
    }
    s = over(&mut e, s + 2);
    e.ingest_decodes_for_test(&[], s + 1);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX DO8JB -11", -9)], s + 1);
    assert_eq!(e.automation_dx().as_deref(), Some("DO8JB"));
    assert!(tx_now(&e).contains("R-"), "{:?}", tx_now(&e));
    assert!(!e.poll_tx(s + 2).is_empty(), "our R-report goes out");
}

/// The limit is the unanswered call's, never the contact's: a station that answered keeps the
/// sequencer's own.
#[test]
fn a_station_that_answered_is_not_given_four_calls_only() {
    let mut e = answering_pilot();
    pick(&mut e, "DO8JB");
    let mut s = over(&mut e, 12);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX DO8JB -11", -9)], s + 1);
    for _ in 0..5 {
        s = over(&mut e, s + 2);
        e.ingest_decodes_for_test(&[], s + 1);
        assert_eq!(
            e.automation_dx().as_deref(),
            Some("DO8JB"),
            "slot {}",
            s + 1
        );
    }
}

/// With the autopilot off, the operator's call goes on as before: no limit of ours.
#[test]
fn with_the_autopilot_off_a_call_goes_on() {
    let mut e = engine();
    pick(&mut e, "DO8JB");
    let mut s = 10;
    for _ in 0..6 {
        s = over(&mut e, s + 2);
        e.ingest_decodes_for_test(&[], s + 1);
    }
    assert_eq!(e.automation_dx().as_deref(), Some("DO8JB"));
}

/// Europe switched off: its CQs and spots are left alone; a European calling us is answered.
#[test]
fn a_skipped_continent_is_not_called_but_its_callers_are_answered() {
    let eu = |e: &mut Engine| {
        e.set_dxcc_resolver(|call| {
            Some(
                if call.starts_with('D') {
                    "Germany"
                } else {
                    "United States"
                }
                .to_string(),
            )
        });
        e.set_automation_entity_resolver(|name| super::autopilot::EntityInfo {
            continent: Some(if name == "Germany" { "EU" } else { "NA" }.into()),
            dxcc: None,
        });
        e.settings.automation.tiers.atno = false;
        e.settings.automation.skip_continents = vec!["EU".into()];
    };
    let mut e = answering_pilot();
    eu(&mut e);
    e.ingest_decodes_for_test(&[], 29);
    for s in [30, 32] {
        e.ingest_decodes_for_test(&[dec_snr("CQ DL1ABC JO31", -3)], s);
        assert_eq!(e.automation_dx(), None, "a European CQ is left alone");
    }
    e.ingest_decodes_for_test(
        &[dec_snr("CQ DL1ABC JO31", -3), dec_snr("CQ N0ABC EM48", -15)],
        34,
    );
    assert_eq!(e.automation_dx().as_deref(), Some("N0ABC"));

    let mut e = answering_pilot();
    eu(&mut e);
    e.ingest_decodes_for_test(&[], 29);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX DL1ABC JO31", -3)], 31);
    assert_eq!(e.automation_dx().as_deref(), Some("DL1ABC"), "answered");

    let mut e = two_band_pilot();
    eu(&mut e);
    e.autopilot_feed(vec![crate::automation::FeedEntry {
        new_slot: true,
        new_state: false,
        ..state_spot("DL1ABC", "17m", 18.101)
    }]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[], 32);
    assert_eq!(e.settings.band, "20m", "its spot is not chased");
}

/// A DXCC slot waits five minutes after an unheard chase, not the ten of a state spot.
#[test]
fn a_dxcc_slot_is_chased_again_after_five_minutes() {
    let mut e = two_band_pilot();
    e.settings.automation.tiers.us_state = true;
    e.autopilot_feed(vec![state_spot("N7ILO", "17m", 18.101)]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[], 32);
    e.automation_backdate_band(400);
    e.ingest_decodes_for_test(&[], 34);
    e.ingest_decodes_for_test(&[], 36);
    let (band, freq) = other_band(&e);
    let here = e.settings.band.clone();
    let slot = |call: &str| crate::automation::FeedEntry {
        new_slot: true,
        new_state: false,
        ..state_spot(call, band, freq)
    };
    e.autopilot_feed(vec![state_spot("K0JWI", band, freq)]);
    e.automation_backdate_band(310);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -5)], 38);
    assert_eq!(e.settings.band, here, "the state spot still waits");
    e.autopilot_feed(vec![slot("5U4JSF")]);
    for s in [40, 42, 44, 46, 48, 50] {
        e.ingest_decodes_for_test(&[], s);
    }
    assert_eq!(e.settings.band, band, "the DXCC slot is chased");
}

/// Operator, 2026-11-23: "I don't see it switching to 30 m, I thought it is configured". Every chase
/// restarted the rotation from the spot's band, so spots on 160 and 80 m kept the later bands from
/// ever coming up. The rotation goes on from the band the chase left.
#[test]
fn a_chase_does_not_restart_the_band_rotation() {
    let mut e = two_band_pilot();
    e.settings.automation.schedule[0].bands =
        vec!["20m".into(), "17m".into(), "15m".into(), "12m".into()];
    let spot = |call: &str| crate::automation::FeedEntry {
        new_slot: true,
        new_state: false,
        ..state_spot(call, "20m", 14.074)
    };
    let mut s = 30;
    // 20 m is quiet: the schedule moves to 17 m.
    while e.settings.band == "20m" && s < 60 {
        e.automation_backdate_band(10_000);
        e.ingest_decodes_for_test(&[], s);
        s += 2;
    }
    assert_eq!(e.settings.band, "17m", "premise");
    // A needed spot on 20 m is chased from 17 m and never heard.
    e.autopilot_feed(vec![spot("5U4JSF")]);
    while e.settings.band == "17m" && s < 100 {
        e.ingest_decodes_for_test(&[], s);
        s += 2;
    }
    assert_eq!(e.settings.band, "20m", "premise: chased");
    e.autopilot_feed(vec![]);
    while e.settings.band == "20m" && s < 160 {
        e.automation_backdate_band(10_000);
        e.ingest_decodes_for_test(&[], s);
        s += 2;
    }
    assert_eq!(e.settings.band, "15m", "on from 17 m, not from 20 m again");
}

/// Live run 2026-11-23: R3WF was picked, VU2NRO called us before our first call went out, and R3WF
/// was called. Nobody has answered us yet, so the caller comes first.
#[test]
fn a_caller_heard_before_our_first_call_is_answered_instead() {
    let mut e = answering_pilot();
    pick(&mut e, "DO8JB");
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", 3)], 11);
    assert_eq!(e.automation_dx().as_deref(), Some(CALLER));
    assert!(tx_now(&e).starts_with("K1ABC OM0RX"), "{:?}", tx_now(&e));
}

/// A contact with DO8JB up to our R-report, after which it calls somebody else. The next slot.
fn answered_then_moved_on(e: &mut Engine) -> u64 {
    pick(e, "DO8JB");
    let s = over(e, 12);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX DO8JB -11", -9)], s + 1);
    let s = over(e, s + 2);
    e.ingest_decodes_for_test(&[dec_snr("IK4CBO <DO8JB> +00", -9)], s + 1);
    assert_eq!(e.automation_dx().as_deref(), Some("DO8JB"));
    s + 1
}

/// Live run 2026-11-23: SQ2MMX answered once, worked three others and called CQ again, and was sent
/// the R-report eight times. It gets the same four as a station that never answered.
#[test]
fn a_station_that_answered_and_then_works_others_gets_four_reports() {
    let mut e = answering_pilot();
    let mut s = answered_then_moved_on(&mut e);
    for text in ["HB9DWR DO8JB -16", "HB9DWR DO8JB RR73", "CQ DO8JB JN88"] {
        s = over(&mut e, s + 1) + 1;
        e.ingest_decodes_for_test(&[dec_snr(text, -9)], s);
        assert_eq!(e.automation_dx().as_deref(), Some("DO8JB"), "slot {s}");
    }
    assert!(e.poll_tx(s + 1).is_empty(), "no fifth report");
    e.ingest_decodes_for_test(&[], s + 1);
    assert_eq!(e.automation_dx(), None, "moved on");
    assert!(e.tx_enabled && e.automation_dto().autopilot_on);
}

/// Live run 2026-11-23: F1CCQ called us, then called and worked others while TA2TDW waited.
#[test]
fn a_caller_is_answered_instead_of_a_station_working_others() {
    let mut e = answering_pilot();
    let s = answered_then_moved_on(&mut e);
    let s = over(&mut e, s + 1) + 1;
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -20)], s);
    assert_eq!(e.automation_dx().as_deref(), Some(CALLER));
}

/// It comes back to us: the contact is under way again, and the limit no longer applies.
#[test]
fn a_station_that_comes_back_is_kept() {
    let mut e = answering_pilot();
    let mut s = answered_then_moved_on(&mut e);
    for _ in 0..2 {
        s = over(&mut e, s + 1) + 1;
        e.ingest_decodes_for_test(&[dec_snr("HB9DWR DO8JB -16", -9)], s);
    }
    s = over(&mut e, s + 1) + 1;
    e.ingest_decodes_for_test(
        &[
            dec_snr("OM0RX DO8JB -11", -9),
            dec_snr("OM0RX K1ABC FN42", -5),
        ],
        s,
    );
    assert_eq!(e.automation_dx().as_deref(), Some("DO8JB"));
    assert!(!e.poll_tx(s + 1).is_empty(), "our R-report goes out again");
}

/// A station running two streams answers us and somebody else in one slot: it has not moved on.
#[test]
fn a_station_working_two_at_once_has_not_moved_on() {
    let mut e = answering_pilot();
    pick(&mut e, "DO8JB");
    let s = over(&mut e, 12);
    e.ingest_decodes_for_test(
        &[
            dec_snr("OM0RX DO8JB -11", -9),
            dec_snr("IK4CBO DO8JB -05", -9),
        ],
        s + 1,
    );
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -5)], s + 1);
    assert_eq!(e.automation_dx().as_deref(), Some("DO8JB"));
}

/// Live run 2026-11-23: VK9XY ran FT4 at 978 Hz, +5 dB, and was never called: a station below
/// 1000 Hz was only called with Hound on, and Hound exists only in FT8. It is called from 1200 Hz
/// above it, as every DXpedition is.
#[test]
fn a_dxpedition_below_1000_hz_is_called_without_hound() {
    for hunt_hound in [true, false] {
        for mode in ["FT4", "FT8"] {
            if mode == "FT8" && hunt_hound {
                continue; // Fox/Hound: `a_fox_switches_the_hunt_to_hound_and_stop_tx_puts_it_back`.
            }
            let mut e = two_band_pilot();
            e.settings.automation.hunt_hound = hunt_hound;
            e.autopilot_feed(vec![crate::automation::FeedEntry {
                mode: Some(mode.into()),
                ..dxped_spot("3Y0K", "40m", 7.056)
            }]);
            e.ingest_decodes_for_test(&[], 30);
            assert_eq!(e.settings.band, "40m", "premise: hunting");
            e.ingest_decodes_for_test(&[dec_at("CQ 3Y0K JD15", 5, 978.0)], 32);
            assert_eq!(e.automation_dx().as_deref(), Some("3Y0K"), "{mode}");
            let above = e.tx_offset_hz - 400.0;
            assert!(
                (1195.0..=2500.0).contains(&above),
                "{mode}: {above} Hz above where it now sits"
            );
            assert!(e.tx_enabled && e.settings.special_op == crate::settings::SpecialOp::None);
        }
    }
}

/// A needed station (a spotted band slot, and by its line a new reference) heard on 17 m, that left
/// four calls unanswered; the slot after it was given up in.
fn an_unanswered_needed_station() -> (Engine, u64) {
    let mut e = two_band_pilot();
    e.autopilot_feed(vec![crate::automation::FeedEntry {
        call: "5U4JSF".into(),
        band: "17m".into(),
        mode: Some("FT8".into()),
        freq_mhz: Some(18.101),
        new_slot: true,
        new_reference: true,
        ..Default::default()
    }]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[], 32);
    assert_eq!(e.settings.band, "17m", "premise: the spot is chased");
    let mut s = 34;
    for _ in 0..3 {
        e.ingest_decodes_for_test(&[dec_snr("CQ 5U4JSF JK73", -15)], s);
        s += 2;
        if e.automation_dx().is_some() {
            break;
        }
    }
    assert_eq!(e.automation_dx().as_deref(), Some("5U4JSF"), "premise");
    for _ in 0..4 {
        s = over(&mut e, s) + 1;
        e.ingest_decodes_for_test(&[dec_snr("CQ 5U4JSF JK73", -15)], s);
    }
    s += 2;
    e.ingest_decodes_for_test(&[], s);
    assert_eq!(e.automation_dx(), None, "premise: given up");
    (e, s + 1)
}

/// Live run 2026-11-23 (xlog.26): 8Q7XX, a needed 80 m slot, left four calls unanswered and went on
/// calling CQ for four minutes, with nobody else to call; the autopilot only listened until the
/// band was quiet, and left. A needed station still calling CQ is called again instead.
#[test]
fn a_needed_station_still_calling_cq_is_called_again_when_nobody_else_is_wanted() {
    let (mut e, s) = an_unanswered_needed_station();
    let cq = [dec_snr("CQ 5U4JSF JK73", -15)];
    e.ingest_decodes_for_test(&cq, s);
    assert_eq!(e.automation_dx(), None, "not at once");
    e.automation_backdate_attempts(90);
    e.ingest_decodes_for_test(&cq, s + 2);
    assert_eq!(e.automation_dx().as_deref(), Some("5U4JSF"));
}

/// Anybody else wanted is called first: four calls, then the next station.
#[test]
fn another_wanted_station_comes_before_calling_one_again() {
    let (mut e, s) = an_unanswered_needed_station();
    e.settings.automation.tiers.grid = true;
    e.automation_backdate_attempts(90);
    e.ingest_decodes_for_test(
        &[dec_snr("CQ 5U4JSF JK73", -5), dec_snr("CQ N0ABC EM48", -18)],
        s,
    );
    assert_eq!(e.automation_dx().as_deref(), Some("N0ABC"));
}

/// Calling it again does not hold the band past its quiet time, and an ordinary station is not
/// called again at all.
#[test]
fn calling_again_does_not_hold_the_band_and_is_not_for_ordinary_calls() {
    let (mut e, mut s) = an_unanswered_needed_station();
    e.automation_backdate_attempts(90);
    e.automation_backdate_band(200);
    e.ingest_decodes_for_test(&[dec_snr("CQ 5U4JSF JK73", -15)], s);
    assert_eq!(e.automation_dx().as_deref(), Some("5U4JSF"), "premise");
    s += 1;
    for _ in 0..4 {
        s = over(&mut e, s) + 1;
        e.ingest_decodes_for_test(&[dec_snr("CQ 5U4JSF JK73", -15)], s);
    }
    e.automation_backdate_band(150);
    e.automation_backdate_attempts(90);
    e.ingest_decodes_for_test(&[], s + 2);
    e.ingest_decodes_for_test(&[dec_snr("CQ 5U4JSF JK73", -15)], s + 4);
    assert_eq!(
        e.settings.band, "20m",
        "left once it has been quiet for five minutes"
    );

    let (mut e, s) = an_unanswered_needed_station();
    e.automation_backdate_attempts(90);
    e.automation_fail_for_test("N0ABC", 90);
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], s);
    assert_eq!(e.automation_dx(), None);
}

/// Same run: the hunt of a station heard and called four times ended as "not heard".
#[test]
fn a_chase_that_called_its_station_does_not_end_as_not_heard() {
    let (mut e, s) = an_unanswered_needed_station();
    e.automation_backdate_band(310);
    e.ingest_decodes_for_test(&[], s);
    let log = e.automation_dto().log;
    assert!(
        log.iter()
            .any(|l| l.contains("5U4JSF did not answer on 17m: back to the schedule")),
        "{log:?}"
    );
    assert!(!log.iter().any(|l| l.contains("not heard")), "{log:?}");
}

/// Live run 2026-11-23 (xlog.26): D2ACE answered just after it was given up, sent us its report
/// three times without copying ours, and was dropped after four R-reports as "did not answer". A
/// late answer is an answer: the contact keeps the sequencer's own limit.
#[test]
fn a_late_answer_is_not_dropped_after_four_calls() {
    let mut e = answering_pilot();
    pick(&mut e, "DO8JB");
    let mut s = 10;
    for _ in 0..4 {
        s = over(&mut e, s + 2);
        e.ingest_decodes_for_test(&[], s + 1);
    }
    e.ingest_decodes_for_test(&[], s + 2);
    assert_eq!(e.automation_dx(), None, "premise: given up");
    e.ingest_decodes_for_test(&[dec_snr("OM0RX DO8JB -11", -9)], s + 3);
    assert_eq!(e.automation_dx().as_deref(), Some("DO8JB"), "premise");
    s += 1;
    for n in 0..6 {
        s = over(&mut e, s + 2);
        e.ingest_decodes_for_test(&[dec_snr("OM0RX DO8JB -11", -9)], s + 1);
        assert_eq!(e.automation_dx().as_deref(), Some("DO8JB"), "over {n}");
    }
    assert_eq!(e.automation_dto().calls_made, None, "not a count of calls");
}

/// Live run 2026-11-23 (xlog.26): in 65 minutes 40 m had three long turns, taken there by chases,
/// and 30 m none: the rotation still stood on 60 m, so 40 m came next once more. The scheduled band
/// that has waited longest comes next, whatever took the radio to the others.
#[test]
fn the_autopilot_moves_to_the_band_that_has_waited_longest() {
    let mut e = two_band_pilot();
    e.settings.automation.schedule[0].bands =
        vec!["20m".into(), "17m".into(), "15m".into(), "12m".into()];
    let mut s = 30;
    while e.settings.band == "20m" && s < 60 {
        e.automation_backdate_band(10_000);
        e.ingest_decodes_for_test(&[], s);
        s += 2;
    }
    assert_eq!(e.settings.band, "17m", "premise");
    e.autopilot_feed(vec![crate::automation::FeedEntry {
        new_slot: true,
        new_state: false,
        ..state_spot("5U4JSF", "20m", 14.074)
    }]);
    while e.settings.band == "17m" && s < 100 {
        e.ingest_decodes_for_test(&[], s);
        s += 2;
    }
    assert_eq!(e.settings.band, "20m", "premise: chased from 17 m");
    // Another chase takes it to 15 m, which has a full turn, and one to 20 m again.
    e.automation_backdate_band(200);
    e.automation_go_for_test("15m");
    e.automation_backdate_band(200);
    e.automation_go_for_test("20m");
    while e.settings.band == "20m" && s < 200 {
        e.automation_backdate_band(10_000);
        e.ingest_decodes_for_test(&[], s);
        s += 2;
    }
    assert_eq!(e.settings.band, "12m", "15 m has just had its turn");
}

/// A band a chase left at once has not had its turn.
#[test]
fn a_band_left_at_once_for_a_spot_is_still_due() {
    let mut e = two_band_pilot();
    e.settings.automation.schedule[0].bands = vec!["20m".into(), "17m".into(), "15m".into()];
    let mut s = 30;
    while e.settings.band == "20m" && s < 60 {
        e.automation_backdate_band(10_000);
        e.ingest_decodes_for_test(&[], s);
        s += 2;
    }
    assert_eq!(e.settings.band, "17m", "premise");
    // 17 m is left for a 15 m spot within its first slots.
    e.autopilot_feed(vec![crate::automation::FeedEntry {
        new_slot: true,
        new_state: false,
        ..state_spot("5U4JSF", "15m", 21.074)
    }]);
    while e.settings.band == "17m" && s < 100 {
        e.ingest_decodes_for_test(&[], s);
        s += 2;
    }
    assert_eq!(e.settings.band, "15m", "premise: chased");
    while e.settings.band == "15m" && s < 160 {
        e.automation_backdate_band(10_000);
        e.ingest_decodes_for_test(&[], s);
        s += 2;
    }
    assert_eq!(e.settings.band, "17m", "20 m had its turn, 17 m did not");
}

/// Live run 2026-11-24 (xlog.27): XE2X, a needed 80 m slot, left eight calls unanswered, then ended
/// two contacts (73, RR73) in three minutes, called by the next station each time, beside an
/// idle autopilot that waited for a CQ. It is called when it ends a contact, not in mid-contact.
#[test]
fn a_needed_station_is_called_again_when_it_ends_a_contact() {
    let (mut e, s) = an_unanswered_needed_station();
    e.automation_backdate_attempts(90);
    e.ingest_decodes_for_test(&[dec_snr("F4IEX 5U4JSF -15", -12)], s);
    assert_eq!(e.automation_dx(), None, "in contact with somebody");
    e.ingest_decodes_for_test(&[dec_snr("F4IEX 5U4JSF RR73", -12)], s + 2);
    assert_eq!(e.automation_dx().as_deref(), Some("5U4JSF"));
    over(&mut e, s + 3);

    let (mut e, s) = an_unanswered_needed_station();
    e.automation_backdate_attempts(90);
    e.ingest_decodes_for_test(&[dec_snr("F4IEX 5U4JSF 73", -12)], s);
    assert_eq!(e.automation_dx().as_deref(), Some("5U4JSF"), "its 73 too");
}

/// A pilot on 17 m for 5U4JSF (needed), calling N0ABC, only a new grid, meanwhile; the slot after
/// its first call.
fn calling_a_grid_while_chasing() -> (Engine, u64) {
    let mut e = two_band_pilot();
    e.settings.automation.tiers.grid = true;
    e.autopilot_feed(vec![crate::automation::FeedEntry {
        call: "5U4JSF".into(),
        band: "17m".into(),
        mode: Some("FT8".into()),
        freq_mhz: Some(18.101),
        new_slot: true,
        new_reference: true,
        ..Default::default()
    }]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[], 32);
    assert_eq!(e.settings.band, "17m", "premise: the spot is chased");
    let mut s = 34;
    while e.automation_dx().is_none() && s < 60 {
        e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], s);
        s += 2;
    }
    assert_eq!(e.automation_dx().as_deref(), Some("N0ABC"), "premise");
    let next = over(&mut e, s - 1) + 1;
    (e, next)
}

/// Live run 2026-11-24 (xlog.27): on 160 m for XE2X, the autopilot called MW7TTA, a new grid, for
/// two minutes while XE2X called CQ, and XE2X was not called.
#[test]
fn an_unanswered_grid_gives_way_to_a_needed_station() {
    let (mut e, s) = calling_a_grid_while_chasing();
    e.ingest_decodes_for_test(&[dec_snr("CQ 5U4JSF JK73", -20)], s);
    assert_eq!(e.automation_dx().as_deref(), Some("5U4JSF"));
    over(&mut e, s + 1);
}

/// A station that has answered is worked to the end, and another grid takes nothing over.
#[test]
fn a_grid_that_answered_and_another_grid_change_nothing() {
    let (mut e, s) = calling_a_grid_while_chasing();
    let me = e.settings.mycall.clone();
    e.ingest_decodes_for_test(
        &[
            dec_snr(&format!("{me} N0ABC -07"), -3),
            dec_snr("CQ 5U4JSF JK73", -20),
        ],
        s,
    );
    assert_eq!(e.automation_dx().as_deref(), Some("N0ABC"), "answered");

    let (mut e, s) = calling_a_grid_while_chasing();
    e.ingest_decodes_for_test(&[dec_snr("CQ K1XYZ FN42", 5)], s);
    assert_eq!(e.automation_dx().as_deref(), Some("N0ABC"), "another grid");
}

/// Live run 2026-11-24 (xlog.28): on 17 m FT4 for XU7O, up to five stations a period were heard
/// calling it and working it, and the hunt ended as "not heard" after five minutes. The operator's
/// rule is ten.
#[test]
fn a_hunt_stays_ten_minutes_while_stations_are_heard_calling_the_dxpedition() {
    let hunt = || {
        let mut e = two_band_pilot();
        e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
        e.ingest_decodes_for_test(&[], 30);
        assert_eq!(e.settings.band, "40m", "premise: hunting");
        e
    };
    let calling = [dec_snr("3Y0K SP6CIK JO80", -7)];
    let mut e = hunt();
    e.automation_backdate_band(310);
    e.ingest_decodes_for_test(&calling, 32);
    assert_eq!(
        e.settings.band, "40m",
        "on the air: stays past five minutes"
    );
    e.autopilot_feed(vec![]);
    e.ingest_decodes_for_test(&[], 34);
    assert_eq!(e.settings.band, "40m", "and past its spot's age");
    e.automation_backdate_band(310);
    e.ingest_decodes_for_test(&calling, 36);
    assert_eq!(e.settings.band, "40m", "ten minutes from when it was heard");
    e.automation_backdate_band(310);
    e.ingest_decodes_for_test(&calling, 38);
    assert_eq!(e.settings.band, "20m", "over after ten");

    // CONTROL: nobody heard calling it, five minutes.
    let mut e = hunt();
    e.automation_backdate_band(310);
    e.ingest_decodes_for_test(&[dec_snr("W1AW SP6CIK JO80", -7)], 32);
    assert_eq!(e.settings.band, "20m", "not on the air: over after five");
}

/// The operator's rule: a DXpedition keeps being called. After any number of hunts it is hunted
/// again an hour later.
#[test]
fn a_dxpedition_is_hunted_again_after_any_number_of_hunts() {
    let mut e = two_band_pilot();
    for _ in 0..5 {
        e.automation_fail_for_test("3Y0K|40M|FT8", 3_700);
    }
    e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
    e.ingest_decodes_for_test(&[], 30);
    assert_eq!(
        e.settings.band, "20m",
        "premise: five failures leave a station for the session"
    );

    let mut e = two_band_pilot();
    for s in [30, 34, 38, 42, 46] {
        e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
        e.ingest_decodes_for_test(&[], s);
        assert_eq!(e.settings.band, "40m", "hunted at slot {s}");
        e.autopilot_feed(vec![]);
        e.ingest_decodes_for_test(&[], s + 2);
        assert_eq!(e.settings.band, "20m", "premise: the hunt is over");
        e.automation_backdate_attempts(3_700);
        e.automation_backdate_chased(3_700);
    }
}

/// A hunt whose DXpedition answered, then went quiet until the sequencer's call cap stalled the
/// contact; the next slot.
fn a_stalled_hunt() -> (Engine, u64) {
    let mut e = two_band_pilot();
    e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[dec_at("CQ 3Y0K JD15", -14, 406.0)], 32);
    assert_eq!(e.automation_dx().as_deref(), Some("3Y0K"), "premise");
    let mut s = over(&mut e, 33) + 1;
    e.ingest_decodes_for_test(&[dec_at("OM0RX 3Y0K -10", -12, 406.0)], s);
    s += 1;
    let stalled = |e: &Engine| matches!(&e.mode, Mode::Qso { station, .. } if station.stalled());
    for _ in 0..60 {
        if stalled(&e) {
            break;
        }
        let _ = e.poll_tx(s);
        s += 1;
        let _ = e.poll_tx(s);
        e.ingest_decodes_for_test(&[], s);
        s += 1;
    }
    assert!(stalled(&e), "premise: the call cap stalled it");
    assert!(e.tx_enabled, "premise");
    (e, s)
}

/// Live run 2026-11-24 (xlog.28): XU7O's contact stalled by its call cap and stayed armed; six
/// minutes later, in the middle of the hunt, the TX watchdog disarmed transmit and the autopilot went
/// off. A stalled contact is ended first; the watchdog itself is unchanged.
#[test]
fn a_stalled_contact_does_not_let_the_watchdog_stop_the_autopilot() {
    let (mut e, mut s) = a_stalled_hunt();
    e.ingest_decodes_for_test(&[], s);
    e.tx_watchdog_start = Some(now_unix_secs() - 400);
    e.ingest_decodes_for_test(&[], s + 1);
    for _ in 0..4 {
        let _ = e.poll_tx(s);
        s += 1;
    }
    assert!(
        e.tx_enabled && e.automation_dto().autopilot_on,
        "kept for a late answer"
    );
    assert_eq!(e.automation_dx().as_deref(), Some("3Y0K"));
    e.automation_backdate_stall(130);
    e.ingest_decodes_for_test(&[], s);
    assert_eq!(e.automation_dx(), None, "then ended");
    e.tx_watchdog_start = Some(now_unix_secs() - 400);
    for _ in 0..4 {
        let _ = e.poll_tx(s);
        s += 1;
    }
    e.ingest_decodes_for_test(&[], s);
    assert!(e.tx_enabled && e.automation_dto().autopilot_on);
    // Still hunting: called again when it is heard again.
    e.ingest_decodes_for_test(&[dec_at("CQ 3Y0K JD15", -14, 406.0)], s + 2);
    assert_eq!(e.automation_dx().as_deref(), Some("3Y0K"));
}

/// The watchdog still stops a stalled contact left armed when the autopilot's step stops running.
#[test]
fn the_watchdog_still_stops_a_stalled_contact_without_the_autopilots_step() {
    let (mut e, mut s) = a_stalled_hunt();
    e.tx_watchdog_start = Some(now_unix_secs() - 400);
    for _ in 0..4 {
        let _ = e.poll_tx(s);
        s += 1;
    }
    assert!(!e.tx_enabled);
}

/// Same run: the transmit offset, held at 1375 Hz, stayed at the DXpedition's 1606 Hz for the calls
/// that followed.
#[test]
fn the_held_transmit_offset_returns_after_a_dxpedition() {
    let mut e = two_band_pilot();
    e.hold_tx_freq = true;
    e.set_tx_offset(1375.0);
    e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[dec_at("CQ 3Y0K JD15", -14, 406.0)], 32);
    assert!((e.tx_offset_hz - 1606.0).abs() < 0.5, "premise");
    over(&mut e, 33);
    e.automation_abandon_for_test();
    e.autopilot_feed(vec![]);
    e.automation_backdate_band(1_000);
    e.ingest_decodes_for_test(&[], 36);
    assert_eq!(e.settings.band, "20m", "premise: the hunt is over");
    for s in [38, 40, 42] {
        e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], s);
    }
    assert_eq!(e.automation_dx().as_deref(), Some("N0ABC"), "premise");
    assert!((e.tx_offset_hz - 1375.0).abs() < 0.5, "{}", e.tx_offset_hz);
}

/// A decode folded and kept in Band Activity, as the live path keeps it.
fn hear(e: &mut Engine, d: modes::Decode, slot: u64) {
    e.ingest_decodes_for_test(std::slice::from_ref(&d), slot);
    e.last_wire_decodes = vec![d];
    e.note_band_activity(slot);
}

/// Live run 2026-11-24 (xlog.29): DK2WH called JT1BZ on 1374 Hz while we called it on our held
/// 1375 Hz, and was answered. A station is called on a frequency clear of the signals of our own
/// period, near the operator's, which comes back for the next call and when the autopilot stops.
#[test]
fn a_station_is_called_on_a_clear_transmit_frequency() {
    let on = |clear: bool| {
        let mut e = two_band_pilot();
        e.settings.automation.clear_tx_offset = clear;
        e.hold_tx_freq = true;
        e.set_tx_offset(1375.0);
        // Our period's audio is slot 30's (folded at 31); slot 31's is the station's own.
        hear(&mut e, dec_at("JT1BZ DK2WH JN49", -3, 1374.0), 31);
        hear(&mut e, dec_at("K1AAA W9XYZ R-10", -3, 1430.0), 32);
        hear(&mut e, dec_at("CQ N0ABC EM48", -3, 700.0), 34);
        hear(&mut e, dec_at("CQ N0ABC EM48", -3, 700.0), 36);
        assert_eq!(e.automation_dx().as_deref(), Some("N0ABC"), "premise");
        e
    };
    let mut e = on(true);
    let moved = e.tx_offset_hz - 1374.0;
    assert!((55.0..=60.0).contains(&moved.abs()), "{}", e.tx_offset_hz);
    e.set_autopilot(false).unwrap();
    assert!((e.tx_offset_hz - 1375.0).abs() < 0.5, "back when it stops");

    // The other period's signals are no reason to move, and neither is the switch off.
    let mut e = on(false);
    assert!((e.tx_offset_hz - 1375.0).abs() < 0.5, "off");
    e.set_autopilot(false).unwrap();

    // Moved by the operator during the call: left where the operator put it.
    let mut e = on(true);
    e.set_tx_offset(900.0);
    e.set_autopilot(false).unwrap();
    assert!((e.tx_offset_hz - 900.0).abs() < 0.5);
}

/// A DXpedition's is clear too, and never below 1200 Hz above it.
#[test]
fn a_dxpedition_is_called_on_a_clear_frequency_above_it() {
    let mut e = two_band_pilot();
    e.hold_tx_freq = true;
    e.set_tx_offset(1375.0);
    e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
    e.ingest_decodes_for_test(&[], 30);
    hear(&mut e, dec_at("3Y0K DL1ABC JO31", -14, 1610.0), 31);
    hear(&mut e, dec_at("CQ 3Y0K JD15", -14, 406.0), 32);
    assert_eq!(e.automation_dx().as_deref(), Some("3Y0K"), "premise");
    assert!(
        (1000.0..=2500.0).contains(&e.tx_offset_hz) && (e.tx_offset_hz - 1610.0).abs() >= 55.0,
        "{}",
        e.tx_offset_hz
    );
}

/// A DXpedition spotted where its entity needs nothing (the board's rows do not list it there).
fn spotted(call: &str, band: &str, freq: f64, mode: &str) -> crate::automation::FeedEntry {
    crate::automation::FeedEntry {
        mode: Some(mode.into()),
        new_slot: false,
        ..dxped_spot(call, band, freq)
    }
}

/// Live run 2026-11-25: 3B8/E70A ran two streams at 2489 Hz on the 18.100 channel. The dial was moved
/// up 2090 Hz to put it at 399 Hz and it was called at 1599 Hz, 3.7 kHz above its dial and outside its
/// receiver, for ten minutes; then it was taken for a Fox at 399 Hz.
#[test]
fn a_dxpedition_high_in_the_channel_is_called_from_below_it() {
    let mut e = two_band_pilot();
    e.hold_tx_freq = true;
    e.set_tx_offset(1375.0);
    e.autopilot_feed(vec![spotted("3B8/E70A", "17m", 18.100, "FT8")]);
    e.ingest_decodes_for_test(&[], 30);
    assert_eq!(e.settings.band, "17m", "premise: hunting");
    let dial = e.settings.dial_mhz;
    e.ingest_decodes_for_test(&[dec_at("CQ 3B8/E70A", -5, 2489.0)], 32);
    assert!(e.automation_dx().is_some(), "premise: called");
    assert!(
        (e.settings.dial_mhz - dial).abs() < 1e-9,
        "the dial stays: {}",
        e.settings.dial_mhz
    );
    assert!(
        (400.0..=1289.0).contains(&e.tx_offset_hz),
        "{}",
        e.tx_offset_hz
    );
}

/// Same run: a hunt broken off by hand left the dial on 18.102, and the autopilot, switched on again,
/// would have listened there. Switched off, it leaves the radio to the operator.
#[test]
fn switched_on_the_autopilot_goes_to_the_bands_channel() {
    let mut e = two_band_pilot();
    e.autopilot_feed(vec![spotted("3B8/E70A", "17m", 18.095, "FT8")]);
    e.ingest_decodes_for_test(&[], 30);
    assert!(
        (e.settings.dial_mhz - 18.095).abs() < 1e-9,
        "premise: hunting"
    );
    e.set_autopilot(false).unwrap();
    assert!(
        (e.settings.dial_mhz - 18.095).abs() < 1e-9,
        "off: left alone"
    );
    e.autopilot_feed(vec![]);
    e.set_tx_enabled(true);
    e.set_autopilot(true).unwrap();
    e.ingest_decodes_for_test(&[], 34);
    assert_eq!(e.settings.band, "17m");
    assert!(
        (e.settings.dial_mhz - 18.100).abs() < 1e-9,
        "{}",
        e.settings.dial_mhz
    );
    assert!(e.tx_enabled, "transmit put back as it was");
}

/// Same run: our own retune is no sign of a Fox.
#[test]
fn a_dxpedition_our_retune_put_low_is_no_fox() {
    let mut e = two_band_pilot();
    e.autopilot_feed(vec![dxped_spot("3Y0K", "40m", 7.056)]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[dec_at("CQ 3Y0K JD15", -5, 1500.0)], 32);
    assert!(
        (e.settings.dial_mhz - 7.0571).abs() < 1e-9,
        "premise: retuned"
    );
    e.automation_abandon_for_test();
    e.ingest_decodes_for_test(&[dec_at("CQ 3Y0K JD15", -5, 400.0)], 34);
    assert_eq!(e.settings.special_op, crate::settings::SpecialOp::None);
}

/// Operator, 2026-11-25: VK9XY, spotted on FT8 and FT4 all day, "should be always prioritised". It
/// was hunted on none of them: Christmas Island was confirmed there, so nothing was needed.
#[test]
fn a_dxpedition_not_worked_there_is_hunted_whatever_its_entity_has() {
    let mut e = two_band_pilot();
    e.autopilot_feed(vec![spotted("VK9XY", "15m", 21.140, "FT4")]);
    e.ingest_decodes_for_test(&[], 30);
    assert_eq!(e.settings.band, "15m");
    assert_eq!(e.app.tier(), Tier::Ft4);
    // CONTROL: a station that is no DXpedition is not.
    let mut e = two_band_pilot();
    e.autopilot_feed(vec![crate::automation::FeedEntry {
        dxpedition: false,
        ..spotted("VK9XY", "15m", 21.140, "FT4")
    }]);
    e.ingest_decodes_for_test(&[], 30);
    assert_eq!(e.settings.band, "20m");
}

/// The feed kept one spot of a call, and one cool-down for all its bands.
#[test]
fn every_band_and_mode_a_dxpedition_is_spotted_on_is_tried() {
    let mut e = two_band_pilot();
    let spots = || {
        vec![
            crate::automation::FeedEntry {
                heard_unix: 200,
                ..spotted("VK9XY", "40m", 7.056, "FT8")
            },
            crate::automation::FeedEntry {
                heard_unix: 100,
                ..spotted("VK9XY", "15m", 21.140, "FT4")
            },
        ]
    };
    e.autopilot_feed(spots());
    e.ingest_decodes_for_test(&[], 30);
    assert_eq!(e.settings.band, "40m", "the newest spot first");
    // Nothing of it decoded in five minutes: back, and after a rest the other one.
    e.automation_backdate_band(310);
    e.autopilot_feed(spots());
    e.ingest_decodes_for_test(&[], 34);
    assert_eq!(e.settings.band, "20m", "not heard");
    e.ingest_decodes_for_test(&[], 36);
    assert_eq!(e.settings.band, "20m", "the schedule's turn");
    e.automation_backdate_chased(200);
    e.autopilot_feed(spots());
    e.ingest_decodes_for_test(&[], 38);
    assert_eq!(e.settings.band, "15m", "not 40 m again");
}

/// A spotted DXpedition does not wait for a station that has not answered; one that has answered is
/// worked to the end.
#[test]
fn a_spotted_dxpedition_ends_an_unanswered_call() {
    let calling = || {
        let mut e = two_band_pilot();
        e.settings.automation.schedule[0].bands = vec!["20m".into()];
        for s in [30, 32] {
            e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], s);
        }
        assert_eq!(e.automation_dx().as_deref(), Some("N0ABC"), "premise");
        let s = over(&mut e, 33);
        (e, s)
    };
    let (mut e, s) = calling();
    e.autopilot_feed(vec![spotted("VK9XY", "40m", 7.056, "FT8")]);
    e.ingest_decodes_for_test(&[], s + 1);
    e.ingest_decodes_for_test(&[], s + 3);
    assert_eq!(e.settings.band, "40m");

    let (mut e, s) = calling();
    e.ingest_decodes_for_test(&[dec_snr("OM0RX N0ABC -09", -3)], s + 1);
    e.autopilot_feed(vec![spotted("VK9XY", "40m", 7.056, "FT8")]);
    e.ingest_decodes_for_test(&[], s + 3);
    assert_eq!(e.settings.band, "20m", "answered: kept");
}

/// Live run 2026-11-25: BG3TFT was picked and dropped for VK9XY at every decode, while the move to VK9XY
/// was held because 8S2DX, just worked, might still be sending. Nothing is called until the move is made.
#[test]
fn no_call_is_started_while_a_spotted_dxpedition_waits() {
    let mut e = two_band_pilot();
    e.settings.automation.schedule[0].bands = vec!["20m".into()];
    e.ingest_decodes_for_test(&[], 28);
    e.autopilot_feed(vec![spotted("VK9XY", "40m", 7.056, "FT8")]);
    // K1ABC calling us holds the band (Answer callers is off); N0ABC calls CQ beside it.
    for s in [30, 32] {
        e.ingest_decodes_for_test(
            &[
                dec_snr("CQ N0ABC EM48", -3),
                dec_snr("OM0RX K1ABC FN42", -5),
            ],
            s,
        );
        assert_eq!(e.settings.band, "20m", "premise: held at {s}");
        assert_eq!(e.automation_dx(), None, "nothing called at {s}");
    }
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 40);
    assert_eq!(e.settings.band, "40m", "then it goes");

    // Nor is a call under way dropped for a move that cannot be made.
    let mut e = two_band_pilot();
    e.settings.automation.schedule[0].bands = vec!["20m".into()];
    for s in [30, 32] {
        e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], s);
    }
    assert_eq!(e.automation_dx().as_deref(), Some("N0ABC"), "premise");
    let s = over(&mut e, 33);
    e.autopilot_feed(vec![spotted("VK9XY", "40m", 7.056, "FT8")]);
    e.ingest_decodes_for_test(&[dec_snr("OM0RX K1ABC FN42", -5)], s + 1);
    assert_eq!(e.automation_dx().as_deref(), Some("N0ABC"), "kept");
}

/// Operator, 2026-11-25: the logger turns the antenna to the DX call, so the DX call is reported while
/// the autopilot listens for a spotted station, not only once it is called.
#[test]
fn the_station_listened_for_is_the_dx_call_reported_to_loggers() {
    let mut e = two_band_pilot();
    assert_eq!(e.wsjtx_tx_status().dx_call, "", "nothing hunted");
    e.autopilot_feed(vec![spotted("VK9XY", "40m", 7.056, "FT8")]);
    e.ingest_decodes_for_test(&[], 30);
    assert_eq!(e.settings.band, "40m", "premise: hunting");
    assert_eq!(e.wsjtx_tx_status().dx_call, "VK9XY");
    // A contact in progress keeps its own call.
    e.ingest_decodes_for_test(&[dec_snr("CQ K1ABC FN42", -9)], 31);
    e.work_station("K1ABC", None, Some("CQ K1ABC FN42"), Some(-9), Some(1500.0))
        .unwrap();
    assert_eq!(e.wsjtx_tx_status().dx_call, "K1ABC");
    e.set_autopilot(false).unwrap();
    e.automation_abandon_for_test();
    assert_eq!(
        e.wsjtx_tx_status().dx_call,
        "",
        "nor with the autopilot off"
    );
}

/// Same run: OM0RX-xLog ignored "<3B8/E70A>", the DX call as it was on the air.
#[test]
fn the_dx_call_reported_has_no_brackets() {
    let mut e = two_band_pilot();
    e.autopilot_feed(vec![spotted("3B8/E70A", "17m", 18.100, "FT8")]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[dec_at("EA3BD <3B8/E70A> -07", -5, 2489.0)], 32);
    assert!(e.automation_dx().is_some(), "premise: called");
    assert_eq!(e.wsjtx_tx_status().dx_call, "3B8/E70A");
}

/// Decoded on the band we are on, low as DXpeditions sit, it is hunted there.
#[test]
fn a_dxpedition_heard_low_on_the_band_is_hunted_in_place() {
    let mut e = two_band_pilot();
    e.settings.automation.hunt_hound = false;
    e.autopilot_feed(vec![spotted("VK9XY", "20m", 14.074, "FT8")]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[dec_at("CQ VK9XY QH29", -9, 450.0)], 32);
    assert_eq!(e.automation_dx().as_deref(), Some("VK9XY"));
    assert!(e.tx_offset_hz >= 1650.0, "{}", e.tx_offset_hz);
    assert_eq!(e.settings.band, "20m");
}

/// Live run 2026-11-24 (xlog.28): new grids, one after another, kept the autopilot on 15 m for half
/// an hour while the other bands waited. Ten minutes of nothing better than grids gives way too.
#[test]
fn a_band_with_only_new_grids_gives_way_after_the_ordinary_dwell() {
    let grid_band = || {
        let mut e = two_band_pilot();
        e.settings.automation.tiers.grid = true;
        e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 30);
        e
    };
    let mut e = grid_band();
    e.automation_backdate_band(590);
    e.ingest_decodes_for_test(&[dec_snr("CQ N0ABC EM48", -3)], 32);
    assert_eq!(
        e.automation_dx().as_deref(),
        Some("N0ABC"),
        "before ten minutes"
    );
    let s = over(&mut e, 33) + 1;
    e.ingest_decodes_for_test(&[], s);
    e.automation_abandon_for_test();
    e.automation_backdate_band(20);
    e.ingest_decodes_for_test(&[dec_snr("CQ K1XYZ FN42", -3)], s + 2);
    assert_eq!(e.settings.band, "17m", "{:?}", e.automation_dto().log);
}

/// A pilot patrolling 20 m and 17 m in FT8 and FT4.
fn two_mode_pilot() -> Engine {
    let mut e = two_band_pilot();
    e.settings.automation.schedule[0].mode = "FT8+FT4".into();
    e
}

/// Operator, 2026-11-24: "it can work both FT4 and FT8 in the same time switching between bands and
/// modes". Every band has its turn in each mode.
#[test]
fn a_patrol_of_two_modes_goes_round_every_band_in_both() {
    let mut e = two_mode_pilot();
    let at = |e: &Engine| format!("{} {}", e.settings.band, mode_name_for_test(e));
    let mut seen = vec![at(&e)];
    let mut s = 30;
    while seen.len() < 5 && s < 400 {
        e.automation_backdate_band(10_000);
        e.ingest_decodes_for_test(&[], s);
        s += 2;
        if *seen.last().unwrap() != at(&e) {
            seen.push(at(&e));
        }
    }
    assert_eq!(
        seen,
        ["20m FT8", "20m FT4", "17m FT8", "17m FT4", "20m FT8"]
    );
    assert!(e.tx_enabled && e.automation_dto().autopilot_on);
}

/// A needed spot in the other mode is chased, on another band and on the same one; in a mode the
/// patrol does not run, it is not.
#[test]
fn a_spot_in_either_mode_of_the_patrol_is_chased() {
    let spot = |band: &str, mode: &str| crate::automation::FeedEntry {
        call: "5U4JSF".into(),
        band: band.into(),
        mode: Some(mode.into()),
        new_slot: true,
        ..Default::default()
    };
    for (band, mode) in [("17m", "FT4"), ("20m", "FT4"), ("17m", "FT8")] {
        let mut e = two_mode_pilot();
        e.autopilot_feed(vec![spot(band, mode)]);
        e.ingest_decodes_for_test(&[], 30);
        e.ingest_decodes_for_test(&[], 32);
        assert_eq!(
            (e.settings.band.as_str(), mode_name_for_test(&e)),
            (band, mode),
            "{:?}",
            e.automation_dto().log
        );
        assert!(e.tx_enabled);
    }
    let mut e = two_band_pilot();
    e.autopilot_feed(vec![spot("17m", "FT4")]);
    e.ingest_decodes_for_test(&[], 30);
    e.ingest_decodes_for_test(&[], 32);
    assert_eq!(e.settings.band, "20m", "FT4 is not on this patrol");
}
