// SPDX-License-Identifier: GPL-3.0-only
//! OM0RX-xNexus automation, the engine half: **Answer callers** and the **supervised autopilot**.
//! The settings, the sign-off and every pure decision live in [`crate::automation`]; this file acts
//! on those decisions through paths the operator's own clicks already take.
//!
//! # How it acts — and how it never does
//!
//! - **A contact is started exactly as a double-click starts one**
//!   ([`Engine::call_station_ctx_arming`] with `arm = false`): same resume table, same parity,
//!   same RX/TX move, same Hound rules, same auto-log. The only difference is that automation never
//!   arms transmit. It acts only while the operator's TX is ON, so a station it starts with TX off
//!   would stay silent — and it does not start one then.
//! - **Band and mode changes go through [`Engine::machinery_tune`] and [`Engine::set_tier`]**, the
//!   app's own retune paths: TX halts for the change as it does for any QSY. Because that context
//!   halt is not the operator's stop, the autopilot puts the operator's arm back afterwards
//!   (watchdog clock included) — the only place it touches the latch, and only to restore what the
//!   operator had set a moment before.
//! - **Every stop wins.** Stop TX (every local and remote stop funnels into `halt_tx`), TX Off and
//!   a tripped watchdog all switch the autopilot off; it never turns itself back on.
//! - **Never a band change in the middle of a QSO** (operator, 2026-11-21, after the first live
//!   test changed band mid-contact). Every band or mode change goes through
//!   [`Engine::automation_switch`], which refuses while [`Engine::automation_band_hold`] names a
//!   reason: a contact being called or worked, a CQ run, the partner of a contact made on our side
//!   still heard (they owe a 73 or are repeating), or anybody addressing us in the last few slots.
//!   Like WSJT-Z's hopper, which moves only from its CALLING state, it changes band only when it
//!   cannot call anyone here and every contact is finished.
//! - **DXpedition hunt.** A needed, listed DXpedition spotted on any band the licence allows is
//!   hunted on the frequency the spot names (DXpeditions often use their own), in the spot's FT
//!   mode, calling only it and stations calling us; Hound comes on when it is heard as a Fox
//!   (below 1000 Hz). Worked, out of its time budget, unheard or no longer spotted, the hunt puts
//!   Hound back and returns to the band, dial and mode it left — through the same hold.
//! - **Presence.** Every [`crate::automation::AutomationSettings::presence_check_min`] minutes the
//!   operator must confirm. Unconfirmed, no new contact starts; once the contact in progress ends,
//!   TX goes off and the autopilot switches off.

use std::collections::{HashMap, HashSet, VecDeque};

use super::{now_unix_secs, Engine, Mode};
use crate::automation::{
    self, answerable_cq, hunt_dial, in_passband, is_hunt_spot, is_rare, next_window_start,
    patrol_at, spot_call_ok, spot_tier, Attempts, AutomationDto, Candidate, FeedEntry, HourlyCount,
    PresenceClock, Stop, Tier,
};
use crate::dto::Tier as LinkTier;
use crate::settings::SpecialOp;
use tempo_core::message::{same_call, Msg};
use tempo_core::qso::State as QsoState;

/// How many slots a caller to the operator stays answerable after it was last heard.
const CALLER_FRESH_SLOTS: u64 = 3;
/// How long a spot chase may go without decoding its station before it is given up, seconds.
const CHASE_UNHEARD_SECS: u64 = 300;
/// How lately a station must have been heard calling it for the DXpedition to count as on the air,
/// seconds.
const HUNT_CALLED_SECS: u64 = 120;
/// After such a hunt, how long before the same DXpedition is hunted on another band or in another
/// mode, seconds: the schedule gets its turn between.
const HUNT_REST_SECS: u64 = 180;
/// Failures counted against a hunted DXpedition at most: it is tried again each hour, never left
/// for the session.
const HUNT_MAX_FAILURES: u32 = 3;
/// How long a contact stalled by the sequencer's call cap is kept under the autopilot, for a late
/// answer, before it is ended, seconds.
const STALL_KEEP_SECS: u64 = 120;
/// How long a chased call is not chased again, seconds.
const CHASE_COOLDOWN_SECS: u64 = 900;
/// After a chase that never heard its station, how long a spot of a lower rule (a US state, a park,
/// a grid) may not move the radio again, seconds: the schedule gets its turn (live run 2026-11-22,
/// xlog.23: five US-state chases back to back from 22:39, 40 s on a band between two, no contact in
/// 22 minutes). New entities, Most Wanted, DXpeditions and DXCC slots are chased at once regardless.
const CHASE_REST_SECS: u64 = 600;
/// How soon a needed station that left our calls unanswered, and still calls CQ, is called again
/// when there is nobody else to call on the band, seconds.
const CALL_AGAIN_SECS: u64 = 60;
/// How long a fed entry stays usable, seconds.
const FEED_FRESH_SECS: u64 = 600;
/// Decisions kept for the panel's log.
const LOG_KEEP: usize = 40;
/// How many slots the band stays put after a station last addressed the operator, or after the
/// partner of an unconfirmed contact was last heard: two of the partner's periods, so a repeat of
/// a report or an RR73 we did not answer is always heard on the band it was sent to.
const BAND_HOLD_SLOTS: u64 = 4;
/// A partner heard within this many slots is still sending to us: start nothing new.
const PARTNER_REPEAT_SLOTS: u64 = 2;
/// How long a finished contact keeps its station out of Answer callers' queue, in slots
/// (ten minutes of FT8): long enough for every repeat of its call and its 73.
const DONE_FORGET_SLOTS: u64 = 40;

/// A station that called the operator.
#[derive(Debug, Clone, PartialEq)]
pub struct QueuedCaller {
    pub call: String,
    pub message: String,
    pub snr: i32,
    pub freq_hz: f32,
    pub slot: u64,
}

/// Has the station we are working answered us? One whose report we are confirming has, even when
/// its answer is what started the contact (live run 2026-11-23: D2ACE's report came just after it
/// was given up, and it was dropped after four R-reports as not answering).
fn has_answered(station: &tempo_core::qso::Station) -> bool {
    station.advanced_on_air || station.state == QsoState::AwaitRr73
}

/// The cool-down key of one band and mode of a hunted DXpedition.
fn hunt_key(call: &str, band: &str, mode: &str) -> String {
    format!("{call}|{band}|{mode}").to_uppercase()
}

/// How long a spot of this rule waits after a chase that heard nothing, seconds, so the band
/// schedule gets its turn. A rare one (ATNO, Most Wanted, DXpedition) never waits. A DXCC slot
/// waits five minutes, as long as the chase that failed: on 2026-11-23 they ran back to back for
/// two and a half hours, 160 m and 80 m took 23 of 27 band changes and 30 m was never reached.
/// States, references, grids and ordinary calls wait [`CHASE_REST_SECS`].
fn chase_rest_secs(t: Tier) -> u64 {
    match t {
        t if is_rare(t) || t == Tier::CallingMe => 0,
        Tier::DxccUnconfirmed | Tier::DxccNewSlot => CHASE_REST_SECS / 2,
        _ => CHASE_REST_SECS,
    }
}

/// The contact the autopilot started, until it ends.
#[derive(Debug, Clone, PartialEq)]
struct Target {
    call: String,
    tier: Tier,
    started_unix: u64,
    band: String,
    /// Calls seen sent in the current step, to notice the operator starting it again.
    calls: u32,
    /// Its time budget ran out: no more calls, only the last one's answer is waited for.
    out_of_time: bool,
    /// Called again because nobody else was wanted: until it answers, it neither keeps the band
    /// from being quiet nor counts as another failure.
    again: bool,
}

/// A needed station spotted on another band, being chased there.
#[derive(Debug, Clone, PartialEq)]
struct Chase {
    call: String,
    band: String,
    since_unix: u64,
    /// A DXpedition hunt (any band, its own frequency, and back afterwards) rather than a hop
    /// to another scheduled band.
    hunt: Option<Hunt>,
    /// Worked: only the way back is left.
    done: bool,
}

/// Where a DXpedition hunt went, and where it came from.
#[derive(Debug, Clone, PartialEq)]
struct Hunt {
    /// The spot's own frequency, MHz.
    spot_mhz: f64,
    back_band: String,
    back_dial_mhz: f64,
    back_tier: LinkTier,
    /// The dial the hunt began on, MHz: a call may move it.
    dial_mhz: f64,
    /// The operator's special operation before the hunt switched to Hound, to put back.
    hound_from: Option<SpecialOp>,
    /// The DXpedition has been decoded here.
    heard: bool,
    /// When a station was last decoded here calling the DXpedition, Unix seconds.
    called_unix: Option<u64>,
}

/// The autopilot's running state.
#[derive(Debug, Default)]
struct Pilot {
    on: bool,
    presence: PresenceClock,
    attempts: HashMap<String, Attempts>,
    /// `CALL|band|mode` worked this session by the autopilot.
    worked: HashSet<String>,
    hourly: HourlyCount,
    band: Option<String>,
    band_since_unix: u64,
    last_worthwhile_unix: u64,
    /// The last moment something better than an ordinary new call was called here.
    last_needed_unix: u64,
    slots_on_band: u32,
    /// The slot the listen counter last counted (it counts slots, not decode folds — REVIEW N-06).
    last_listen_slot: Option<u64>,
    target: Option<Target>,
    chase: Option<Chase>,
    chased: HashMap<String, u64>,
    /// When the last chase that heard nothing was given up: the next one waits [`chase_rest_secs`].
    chase_failed_unix: u64,
    /// When the contact on the sequencer was first seen stalled by its call cap, Unix seconds.
    stalled_unix: Option<u64>,
    /// Just switched on: the dial goes to the band's channel first.
    to_channel: bool,
    /// The transmit offset before a DXpedition's call moved it, and the offset it was moved to, Hz.
    tx_before: Option<(f32, f32)>,
    /// The scheduled band a chase left, so the rotation goes on from there and not from wherever
    /// the spot was.
    rotation: Option<Stop>,
    /// When each scheduled band in a mode last had its turn (a stay of at least the minimum dwell),
    /// by the order the turns ended in. The one that has waited longest comes next.
    turns: HashMap<String, u64>,
    /// The number of the last turn in [`Self::turns`].
    last_turn: u64,
    status: &'static str,
    reason: String,
    log: VecDeque<String>,
}

/// Answer callers' queue and the autopilot, held by the engine.
#[derive(Debug, Default)]
pub struct Automation {
    callers: Vec<QueuedCaller>,
    /// Callers answered in a row.
    chain: u32,
    /// Read by the TX path when our closing over leaves: hold TX for the next contact instead of
    /// "Disable Tx after 73". Refreshed after every decode.
    pub(super) keeps_tx_after_73: bool,
    /// The TX path held its after-73 disable for us; if no next contact starts, we disarm.
    pub(super) held_disable: bool,
    /// Our own band change is under way: its context halt is not a stop.
    self_tuning: bool,
    /// Transmit stopped for a tuner or amplifier fault, and why ([`crate::station_guard`]).
    station_hold: Option<String>,
    /// By call: every band and mode it is spotted in, and when it was fed, Unix seconds.
    feed: HashMap<String, Vec<(FeedEntry, u64)>>,
    pilot: Pilot,
    /// Every station heard addressing the operator (anything but a closing `73`), with the slot it
    /// was last heard in. Kept whether or not Answer callers is on: while any is fresh, the band
    /// does not change ([`Engine::automation_band_hold`]).
    to_me: HashMap<String, u64>,
    /// Stations heard calling CQ or somebody else since they last addressed the operator, with the
    /// slot. A station we are in contact with that is here has gone on to others
    /// ([`Automation::moved_on`]).
    elsewhere: HashMap<String, u64>,
    /// The contact that is made on our side but not yet closed by the partner (they owe a 73, or
    /// stopped mid-exchange), and the slot it was first seen in that state.
    settle: Option<(String, u64)>,
    /// The slot of the last decode fold.
    last_slot: u64,
    /// The slot our last over of a contact went out in.
    pub(super) last_over_slot: u64,
    /// The last "holding the band" reason written to the log, so a hold is noted once.
    last_hold: Option<String>,
    /// Stations whose contact is made, with the slot it was last seen made: their calls to us are
    /// not answered again (field report 2026-11-22: SP4NKJ called while we finished EW4M, we then
    /// worked SP4NKJ, and the stale queue entry answered it with a fresh report after its 73).
    done: HashMap<String, u64>,
    /// What the autopilot knows about a DXCC entity by its cty.dat name (continent, number), wired
    /// by the launch ([`Engine::set_automation_entity_resolver`]); absent in headless tests.
    entity_info: EntityInfoResolver,
    /// Everything decoded so far in the slot the autopilot last saw, across its folds.
    slot_heard: Option<(u64, Vec<modes::Decode>)>,
}

/// A DXCC entity's continent code and ARRL number, as the autopilot ranks stations by them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntityInfo {
    pub continent: Option<String>,
    pub dxcc: Option<u32>,
}

/// An entity name to its [`EntityInfo`].
type EntityInfoFn = dyn Fn(&str) -> EntityInfo + Send + Sync;

/// The resolver behind [`Automation::entity_info`].
#[derive(Default, Clone)]
struct EntityInfoResolver(Option<std::sync::Arc<EntityInfoFn>>);

impl std::fmt::Debug for EntityInfoResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_some() {
            "EntityInfoResolver(set)"
        } else {
            "EntityInfoResolver(none)"
        })
    }
}

impl Automation {
    /// Was `call` last heard calling CQ or somebody else, not us? Live run 2026-11-23: SQ2MMX
    /// answered us once, then worked three others, and was sent our R-report eight times.
    fn moved_on(&self, call: &str) -> bool {
        self.elsewhere.keys().any(|c| same_call(c, call))
    }

    /// Has the autopilot's target had all its calls without answering? Read by the TX path, which
    /// then sends it nothing more. Never for a rare target, nor once the station has answered,
    /// unless it has since gone on to others.
    pub(super) fn calls_spent(&self, station: &tempo_core::qso::Station, max: u32) -> bool {
        let answered = has_answered(station);
        self.pilot.on
            && (!answered
                || station
                    .dxcall
                    .as_deref()
                    .is_some_and(|dx| self.moved_on(dx)))
            && matches!(
                station.state,
                QsoState::AwaitReport | QsoState::AwaitRoger | QsoState::AwaitRr73
            )
            && self.pilot.target.as_ref().is_some_and(|t| {
                !is_rare(t.tier)
                    && station
                        .dxcall
                        .as_deref()
                        .is_some_and(|dx| same_call(dx, &t.call))
                    && station.tx_count >= if t.out_of_time && !answered { 1 } else { max }
            })
    }

    /// The special operation a hunt replaced with Hound, when one is under way: what to put back
    /// if the autopilot stops before the hunt ends.
    fn hunt_hound_from(&self) -> Option<SpecialOp> {
        self.pilot
            .chase
            .as_ref()
            .and_then(|c| c.hunt.as_ref())
            .and_then(|h| h.hound_from)
    }

    /// A stop (Stop TX, a remote stop, a broker halt, a context halt that is not ours). Returns
    /// the special operation to put back when a hunt had switched to Hound.
    pub(super) fn on_halt(&mut self) -> Option<SpecialOp> {
        if self.self_tuning {
            return None;
        }
        let restore = self.hunt_hound_from();
        self.station_hold = None;
        self.callers.clear();
        self.chain = 0;
        self.keeps_tx_after_73 = false;
        self.held_disable = false;
        if self.pilot.on {
            self.pilot.on = false;
            self.pilot.target = None;
            self.pilot.chase = None;
            self.pilot.status = "off";
            self.pilot.reason = "stopped".into();
            note(&mut self.pilot.log, "stopped by Stop TX");
        }
        restore
    }
}

fn note(log: &mut VecDeque<String>, text: &str) {
    let t = now_unix_secs() % 86_400;
    log.push_back(format!(
        "{:02}:{:02}:{:02} {text}",
        t / 3600,
        (t / 60) % 60,
        t % 60
    ));
    while log.len() > LOG_KEEP {
        log.pop_front();
    }
}

fn utc_minute(now: u64) -> u32 {
    ((now % 86_400) / 60) as u32
}

fn link_tier_of(mode: &str) -> Option<LinkTier> {
    match mode.trim().to_uppercase().as_str() {
        "FT8" => Some(LinkTier::Ft8),
        "FT4" => Some(LinkTier::Ft4),
        "FT2" => Some(LinkTier::Ft2),
        _ => None,
    }
}

fn mode_name(t: LinkTier) -> Option<&'static str> {
    match t {
        LinkTier::Ft8 => Some("FT8"),
        LinkTier::Ft4 => Some("FT4"),
        LinkTier::Ft2 => Some("FT2"),
        _ => None,
    }
}

/// Test hook: the mode running.
#[cfg(test)]
pub(super) fn mode_name_for_test(e: &Engine) -> &'static str {
    mode_name(e.app.tier()).unwrap_or("")
}

/// The key of a band's turns in a mode.
fn turn_key(band: &str, mode: &str) -> String {
    format!("{}|{}", band.to_lowercase(), mode.to_lowercase())
}

/// A stop in the log's words: the band, and the mode where the patrol has more than one.
fn stop_name(stop: &Stop, patrol: &automation::Patrol) -> String {
    if patrol.many_modes() {
        format!("{} {}", stop.band, stop.mode)
    } else {
        stop.band.clone()
    }
}

impl Engine {
    // ---------------------------------------------------------------- operator verbs

    /// Turn the autopilot on or off. On needs Settings ▸ Automation to allow it, transmit already
    /// armed by the operator, and an FT8, FT4 or FT2 tier. Turning it on counts as the first
    /// presence confirmation.
    pub fn set_autopilot(&mut self, on: bool) -> Result<(), String> {
        let now = now_unix_secs();
        if !on {
            self.automation_restore_hound();
            if self.automation.pilot.on {
                self.automation.pilot.on = false;
                self.automation.pilot.target = None;
                self.automation.pilot.chase = None;
                self.automation.pilot.status = "off";
                self.automation.pilot.reason = "switched off".into();
                self.automation_tx_back();
                note(
                    &mut self.automation.pilot.log,
                    "switched off by the operator",
                );
            }
            self.automation_refresh_keep();
            return Ok(());
        }
        if !self.settings.automation.autopilot_enabled {
            return Err("The autopilot is switched off in Settings ▸ Automation.".into());
        }
        if !self.tx_enabled {
            return Err("Turn transmit on first, then the autopilot.".into());
        }
        if mode_name(self.app.tier()).is_none() {
            return Err("The autopilot runs on FT8, FT4 or FT2.".into());
        }
        let p = &mut self.automation.pilot;
        p.on = true;
        p.to_channel = true;
        p.presence.ack(now);
        p.band = (!self.settings.band.is_empty()).then(|| self.settings.band.clone());
        p.band_since_unix = now;
        p.last_worthwhile_unix = now;
        p.last_needed_unix = now;
        p.slots_on_band = 0;
        p.last_listen_slot = None;
        p.target = None;
        p.chase = None;
        p.status = "listening";
        p.reason = "started".into();
        note(&mut p.log, "switched on by the operator");
        self.automation_refresh_keep();
        Ok(())
    }

    /// A hunt that switched to Hound puts the operator's own setting back (the hunt is ending
    /// with the autopilot).
    fn automation_restore_hound(&mut self) {
        if let Some(prev) = self.automation.hunt_hound_from() {
            self.settings.special_op = prev;
            if let Some(h) = self
                .automation
                .pilot
                .chase
                .as_mut()
                .and_then(|c| c.hunt.as_mut())
            {
                h.hound_from = None;
            }
        }
    }

    /// The operator confirms they are at the station.
    pub fn autopilot_confirm_presence(&mut self) {
        let now = now_unix_secs();
        self.automation.pilot.presence.ack(now);
        if self.automation.pilot.on {
            note(&mut self.automation.pilot.log, "presence confirmed");
            // A confirmation is an operator action at the station: the watchdog starts over.
            self.tx_watchdog_start = None;
        }
        self.automation_refresh_keep();
    }

    /// Wire what the autopilot reads about a DXCC entity by its name: the continent (stations
    /// outside Europe first) and the ARRL number (the Club Log Most Wanted list). The launch hands
    /// it cty.dat's own; nothing else needs it.
    pub fn set_automation_entity_resolver(
        &mut self,
        resolve: impl Fn(&str) -> EntityInfo + Send + Sync + 'static,
    ) {
        self.automation.entity_info = EntityInfoResolver(Some(std::sync::Arc::new(resolve)));
    }

    /// The UI's need feed (cluster, RBN, OTA, DXpeditions), replacing the last one. A spot whose
    /// call is no callsign (a false decode someone's software reported) is left out.
    pub fn autopilot_feed(&mut self, entries: Vec<FeedEntry>) {
        let now = now_unix_secs();
        let mut feed: HashMap<String, Vec<(FeedEntry, u64)>> = HashMap::new();
        for e in entries.into_iter().filter(|e| spot_call_ok(&e.call)) {
            feed.entry(e.call.trim().to_uppercase())
                .or_default()
                .push((e, now));
        }
        self.automation.feed = feed;
    }

    /// Every fresh spot in the feed.
    fn automation_spots(&self, now: u64) -> impl Iterator<Item = &FeedEntry> {
        self.automation
            .feed
            .values()
            .flatten()
            .filter(move |(_, at)| now.saturating_sub(*at) <= FEED_FRESH_SECS)
            .map(|(e, _)| e)
    }

    // ---------------------------------------------------------------- the station guard

    /// Is the station to be watched for faults: the autopilot runs, or waits for one?
    pub fn station_guard_wanted(&self) -> bool {
        self.settings.automation.station_guard && self.automation.pilot.on
    }

    /// The tuner or the amplifier has faulted: transmit off now, an over in flight cut, and the
    /// autopilot waits. `false` = it does not run, so the station is the operator's.
    pub fn station_fault_hold(&mut self, why: &str) -> bool {
        if !self.automation.pilot.on {
            return false;
        }
        self.automation.station_hold = Some(why.to_string());
        self.automation_abandon_contact();
        self.automation.pilot.target = None;
        self.slot_tx_abort = true;
        self.set_tx_enabled(false);
        self.station_fault_note(&format!("{why}: transmit off, autopilot paused"));
        true
    }

    /// Still held, with nothing done by the operator since?
    pub fn station_fault_held(&self) -> bool {
        self.automation.station_hold.is_some() && self.automation.pilot.on && !self.tx_enabled
    }

    /// What the guard does now, for the panel and the log.
    pub fn station_fault_note(&mut self, text: &str) {
        if self.automation.station_hold.is_some() {
            self.automation.pilot.status = "stationFault";
            self.automation.pilot.reason = text.to_string();
        }
        self.automation_note(text);
    }

    /// The station is ready again: transmit on as it was, and the autopilot goes on. Only when the
    /// operator has done nothing since the hold.
    pub fn station_fault_resume(&mut self) -> bool {
        let held = self.station_fault_held();
        self.automation.station_hold = None;
        if held {
            self.set_tx_enabled(true);
            self.automation_watchdog_restart();
            self.automation_note("the station is ready: transmit on, autopilot goes on");
        }
        held
    }

    /// The operator's own TX On or Off ends the hold: the station is theirs.
    pub(super) fn station_fault_operator(&mut self) {
        self.automation.station_hold = None;
    }

    /// The station the autopilot has moved here for and is listening for, until it is worked.
    pub(super) fn automation_listening_for(&self) -> Option<String> {
        let p = &self.automation.pilot;
        p.chase
            .as_ref()
            .filter(|ch| p.on && !ch.done)
            .map(|ch| ch.call.clone())
    }

    /// A DXpedition or a new entity, by the feed.
    fn automation_fed_rare(&self, call: &str) -> bool {
        self.automation
            .feed
            .get(&call.to_uppercase())
            .is_some_and(|l| l.iter().any(|(e, _)| e.dxpedition || e.new_entity))
    }

    /// Is this station not worked on that band in that mode's class?
    fn automation_call_new(&self, call: &str, band: &str, mode: &str) -> bool {
        self.station
            .hot()
            .call_status(call, band, mode)
            .starts_with("New")
    }

    /// Test hook: move the last presence confirmation `secs` into the past.
    #[cfg(test)]
    pub(super) fn automation_backdate_presence(&mut self, secs: u64) {
        let p = &mut self.automation.pilot.presence;
        p.last_ack_unix = p.last_ack_unix.saturating_sub(secs);
    }

    /// Test hook: move the band clocks (time on band, last worthwhile moment, a chase's start, the
    /// chase rest) `secs` into the past.
    #[cfg(test)]
    pub(super) fn automation_backdate_band(&mut self, secs: u64) {
        let p = &mut self.automation.pilot;
        p.band_since_unix = p.band_since_unix.saturating_sub(secs);
        p.last_worthwhile_unix = p.last_worthwhile_unix.saturating_sub(secs);
        p.last_needed_unix = p.last_needed_unix.saturating_sub(secs);
        p.chase_failed_unix = p.chase_failed_unix.saturating_sub(secs);
        if let Some(ch) = p.chase.as_mut() {
            ch.since_unix = ch.since_unix.saturating_sub(secs);
        }
    }

    /// Test hook: the autopilot's own band change.
    #[cfg(test)]
    pub(super) fn automation_go_for_test(&mut self, band: &str) {
        assert!(self.automation_switch(None, band, None));
    }

    /// Test hook: the contact stalled `secs` earlier than it did.
    #[cfg(test)]
    pub(super) fn automation_backdate_stall(&mut self, secs: u64) {
        let at = &mut self.automation.pilot.stalled_unix;
        *at = at.map(|t| t.saturating_sub(secs));
    }

    /// Test hook: the running call is dropped.
    #[cfg(test)]
    pub(super) fn automation_abandon_for_test(&mut self) {
        self.automation_abandon_contact();
        self.automation.pilot.target = None;
    }

    /// Test hook: move every chase `secs` into the past.
    #[cfg(test)]
    pub(super) fn automation_backdate_chased(&mut self, secs: u64) {
        for at in self.automation.pilot.chased.values_mut() {
            *at = at.saturating_sub(secs);
        }
    }

    /// Test hook: move every failed attempt `secs` into the past.
    #[cfg(test)]
    pub(super) fn automation_backdate_attempts(&mut self, secs: u64) {
        for a in self.automation.pilot.attempts.values_mut() {
            a.last_unix = a.last_unix.saturating_sub(secs);
        }
    }

    /// Test hook: `call` left our calls unanswered `secs` ago.
    #[cfg(test)]
    pub(super) fn automation_fail_for_test(&mut self, call: &str, secs: u64) {
        self.note_failed(call);
        if let Some(a) = self.automation.pilot.attempts.get_mut(call) {
            a.last_unix = a.last_unix.saturating_sub(secs);
        }
    }

    /// Test hook: the station the sequencer is working.
    #[cfg(test)]
    pub(super) fn automation_dx(&self) -> Option<String> {
        match &self.mode {
            Mode::Qso { station, .. } => station.dxcall.clone(),
            _ => None,
        }
    }

    /// What the UI shows.
    pub fn automation_dto(&self) -> AutomationDto {
        let a = &self.automation;
        let p = &a.pilot;
        let s = &self.settings.automation;
        let now = now_unix_secs();
        let mut hourly = p.hourly.clone();
        let calls = match (&self.mode, &p.target) {
            (Mode::Qso { station, .. }, Some(t))
                if p.on && !is_rare(t.tier) && !has_answered(station) =>
            {
                Some((station.tx_count, s.target_max_calls.max(1)))
            }
            _ => None,
        };
        AutomationDto {
            answer_callers: s.answer_callers,
            callers: a.callers.iter().map(|c| c.call.clone()).collect(),
            chain: a.chain,
            autopilot_enabled: s.autopilot_enabled,
            autopilot_on: p.on,
            status: if p.on { p.status } else { "off" }.into(),
            reason: p.reason.clone(),
            band: p.band.clone(),
            mode: mode_name(self.app.tier()).map(String::from),
            target: p.target.as_ref().map(|t| t.call.clone()),
            target_tier: p.target.as_ref().map(|t| t.tier),
            calls_made: calls.map(|(n, _)| n),
            calls_max: calls.map(|(_, max)| max),
            presence_remaining_secs: if p.on {
                p.presence.remaining_secs(now, s.presence_minutes())
            } else {
                0
            },
            presence_minutes: s.presence_minutes(),
            next_window: next_window_start(&s.schedule, utc_minute(now)),
            qsos_last_hour: hourly.count(now) as u32,
            log: p.log.iter().cloned().collect(),
        }
    }

    // ---------------------------------------------------------------- the decode hook

    /// After every decode fold: Answer callers, then the autopilot.
    pub(super) fn automation_after_decodes(&mut self, decodes: &[modes::Decode], slot: u64) {
        // Transmit armed during a hold was armed by the operator (a station clicked): theirs again.
        if self.tx_enabled {
            self.automation.station_hold = None;
        }
        // A watchdog trip, or the operator's TX Off, ends the autopilot: it never re-arms. Held for a
        // station fault it waits (any stop drops the hold).
        if self.automation.pilot.on && !self.tx_enabled && self.automation.station_hold.is_none() {
            self.automation_restore_hound();
            let p = &mut self.automation.pilot;
            p.on = false;
            p.target = None;
            p.chase = None;
            p.status = "off";
            p.reason = "transmit went off".into();
            note(&mut p.log, "transmit went off: autopilot off");
        }
        // Switched off in Settings ▸ Automation while running: stop as the off switch does
        // (REVIEW N-05: it kept running with its control hidden).
        if self.automation.pilot.on && !self.settings.automation.autopilot_enabled {
            let _ = self.set_autopilot(false);
        }
        self.automation_track_to_me(decodes, slot);
        self.answer_callers_step(decodes, slot);
        // The autopilot ranks everything heard this slot so far, not only this fold: a slot comes in
        // two folds (the early decode and the boundary one, which carries only what the early one
        // did not), and a station heard in the first must still count in the second (live run
        // 2026-11-22: VK9XY's CQ came in a fold spent listening; the next fold of the same slot
        // held only other stations, and a US-state spot was chased instead).
        let heard = match &mut self.automation.slot_heard {
            Some((s, all)) if *s == slot => {
                all.extend(decodes.iter().cloned());
                all.clone()
            }
            _ => {
                self.automation.slot_heard = Some((slot, decodes.to_vec()));
                decodes.to_vec()
            }
        };
        if self.automation.pilot.on {
            self.autopilot_step(&heard, slot);
        }
        // The TX path held its after-73 disable for a next contact that did not come.
        if self.automation.held_disable && !self.automation_next_contact_started() {
            self.automation.held_disable = false;
            if !self.automation.pilot.on {
                self.set_tx_enabled(false);
            }
        }
        self.automation_refresh_keep();
    }

    /// Note who is addressing the operator, and whether the contact in hand still waits on its
    /// partner. Feeds the band hold.
    fn automation_track_to_me(&mut self, decodes: &[modes::Decode], slot: u64) {
        let mycall = self.settings.mycall.clone();
        self.automation.last_slot = slot;
        for d in decodes {
            let m = Msg::parse(&d.message);
            let Some(de) = m.sender().filter(|de| !de.contains("<...>")) else {
                continue;
            };
            if !m.addressee().is_some_and(|to| same_call(to, &mycall)) {
                // To somebody else, or a CQ. Not one of two streams sent in the slot that also
                // answered us.
                let de = de.trim_matches(['<', '>']).to_uppercase();
                if self.automation.to_me.get(&de) != Some(&slot) {
                    self.automation.elsewhere.insert(de, slot);
                }
                continue;
            }
            let of = de.trim_matches(['<', '>']);
            self.automation.elsewhere.retain(|c, _| !same_call(c, of));
            // A plain 73 closes; everything else (a call, a report, a repeated RR73) still wants us.
            if matches!(m, Msg::Bye73 { .. }) {
                self.automation.to_me.remove(&de.to_uppercase());
                continue;
            }
            self.automation.to_me.insert(de.to_uppercase(), slot);
        }
        self.automation
            .to_me
            .retain(|_, s| slot.saturating_sub(*s) < BAND_HOLD_SLOTS);
        self.automation
            .elsewhere
            .retain(|_, s| slot.saturating_sub(*s) < DONE_FORGET_SLOTS);
        // A contact made with the station we work now closes any older call it queued (it called
        // us while we finished someone else, and we then worked it): it is never answered again.
        if let Mode::Qso { station, .. } = &self.mode {
            if let Some(dx) = station.dxcall.clone() {
                if super::ft_contact_complete(station, self.qso_report_sent) {
                    self.automation.done.insert(dx.to_uppercase(), slot);
                }
            }
        }
        self.automation
            .done
            .retain(|_, s| slot.saturating_sub(*s) < DONE_FORGET_SLOTS);
        let unsettled = match &self.mode {
            Mode::Qso { station, .. } => station.dxcall.clone().filter(|_| {
                matches!(
                    station.state,
                    QsoState::AwaitRoger | QsoState::AwaitRr73 | QsoState::Confirming
                ) || (station.state == QsoState::AwaitReport && station.advanced_on_air)
            }),
            _ => None,
        };
        match unsettled {
            Some(call) => {
                let fresh = self
                    .automation
                    .settle
                    .as_ref()
                    .is_none_or(|(c, _)| !same_call(c, &call));
                if fresh {
                    self.automation.settle = Some((call.to_uppercase(), slot));
                }
            }
            None => self.automation.settle = None,
        }
    }

    /// Why the band must not change now, or `None` when it may. The operator's rule: the
    /// autopilot changes band only when every contact is finished and nobody is calling —
    /// never in the middle of a QSO.
    ///
    /// - a contact is being called or worked ([`Self::automation_contact_in_progress`]);
    /// - a CQ run is going;
    /// - the partner of a contact made on our side (our RR73 is out, or they stopped mid-way) has
    ///   been heard within [`BAND_HOLD_SLOTS`], or the contact entered that state more recently;
    /// - any station addressed us within [`BAND_HOLD_SLOTS`] (a caller, or a partner repeating);
    /// - Answer callers has a caller queued.
    fn automation_band_hold(&self) -> Option<String> {
        if self.automation_contact_in_progress() {
            let dx = match &self.mode {
                Mode::Qso { station, .. } => station.dxcall.clone(),
                _ => None,
            };
            return Some(format!(
                "contact with {} in progress",
                dx.unwrap_or_else(|| "a station".into())
            ));
        }
        self.automation_hold_without_contact()
    }

    /// What holds the band beside the contact on the sequencer ([`Self::automation_band_hold`]).
    fn automation_hold_without_contact(&self) -> Option<String> {
        let a = &self.automation;
        let slot = a.last_slot;
        if self.cq_running {
            return Some("CQ run in progress".into());
        }
        if let Some((call, since)) = &a.settle {
            let last = a.to_me.get(call).copied().unwrap_or(0).max(*since);
            if slot.saturating_sub(last) < BAND_HOLD_SLOTS {
                return Some(format!("waiting for {call} to finish"));
            }
        }
        let blocked = &self.settings.blocked_calls;
        if let Some(call) = a
            .to_me
            .iter()
            .filter(|(c, s)| {
                slot.saturating_sub(**s) < BAND_HOLD_SLOTS
                    && !blocked.iter().any(|b| same_call(b, c))
            })
            .map(|(c, _)| c.clone())
            .min()
        {
            return Some(format!("{call} is calling"));
        }
        if !a.callers.is_empty() {
            return Some(format!("{} waiting to be answered", a.callers[0].call));
        }
        None
    }

    /// The partner of the contact just made is still sending to us (a repeated report or RR73):
    /// nothing new starts until they stop or finish.
    fn automation_partner_repeating(&self) -> Option<String> {
        let a = &self.automation;
        let (call, _) = a.settle.as_ref()?;
        let heard = *a.to_me.get(call)?;
        (a.last_slot.saturating_sub(heard) < PARTNER_REPEAT_SLOTS).then(|| call.clone())
    }

    /// Is a contact under way that started after the held disable (a new station is being called)?
    fn automation_next_contact_started(&self) -> bool {
        self.automation_contact_in_progress()
    }

    /// Should the next "Disable Tx after 73" be held? Only while automation will start the next
    /// contact itself.
    fn automation_refresh_keep(&mut self) {
        let s = &self.settings.automation;
        let callers_waiting = s.answer_callers
            && !self.automation.callers.is_empty()
            && self.automation.chain < s.answer_chain_limit.max(1);
        let pilot = self.automation.pilot.on
            && !self
                .automation
                .pilot
                .presence
                .expired(now_unix_secs(), s.presence_minutes());
        self.automation.keeps_tx_after_73 = self.tx_enabled && (callers_waiting || pilot);
    }

    // ---------------------------------------------------------------- Answer callers

    fn answer_callers_step(&mut self, decodes: &[modes::Decode], slot: u64) {
        if !self.settings.automation.answer_callers || self.cq_running {
            self.automation.callers.clear();
            self.automation.chain = 0;
            return;
        }
        let mycall = self.settings.mycall.clone();
        let (dx, state, advanced, tx_count, complete, closing_owed) = match &self.mode {
            Mode::Qso { station, .. } => (
                station.dxcall.clone(),
                station.state,
                station.advanced_on_air,
                station.tx_count,
                super::ft_contact_complete(station, self.qso_report_sent),
                station.pending.is_some(),
            ),
            _ => {
                self.automation.callers.clear();
                return;
            }
        };
        // Only while working a station we called (S&P): never from plain listening, which
        // answers nobody (qso.rs `observe`).
        let Some(dx) = dx else {
            self.automation.callers.clear();
            self.automation.chain = 0;
            return;
        };
        // Queue every station calling us that is not the station we are working.
        for d in decodes {
            let m = Msg::parse(&d.message);
            let (Some(to), Some(de)) = (m.addressee(), m.sender()) else {
                continue;
            };
            if !same_call(to, &mycall) || same_call(de, &dx) || de.contains("<...>") {
                continue;
            }
            if self.settings.blocked_calls.iter().any(|b| same_call(b, de)) {
                continue;
            }
            // A caller that already signed off is not calling — and an older call it queued is
            // withdrawn with it.
            if matches!(m, Msg::Bye73 { .. } | Msg::Rr73 { .. } | Msg::Rrr { .. }) {
                self.automation.callers.retain(|c| !same_call(&c.call, de));
                continue;
            }
            // Just worked: its call to us is the contact we already made, not a new one.
            if self.automation.done.contains_key(&de.to_uppercase()) {
                continue;
            }
            let entry = QueuedCaller {
                call: de.to_uppercase(),
                message: d.message.clone(),
                snr: d.snr,
                freq_hz: d.freq,
                slot,
            };
            match self
                .automation
                .callers
                .iter_mut()
                .find(|c| same_call(&c.call, de))
            {
                Some(c) => *c = entry,
                None => self.automation.callers.push(entry),
            }
        }
        // Never the station being worked now (it may have been queued before we started working
        // it), nor one whose contact is already made.
        let done = &self.automation.done;
        self.automation.callers.retain(|c| {
            slot.saturating_sub(c.slot) <= CALLER_FRESH_SLOTS
                && !same_call(&c.call, &dx)
                && !done.contains_key(&c.call.to_uppercase())
        });
        if self.automation.callers.is_empty() || !self.tx_enabled {
            return;
        }
        let s = self.settings.automation.clone().sanitized();
        // With the autopilot on, its presence check and hourly cap bound these contacts too:
        // unanswered, it starts no new contact (module docs), and the cap is a cap.
        if self.automation.pilot.on {
            let now = now_unix_secs();
            if self
                .automation
                .pilot
                .presence
                .expired(now, s.presence_minutes())
                || self.automation.pilot.hourly.count(now) >= s.max_qso_per_hour as usize
            {
                return;
            }
        }
        // Take-over: our first call has gone unanswered for N slots and somebody is calling us.
        // It counts toward the chain limit like any other answered caller.
        // A station that answered and has since gone on to others is as good as unanswered.
        let moved_on = self.automation.pilot.on && self.automation.moved_on(&dx);
        let unanswered = (matches!(state, QsoState::AwaitReport | QsoState::AwaitRoger)
            && !advanced)
            || (matches!(
                state,
                QsoState::AwaitReport | QsoState::AwaitRoger | QsoState::AwaitRr73
            ) && moved_on);
        // Never from a DXpedition, a new entity or a rare autopilot target: those are long calls
        // by nature (live run 2026-11-22: the operator's call to VK9XY was dropped for N5EI).
        let fed_rare = self.automation_fed_rare(&dx);
        let target_rare = self
            .automation
            .pilot
            .target
            .as_ref()
            .is_some_and(|t| same_call(&t.call, &dx) && is_rare(t.tier));
        // With the autopilot on, nobody waits for a station that has not answered (operator,
        // 2026-11-23: "if no ongoing QSO is happening and someone is calling us, we answer"; N9ATF
        // called while DO8JB left nine calls unanswered), nor for one picked and not called yet
        // (live run 2026-11-23: VU2NRO called between the pick of R3WF and the first call to it). A
        // station that has addressed us is a contact under way, so it keeps its turns, until it
        // goes on to others. A caller counts from the slot before our last over, whose decodes can
        // come in after that over has begun, and is never answered in the middle of an over. The
        // chain limit is about turning TX off, which the autopilot does not do, so it does not
        // apply either.
        let pilot_on = self.automation.pilot.on;
        let addressed_us = self.automation.to_me.keys().any(|c| same_call(c, &dx));
        let at_once =
            pilot_on && (!addressed_us || moved_on) && slot > self.automation.last_over_slot;
        let chain_ok = pilot_on || self.automation.chain < s.answer_chain_limit.max(1);
        let waited = s.answer_takeover_slots > 0 && tx_count >= s.answer_takeover_slots;
        if !fed_rare && !target_rare && unanswered && (at_once || waited) && chain_ok {
            let from = if waited {
                0
            } else {
                self.automation.last_over_slot.saturating_sub(1)
            };
            if let Some(c) = self.best_caller(from) {
                self.automation.chain += 1;
                self.note_failed(&dx);
                self.automation_note(&format!(
                    "{dx} did not answer: answering {} instead",
                    c.call
                ));
                self.start_auto_contact(&c, Tier::CallingMe);
                return;
            }
        }
        // After the contact — our closing 73 or RR73 already on the air — answer the next caller
        // instead of turning TX off.
        let finished = complete
            && ((state == QsoState::Done && !closing_owed)
                || (state == QsoState::Confirming && tx_count >= 1));
        if finished && chain_ok {
            if let Some(c) = self.best_caller(0) {
                self.automation.chain += 1;
                // The chain limit does not apply with the autopilot on, so it is not shown.
                let count = if pilot_on {
                    String::new()
                } else {
                    format!(
                        " ({} of {})",
                        self.automation.chain,
                        s.answer_chain_limit.max(1)
                    )
                };
                self.automation_note(&format!(
                    "contact with {dx} done: answering caller {}{count}",
                    c.call
                ));
                self.start_auto_contact(&c, Tier::CallingMe);
            }
        }
    }

    /// The caller to answer: the operator's best-caller strategy over the fresh queue, among those
    /// heard in slot `from` or later. One that has since called somebody else comes last.
    fn best_caller(&self, from: u64) -> Option<QueuedCaller> {
        let floor = self.settings.best_caller_min_snr;
        let mut list: Vec<&QueuedCaller> = self
            .automation
            .callers
            .iter()
            .filter(|c| c.slot >= from && floor.is_none_or(|f| c.snr >= f))
            .collect();
        match self.settings.best_caller.as_str() {
            "strongest" => list.sort_by(|a, b| b.snr.cmp(&a.snr)),
            // "first" and every other strategy: the longest-waiting caller first.
            _ => list.sort_by_key(|c| c.slot),
        }
        list.sort_by_key(|c| self.automation.moved_on(&c.call));
        list.first().map(|c| (*c).clone())
    }

    /// Start a contact exactly as a double-click on `c`'s line would, without arming.
    fn start_auto_contact(&mut self, c: &QueuedCaller, tier: Tier) {
        if !self.tx_enabled {
            return;
        }
        let call = c.call.clone();
        let rare = matches!(tier, Tier::Dxpedition | Tier::Atno) && c.freq_hz > 0.0;
        let mut c = c.clone();
        let clear = self.automation.pilot.on && self.settings.automation.clear_tx_offset;
        // The transmit offset goes back where it was before the autopilot moved it, unless the
        // operator has moved it since. A station calling us is answered where we are: it may be
        // answering a call made there.
        let stays = rare || (clear && tier == Tier::CallingMe);
        if !stays {
            self.automation_tx_back();
        }
        let mut used = if clear && c.freq_hz > 0.0 && tier != Tier::CallingMe {
            self.automation_used_offsets(c.slot)
        } else {
            Vec::new()
        };
        // A DXpedition sits under 500 Hz and is called from 1000-2500 (operator, 2026-11-22 and
        // 2026-11-25). One heard higher is brought there first: the dial moves up so its signal lands
        // at DXPED_PLACE_HZ, through the autopilot's own switch, so it happens only between contacts
        // and transmit is put back as it was. Held, the call goes ahead from where it is. Its own dial
        // is the one the hunt began on.
        let mut shift_hz = self
            .automation
            .pilot
            .chase
            .as_ref()
            .and_then(|ch| ch.hunt.as_ref())
            .map_or(0.0, |h| {
                ((self.settings.dial_mhz - h.dial_mhz) * 1e6).max(0.0) as f32
            });
        if rare && shift_hz < 1.0 {
            let up = automation::dxped_shift_hz(c.freq_hz);
            let dial = self.settings.dial_mhz + f64::from(up) / 1e6;
            let band = self.settings.band.clone();
            if up > 0.0 && self.automation_switch(None, &band, Some(dial)) {
                self.automation_note(&format!(
                    "{call} heard at {:.0} Hz: dial up {up:.0} Hz to {dial:.6} MHz, \
                     so it sits at {:.0} Hz",
                    c.freq_hz,
                    c.freq_hz - up
                ));
                c.freq_hz -= up;
                shift_hz = up;
                for hz in &mut used {
                    *hz -= up;
                }
            }
        }
        let held = self.tx_offset_hz;
        let res = self.call_station_ctx_arming(
            &call,
            None,
            Some(&c.message),
            Some(c.snr),
            Some(c.freq_hz),
            false,
        );
        match res {
            Ok(()) => {
                // A DXpedition (or an ATNO, as rare and as crowded) is never called on its own
                // frequency, and always inside its receiver.
                if rare {
                    let want = automation::dxped_call_hz(c.freq_hz, shift_hz, self.tx_offset_hz);
                    let (low, top) = automation::dxped_range(c.freq_hz, shift_hz);
                    let tx = automation::clear_tx_hz(want, &used, self.tx_width_hz(), low, top);
                    let before = self.automation.pilot.tx_before.map_or(held, |(b, _)| b);
                    self.set_tx_offset(tx);
                    self.automation.pilot.tx_before = Some((before, self.tx_offset_hz));
                    self.automation_note(&format!(
                        "{call} heard at {:.0} Hz: calling at {:.0} Hz",
                        c.freq_hz, self.tx_offset_hz
                    ));
                } else if !used.is_empty() {
                    // Any other is called on a frequency nobody was heard using in our period
                    // (operator, 2026-11-24), the nearest to the operator's own.
                    let want = self.tx_offset_hz;
                    let tx = automation::clear_tx_hz(
                        want,
                        &used,
                        self.tx_width_hz(),
                        automation::CLEAR_LOW_HZ,
                        automation::CLEAR_TOP_HZ,
                    );
                    if (tx - want).abs() >= 0.5 {
                        self.set_tx_offset(tx);
                        // Without Hold Tx there is nothing to go back to.
                        if self.hold_tx_freq {
                            self.automation.pilot.tx_before = Some((want, self.tx_offset_hz));
                        }
                        self.automation_note(&format!(
                            "{call}: {want:.0} Hz is in use, calling at {:.0} Hz",
                            self.tx_offset_hz
                        ));
                    }
                }
                self.automation.held_disable = false;
                self.automation
                    .callers
                    .retain(|q| !same_call(&q.call, &call));
                if self.automation.pilot.on {
                    self.automation_watchdog_restart();
                    let now = now_unix_secs();
                    let band = self.settings.band.clone();
                    let p = &mut self.automation.pilot;
                    p.target = Some(Target {
                        call: call.clone(),
                        tier,
                        started_unix: now,
                        band,
                        calls: 0,
                        out_of_time: false,
                        again: false,
                    });
                    p.hourly.note(now);
                    p.status = "calling";
                }
            }
            Err(e) => self.automation_note(&format!("could not call {call}: {e}")),
        }
    }

    /// Put the transmit offset back where it was before the autopilot moved it, unless the operator
    /// has moved it since.
    fn automation_tx_back(&mut self) {
        if let Some((before, moved_to)) = self.automation.pilot.tx_before.take() {
            if (self.tx_offset_hz - moved_to).abs() < 0.5 {
                self.set_tx_offset(before);
            }
        }
    }

    /// The audio offsets signals were heard at lately in the period we would answer a station
    /// decoded at `slot` in: that slot's own period, as a decode carries its audio's slot + 1. From
    /// Band Activity, which reaches back over a round of calls on a full band.
    fn automation_used_offsets(&self, slot: u64) -> Vec<f32> {
        let secs = self.active_slot_secs();
        self.band_activity()
            .filter_map(|(start, d)| {
                // A period's start is kept in whole seconds (FT4's periods are 7.5 s long).
                let heard = ((start as f64 + 0.5) / secs) as u64;
                (heard % 2 == slot % 2 && heard + automation::CLEAR_LOOKBACK_SLOTS >= slot)
                    .then_some(d.freq)
            })
            .collect()
    }

    /// How far apart two signals' lowest tones must be not to overlap, Hz, with a little to spare.
    fn tx_width_hz(&self) -> f32 {
        match self.app.tier() {
            LinkTier::Ft4 => 90.0,
            _ => 55.0,
        }
    }

    /// Restart the TX watchdog's clock for a contact the running autopilot is responsible for.
    ///
    /// Operator ruling 2026-11-22 (OM0RX, after live runs where the 6-minute watchdog switched
    /// the autopilot off while it was calling a DXpedition): "make sure the TX watchdog can't
    /// just stop it for no reason". While the supervised autopilot runs, each contact it keeps is
    /// already bounded — by its own time budget (ordinary or rare), the sequencer's call cap, the
    /// hourly cap and the presence check — so those, not the watchdog, end it. The watchdog is
    /// NOT weakened: it still trips on everything else, and on the autopilot too if its step stops
    /// running (no decode folds, a wedged loop), because only that step restarts the clock. The
    /// same reasoning as the CQ pause's restart in `plan_tx`.
    fn automation_watchdog_restart(&mut self) {
        let s = &self.settings.automation;
        let pilot_live = self.automation.pilot.on
            && self.tx_enabled
            && !self
                .automation
                .pilot
                .presence
                .expired(now_unix_secs(), s.presence_minutes());
        if pilot_live {
            self.tx_watchdog_start = None;
        }
    }

    fn automation_note(&mut self, text: &str) {
        tempo_core::applog::info("automation", text);
        self.automation.pilot.reason = text.to_string();
        note(&mut self.automation.pilot.log, text);
    }

    fn note_failed(&mut self, call: &str) {
        let now = now_unix_secs();
        let a = self
            .automation
            .pilot
            .attempts
            .entry(call.to_uppercase())
            .or_default();
        a.failures += 1;
        a.last_unix = now;
    }

    /// A station called again ([`CALL_AGAIN_SECS`]) did not answer either: its wait starts again,
    /// and it is not one more failure towards being left for the session.
    fn note_failed_again(&mut self, call: &str) {
        let now = now_unix_secs();
        match self.automation.pilot.attempts.get_mut(&call.to_uppercase()) {
            Some(a) => a.last_unix = now,
            None => self.note_failed(call),
        }
    }

    // ---------------------------------------------------------------- the autopilot

    fn autopilot_step(&mut self, decodes: &[modes::Decode], slot: u64) {
        let now = now_unix_secs();
        let s = self.settings.automation.clone().sanitized();
        let busy = self.automation_contact_in_progress();

        // Presence: unconfirmed, no new contact; once idle, TX off and the autopilot off.
        if self
            .automation
            .pilot
            .presence
            .expired(now, s.presence_minutes())
        {
            self.automation.pilot.status = "presenceDue";
            if !busy {
                self.automation_note("presence not confirmed: transmit off, autopilot off");
                self.automation_restore_hound();
                self.automation.pilot.on = false;
                self.automation.pilot.chase = None;
                self.automation.pilot.target = None;
                self.set_tx_enabled(false);
            }
            return;
        }

        // Held for a tuner or amplifier fault: nothing is called and no band changed meanwhile.
        if self.automation.station_hold.is_some() {
            self.automation.pilot.status = "stationFault";
            return;
        }

        let Some(cur_mode) = mode_name(self.app.tier()) else {
            self.automation.pilot.status = "wrongMode";
            return;
        };

        // A contact the operator started is kept by the same limits as one of the autopilot's own
        // (operator, 2026-11-23: a double-clicked DO8JB was called nine times). A CQ run's is not.
        if busy && !self.cq_running {
            self.automation_adopt_contact(now, slot, cur_mode);
        }

        self.automation_end_stalled(now);

        // The contact the autopilot keeps: finished, still running, or out of calls or time.
        if let Some(t) = self.automation.pilot.target.clone() {
            if busy {
                let (advanced, calls, spent) = match &self.mode {
                    Mode::Qso { station, .. } => (
                        has_answered(station),
                        station.tx_count,
                        self.automation.calls_spent(station, s.target_max_calls),
                    ),
                    _ => (false, 0, false),
                };
                let rare = is_rare(t.tier);
                let budget = if rare {
                    s.dxped_call_secs()
                } else {
                    s.target_budget_secs
                };
                let late = !advanced && now.saturating_sub(t.started_unix) >= u64::from(budget);
                // The last call's answer comes in the next slot, in two folds. Given up from the
                // slot after it, so an answer in either fold is taken (a budget that ran out in
                // the middle of an over used to drop the station before it could answer).
                let answer_heard_out = slot >= self.automation.last_over_slot + 2;
                let give_up = if rare || calls == 0 {
                    late
                } else {
                    spent && answer_heard_out
                };
                if give_up {
                    if t.again {
                        self.note_failed_again(&t.call);
                    } else {
                        self.note_failed(&t.call);
                    }
                    self.automation_note(&if rare || calls == 0 {
                        format!("{} did not answer in {budget} s: moving on", t.call)
                    } else if advanced {
                        format!("{} answered, then went on to others: moving on", t.call)
                    } else {
                        format!(
                            "{} did not answer {calls} call{}: moving on",
                            t.call,
                            if calls == 1 { "" } else { "s" }
                        )
                    });
                    self.automation_abandon_contact();
                    self.automation.pilot.target = None;
                    // Somebody else has been calling meanwhile: answer them.
                    if let Some(c) = self.best_caller(0) {
                        self.start_auto_contact(&c, Tier::CallingMe);
                        return;
                    }
                } else if let Some((e, _)) = (!advanced
                    && !is_rare(t.tier)
                    && t.tier != Tier::CallingMe
                    && s.hunt_dxpeditions
                    && !self.automation_hunting()
                    // The move can be made now, or the same call would be picked again.
                    && self.automation_hold_without_contact().is_none())
                .then(|| self.automation_hunt_pick(cur_mode, now))
                .flatten()
                {
                    // A DXpedition is spotted and the station being called has not answered
                    // (operator, 2026-11-25: a DXpedition spot is always prioritised).
                    if calls > 0 {
                        self.note_failed(&t.call);
                    }
                    self.automation_note(&format!(
                        "{} did not answer: DXpedition {} is spotted on {}",
                        t.call, e.call, e.band
                    ));
                    self.automation_abandon_contact();
                    self.automation.pilot.target = None;
                } else if let Some((c, tier)) = (!advanced)
                    .then(|| self.automation_needed_instead(&t, decodes, slot, cur_mode, &s, now))
                    .flatten()
                {
                    // A needed station is free here and the grid or new call being called has not
                    // answered (live run 2026-11-24: XE2X, needed on 160 m, called CQ while a new
                    // grid was called for two minutes). One picked and not called yet has left
                    // nothing unanswered (RA9ACA, 2026-11-25).
                    if calls > 0 {
                        self.note_failed(&t.call);
                    }
                    self.automation_note(&format!(
                        "{} {}: calling {} instead ({tier:?}, {} dB)",
                        t.call,
                        if calls > 0 {
                            "did not answer"
                        } else {
                            "not called yet"
                        },
                        c.call,
                        c.snr
                    ));
                    self.automation_abandon_contact();
                    self.automation.pilot.target = None;
                    self.automation.pilot.last_worthwhile_unix = now;
                    self.automation.pilot.last_needed_unix = now;
                    self.start_auto_contact(&c, tier);
                    return;
                } else {
                    if late && !rare {
                        if let Some(t) = self.automation.pilot.target.as_mut() {
                            t.out_of_time = true;
                        }
                    }
                    self.automation.pilot.status = if advanced { "inQso" } else { "calling" };
                    if advanced || !t.again {
                        self.automation.pilot.last_worthwhile_unix = now;
                    }
                    // Its own time budget bounds this contact, so the watchdog clock restarts
                    // on each step that keeps it (see `automation_watchdog_restart`).
                    self.automation_watchdog_restart();
                    return;
                }
            } else {
                // Ended: worked (logged, or the exchange complete on the air — REVIEW N-04: with
                // auto-log off nothing is logged, and the station was called again) or not.
                let worked =
                    self.qso_logged || self.automation.done.contains_key(&t.call.to_uppercase());
                if worked {
                    self.automation
                        .pilot
                        .attempts
                        .remove(&t.call.to_uppercase());
                    self.automation.pilot.worked.insert(format!(
                        "{}|{}|{}",
                        t.call.to_uppercase(),
                        t.band,
                        cur_mode
                    ));
                    self.automation_note(&format!("worked {} on {}", t.call, t.band));
                } else {
                    self.note_failed(&t.call);
                }
                self.automation.pilot.target = None;
                let mut clear = false;
                if let Some(ch) = self.automation.pilot.chase.as_mut() {
                    if same_call(&ch.call, &t.call) {
                        if ch.hunt.is_some() {
                            // A hunt ends by going back, not by staying where it hunted.
                            ch.done = ch.done || worked;
                        } else {
                            clear = true;
                        }
                    }
                }
                if clear {
                    self.automation.pilot.chase = None;
                }
            }
        } else if busy {
            // A CQ run's contact: leave it alone.
            self.automation.pilot.status = "inQso";
            self.automation.pilot.last_worthwhile_unix = now;
            return;
        }

        // The partner of the contact just made is still sending to us: let it finish.
        if let Some(call) = self.automation_partner_repeating() {
            self.automation.pilot.status = "finishing";
            self.automation.pilot.reason = format!("waiting for {call} to finish");
            self.automation.pilot.last_worthwhile_unix = now;
            return;
        }
        // While the band is held, the quiet clock does not run: a band is "quiet" only after a
        // full quiet time with every contact finished and nobody calling.
        if self.automation_band_hold().is_some() {
            self.automation.pilot.last_worthwhile_unix = now;
        }

        // The hourly cap.
        if self.automation.pilot.hourly.count(now) >= s.max_qso_per_hour as usize {
            self.automation.pilot.status = "paused";
            self.automation.pilot.reason =
                format!("{} contacts this hour: waiting", s.max_qso_per_hour);
            return;
        }

        // A DXpedition hunt runs outside the schedule: start one, or carry the one under way.
        if (s.hunt_dxpeditions || self.automation_hunting())
            && self.automation_hunt_step(decodes, slot, &s, now)
        {
            return;
        }

        // The schedule: which mode and bands now.
        let Some(patrol) = patrol_at(&s.schedule, utc_minute(now)) else {
            self.automation.pilot.status = "noSchedule";
            return;
        };
        let cur_band = self.settings.band.clone();
        let chasing = self.automation.pilot.chase.clone();
        if !patrol.has_mode(cur_mode) {
            // Its own band in the first of the patrol's modes, else the patrol's first stop.
            let stop = patrol
                .stops()
                .iter()
                .find(|stop| stop.band.eq_ignore_ascii_case(&cur_band))
                .unwrap_or(&patrol.stops()[0])
                .clone();
            if let Some(t) = link_tier_of(&stop.mode) {
                if self.automation_switch(Some(t), &stop.band, None) {
                    self.automation_note(&format!("schedule: {} on {}", stop.mode, stop.band));
                    return;
                }
            }
        }
        if !patrol.has(&cur_band, cur_mode) && chasing.is_none() {
            let stop = patrol
                .stops()
                .iter()
                .find(|stop| stop.mode.eq_ignore_ascii_case(cur_mode))
                .unwrap_or(&patrol.stops()[0])
                .clone();
            if self.automation_go(&stop, cur_mode) {
                self.automation_note(&format!("schedule: moving to {}", stop.band));
                return;
            }
        }

        // Switched on away from the band's channel, as a hunt broken off by hand leaves the dial (live
        // run 2026-11-25: 18.102 after 3B8/E70A): the channel first. Switching off leaves the radio
        // alone, for the operator to go on by hand.
        if chasing.is_none() && std::mem::take(&mut self.automation.pilot.to_channel) {
            let plan = self.band_plan_for_tier(self.app.tier());
            let channel = plan
                .iter()
                .find(|c| c.band.eq_ignore_ascii_case(&cur_band))
                .map(|c| c.dial_mhz)
                .filter(|d| (d - self.settings.dial_mhz).abs() > 1e-6);
            if let Some(dial) = channel {
                if self.automation_switch(None, &cur_band, Some(dial)) {
                    self.automation_note(&format!("to the {cur_band} channel: {dial:.3} MHz"));
                    return;
                }
                self.automation.pilot.to_channel = true;
            }
        }

        // Rank what was heard this slot.
        let candidates = self.automation_candidates(decodes, slot, &cur_band, cur_mode);
        let skip = |c: &Candidate| self.automation_skip(c, &cur_band, cur_mode);
        let attempts = &self.automation.pilot.attempts;
        let picked = automation::pick(&candidates, &s, attempts, now, |c| skip(c))
            .map(|(c, t)| (c.clone(), t));
        let chase_call = chasing
            .as_ref()
            .filter(|ch| ch.hunt.is_none())
            .map(|ch| ch.call.clone());
        // A chase moved here for one needed station: an ordinary new call started now could still
        // be running (two minutes) when that station shows up, so the chase calls its station the
        // moment it is heard, anything better than ordinary, and nothing ordinary (field report
        // 2026-11-22: chasing 5U4JSF on 20 m, the autopilot called SA7SVR). The chase gives up
        // after CHASE_UNHEARD_SECS.
        let picked = match (chase_call, picked) {
            (Some(_), Some((c, t))) if t != Tier::NewCall => Some((c, t)),
            (Some(want), _) => automation::pick(&candidates, &s, attempts, now, |c| {
                skip(c) || !same_call(&c.call, &want)
            })
            .map(|(c, t)| (c.clone(), t)),
            (None, p) => p,
        };

        // Listen first: whole slots, however many decode passes each one brings. A station calling
        // us is answered without waiting them out.
        if self.automation.pilot.slots_on_band < s.listen_slots
            && picked.as_ref().is_none_or(|(_, t)| *t != Tier::CallingMe)
        {
            if self.automation.pilot.last_listen_slot != Some(slot) {
                self.automation.pilot.last_listen_slot = Some(slot);
                self.automation.pilot.slots_on_band += 1;
            }
            self.automation.pilot.status = "listening";
            return;
        }

        // A better spot on another scheduled band beats what is heard here.
        let spot = if s.chase_spots {
            self.automation_best_spot(&patrol, &cur_band, cur_mode, now)
        } else {
            None
        };
        if let Some((spot_call, spot_stop, spot_t)) = spot.clone() {
            let beats = picked
                .as_ref()
                .is_none_or(|(_, t)| s.rank(spot_t) < s.rank(*t));
            let rested = now
                >= self
                    .automation
                    .pilot
                    .chase_failed_unix
                    .saturating_add(chase_rest_secs(spot_t));
            if beats && rested && chasing.is_none() && self.automation_band_hold().is_none() {
                self.automation.pilot.chased.insert(spot_call.clone(), now);
                if self.automation.pilot.rotation.is_none() {
                    self.automation.pilot.rotation = Some(Stop {
                        band: cur_band.clone(),
                        mode: cur_mode.to_string(),
                    });
                }
                self.automation.pilot.chase = Some(Chase {
                    call: spot_call.clone(),
                    band: spot_stop.band.clone(),
                    since_unix: now,
                    hunt: None,
                    done: false,
                });
                self.automation_note(&format!(
                    "spot: {spot_call} ({spot_t:?}) on {}: listening there first",
                    stop_name(&spot_stop, &patrol)
                ));
                self.automation_go(&spot_stop, cur_mode);
                return;
            }
        }

        // A band offering only ordinary calls (new calls, new grids) for its whole ordinary dwell gives way to the
        // next one, to look for needed stations there (between contacts only: the switch holds).
        let p = &self.automation.pilot;
        let ordinary_for = now.saturating_sub(p.last_needed_unix.max(p.band_since_unix));
        let only_ordinary = picked
            .as_ref()
            .is_none_or(|(_, t)| automation::is_ordinary(*t));
        if only_ordinary
            && chasing.is_none()
            && now.saturating_sub(p.band_since_unix) >= u64::from(s.min_dwell_secs)
            && ordinary_for >= u64::from(s.ordinary_dwell_secs)
        {
            if let Some(next) = self.automation_next_stop(&patrol, &cur_band, cur_mode) {
                if !next.is(&cur_band, cur_mode) {
                    if self.automation_go(&next, cur_mode) {
                        self.automation.pilot.rotation = None;
                        self.automation_note(&format!(
                            "only ordinary calls on {cur_band} for {} min: moving to {}",
                            ordinary_for / 60,
                            stop_name(&next, &patrol)
                        ));
                        return;
                    }
                    // The move is due but held (the last partner still finishing, somebody
                    // calling): starting another ordinary call here would hold the band again,
                    // and the move would never come. Wait instead; every hold reason lapses
                    // within a few slots (field report 2026-11-22: BA3KY finished, RU4LM called).
                    self.automation.pilot.status = "holding";
                    return;
                }
            }
        }

        // A spotted DXpedition is waiting for the band to be free (a partner that may still be sending):
        // no new call that would be dropped for it (live run 2026-11-25: BG3TFT, picked and dropped for
        // VK9XY at every decode while 8S2DX was settling).
        if s.hunt_dxpeditions
            && picked
                .as_ref()
                .is_some_and(|(_, t)| !is_rare(*t) && *t != Tier::CallingMe)
            && self.automation_hunt_pick(cur_mode, now).is_some()
        {
            self.automation.pilot.status = "holding";
            return;
        }

        if let Some((c, t)) = picked {
            self.automation.pilot.last_worthwhile_unix = now;
            if !automation::is_ordinary(t) {
                self.automation.pilot.last_needed_unix = now;
            }
            self.automation_note(&format!(
                "calling {} ({t:?}, {} dB) on {cur_band}",
                c.call, c.snr
            ));
            let q = QueuedCaller {
                call: c.call.clone(),
                message: c.message.clone(),
                snr: c.snr,
                freq_hz: c.freq_hz,
                slot: c.slot,
            };
            self.start_auto_contact(&q, t);
            return;
        }

        // A chase that has not heard its station in time, or not worked it, is given up.
        let mut still_chasing = false;
        if let Some(ch) = &chasing {
            if now.saturating_sub(ch.since_unix) >= CHASE_UNHEARD_SECS {
                let call = ch.call.clone();
                let called = self
                    .automation
                    .pilot
                    .attempts
                    .get(&call.to_uppercase())
                    .is_some_and(|a| a.last_unix >= ch.since_unix);
                self.automation.pilot.chase = None;
                // Never heard: not chased again for half an hour, not the plain cooldown (live run
                // 2026-11-22: W5BUY on 12 m, chased at 13:05 and again at 13:22, heard neither time).
                let until = if called {
                    now
                } else {
                    now + CHASE_COOLDOWN_SECS
                };
                self.automation
                    .pilot
                    .chased
                    .insert(call.to_uppercase(), until);
                self.automation.pilot.chase_failed_unix = now;
                self.automation_note(&format!(
                    "{call} {} on {}: back to the schedule",
                    if called {
                        "did not answer"
                    } else {
                        "not heard"
                    },
                    ch.band
                ));
            } else {
                still_chasing = true;
            }
        }

        // Nothing worth calling: move on once the band has had its time.
        let p = &self.automation.pilot;
        let on_band = now.saturating_sub(p.band_since_unix);
        let quiet = now.saturating_sub(p.last_worthwhile_unix.max(p.band_since_unix));
        if !still_chasing
            && on_band >= u64::from(s.min_dwell_secs)
            && quiet >= u64::from(s.quiet_secs)
        {
            if let Some(next) = self.automation_next_stop(&patrol, &cur_band, cur_mode) {
                if !next.is(&cur_band, cur_mode) && self.automation_go(&next, cur_mode) {
                    self.automation.pilot.rotation = None;
                    self.automation_note(&format!(
                        "nothing to call on {cur_band}: moving to {}",
                        stop_name(&next, &patrol)
                    ));
                    return;
                }
            }
        }
        // Nobody else is wanted here, and a needed station that left our calls unanswered still
        // calls CQ (or ends a contact): called again rather than listened to (live run 2026-11-23: 8Q7XX, a needed 80 m
        // slot, called CQ beside an idle autopilot for four minutes). The band's quiet time is not
        // put off by it, so the schedule still moves on.
        let none = HashMap::new();
        let again = automation::pick(&candidates, &s, &none, now, |c| {
            let rested = self
                .automation
                .pilot
                .attempts
                .get(&c.call.to_uppercase())
                .is_some_and(|a| now >= a.last_unix.saturating_add(CALL_AGAIN_SECS));
            let needed = automation::best_allowed_tier(c, &s).is_some_and(automation::is_needed);
            self.automation_skip(c, &cur_band, cur_mode) || !rested || !needed
        })
        .map(|(c, t)| (c.clone(), t));
        if let Some((c, t)) = again {
            self.automation_note(&format!(
                "calling {} again ({t:?}, {} dB) on {cur_band}",
                c.call, c.snr
            ));
            let q = QueuedCaller {
                call: c.call.clone(),
                message: c.message.clone(),
                snr: c.snr,
                freq_hz: c.freq_hz,
                slot: c.slot,
            };
            self.start_auto_contact(&q, t);
            if let Some(target) = self.automation.pilot.target.as_mut() {
                target.again = same_call(&target.call, &c.call);
            }
            return;
        }
        if still_chasing {
            self.automation.pilot.status = "listening";
            return;
        }
        self.automation.pilot.status = if self.automation_band_hold().is_some() {
            "holding"
        } else {
            "listening"
        };
    }

    /// Is this station never called here, whatever its tier?
    fn automation_skip(&self, c: &Candidate, band: &str, mode: &str) -> bool {
        let hound = self.settings.special_op != SpecialOp::None;
        self.automation_skip_call(&c.call, band, mode)
            // A DXpedition running Fox/Hound (FT8) calls below 1000 Hz: Hound is the operator's
            // per-DXpedition choice, so the ordinary pick leaves a Fox alone.
            || (c.dxpedition
                && c.freq_hz < 1000.0
                && !hound
                && self.app.tier() == LinkTier::Ft8)
    }

    /// A call never called here: our own, a blocked one, or one worked already.
    fn automation_skip_call(&self, call: &str, band: &str, mode: &str) -> bool {
        same_call(call, &self.settings.mycall)
            || self
                .settings
                .blocked_calls
                .iter()
                .any(|b| same_call(b, call))
            || self
                .automation
                .pilot
                .worked
                .contains(&format!("{}|{band}|{mode}", call.to_uppercase()))
            // A contact just made: its repeats and 73s are not a new call.
            || self.automation.done.contains_key(&call.to_uppercase())
    }

    /// A needed station free to be called while `t`, only a grid or an ordinary new call, has not
    /// answered: it is called instead. None while a station that calls us is being answered.
    fn automation_needed_instead(
        &self,
        t: &Target,
        decodes: &[modes::Decode],
        slot: u64,
        mode: &str,
        s: &automation::AutomationSettings,
        now: u64,
    ) -> Option<(QueuedCaller, Tier)> {
        if !automation::is_ordinary(t.tier) {
            return None;
        }
        let band = &self.settings.band;
        let list = self.automation_candidates(decodes, slot, band, mode);
        let attempts = &self.automation.pilot.attempts;
        automation::pick(&list, s, attempts, now, |c| {
            c.to_me || same_call(&c.call, &t.call) || self.automation_skip(c, band, mode)
        })
        .filter(|(_, tier)| automation::is_needed(*tier))
        .map(|(c, tier)| {
            let q = QueuedCaller {
                call: c.call.clone(),
                message: c.message.clone(),
                snr: c.snr,
                freq_hz: c.freq_hz,
                slot: c.slot,
            };
            (q, tier)
        })
    }

    /// A contact the sequencer's call cap has stalled is sending nothing but still armed, and the TX
    /// watchdog, rightly, disarms a transmitter left like that (live run 2026-11-24: XU7O's stalled
    /// over switched the autopilot off in the middle of its hunt). Under the autopilot it is kept
    /// [`STALL_KEEP_SECS`] for a late answer, and then ended, so nothing is left armed for the
    /// watchdog to find. Bounded by the wall clock, so the watchdog is held off no longer than that.
    fn automation_end_stalled(&mut self, now: u64) {
        let stalled = !self.cq_running
            && matches!(&self.mode, Mode::Qso { station, .. } if station.dxcall.is_some() && station.stalled());
        if !stalled {
            self.automation.pilot.stalled_unix = None;
            return;
        }
        let since = *self.automation.pilot.stalled_unix.get_or_insert(now);
        if now.saturating_sub(since) < STALL_KEEP_SECS {
            self.automation_watchdog_restart();
            return;
        }
        self.automation.pilot.stalled_unix = None;
        let call = match &self.mode {
            Mode::Qso { station, .. } => station.dxcall.clone().unwrap_or_default(),
            _ => String::new(),
        };
        self.automation_note(&format!("{call} went quiet: call ended"));
        self.automation_abandon_contact();
    }

    /// Is a contact running (a station being called or worked, not yet finished)?
    fn automation_contact_in_progress(&self) -> bool {
        match &self.mode {
            Mode::Qso { station, .. } => {
                station.dxcall.is_some()
                    && match station.state {
                        // Our closing 73 still to go out.
                        QsoState::Done => station.pending.is_some(),
                        QsoState::Listening | QsoState::CallingCq => false,
                        // Our RR73 has gone: the contact is made.
                        QsoState::Confirming => station.tx_count == 0,
                        _ => !station.stalled(),
                    }
            }
            _ => false,
        }
    }

    /// The stop of the patrol (a band in a mode) that has waited longest for its turn, never the one
    /// we are on. Stops that have waited alike (none of them visited yet) come in the schedule's
    /// order, from the one a chase left, when one took us away, else from the current one.
    fn automation_next_stop(
        &self,
        patrol: &automation::Patrol,
        cur_band: &str,
        cur_mode: &str,
    ) -> Option<Stop> {
        let p = &self.automation.pilot;
        let (band, mode) = p
            .rotation
            .as_ref()
            .map_or((cur_band, cur_mode), |s| (s.band.as_str(), s.mode.as_str()));
        patrol
            .after(band, mode)
            .into_iter()
            .filter(|s| !s.is(cur_band, cur_mode))
            .min_by_key(|s| {
                p.turns
                    .get(&turn_key(&s.band, &s.mode))
                    .copied()
                    .unwrap_or(0)
            })
            .cloned()
    }

    /// Go to a stop of the patrol: its band, and its mode when that is not the one running.
    fn automation_go(&mut self, stop: &Stop, cur_mode: &str) -> bool {
        let tier = (!stop.mode.eq_ignore_ascii_case(cur_mode))
            .then(|| link_tier_of(&stop.mode))
            .flatten();
        self.automation_switch(tier, &stop.band, None)
    }

    /// Take the contact under way as the autopilot's target when it is not already: the operator
    /// picked it, or picked the same station again. Counted against neither the hourly cap nor the
    /// schedule; only its limits and its outcome are the autopilot's.
    fn automation_adopt_contact(&mut self, now: u64, slot: u64, mode: &str) {
        let (dx, calls, advanced) = match &self.mode {
            Mode::Qso { station, .. } => match station.dxcall.clone() {
                Some(dx) => (dx, station.tx_count, station.advanced_on_air),
                None => return,
            },
            _ => return,
        };
        if let Some(t) = self.automation.pilot.target.as_mut() {
            if same_call(&t.call, &dx) {
                if calls < t.calls && !advanced {
                    // The same station started again: its clock starts again with it.
                    t.started_unix = now;
                    t.out_of_time = false;
                }
                t.calls = calls;
                return;
            }
        }
        let tier = self.automation_tier_of_call(&dx, slot, mode);
        let band = self.settings.band.clone();
        self.automation.pilot.target = Some(Target {
            call: dx,
            tier,
            started_unix: now,
            band,
            calls,
            out_of_time: false,
            again: false,
        });
    }

    /// The rule a station falls in, known only by its call.
    fn automation_tier_of_call(&self, call: &str, slot: u64, mode: &str) -> Tier {
        if self.automation_fed_rare(call) {
            return Tier::Dxpedition;
        }
        let line = modes::Decode {
            message: format!("CQ {call}"),
            sync: 0.0,
            snr: 0,
            dt: 0.0,
            freq: 0.0,
            nap: 0,
            qual: 1.0,
            rv: None,
            mode: None,
            raw: None,
            unchecked: false,
        };
        self.automation_candidates(&[line], slot, &self.settings.band, mode)
            .first()
            .and_then(automation::tier_of)
            .unwrap_or(Tier::NewCall)
    }

    /// Does the operator call stations where this call is? `true` for one that cannot be placed.
    fn automation_call_wanted(&self, call: &str) -> bool {
        let skip = &self.settings.automation.skip_continents;
        if skip.is_empty() {
            return true;
        }
        let continent = match (&self.station.dxcc_resolve, &self.automation.entity_info.0) {
            (Some(entity), Some(info)) => entity(call).and_then(|name| info(&name).continent),
            _ => None,
        };
        automation::continent_wanted(continent.as_deref(), skip)
    }

    /// Stop calling a station that did not answer: back to listening, TX left as the operator set
    /// it. Nothing more is sent to that station.
    fn automation_abandon_contact(&mut self) {
        let mycall = self.settings.mycall.clone();
        let mygrid = self.settings.mygrid.clone();
        self.mode = Mode::Qso {
            station: Box::new(tempo_core::qso::Station::monitoring(&mycall, &mygrid)),
            running: false,
        };
        self.qso_report_sent = None;
        self.qso_start_unix = None;
        self.immediate_tx = false;
        self.tx_queue.clear();
    }

    /// The decodes of this slot as candidates, with what the feed knows about each station.
    fn automation_candidates(
        &self,
        decodes: &[modes::Decode],
        slot: u64,
        band: &str,
        mode: &str,
    ) -> Vec<Candidate> {
        if band.is_empty() {
            return Vec::new();
        }
        let fold_mode = self.settings.b4_match_mode;
        let band_key =
            tempo_core::logbook::Logbook::band_key(band, self.adif_mode_for_tier(), fold_mode);
        let link_tier = self.app.tier();
        let hot = self.station.hot();
        let now = now_unix_secs();
        decodes
            .iter()
            .filter(|d| !d.unchecked)
            .filter_map(|d| {
                let row = self.decode_row(d, &hot, Some(band), &band_key, fold_mode, link_tier);
                let call = row.from.clone()?;
                if row.mine || call.contains("<...>") {
                    return None;
                }
                let is_cq =
                    matches!(Msg::parse(&d.message), Msg::Cq { ref dir, .. } if answerable_cq(dir));
                let fed: Vec<&FeedEntry> = self
                    .automation
                    .feed
                    .get(&call.to_uppercase())
                    .into_iter()
                    .flatten()
                    .filter(|(_, at)| now.saturating_sub(*at) <= FEED_FRESH_SECS)
                    .map(|(e, _)| e)
                    .collect();
                let here: Vec<&FeedEntry> = fed
                    .iter()
                    .copied()
                    .filter(|e| {
                        e.band.eq_ignore_ascii_case(band)
                            && e.mode
                                .as_deref()
                                .is_none_or(|m| m.eq_ignore_ascii_case(mode))
                    })
                    .collect();
                // A needed state (the board's NewState on this band) the operator wants.
                let new_state = here.iter().any(|e| {
                    let resolved = self
                        .station
                        .state_resolve
                        .as_ref()
                        .and_then(|r| r(&call, row.grid.as_deref()));
                    e.new_state
                        && self
                            .settings
                            .automation
                            .wants_state(e.state.as_deref().or(resolved.as_deref()))
                });
                let info = match (&self.automation.entity_info.0, row.country.as_deref()) {
                    (Some(r), Some(name)) => r(name),
                    _ => EntityInfo::default(),
                };
                Some(Candidate {
                    call,
                    message: d.message.clone(),
                    snr: d.snr,
                    freq_hz: d.freq,
                    slot,
                    call_status: row.call_status.clone(),
                    dxcc_status: row.dxcc_status.clone(),
                    is_cq,
                    ending: !row.directed_to_me
                        && matches!(
                            Msg::parse(&d.message),
                            Msg::Bye73 { .. } | Msg::Rr73 { .. } | Msg::Rrr { .. }
                        ),
                    to_me: row.directed_to_me
                        && self.settings.automation.answer_callers
                        && !matches!(
                            Msg::parse(&d.message),
                            Msg::Bye73 { .. } | Msg::Rr73 { .. } | Msg::Rrr { .. }
                        ),
                    dxpedition: fed.iter().any(|e| e.dxpedition),
                    new_reference: here.iter().any(|e| e.new_reference),
                    new_state,
                    lotw: row.lotw_user,
                    distance_km: row
                        .grid
                        .as_deref()
                        .filter(|g| !g.eq_ignore_ascii_case("RR73"))
                        .and_then(|g| automation::grid_km(&self.settings.mygrid, g)),
                    continent: info.continent,
                    most_wanted: info.dxcc.is_some_and(automation::is_most_wanted),
                    new_grid: row.new_grid,
                    // The entity is worked on this band (not new here, not ATNO) and its slot here
                    // is not confirmed.
                    slot_unconfirmed: row.country.is_some()
                        && !row.new_dxcc
                        && !row.new_band
                        && !row.confirmed_band,
                })
            })
            .collect()
    }

    /// The best needed spot at another stop of the patrol (another band, or this band in another of
    /// its modes), not chased lately and not worked: `(call, stop, tier)`. Of two alike, the one in
    /// the mode running.
    fn automation_best_spot(
        &self,
        patrol: &automation::Patrol,
        cur_band: &str,
        cur_mode: &str,
        now: u64,
    ) -> Option<(String, Stop, Tier)> {
        let p = &self.automation.pilot;
        self.automation_spots(now)
            .filter(|e| e.new_entity || self.automation_call_wanted(&e.call))
            .filter_map(|e| {
                let stop = Stop {
                    band: e.band.clone(),
                    mode: e.mode.as_deref()?.to_uppercase(),
                };
                (patrol.has(&stop.band, &stop.mode) && !stop.is(cur_band, cur_mode))
                    .then_some((e, stop))
            })
            .filter(|(e, _)| {
                p.chased
                    .get(&e.call.to_uppercase())
                    .is_none_or(|t| now.saturating_sub(*t) >= CHASE_COOLDOWN_SECS)
            })
            .filter(|(e, _)| automation::may_retry(p.attempts.get(&e.call.to_uppercase()), now))
            .filter(|(e, stop)| {
                !p.worked.contains(&format!(
                    "{}|{}|{}",
                    e.call.to_uppercase(),
                    e.band,
                    stop.mode
                ))
            })
            .filter_map(|(e, stop)| {
                let call_new =
                    e.dxpedition && self.automation_call_new(&e.call, &e.band, &stop.mode);
                spot_tier(e, call_new).map(|t| (e, stop, self.automation_spot_most_wanted(e, t)))
            })
            // A DXpedition is hunted, not chased.
            .filter(|(_, _, t)| {
                *t != Tier::Dxpedition || !self.settings.automation.hunt_dxpeditions
            })
            .filter(|(_, _, t)| automation::tier_allowed(*t, &self.settings.automation.tiers))
            .filter(|(e, _, t)| {
                *t != Tier::UsState || self.settings.automation.wants_state(e.state.as_deref())
            })
            .min_by(|(a, sa, ta), (b, sb, tb)| {
                let s = &self.settings.automation;
                let other = |stop: &Stop| !stop.mode.eq_ignore_ascii_case(cur_mode);
                s.rank(*ta)
                    .cmp(&s.rank(*tb))
                    .then(other(sa).cmp(&other(sb)))
                    .then(a.call.cmp(&b.call))
            })
            .map(|(e, stop, t)| (e.call.to_uppercase(), stop, t))
    }

    /// A needed band slot of a Club Log top-50 entity is a Most Wanted spot: the operator's rule
    /// for those, not the plain new-slot one.
    fn automation_spot_most_wanted(&self, e: &FeedEntry, t: Tier) -> Tier {
        if t != Tier::DxccNewSlot {
            return t;
        }
        let entity = self.station.dxcc_resolve.as_ref().and_then(|r| r(&e.call));
        let dxcc = match (&self.automation.entity_info.0, entity.as_deref()) {
            (Some(info), Some(name)) => info(name).dxcc,
            _ => None,
        };
        if dxcc.is_some_and(automation::is_most_wanted) {
            Tier::MostWanted
        } else {
            t
        }
    }

    /// Change mode (when `tier` is given) and band through the app's own retune paths, then put the
    /// operator's transmit arm back as it was: the context halt of a QSY is not their stop.
    ///
    /// ⛔ Refused — returns `false` and changes nothing — while [`Self::automation_band_hold`]
    /// names a reason: no band or mode change ever happens in the middle of a contact, during a
    /// CQ run, or while somebody is calling. Every band change the autopilot makes goes through
    /// here, so this is the one place the rule is enforced.
    fn automation_switch(
        &mut self,
        tier: Option<LinkTier>,
        band: &str,
        dial_mhz: Option<f64>,
    ) -> bool {
        if let Some(why) = self.automation_band_hold() {
            if self.automation.last_hold.as_deref() != Some(why.as_str()) {
                self.automation_note(&format!("band change held: {why}"));
                self.automation.last_hold = Some(why);
            }
            return false;
        }
        self.automation.last_hold = None;
        // The band we leave (in this mode) has had its turn, when it was given the minimum dwell.
        let mode = mode_name(self.app.tier()).unwrap_or_default();
        let left = turn_key(&self.settings.band, mode);
        if left != turn_key(band, tier.and_then(mode_name).unwrap_or(mode)) {
            let dwell = self.settings.automation.clone().sanitized().min_dwell_secs;
            let p = &mut self.automation.pilot;
            if now_unix_secs().saturating_sub(p.band_since_unix) >= u64::from(dwell) {
                p.last_turn += 1;
                p.turns.insert(left, p.last_turn);
            }
        }
        let was_tx = self.tx_enabled;
        let watchdog = self.tx_watchdog_start;
        let radio_before = self.settings.active_radio;
        let cleared_dx = self.cleared_dx_calls.clone();
        let remote_tx = self.remote_transmit.clone();
        self.automation.self_tuning = true;
        if let Some(t) = tier {
            self.set_tier(t);
        }
        // The station's own plan (#175: 60 m follows the licence country) with the operator's
        // frequency overrides, the one the band buttons tune: never the raw US-default table.
        let plan = self.band_plan_for_tier(self.app.tier());
        if let Some(ch) = plan.iter().find(|c| c.band.eq_ignore_ascii_case(band)) {
            let (dial, b, m) = (
                dial_mhz.unwrap_or(ch.dial_mhz),
                ch.band.clone(),
                ch.mode.clone(),
            );
            if (self.settings.dial_mhz - dial).abs() > 1e-6
                || !self.settings.band.eq_ignore_ascii_case(&b)
            {
                self.machinery_tune(dial, &b, &m);
            }
        } else {
            self.automation_note(&format!("no {band} channel for this mode"));
        }
        if was_tx && !self.tx_enabled {
            if self.settings.active_radio != radio_before {
                // Band routing moved us to another radio: upstream deliberately leaves TX off
                // after a radio handoff, and an unattended re-arm would key a different rig,
                // antenna or amplifier. TX stays off, which ends the autopilot on the next pass.
                self.automation_note("the band change moved to another radio: transmit left off");
            } else {
                self.set_tx_enabled(true);
                self.tx_watchdog_start = watchdog;
                // The arm above clears these; this restore is the autopilot's, not an operator's
                // fresh consent, so put back what the context halt kept (as upstream's manual
                // restore does): a CAT-broker client may not key until the operator arms, Clear
                // DX's dropped callers stay dropped, and a Remote device keeps ownership.
                self.broker_context_hold = true;
                self.cleared_dx_calls = cleared_dx;
                self.remote_transmit = remote_tx;
            }
        }
        self.automation.self_tuning = false;
        let now = now_unix_secs();
        let p = &mut self.automation.pilot;
        p.band = Some(self.settings.band.clone());
        p.band_since_unix = now;
        p.last_worthwhile_unix = now;
        p.last_needed_unix = now;
        p.slots_on_band = 0;
        p.last_listen_slot = None;
        p.status = "moving";
        true
    }

    // ---------------------------------------------------------------- DXpedition hunt

    /// Is a DXpedition hunt under way?
    fn automation_hunting(&self) -> bool {
        self.automation
            .pilot
            .chase
            .as_ref()
            .is_some_and(|c| c.hunt.is_some())
    }

    /// The best DXpedition to hunt now: a needed, listed DXpedition from the feed, in an FT mode,
    /// on a band the licence allows (the feed drops the rest), not audible from here, not hunted
    /// lately and not worked. `(entry, tier)`.
    fn automation_hunt_pick(&self, cur_mode: &str, now: u64) -> Option<(FeedEntry, Tier)> {
        let p = &self.automation.pilot;
        let dial = self.settings.dial_mhz;
        let cur_band = &self.settings.band;
        let cooled = |key: &str| {
            p.chased
                .get(key)
                .is_none_or(|t| now.saturating_sub(*t) >= CHASE_COOLDOWN_SECS)
        };
        let mode_of = |e: &FeedEntry| e.mode.as_deref().unwrap_or(cur_mode).to_uppercase();
        self.automation_spots(now)
            .map(|e| {
                let call_new =
                    e.dxpedition && self.automation_call_new(&e.call, &e.band, &mode_of(e));
                (e, call_new)
            })
            .filter(|(e, call_new)| is_hunt_spot(e, *call_new))
            .filter(|(e, _)| e.new_entity || self.automation_call_wanted(&e.call))
            .filter(|(e, _)| e.mode.as_deref().is_none_or(|m| link_tier_of(m).is_some()))
            // Already audible where we are: the ordinary pick calls it here.
            .filter(|(e, _)| {
                !(e.band.eq_ignore_ascii_case(cur_band)
                    && mode_of(e).eq_ignore_ascii_case(cur_mode)
                    && e.freq_mhz.is_none_or(|f| in_passband(f, dial)))
            })
            // Each band and mode of a DXpedition cools down by itself; the call itself only rests
            // after a hunt that decoded nothing.
            .filter(|(e, _)| {
                let key = hunt_key(&e.call, &e.band, &mode_of(e));
                cooled(&key)
                    && cooled(&e.call.to_uppercase())
                    && automation::may_retry(p.attempts.get(&key), now)
            })
            .filter(|(e, _)| {
                !p.worked.contains(&format!(
                    "{}|{}|{}",
                    e.call.to_uppercase(),
                    e.band,
                    mode_of(e)
                ))
            })
            .filter_map(|(e, call_new)| spot_tier(e, call_new).map(|t| (e, t)))
            .filter(|(_, t)| automation::tier_allowed(*t, &self.settings.automation.tiers))
            .min_by(|(a, ta), (b, tb)| {
                let pr = &self.settings.automation.priority;
                automation::rank_in(pr, *ta)
                    .cmp(&automation::rank_in(pr, *tb))
                    // The newest spot first: the band it is heard on now.
                    .then(b.heard_unix.cmp(&a.heard_unix))
                    .then(a.call.cmp(&b.call))
            })
            .map(|(e, t)| (e.clone(), t))
    }

    /// Start, carry or end a DXpedition hunt. `true` = this slot is the hunt's (the schedule
    /// and the ordinary pick do not run).
    ///
    /// Every move — out to the DXpedition and back — goes through [`Self::automation_switch`],
    /// so none happens in the middle of a contact or while somebody is calling.
    fn automation_hunt_step(
        &mut self,
        decodes: &[modes::Decode],
        slot: u64,
        s: &automation::AutomationSettings,
        now: u64,
    ) -> bool {
        let cur_mode = mode_name(self.app.tier()).unwrap_or("FT8").to_string();
        let hunting = self
            .automation
            .pilot
            .chase
            .clone()
            .filter(|c| c.hunt.is_some());
        let heard_now = if hunting.is_none() && s.hunt_dxpeditions {
            self.automation_hunt_here(decodes, slot, s, now)
        } else {
            None
        };
        let Some(ch) = hunting.or(heard_now) else {
            // No hunt yet: is there a DXpedition worth one? A plain chase gives way to it.
            if !s.hunt_dxpeditions || self.automation_band_hold().is_some() {
                return false;
            }
            let Some((e, t)) = self.automation_hunt_pick(&cur_mode, now) else {
                return false;
            };
            let tier = e
                .mode
                .as_deref()
                .and_then(link_tier_of)
                .unwrap_or(self.app.tier());
            let plan = self.band_plan_for_tier(tier);
            let channel = plan
                .iter()
                .find(|c| c.band.eq_ignore_ascii_case(&e.band))
                .map(|c| c.dial_mhz);
            let dial = e.freq_mhz.map(|f| hunt_dial(f, channel));
            let back = Hunt {
                dial_mhz: 0.0,
                spot_mhz: e.freq_mhz.unwrap_or(0.0),
                back_band: self.settings.band.clone(),
                back_dial_mhz: self.settings.dial_mhz,
                back_tier: self.app.tier(),
                hound_from: None,
                heard: false,
                called_unix: None,
            };
            let call = e.call.to_uppercase();
            // Never hunt on a spot whose frequency is off the band its label names, or whose
            // TX passband (dial + 3 kHz) would cross that band's edge: under the Open licence
            // class the keying gate accepts any frequency, so this is the only check. A spot
            // with no frequency uses the band's standard channel and needs none.
            if let (Some(spot_mhz), Some(d)) = (e.freq_mhz, dial) {
                let band = crate::bandplan::band_for_dial(spot_mhz);
                let fits = band.is_some_and(|b| b.eq_ignore_ascii_case(&e.band))
                    && crate::bandplan::band_for_dial(d) == band
                    && crate::bandplan::band_for_dial(d + 0.003) == band;
                if !fits {
                    let mode = mode_name(tier).unwrap_or("FT8");
                    self.note_failed(&hunt_key(&call, &e.band, mode));
                    self.automation_note(&format!(
                        "DXpedition {call} spotted at {spot_mhz:.3} MHz as {}: not inside that band                          with room for our signal, not hunted",
                        e.band
                    ));
                    return false;
                }
            }
            let mode_switch = (tier != self.app.tier()).then_some(tier);
            if !self.automation_switch(mode_switch, &e.band, dial) {
                return false;
            }
            let key = hunt_key(&call, &e.band, mode_name(tier).unwrap_or("FT8"));
            self.automation.pilot.chased.insert(key, now);
            self.automation.pilot.chase = Some(Chase {
                call: call.clone(),
                band: e.band.clone(),
                since_unix: now,
                hunt: Some(Hunt {
                    dial_mhz: self.settings.dial_mhz,
                    ..back
                }),
                done: false,
            });
            self.automation_note(&format!(
                "DXpedition {call} ({t:?}) spotted on {} {}: hunting it there",
                e.band,
                dial.map_or_else(|| "(band channel)".to_string(), |d| format!("{d:.3} MHz"))
            ));
            return true;
        };
        let Some(mut hunt) = ch.hunt.clone() else {
            return false;
        };

        let was_on_air = hunt.heard || hunt.called_unix.is_some();

        // Heard? And does it run Fox/Hound (transmitting below 1000 Hz)?
        let heard_here = decodes.iter().find(|d| {
            Msg::parse(&d.message)
                .sender()
                .is_some_and(|de| same_call(de, &ch.call))
        });
        if let Some(d) = heard_here {
            hunt.heard = true;
            // Fox/Hound exists only in FT8 (REVIEW N-11).
            if s.hunt_hound
                && self.app.tier() == LinkTier::Ft8
                // Where it sits on the dial it was found on, not where our own retune put it
                // (3B8/E70A, 2026-11-25: taken for a Fox at 399 Hz, which was 2489).
                && f64::from(d.freq) + (self.settings.dial_mhz - hunt.dial_mhz) * 1e6 < 1000.0
                && self.settings.special_op == SpecialOp::None
                && hunt.hound_from.is_none()
            {
                hunt.hound_from = Some(self.settings.special_op);
                self.settings.special_op = SpecialOp::Hound;
                self.automation_note(&format!("{} runs Fox/Hound: Hound on", ch.call));
            }
        }
        // Not heard here, but on the air: others are heard calling it or working it.
        if decodes.iter().any(|d| {
            Msg::parse(&d.message)
                .addressee()
                .is_some_and(|to| same_call(to, &ch.call))
        }) {
            hunt.called_unix = Some(now);
        }
        let on_air = hunt
            .called_unix
            .is_some_and(|t| now.saturating_sub(t) <= HUNT_CALLED_SECS);
        let unheard_secs = if on_air {
            u64::from(s.dxped_call_secs())
        } else {
            CHASE_UNHEARD_SECS
        };
        let mut ch = ch;
        // Followed for the operator's time from the moment it, or somebody calling it, is heard.
        if !was_on_air && (hunt.heard || on_air) {
            ch.since_unix = now;
        }
        if let Some(c) = self.automation.pilot.chase.as_mut() {
            c.since_unix = ch.since_unix;
            c.hunt = Some(hunt.clone());
        }

        // Over? Worked, out of time, never heard, or no longer spotted and not heard.
        let elapsed = now.saturating_sub(ch.since_unix);
        let spotted = self.automation.feed.get(&ch.call).is_some_and(|l| {
            l.iter()
                .any(|(_, at)| now.saturating_sub(*at) <= FEED_FRESH_SECS)
        });
        let why = if ch.done {
            Some(format!("worked {}", ch.call))
        } else if elapsed >= u64::from(s.dxped_call_secs()) && (hunt.heard || on_air) {
            Some(format!(
                "{} not worked in {} min",
                ch.call, s.dxped_call_mins
            ))
        } else if !hunt.heard && elapsed >= unheard_secs {
            Some(format!("{} not heard", ch.call))
        } else if !hunt.heard && !spotted && !on_air {
            Some(format!("{} no longer spotted", ch.call))
        } else if !s.hunt_dxpeditions {
            Some("hunting switched off".to_string())
        } else {
            None
        };
        if let Some(why) = why {
            return self.automation_hunt_return(&ch, &hunt, &why);
        }

        // Still hunting: call it when heard (callers to us are answered as ever), else listen.
        if self.automation_contact_in_progress() || self.automation_partner_repeating().is_some() {
            return true;
        }
        let band = self.settings.band.clone();
        let candidates = self.automation_candidates(decodes, slot, &band, &cur_mode);
        // Wherever it is heard: a Fox in FT8 has put Hound on above, and any other DXpedition is
        // called from 1200 Hz above it (live run 2026-11-23: one below 1000 Hz was never called
        // in FT4, where there is no Hound).
        let target = candidates
            .iter()
            .filter(|c| c.to_me || same_call(&c.call, &ch.call))
            .min_by_key(|c| (!c.to_me, -c.snr))
            .cloned();
        if let Some(c) = target {
            let tier = if c.to_me {
                Tier::CallingMe
            } else {
                Tier::Dxpedition
            };
            self.automation_note(&format!(
                "calling {} ({tier:?}, {} dB) on {band}",
                c.call, c.snr
            ));
            let q = QueuedCaller {
                call: c.call.clone(),
                message: c.message.clone(),
                snr: c.snr,
                freq_hz: c.freq_hz,
                slot: c.slot,
            };
            self.start_auto_contact(&q, tier);
        } else {
            self.automation.pilot.status = "hunting";
            self.automation.pilot.reason = if hunt.spot_mhz > 0.0 {
                format!(
                    "hunting {} on {band} (spotted {:.4} MHz)",
                    ch.call, hunt.spot_mhz
                )
            } else {
                format!("hunting {} on {band}", ch.call)
            };
        }
        true
    }

    /// A listed DXpedition decoded where we are, under 1000 Hz in FT8, not worked here: hunted in
    /// place, as the ordinary pick leaves a possible Fox alone.
    fn automation_hunt_here(
        &mut self,
        decodes: &[modes::Decode],
        slot: u64,
        s: &automation::AutomationSettings,
        now: u64,
    ) -> Option<Chase> {
        if self.app.tier() != LinkTier::Ft8
            || self.settings.special_op != SpecialOp::None
            || self.automation_band_hold().is_some()
        {
            return None;
        }
        let band = self.settings.band.clone();
        let p = &self.automation.pilot;
        let c = self
            .automation_candidates(decodes, slot, &band, "FT8")
            .into_iter()
            .filter(|c| c.dxpedition && c.freq_hz > 0.0 && c.freq_hz < 1000.0)
            .filter(|c| automation::best_allowed_tier(c, s).is_some_and(is_rare))
            .filter(|c| {
                let key = hunt_key(&c.call, &band, "FT8");
                p.chased
                    .get(&key)
                    .is_none_or(|t| now.saturating_sub(*t) >= CHASE_COOLDOWN_SECS)
                    && automation::may_retry(p.attempts.get(&key), now)
            })
            .find(|c| !self.automation_skip_call(&c.call, &band, "FT8"))?;
        let call = c.call.to_uppercase();
        let chase = Chase {
            call: call.clone(),
            band: band.clone(),
            since_unix: now,
            hunt: Some(Hunt {
                dial_mhz: self.settings.dial_mhz,
                spot_mhz: self.settings.dial_mhz + f64::from(c.freq_hz) / 1e6,
                back_band: band.clone(),
                back_dial_mhz: self.settings.dial_mhz,
                back_tier: self.app.tier(),
                hound_from: None,
                heard: true,
                called_unix: None,
            }),
            done: false,
        };
        let p = &mut self.automation.pilot;
        p.chased.insert(hunt_key(&call, &band, "FT8"), now);
        p.chase = Some(chase.clone());
        self.automation_note(&format!(
            "DXpedition {call} heard on {band}: hunting it here"
        ));
        Some(chase)
    }

    /// End a hunt: Hound back as the operator had it, then back to the band, dial and mode the
    /// hunt left — once nothing holds the band. `true` while the way back is still held.
    fn automation_hunt_return(&mut self, ch: &Chase, hunt: &Hunt, why: &str) -> bool {
        if self.automation_band_hold().is_some() {
            // automation_switch notes the hold; stay until it clears.
            let _ = self.automation_switch(None, &hunt.back_band, Some(hunt.back_dial_mhz));
            self.automation.pilot.status = "holding";
            return true;
        }
        // Counted once, when the hunt really ends (not on every held slot).
        let key = hunt_key(
            &ch.call,
            &ch.band,
            mode_name(self.app.tier()).unwrap_or("FT8"),
        );
        if !ch.done {
            // Tried again each hour, never left for the session (operator: a DXpedition keeps being
            // called).
            for k in [&key, &ch.call.to_uppercase()] {
                self.note_failed(k);
                if let Some(a) = self.automation.pilot.attempts.get_mut(k) {
                    a.failures = a.failures.min(HUNT_MAX_FAILURES);
                }
            }
        }
        // Never heard there: the band is most likely closed to it (a 15 m spot at night), so it
        // waits twice the usual cool-down before it is hunted again; the usual one when stations
        // here were heard calling it.
        if !hunt.heard && !ch.done {
            let now = now_unix_secs();
            let mut until = now;
            if hunt.called_unix.is_none() {
                until += CHASE_COOLDOWN_SECS;
            }
            let chased = &mut self.automation.pilot.chased;
            chased.insert(key, until);
            // Its other bands and modes wait only a moment.
            chased.insert(
                ch.call.to_uppercase(),
                (now + HUNT_REST_SECS).saturating_sub(CHASE_COOLDOWN_SECS),
            );
        }
        if let Some(prev) = hunt.hound_from {
            self.settings.special_op = prev;
        }
        let tier = (hunt.back_tier != self.app.tier()).then_some(hunt.back_tier);
        self.automation_switch(tier, &hunt.back_band, Some(hunt.back_dial_mhz));
        self.automation.pilot.chase = None;
        self.automation_note(&format!(
            "{why}: hunt over, back to {} {:.3} MHz",
            hunt.back_band, hunt.back_dial_mhz
        ));
        true
    }
}
