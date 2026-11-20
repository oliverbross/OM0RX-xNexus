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
//! - **Presence.** Every [`crate::automation::AutomationSettings::presence_check_min`] minutes the
//!   operator must confirm. Unconfirmed, no new contact starts; once the contact in progress ends,
//!   TX goes off and the autopilot switches off.

use std::collections::{HashMap, HashSet, VecDeque};

use super::{now_unix_secs, Engine, Mode};
use crate::automation::{
    self, answerable_cq, is_rare, next_band, next_window_start, spot_tier, window_at, Attempts,
    AutomationDto, Candidate, FeedEntry, HourlyCount, PresenceClock, Tier,
};
use crate::dto::Tier as LinkTier;
use tempo_core::message::{same_call, Msg};
use tempo_core::qso::State as QsoState;

/// How many slots a caller to the operator stays answerable after it was last heard.
const CALLER_FRESH_SLOTS: u64 = 3;
/// How long a spot chase may go without decoding its station before it is given up, seconds.
const CHASE_UNHEARD_SECS: u64 = 300;
/// How long a chased call is not chased again, seconds.
const CHASE_COOLDOWN_SECS: u64 = 900;
/// How long a fed entry stays usable, seconds.
const FEED_FRESH_SECS: u64 = 600;
/// Decisions kept for the panel's log.
const LOG_KEEP: usize = 40;

/// A station that called the operator.
#[derive(Debug, Clone, PartialEq)]
pub struct QueuedCaller {
    pub call: String,
    pub message: String,
    pub snr: i32,
    pub freq_hz: f32,
    pub slot: u64,
}

/// The contact the autopilot started, until it ends.
#[derive(Debug, Clone, PartialEq)]
struct Target {
    call: String,
    tier: Tier,
    started_unix: u64,
    band: String,
}

/// A needed station spotted on another band, being chased there.
#[derive(Debug, Clone, PartialEq)]
struct Chase {
    call: String,
    band: String,
    since_unix: u64,
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
    slots_on_band: u32,
    target: Option<Target>,
    chase: Option<Chase>,
    chased: HashMap<String, u64>,
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
    feed: HashMap<String, (FeedEntry, u64)>,
    pilot: Pilot,
}

impl Automation {
    /// A stop (Stop TX, a remote stop, a broker halt, a context halt that is not ours).
    pub(super) fn on_halt(&mut self) {
        if self.self_tuning {
            return;
        }
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

impl Engine {
    // ---------------------------------------------------------------- operator verbs

    /// Turn the autopilot on or off. On needs Settings ▸ Automation to allow it, transmit already
    /// armed by the operator, and an FT8, FT4 or FT2 tier. Turning it on counts as the first
    /// presence confirmation.
    pub fn set_autopilot(&mut self, on: bool) -> Result<(), String> {
        let now = now_unix_secs();
        if !on {
            if self.automation.pilot.on {
                self.automation.pilot.on = false;
                self.automation.pilot.target = None;
                self.automation.pilot.chase = None;
                self.automation.pilot.status = "off";
                self.automation.pilot.reason = "switched off".into();
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
        p.presence.ack(now);
        p.band = (!self.settings.band.is_empty()).then(|| self.settings.band.clone());
        p.band_since_unix = now;
        p.last_worthwhile_unix = now;
        p.slots_on_band = 0;
        p.target = None;
        p.chase = None;
        p.status = "listening";
        p.reason = "started".into();
        note(&mut p.log, "switched on by the operator");
        self.automation_refresh_keep();
        Ok(())
    }

    /// The operator confirms they are at the station.
    pub fn autopilot_confirm_presence(&mut self) {
        let now = now_unix_secs();
        self.automation.pilot.presence.ack(now);
        if self.automation.pilot.on {
            note(&mut self.automation.pilot.log, "presence confirmed");
        }
        self.automation_refresh_keep();
    }

    /// The UI's need feed (cluster, RBN, OTA, DXpeditions), replacing the last one.
    pub fn autopilot_feed(&mut self, entries: Vec<FeedEntry>) {
        let now = now_unix_secs();
        self.automation.feed = entries
            .into_iter()
            .filter(|e| !e.call.trim().is_empty())
            .map(|e| (e.call.trim().to_uppercase(), (e, now)))
            .collect();
    }

    /// Test hook: move the last presence confirmation `secs` into the past.
    #[cfg(test)]
    pub(super) fn automation_backdate_presence(&mut self, secs: u64) {
        let p = &mut self.automation.pilot.presence;
        p.last_ack_unix = p.last_ack_unix.saturating_sub(secs);
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
        // A watchdog trip, or the operator's TX Off, ends the autopilot: it never re-arms.
        if self.automation.pilot.on && !self.tx_enabled {
            let p = &mut self.automation.pilot;
            p.on = false;
            p.target = None;
            p.chase = None;
            p.status = "off";
            p.reason = "transmit went off".into();
            note(&mut p.log, "transmit went off: autopilot off");
        }
        self.answer_callers_step(decodes, slot);
        if self.automation.pilot.on {
            self.autopilot_step(decodes, slot);
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
            // A caller that already signed off is not calling.
            if matches!(m, Msg::Bye73 { .. } | Msg::Rr73 { .. } | Msg::Rrr { .. }) {
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
        self.automation
            .callers
            .retain(|c| slot.saturating_sub(c.slot) <= CALLER_FRESH_SLOTS);
        if self.automation.callers.is_empty() || !self.tx_enabled {
            return;
        }
        let s = self.settings.automation.clone().sanitized();
        // Take-over: our first call has gone unanswered for N slots and somebody is calling us.
        let unanswered = matches!(state, QsoState::AwaitReport | QsoState::AwaitRoger) && !advanced;
        if s.answer_takeover_slots > 0 && unanswered && tx_count >= s.answer_takeover_slots {
            if let Some(c) = self.best_caller() {
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
        if finished && self.automation.chain < s.answer_chain_limit.max(1) {
            if let Some(c) = self.best_caller() {
                self.automation.chain += 1;
                self.automation_note(&format!(
                    "contact with {dx} done: answering caller {} ({} of {})",
                    c.call,
                    self.automation.chain,
                    s.answer_chain_limit.max(1)
                ));
                self.start_auto_contact(&c, Tier::CallingMe);
            }
        }
    }

    /// The caller to answer: the operator's best-caller strategy over the fresh queue.
    fn best_caller(&self) -> Option<QueuedCaller> {
        let floor = self.settings.best_caller_min_snr;
        let mut list: Vec<&QueuedCaller> = self
            .automation
            .callers
            .iter()
            .filter(|c| floor.is_none_or(|f| c.snr >= f))
            .collect();
        match self.settings.best_caller.as_str() {
            "strongest" => list.sort_by(|a, b| b.snr.cmp(&a.snr)),
            // "first" and every other strategy: the longest-waiting caller first.
            _ => list.sort_by_key(|c| c.slot),
        }
        list.first().map(|c| (*c).clone())
    }

    /// Start a contact exactly as a double-click on `c`'s line would, without arming.
    fn start_auto_contact(&mut self, c: &QueuedCaller, tier: Tier) {
        if !self.tx_enabled {
            return;
        }
        let call = c.call.clone();
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
                self.automation.held_disable = false;
                self.automation
                    .callers
                    .retain(|q| !same_call(&q.call, &call));
                if self.automation.pilot.on {
                    let now = now_unix_secs();
                    let band = self.settings.band.clone();
                    let p = &mut self.automation.pilot;
                    p.target = Some(Target {
                        call: call.clone(),
                        tier,
                        started_unix: now,
                        band,
                    });
                    p.hourly.note(now);
                    p.status = "calling";
                }
            }
            Err(e) => self.automation_note(&format!("could not call {call}: {e}")),
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
                self.automation.pilot.on = false;
                self.automation.pilot.target = None;
                self.set_tx_enabled(false);
            }
            return;
        }

        let Some(cur_mode) = mode_name(self.app.tier()) else {
            self.automation.pilot.status = "wrongMode";
            return;
        };

        // The contact the autopilot started: finished, still running, or out of time.
        if let Some(t) = self.automation.pilot.target.clone() {
            if busy {
                let advanced =
                    matches!(&self.mode, Mode::Qso { station, .. } if station.advanced_on_air);
                let budget = if is_rare(t.tier) {
                    s.rare_budget_secs
                } else {
                    s.target_budget_secs
                };
                if !advanced && now.saturating_sub(t.started_unix) >= u64::from(budget) {
                    self.note_failed(&t.call);
                    self.automation_note(&format!(
                        "{} did not answer in {budget} s: moving on",
                        t.call
                    ));
                    self.automation_abandon_contact();
                    self.automation.pilot.target = None;
                } else {
                    self.automation.pilot.status = if advanced { "inQso" } else { "calling" };
                    return;
                }
            } else {
                // Ended: logged (worked) or not.
                let worked = self.qso_logged;
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
                if self
                    .automation
                    .pilot
                    .chase
                    .as_ref()
                    .is_some_and(|c| same_call(&c.call, &t.call))
                {
                    self.automation.pilot.chase = None;
                }
            }
        } else if busy {
            // A contact the operator (or Answer callers) started: leave it alone.
            self.automation.pilot.status = "inQso";
            return;
        }

        // The hourly cap.
        if self.automation.pilot.hourly.count(now) >= s.max_qso_per_hour as usize {
            self.automation.pilot.status = "paused";
            self.automation.pilot.reason =
                format!("{} contacts this hour: waiting", s.max_qso_per_hour);
            return;
        }

        // The schedule: which mode and bands now.
        let Some(window) = window_at(&s.schedule, utc_minute(now)).cloned() else {
            self.automation.pilot.status = "noSchedule";
            return;
        };
        let cur_band = self.settings.band.clone();
        let chasing = self.automation.pilot.chase.clone();
        if !window.mode.eq_ignore_ascii_case(cur_mode) {
            if let Some(t) = link_tier_of(&window.mode) {
                let band = if window
                    .bands
                    .iter()
                    .any(|b| b.eq_ignore_ascii_case(&cur_band))
                {
                    cur_band.clone()
                } else {
                    window.bands[0].clone()
                };
                self.automation_note(&format!("schedule: {} on {band}", window.mode));
                self.automation_switch(Some(t), &band);
            }
            return;
        }
        let in_window = window
            .bands
            .iter()
            .any(|b| b.eq_ignore_ascii_case(&cur_band));
        if !in_window && chasing.is_none() {
            let band = window.bands[0].clone();
            self.automation_note(&format!("schedule: moving to {band}"));
            self.automation_switch(None, &band);
            return;
        }

        // Listen first.
        if self.automation.pilot.slots_on_band < s.listen_slots {
            self.automation.pilot.slots_on_band += 1;
            self.automation.pilot.status = "listening";
            return;
        }

        // Rank what was heard this slot.
        let candidates = self.automation_candidates(decodes, slot, &cur_band, cur_mode);
        let mycall = self.settings.mycall.clone();
        let blocked = self.settings.blocked_calls.clone();
        let worked = self.automation.pilot.worked.clone();
        let picked = automation::pick(
            &candidates,
            &s.tiers,
            &self.automation.pilot.attempts,
            now,
            |c| {
                same_call(&c.call, &mycall)
                    || blocked.iter().any(|b| same_call(b, &c.call))
                    || worked.contains(&format!("{}|{}|{}", c.call.to_uppercase(), cur_band, cur_mode))
                    // A DXpedition running Fox/Hound calls below 1000 Hz: Hound is the operator's
                    // per-DXpedition choice, so the autopilot leaves a Fox alone.
                    || (c.dxpedition && c.freq_hz < 1000.0)
            },
        )
        .map(|(c, t)| (c.clone(), t));

        // A better spot on another scheduled band beats what is heard here.
        let spot = if s.chase_spots {
            self.automation_best_spot(&window, &cur_band, cur_mode, now)
        } else {
            None
        };
        if let Some((spot_call, spot_band, spot_t)) = spot.clone() {
            let beats = picked.as_ref().is_none_or(|(_, t)| spot_t < *t);
            if beats && chasing.is_none() {
                self.automation.pilot.chased.insert(spot_call.clone(), now);
                self.automation.pilot.chase = Some(Chase {
                    call: spot_call.clone(),
                    band: spot_band.clone(),
                    since_unix: now,
                });
                self.automation_note(&format!(
                    "spot: {spot_call} ({spot_t:?}) on {spot_band}: listening there first"
                ));
                self.automation_switch(None, &spot_band);
                return;
            }
        }

        if let Some((c, t)) = picked {
            self.automation.pilot.last_worthwhile_unix = now;
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

        // A chase that has not heard its station in time is given up.
        if let Some(ch) = &chasing {
            if now.saturating_sub(ch.since_unix) >= CHASE_UNHEARD_SECS {
                let call = ch.call.clone();
                self.automation.pilot.chase = None;
                self.automation_note(&format!(
                    "{call} not heard on {}: back to the schedule",
                    ch.band
                ));
            } else {
                self.automation.pilot.status = "listening";
                return;
            }
        }

        // Nothing worth calling: move on once the band has had its time.
        let p = &self.automation.pilot;
        let on_band = now.saturating_sub(p.band_since_unix);
        let quiet = now.saturating_sub(p.last_worthwhile_unix.max(p.band_since_unix));
        if on_band >= u64::from(s.min_dwell_secs) && quiet >= u64::from(s.quiet_secs) {
            if let Some(next) = next_band(&window, Some(&cur_band)).map(String::from) {
                if !next.eq_ignore_ascii_case(&cur_band) {
                    self.automation_note(&format!(
                        "nothing to call on {cur_band}: moving to {next}"
                    ));
                    self.automation_switch(None, &next);
                    return;
                }
            }
        }
        self.automation.pilot.status = "listening";
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
                let fed = self
                    .automation
                    .feed
                    .get(&call.to_uppercase())
                    .filter(|(_, at)| now.saturating_sub(*at) <= FEED_FRESH_SECS)
                    .map(|(e, _)| e);
                let on_this_band = |e: &&FeedEntry| {
                    e.band.eq_ignore_ascii_case(band)
                        && e.mode
                            .as_deref()
                            .is_none_or(|m| m.eq_ignore_ascii_case(mode))
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
                    to_me: row.directed_to_me
                        && self.settings.automation.answer_callers
                        && !matches!(
                            Msg::parse(&d.message),
                            Msg::Bye73 { .. } | Msg::Rr73 { .. } | Msg::Rrr { .. }
                        ),
                    dxpedition: fed.is_some_and(|e| e.dxpedition),
                    new_reference: fed.filter(on_this_band).is_some_and(|e| e.new_reference),
                    lotw: row.lotw_user,
                })
            })
            .collect()
    }

    /// The best needed spot on another band of this window, in this mode, not chased lately and
    /// not worked: `(call, band, tier)`.
    fn automation_best_spot(
        &self,
        window: &automation::AutopilotWindow,
        cur_band: &str,
        mode: &str,
        now: u64,
    ) -> Option<(String, String, Tier)> {
        let p = &self.automation.pilot;
        self.automation
            .feed
            .values()
            .filter(|(_, at)| now.saturating_sub(*at) <= FEED_FRESH_SECS)
            .map(|(e, _)| e)
            .filter(|e| !e.band.eq_ignore_ascii_case(cur_band))
            .filter(|e| window.bands.iter().any(|b| b.eq_ignore_ascii_case(&e.band)))
            .filter(|e| {
                e.mode
                    .as_deref()
                    .is_some_and(|m| m.eq_ignore_ascii_case(mode))
            })
            .filter(|e| {
                p.chased
                    .get(&e.call.to_uppercase())
                    .is_none_or(|t| now.saturating_sub(*t) >= CHASE_COOLDOWN_SECS)
            })
            .filter(|e| automation::may_retry(p.attempts.get(&e.call.to_uppercase()), now))
            .filter(|e| {
                !p.worked
                    .contains(&format!("{}|{}|{}", e.call.to_uppercase(), e.band, mode))
            })
            .filter_map(|e| spot_tier(e).map(|t| (e, t)))
            .filter(|(_, t)| automation::tier_allowed(*t, &self.settings.automation.tiers))
            .min_by(|(a, ta), (b, tb)| ta.cmp(tb).then(a.call.cmp(&b.call)))
            .map(|(e, t)| (e.call.to_uppercase(), e.band.clone(), t))
    }

    /// Change mode (when `tier` is given) and band through the app's own retune paths, then put the
    /// operator's transmit arm back as it was: the context halt of a QSY is not their stop.
    fn automation_switch(&mut self, tier: Option<LinkTier>, band: &str) {
        let was_tx = self.tx_enabled;
        let watchdog = self.tx_watchdog_start;
        self.automation.self_tuning = true;
        if let Some(t) = tier {
            self.set_tier(t);
        }
        let plan = crate::bandplan::band_plan_for(self.app.tier());
        if let Some(ch) = plan.iter().find(|c| c.band.eq_ignore_ascii_case(band)) {
            let (dial, b, m) = (ch.dial_mhz, ch.band.clone(), ch.mode.clone());
            if (self.settings.dial_mhz - dial).abs() > 1e-6
                || !self.settings.band.eq_ignore_ascii_case(&b)
            {
                self.machinery_tune(dial, &b, &m);
            }
        } else {
            self.automation_note(&format!("no {band} channel for this mode"));
        }
        if was_tx && !self.tx_enabled {
            self.set_tx_enabled(true);
            self.tx_watchdog_start = watchdog;
        }
        self.automation.self_tuning = false;
        let now = now_unix_secs();
        let p = &mut self.automation.pilot;
        p.band = Some(self.settings.band.clone());
        p.band_since_unix = now;
        p.last_worthwhile_unix = now;
        p.slots_on_band = 0;
        p.status = "moving";
    }
}
