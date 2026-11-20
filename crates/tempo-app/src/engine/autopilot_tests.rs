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
