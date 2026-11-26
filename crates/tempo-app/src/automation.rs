// SPDX-License-Identifier: GPL-3.0-only
//! OM0RX-xNexus automation: **Answer callers** and the **supervised autopilot** — the settings and
//! every decision that can be made without the engine (which band and mode the schedule wants now,
//! which heard station is worth calling, when a failed target may be tried again, when the
//! operator's presence check is due). The engine half, which acts on these decisions through the
//! existing QSO sequencer and tuning paths, is `engine/autopilot.rs`.
//!
//! ⛔ THE SIGN-OFF AND ITS SCOPE (operator OM0RX, 2026-11-20, accepting the xNexus automation
//! proposal): a fork-only divergence from WSJT-X, approved for testing.
//!
//! - **Off by default.** `answer_callers` and `autopilot_enabled` both start false; with both off
//!   nothing here changes how Nexus behaves on the air.
//! - **Supervised, never unattended.** The autopilot asks the operator to confirm presence every
//!   [`AutomationSettings::presence_check_min`] minutes (2–999, default 10). Unanswered, it starts
//!   no new contact, lets the one in progress finish, and turns transmit off.
//! - **It never arms transmit by itself from nothing.** The operator turns TX on and then the
//!   autopilot on, two separate acts; Stop TX or TX Off stops the autopilot and switches it off.
//! - **The TX invariants stay.** The TX-enable latch, licence gating, identity validation and the
//!   wall-clock watchdog are untouched: every over still passes through them.

use serde::{Deserialize, Serialize};

/// The smallest and largest presence-check interval the operator may set, in minutes.
pub const PRESENCE_MIN_RANGE: std::ops::RangeInclusive<u32> = 2..=999;
/// The presence-check interval out of the box, in minutes.
pub const DEFAULT_PRESENCE_MIN: u32 = 10;

/// One window of the autopilot's band schedule: from `start` (UTC, `HH:MM`) until the next
/// window's start, patrol `bands` in order on `mode`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AutopilotWindow {
    /// UTC start, `HH:MM`.
    pub start: String,
    /// Bands to patrol, in order (`"20m"`, `"17m"`, …).
    pub bands: Vec<String>,
    /// `"FT8"`, `"FT4"` or `"FT2"`, or several of them joined by `+` (`"FT8+FT4"`): every band is
    /// then worked in each of them.
    pub mode: String,
}

impl AutopilotWindow {
    /// The window's modes, in order.
    pub fn modes(&self) -> impl Iterator<Item = &str> {
        self.mode
            .split('+')
            .map(str::trim)
            .filter(|m| !m.is_empty())
    }
}

/// A band in a mode: one stop of the autopilot's patrol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    pub band: String,
    pub mode: String,
}

impl Stop {
    pub fn is(&self, band: &str, mode: &str) -> bool {
        self.band.eq_ignore_ascii_case(band) && self.mode.eq_ignore_ascii_case(mode)
    }
}

/// The patrol in force: every band in every mode the running windows give it, band by band in the
/// schedule's order. Windows with the same start run together (operator, 2026-11-24: FT8 and FT4
/// "in the same time switching between bands and modes"). Never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patrol(Vec<Stop>);

impl Patrol {
    pub fn stops(&self) -> &[Stop] {
        &self.0
    }
    pub fn has(&self, band: &str, mode: &str) -> bool {
        self.0.iter().any(|s| s.is(band, mode))
    }
    pub fn has_mode(&self, mode: &str) -> bool {
        self.0.iter().any(|s| s.mode.eq_ignore_ascii_case(mode))
    }
    pub fn many_modes(&self) -> bool {
        self.0.iter().any(|s| s.mode != self.0[0].mode)
    }
    /// The stops in order from the one after `band` in `mode`: all of them, that one last, from the
    /// first when it is not on the patrol.
    pub fn after(&self, band: &str, mode: &str) -> Vec<&Stop> {
        let at = self.0.iter().position(|s| s.is(band, mode));
        let from = at.map_or(0, |i| i + 1);
        (0..self.0.len())
            .map(|i| &self.0[(from + i) % self.0.len()])
            .collect()
    }
}

impl Default for AutopilotWindow {
    fn default() -> Self {
        Self {
            start: "00:00".into(),
            bands: vec!["20m".into()],
            mode: "FT8".into(),
        }
    }
}

/// Which target tiers the autopilot may call. Callers to the operator are always answered while
/// Answer callers is on; Worked and Confirmed are never called.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AutopilotTiers {
    /// DS ATNO: an entity never worked.
    pub atno: bool,
    /// A station on the DXpedition list, on a band or mode still needed.
    pub dxpedition: bool,
    /// DXCC worked but not confirmed on this band or mode (W/New …).
    pub dxcc_unconfirmed: bool,
    /// DXCC confirmed elsewhere, new on this band or mode (C/New …).
    pub dxcc_new_slot: bool,
    /// A US state not yet worked on this band (WAS), the need board's NewState. Off by default:
    /// from Europe it would otherwise call every US station on a band. Narrow it further with
    /// [`AutomationSettings::wanted_states`].
    pub us_state: bool,
    /// A new POTA park, IOTA, SOTA or WWFF reference.
    pub reference: bool,
    /// A call new on this band and mode (CS New Call, New Band, New Mode, New Band+Mode).
    pub new_call: bool,
    /// An entity on Club Log's Most Wanted top 50, not yet confirmed on this band.
    pub most_wanted: bool,
    /// A Maidenhead grid never worked.
    pub grid: bool,
}

impl Default for AutopilotTiers {
    fn default() -> Self {
        Self {
            atno: true,
            dxpedition: true,
            dxcc_unconfirmed: true,
            dxcc_new_slot: true,
            us_state: false,
            reference: true,
            new_call: true,
            most_wanted: true,
            grid: true,
        }
    }
}

/// The rules in the order the autopilot weighs them by default (operator, 2026-11-22: new DXCC,
/// the Club Log top 50 "are a must", DXpeditions, unconfirmed DXCC, unconfirmed band slots, US
/// states, POTA/SOTA/IOTA, grids, then any new call). A station calling the operator is always
/// first and is not in the list.
pub const DEFAULT_PRIORITY: [Tier; 9] = [
    Tier::Atno,
    Tier::MostWanted,
    Tier::Dxpedition,
    Tier::DxccUnconfirmed,
    Tier::DxccNewSlot,
    Tier::UsState,
    Tier::Reference,
    Tier::Grid,
    Tier::NewCall,
];

/// Club Log's DXCC Most Wanted list, ranks 1–50, as ARRL DXCC entity numbers (fetched from
/// `clublog.org/mostwanted.php?api=1` on 2026-11-22). The list moves slowly; refresh it with a
/// release when it does.
pub const CLUBLOG_TOP50: [u32; 50] = [
    344, 123, 138, 217, 131, 199, 506, 17, 505, 174, 280, 253, 153, 384, 240, 16, 133, 276, 512,
    204, 111, 235, 20, 171, 161, 182, 124, 10, 201, 177, 37, 273, 297, 180, 309, 270, 303, 195, 41,
    247, 238, 99, 33, 43, 197, 24, 51, 118, 490, 507,
];

/// The priority list as read from settings: every name this build knows, in order; anything else
/// (a newer build's rule, a typo, a non-string) is dropped, never an error.
fn de_priority<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Tier>, D::Error> {
    let raw = Vec::<serde_json::Value>::deserialize(d).unwrap_or_default();
    Ok(raw
        .into_iter()
        .filter_map(|v| serde_json::from_value::<Tier>(v).ok())
        .collect())
}

/// The operator's order made whole: a station calling the operator never in it (it is always
/// first), each rule once, and any rule missing from it added where [`DEFAULT_PRIORITY`] has it
/// relative to the rest — after the rules that precede it there.
pub fn sanitized_priority(p: &[Tier]) -> Vec<Tier> {
    let mut out: Vec<Tier> = Vec::new();
    for t in p {
        if *t != Tier::CallingMe && !out.contains(t) {
            out.push(*t);
        }
    }
    for (i, t) in DEFAULT_PRIORITY.iter().enumerate() {
        if out.contains(t) {
            continue;
        }
        // After the last rule that precedes it in the default order, or first.
        let at = DEFAULT_PRIORITY[..i]
            .iter()
            .filter_map(|b| out.iter().position(|x| x == b))
            .max()
            .map_or(0, |p| p + 1);
        out.insert(at, *t);
    }
    out
}

/// [`AutomationSettings::rank`] over a bare list.
pub fn rank_in(priority: &[Tier], t: Tier) -> usize {
    if t == Tier::CallingMe {
        return 0;
    }
    priority
        .iter()
        .position(|x| *x == t)
        .map_or(priority.len() + 1, |i| i + 1)
}

/// Is this DXCC number on the Club Log Most Wanted top 50?
pub fn is_most_wanted(dxcc: u32) -> bool {
    CLUBLOG_TOP50.contains(&dxcc)
}

/// Everything the operator configures for Answer callers and the autopilot (Settings ▸ Automation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AutomationSettings {
    /// Answer stations that call you while you work, or have just finished, a station you called.
    pub answer_callers: bool,
    /// Slots our first call may go unanswered before a station calling us takes over. 0 = never.
    pub answer_takeover_slots: u32,
    /// Callers answered in a row before transmit goes off as it does today.
    pub answer_chain_limit: u32,
    /// Show and allow the autopilot.
    pub autopilot_enabled: bool,
    /// Minutes between presence checks, 2–999 ([`Self::presence_minutes`] clamps).
    pub presence_check_min: u32,
    /// The band schedule.
    pub schedule: Vec<AutopilotWindow>,
    /// Slots to listen on a band before calling anyone.
    pub listen_slots: u32,
    /// Least time on a band before moving on, seconds.
    pub min_dwell_secs: u32,
    /// Time with nothing worth calling before moving on, seconds.
    pub quiet_secs: u32,
    /// Time on a band that offers only ordinary new calls (nothing needed, no DXCC slot, no
    /// DXpedition) before moving on to look for better ones, seconds.
    pub ordinary_dwell_secs: u32,
    /// Time budget for an ordinary target, seconds.
    pub target_budget_secs: u32,
    /// Calls a station may leave unanswered before the autopilot moves on (operator, 2026-11-23:
    /// "just 4 calls, no answer move on … need it, call again later"). An ATNO, DXpedition or
    /// top-50 station has no such limit, only its own time budget.
    pub target_max_calls: u32,
    /// How long a DXpedition, ATNO or top-50 station is called, and a hunted DXpedition followed from
    /// the moment it or its callers are heard, minutes (operator, 2026-11-25: ten by default, "but
    /// user can change this").
    pub dxped_call_mins: u32,
    /// Most contacts the autopilot starts in an hour.
    pub max_qso_per_hour: u32,
    /// Which tiers may be called.
    pub tiers: AutopilotTiers,
    /// Move to another scheduled band for a needed spot (receive first, call only once decoded).
    pub chase_spots: bool,
    /// The US states to chase (ADIF `STATE` codes, `"VT"`), when only some are wanted. Empty =
    /// every state still needed on the band. A listed state is still called only while it is
    /// needed there.
    pub wanted_states: Vec<String>,
    /// Hunt a needed DXpedition spotted on ANY band the licence allows — not only the scheduled
    /// ones — on the frequency the spot names, then return to where the autopilot was. Waits, like
    /// every band change, until every contact is finished and nobody is calling.
    pub hunt_dxpeditions: bool,
    /// While hunting, switch to Hound when the DXpedition is heard running Fox/Hound (transmitting
    /// below 1000 Hz), and back when the hunt ends.
    pub hunt_hound: bool,
    /// Pause the autopilot and stop transmit when the OM0RX Station Agent on this PC reports a tuner
    /// (KAT500) or amplifier (KPA500) fault ([`crate::station_guard`]).
    pub station_guard: bool,
    /// Then clear the fault, tune and align through the Agent, and only then transmit again. The tune
    /// transmits by itself, so this is off until the operator switches it on.
    pub station_recover: bool,
    /// The rules, strongest first: the operator's own order ([`DEFAULT_PRIORITY`] until changed).
    /// Each rule is switched on or off in [`Self::tiers`]. A name this build does not know (a
    /// newer build's rule) is skipped rather than failing the whole settings file.
    #[serde(deserialize_with = "de_priority")]
    pub priority: Vec<Tier>,
    /// Within a rule, call a station outside Europe before a European one (operator, 2026-11-22:
    /// "always prioritising DX from outside of Europe unless the EU station is an all-time new
    /// one" — an ATNO is its own, stronger rule).
    pub prefer_outside_eu: bool,
    /// The continents not called ([`CONTINENTS`] codes); empty = every continent. An ATNO is
    /// called wherever it is, and a station calling the operator is always answered.
    pub skip_continents: Vec<String>,
    /// Call each station on a clear transmit frequency near the operator's own ([`clear_tx_hz`];
    /// operator, 2026-11-24). Off: always on the operator's own.
    pub clear_tx_offset: bool,
}

/// The continents a station can be skipped by: cty.dat's codes.
pub const CONTINENTS: [&str; 7] = ["EU", "AS", "AF", "NA", "SA", "OC", "AN"];

/// Does the operator call stations on this continent? One that is not known is called.
pub fn continent_wanted(continent: Option<&str>, skip: &[String]) -> bool {
    continent.is_none_or(|c| !skip.iter().any(|s| s.eq_ignore_ascii_case(c)))
}

impl Default for AutomationSettings {
    fn default() -> Self {
        Self {
            answer_callers: false,
            answer_takeover_slots: 2,
            answer_chain_limit: 3,
            autopilot_enabled: false,
            presence_check_min: DEFAULT_PRESENCE_MIN,
            // OM0RX's WSJT-Z band hopper: low bands overnight, high bands by day.
            schedule: vec![
                AutopilotWindow {
                    start: "07:00".into(),
                    bands: ["20m", "17m", "15m", "12m", "10m"]
                        .map(String::from)
                        .to_vec(),
                    mode: "FT8".into(),
                },
                AutopilotWindow {
                    start: "20:00".into(),
                    bands: ["160m", "80m", "40m", "30m", "20m"]
                        .map(String::from)
                        .to_vec(),
                    mode: "FT8".into(),
                },
            ],
            listen_slots: 2,
            min_dwell_secs: 180,
            quiet_secs: 300,
            ordinary_dwell_secs: 600,
            target_budget_secs: 120,
            dxped_call_mins: 10,
            max_qso_per_hour: 60,
            tiers: AutopilotTiers::default(),
            chase_spots: true,
            wanted_states: Vec::new(),
            hunt_dxpeditions: true,
            hunt_hound: true,
            station_guard: true,
            station_recover: false,
            priority: DEFAULT_PRIORITY.to_vec(),
            prefer_outside_eu: true,
            target_max_calls: 4,
            skip_continents: Vec::new(),
            clear_tx_offset: true,
        }
    }
}

impl AutomationSettings {
    /// [`Self::dxped_call_mins`] in seconds.
    pub fn dxped_call_secs(&self) -> u32 {
        self.dxped_call_mins.saturating_mul(60)
    }

    /// The presence-check interval the operator set, held to 2–999 minutes.
    pub fn presence_minutes(&self) -> u32 {
        self.presence_check_min
            .clamp(*PRESENCE_MIN_RANGE.start(), *PRESENCE_MIN_RANGE.end())
    }

    /// Clamp every number to a range that cannot wedge the autopilot (a form or a hand edit can
    /// write anything). Called on load and on every save.
    pub fn sanitized(mut self) -> Self {
        self.presence_check_min = self.presence_minutes();
        self.answer_takeover_slots = self.answer_takeover_slots.min(20);
        self.answer_chain_limit = self.answer_chain_limit.clamp(1, 20);
        self.listen_slots = self.listen_slots.clamp(1, 20);
        self.min_dwell_secs = self.min_dwell_secs.clamp(30, 3600);
        self.quiet_secs = self.quiet_secs.clamp(60, 7200);
        self.ordinary_dwell_secs = self.ordinary_dwell_secs.clamp(120, 7200);
        self.target_budget_secs = self.target_budget_secs.clamp(30, 1800);
        self.target_max_calls = self.target_max_calls.clamp(1, 30);
        self.skip_continents = CONTINENTS
            .iter()
            .filter(|k| {
                self.skip_continents
                    .iter()
                    .any(|s| s.trim().eq_ignore_ascii_case(k))
            })
            .map(|k| (*k).to_string())
            .collect();
        self.dxped_call_mins = self.dxped_call_mins.clamp(1, 180);
        self.max_qso_per_hour = self.max_qso_per_hour.clamp(1, 300);
        self.schedule
            .retain(|w| parse_hhmm(&w.start).is_some() && w.modes().any(is_auto_mode));
        for w in &mut self.schedule {
            w.bands.retain(|b| !b.trim().is_empty());
            let modes: Vec<String> = w
                .modes()
                .filter(|m| is_auto_mode(m))
                .map(str::to_uppercase)
                .collect();
            w.mode = modes.join("+");
        }
        self.schedule.retain(|w| !w.bands.is_empty());
        self.wanted_states = self
            .wanted_states
            .iter()
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();
        self.wanted_states.sort();
        self.wanted_states.dedup();
        self.priority = sanitized_priority(&self.priority);
        self
    }

    /// Where `t` stands in the operator's order: 0 for a station calling the operator, then 1, 2…
    /// down the list. Lower wins.
    pub fn rank(&self, t: Tier) -> usize {
        rank_in(&self.priority, t)
    }

    /// Is `state` one the operator chases (every state when the list is empty)?
    pub fn wants_state(&self, state: Option<&str>) -> bool {
        self.wanted_states.is_empty()
            || state.is_some_and(|s| {
                self.wanted_states
                    .iter()
                    .any(|w| w.trim().eq_ignore_ascii_case(s.trim()))
            })
    }
}

/// A mode the autopilot can run.
pub fn is_auto_mode(mode: &str) -> bool {
    matches!(mode.trim().to_uppercase().as_str(), "FT8" | "FT4" | "FT2")
}

/// `HH:MM` → minutes after midnight UTC.
pub fn parse_hhmm(s: &str) -> Option<u32> {
    let (h, m) = s.trim().split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// The window in force at `minute` (0–1439 UTC): the one whose start is the latest at or before
/// it, wrapping past midnight to the day's last window.
pub fn window_at(schedule: &[AutopilotWindow], minute: u32) -> Option<&AutopilotWindow> {
    let mut timed: Vec<(u32, &AutopilotWindow)> = schedule
        .iter()
        .filter_map(|w| parse_hhmm(&w.start).map(|m| (m, w)))
        .collect();
    timed.sort_by_key(|(m, _)| *m);
    timed
        .iter()
        .rev()
        .find(|(m, _)| *m <= minute)
        .or_else(|| timed.last())
        .map(|(_, w)| *w)
}

/// The patrol at `minute`: the windows that share the start [`window_at`] chooses.
pub fn patrol_at(schedule: &[AutopilotWindow], minute: u32) -> Option<Patrol> {
    let start = parse_hhmm(&window_at(schedule, minute)?.start);
    let running: Vec<&AutopilotWindow> = schedule
        .iter()
        .filter(|w| parse_hhmm(&w.start) == start)
        .collect();
    let mut bands: Vec<&str> = Vec::new();
    for b in running.iter().flat_map(|w| &w.bands) {
        if !bands.iter().any(|have| have.eq_ignore_ascii_case(b)) {
            bands.push(b);
        }
    }
    let mut stops: Vec<Stop> = Vec::new();
    for band in bands {
        for w in &running {
            if !w.bands.iter().any(|b| b.eq_ignore_ascii_case(band)) {
                continue;
            }
            for mode in w.modes().filter(|m| is_auto_mode(m)) {
                if !stops.iter().any(|s| s.is(band, mode)) {
                    stops.push(Stop {
                        band: band.to_string(),
                        mode: mode.to_uppercase(),
                    });
                }
            }
        }
    }
    (!stops.is_empty()).then_some(Patrol(stops))
}

/// The start (`HH:MM`) of the next window after `minute`, wrapping to tomorrow's first; `None` with
/// fewer than two windows (nothing ever changes).
pub fn next_window_start(schedule: &[AutopilotWindow], minute: u32) -> Option<String> {
    let mut starts: Vec<u32> = schedule
        .iter()
        .filter_map(|w| parse_hhmm(&w.start))
        .collect();
    starts.sort_unstable();
    starts.dedup();
    if starts.len() < 2 {
        return None;
    }
    let next = starts
        .iter()
        .find(|m| **m > minute)
        .or(starts.first())
        .copied()?;
    Some(format!("{:02}:{:02}", next / 60, next % 60))
}

/// The target rules ("tiers"). Their strength is the operator's order
/// ([`AutomationSettings::priority`], [`AutomationSettings::rank`]), never this declaration's: the
/// derived `Ord` is only the default order and must not be used to rank a pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    /// A station calling the operator.
    CallingMe = 1,
    /// An entity never worked.
    Atno = 2,
    /// An entity on Club Log's Most Wanted top 50, not confirmed on this band.
    MostWanted = 3,
    /// A listed DXpedition, needed here.
    Dxpedition = 4,
    /// DXCC never confirmed: new here, or worked here and still unconfirmed.
    DxccUnconfirmed = 5,
    /// DXCC confirmed elsewhere, new on this band or mode, or worked here and not confirmed here.
    DxccNewSlot = 6,
    /// A US state needed on this band (WAS), and wanted.
    UsState = 7,
    /// A new park, island, summit or WWFF reference.
    Reference = 8,
    /// A Maidenhead grid never worked.
    Grid = 9,
    /// A call new on this band and mode: the ordinary call.
    NewCall = 10,
}

/// One heard station the autopilot could call: what a decode row says about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub call: String,
    /// The decoded line (what a double-click would answer).
    pub message: String,
    pub snr: i32,
    pub freq_hz: f32,
    /// The slot it was decoded in.
    pub slot: u64,
    /// CS on this band and mode (`"New Call"`, `"Worked"`, …), or `None` when unknown.
    pub call_status: Option<String>,
    /// DS on this band and mode (`"ATNO"`, `"W/New Band"`, …), or `None`.
    pub dxcc_status: Option<String>,
    /// A plain or acceptable directed CQ.
    pub is_cq: bool,
    /// Ending a contact with somebody else (`RR73`, `RRR` or `73`): free for the next call. Only
    /// a needed station is called on that ([`is_needed`]); anybody else is called on a CQ.
    pub ending: bool,
    /// Addressed to the operator.
    pub to_me: bool,
    /// On the DXpedition list.
    pub dxpedition: bool,
    /// Activating a reference (park, island, summit) not yet worked.
    pub new_reference: bool,
    /// In a US state needed on this band that the operator wants
    /// ([`AutomationSettings::wants_state`] already applied).
    pub new_state: bool,
    pub lotw: bool,
    /// Great-circle distance from the operator's grid to the grid the station sent, when both
    /// are known.
    pub distance_km: Option<f64>,
    /// The continent code of the station's DXCC entity (`"EU"`, `"NA"`, …), when known.
    pub continent: Option<String>,
    /// The entity is on the Club Log Most Wanted top 50.
    pub most_wanted: bool,
    /// The grid it sent was never worked.
    pub new_grid: bool,
    /// The entity is worked on this band but the slot is not confirmed (it may be confirmed
    /// elsewhere): another contact there can still confirm it.
    pub slot_unconfirmed: bool,
}

/// From this far away a station counts as DX and is called before a nearer one of the same tier
/// (operator, 2026-11-22: "it really needs to prioritize DX stations"). 3000 km takes a European
/// station outside Europe, and a North American one off its own coast.
pub const DX_KM: f64 = 3000.0;

/// Is the candidate DX ([`DX_KM`] or more away)?
pub fn is_dx(c: &Candidate) -> bool {
    c.distance_km.is_some_and(|d| d >= DX_KM)
}

/// Is the candidate "far" for ranking within a rule: outside Europe by its entity's continent when
/// that is known and asked for, else [`is_dx`] by distance.
pub fn is_far(c: &Candidate, prefer_outside_eu: bool) -> bool {
    match (&c.continent, prefer_outside_eu) {
        (Some(k), true) => !k.eq_ignore_ascii_case("EU"),
        _ => is_dx(c),
    }
}

/// The centre of a 4- or 6-character Maidenhead locator, `(lat, lon)` in degrees.
pub fn grid_latlon(grid: &str) -> Option<(f64, f64)> {
    let g: Vec<char> = grid.trim().to_uppercase().chars().collect();
    if g.len() < 4 {
        return None;
    }
    let field = |c: char| (('A'..='R').contains(&c)).then(|| f64::from(c as u8 - b'A'));
    let digit = |c: char| c.to_digit(10).map(f64::from);
    let mut lon = field(g[0])? * 20.0 - 180.0 + digit(g[2])? * 2.0;
    let mut lat = field(g[1])? * 10.0 - 90.0 + digit(g[3])?;
    let sub = |c: char| (('A'..='X').contains(&c)).then(|| f64::from(c as u8 - b'A'));
    match (
        g.get(4).copied().and_then(sub),
        g.get(5).copied().and_then(sub),
    ) {
        (Some(x), Some(y)) if g.len() >= 6 => {
            lon += x * (2.0 / 24.0) + 1.0 / 24.0;
            lat += y * (1.0 / 24.0) + 0.5 / 24.0;
        }
        _ => {
            lon += 1.0;
            lat += 0.5;
        }
    }
    Some((lat, lon))
}

/// Great-circle distance between two locators, km.
pub fn grid_km(a: &str, b: &str) -> Option<f64> {
    let (la1, lo1) = grid_latlon(a)?;
    let (la2, lo2) = grid_latlon(b)?;
    let (p1, p2) = (la1.to_radians(), la2.to_radians());
    let dp = p2 - p1;
    let dl = (lo2 - lo1).to_radians();
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    Some(2.0 * 6371.0 * h.sqrt().min(1.0).asin())
}

/// Every tier a candidate falls in, strongest first (empty when it is not worth calling: Worked,
/// Confirmed, or nothing known about it). A station can be in several — an ATNO in a needed
/// state — so a switched-off tier never hides one that is switched on ([`best_allowed_tier`]).
pub fn tiers_of(c: &Candidate) -> Vec<Tier> {
    let mut out = Vec::new();
    if c.to_me {
        out.push(Tier::CallingMe);
    }
    let ds = c.dxcc_status.as_deref().unwrap_or("");
    let cs = c.call_status.as_deref().unwrap_or("");
    let call_new = matches!(cs, "New Call" | "New Band" | "New Mode" | "New Band+Mode");
    if ds == "ATNO" {
        out.push(Tier::Atno);
    }
    // A top-50 entity not confirmed on this band: a new contact can still confirm it.
    if c.most_wanted && ds != "ATNO" && ds != "Confirmed" && call_new {
        out.push(Tier::MostWanted);
    }
    if c.dxpedition && call_new {
        out.push(Tier::Dxpedition);
    }
    // Never confirmed anywhere: new on this band or mode (W/…), or worked here already ("Worked")
    // with a new call that may confirm it.
    if ds.starts_with("W/") || (ds == "Worked" && call_new) {
        out.push(Tier::DxccUnconfirmed);
    }
    // Confirmed elsewhere: new on this band or mode (C/…), or worked here and not confirmed here.
    if ds.starts_with("C/") || (c.slot_unconfirmed && call_new) {
        out.push(Tier::DxccNewSlot);
    }
    if c.new_state {
        out.push(Tier::UsState);
    }
    if c.new_reference && call_new {
        out.push(Tier::Reference);
    }
    if c.new_grid && call_new {
        out.push(Tier::Grid);
    }
    if call_new {
        out.push(Tier::NewCall);
    }
    out
}

/// The strongest tier a candidate falls in by the default order, or `None` when it is not worth
/// calling.
pub fn tier_of(c: &Candidate) -> Option<Tier> {
    let mut all = tiers_of(c);
    all.sort_by_key(|t| rank_in(&DEFAULT_PRIORITY, *t));
    all.into_iter().next()
}

/// The strongest tier a candidate falls in that the operator switched on, by their order.
pub fn best_allowed_tier(c: &Candidate, s: &AutomationSettings) -> Option<Tier> {
    tiers_of(c)
        .into_iter()
        .filter(|t| tier_allowed(*t, &s.tiers))
        .min_by_key(|t| s.rank(*t))
}

/// Is this tier switched on? Callers to the operator are governed by Answer callers instead.
pub fn tier_allowed(t: Tier, tiers: &AutopilotTiers) -> bool {
    match t {
        Tier::CallingMe => true,
        Tier::Atno => tiers.atno,
        Tier::Dxpedition => tiers.dxpedition,
        Tier::DxccUnconfirmed => tiers.dxcc_unconfirmed,
        Tier::DxccNewSlot => tiers.dxcc_new_slot,
        Tier::UsState => tiers.us_state,
        Tier::Reference => tiers.reference,
        Tier::NewCall => tiers.new_call,
        Tier::MostWanted => tiers.most_wanted,
        Tier::Grid => tiers.grid,
    }
}

/// A tier worth calling a station for that calls no CQ, and worth calling it again: anything but a
/// grid, an ordinary new call and a caller to us.
pub fn is_needed(t: Tier) -> bool {
    !matches!(t, Tier::Grid | Tier::NewCall | Tier::CallingMe)
}

/// An ordinary call, which holds neither the band nor a contact: a new call, or only a new grid.
pub fn is_ordinary(t: Tier) -> bool {
    matches!(t, Tier::Grid | Tier::NewCall)
}

/// A rare target gets the long time budget, and is called from 1200 Hz above.
pub fn is_rare(t: Tier) -> bool {
    matches!(t, Tier::Atno | Tier::Dxpedition | Tier::MostWanted)
}

/// Which CQs the autopilot answers: a plain CQ, and the directed CQs a European station may answer
/// (activity programmes and `EU`). Any other directed CQ (`CQ NA`, `CQ JA`, `CQ DX` from a
/// European) is left alone — answering it would be calling a station that asked for someone else.
pub fn answerable_cq(dir: &str) -> bool {
    matches!(
        dir.trim().to_uppercase().as_str(),
        "" | "EU" | "POTA" | "SOTA" | "WWFF" | "IOTA" | "TEST" | "FD"
    )
}

/// What the autopilot remembers about a call it tried and did not work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Attempts {
    pub failures: u32,
    /// Rounds of calling it again that went unanswered since ([`call_again_after_secs`]).
    pub again: u32,
    /// When the last attempt ended, Unix seconds.
    pub last_unix: u64,
}

/// After `again` unanswered rounds of calling a needed station again, how long until the next: one
/// minute, doubled each time, half an hour at most (live run 2026-11-26: VP2MAA, -18 dB on 160 m,
/// had five rounds of four calls in a quarter of an hour), seconds.
pub fn call_again_after_secs(again: u32) -> u64 {
    (60u64 << again.min(5)).min(1800)
}

/// After `failures` failed attempts, how long before the call may be tried again (10, 30, then
/// 60 minutes), or `None` once it has failed three times and is left for the session.
pub fn retry_after_secs(failures: u32) -> Option<u64> {
    match failures {
        0 => Some(0),
        1 => Some(600),
        2 => Some(1800),
        3 => Some(3600),
        _ => None,
    }
}

/// May this call be tried now?
pub fn may_retry(a: Option<&Attempts>, now_unix: u64) -> bool {
    match a {
        None => true,
        Some(a) => retry_after_secs(a.failures)
            .is_some_and(|wait| now_unix >= a.last_unix.saturating_add(wait)),
    }
}

/// The best candidate: strongest tier first, then DX before a nearer station, then signal, then
/// LoTW, then fewest failures, then the call (so the pick is stable). Only candidates in an allowed tier that may be tried now, and not
/// skipped by `skip`, are considered.
pub fn pick<'a>(
    candidates: &'a [Candidate],
    s: &AutomationSettings,
    attempts: &std::collections::HashMap<String, Attempts>,
    now_unix: u64,
    skip: impl Fn(&Candidate) -> bool,
) -> Option<(&'a Candidate, Tier)> {
    let far = |c: &Candidate| is_far(c, s.prefer_outside_eu);
    candidates
        .iter()
        .filter(|c| c.to_me || c.is_cq || c.ending)
        .filter(|c| !skip(c))
        .filter_map(|c| best_allowed_tier(c, s).map(|t| (c, t)))
        // One that only ended its contact: called when it is needed.
        .filter(|(c, t)| c.to_me || c.is_cq || is_needed(*t))
        // A station on a skipped continent: only when it calls us, or is an ATNO.
        .filter(|(c, t)| {
            *t == Tier::CallingMe
                || (s.tiers.atno && tiers_of(c).contains(&Tier::Atno))
                || continent_wanted(c.continent.as_deref(), &s.skip_continents)
        })
        // One that calls us is answered now, however lately it left our calls unanswered.
        .filter(|(c, _)| c.to_me || may_retry(attempts.get(&c.call.to_uppercase()), now_unix))
        .min_by(|(a, ta), (b, tb)| {
            s.rank(*ta)
                .cmp(&s.rank(*tb))
                .then(far(b).cmp(&far(a)))
                .then(b.snr.cmp(&a.snr))
                .then(b.lotw.cmp(&a.lotw))
                .then(failures(attempts, &a.call).cmp(&failures(attempts, &b.call)))
                .then(a.call.cmp(&b.call))
        })
}

fn failures(attempts: &std::collections::HashMap<String, Attempts>, call: &str) -> u32 {
    attempts.get(&call.to_uppercase()).map_or(0, |a| a.failures)
}

/// The operator's presence clock: when they last confirmed, and whether a confirmation is due.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PresenceClock {
    /// Unix seconds of the last confirmation (or of turning the autopilot on).
    pub last_ack_unix: u64,
}

impl PresenceClock {
    pub fn ack(&mut self, now_unix: u64) {
        self.last_ack_unix = now_unix;
    }
    /// Seconds until the next confirmation is due (0 = due now).
    pub fn remaining_secs(&self, now_unix: u64, minutes: u32) -> u64 {
        let due = self.last_ack_unix.saturating_add(u64::from(minutes) * 60);
        due.saturating_sub(now_unix)
    }
    pub fn expired(&self, now_unix: u64, minutes: u32) -> bool {
        self.remaining_secs(now_unix, minutes) == 0
    }
}

/// Contacts started in the last hour, for the hourly cap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HourlyCount {
    starts: std::collections::VecDeque<u64>,
}

impl HourlyCount {
    pub fn note(&mut self, now_unix: u64) {
        self.starts.push_back(now_unix);
        self.trim(now_unix);
    }
    pub fn count(&mut self, now_unix: u64) -> usize {
        self.trim(now_unix);
        self.starts.len()
    }
    fn trim(&mut self, now_unix: u64) {
        while self
            .starts
            .front()
            .is_some_and(|t| now_unix.saturating_sub(*t) >= 3600)
        {
            self.starts.pop_front();
        }
    }
}

/// One station the UI's need feed knows about (cluster, RBN, POTA/SOTA/WWFF, DXpeditions): the
/// facts a decode cannot carry. Fed to the engine while the autopilot runs; data only — every
/// decision stays in the engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedEntry {
    pub call: String,
    pub band: String,
    /// The exact mode the spot names (`"FT8"`), when it names one.
    pub mode: Option<String>,
    pub freq_mhz: Option<f64>,
    pub dxpedition: bool,
    /// A park, summit, WWFF area or island not yet worked (NewPark / NewIota).
    pub new_reference: bool,
    /// An entity never worked (NewEntity).
    pub new_entity: bool,
    /// The entity is new on that band or in that mode (NewBand / NewMode).
    pub new_slot: bool,
    /// A US state not yet worked on that band (NewState).
    pub new_state: bool,
    /// The US state (ADIF `STATE`) the board placed the station in, when it placed it.
    pub state: Option<String>,
    /// When the spot came in, Unix seconds; 0 = not known.
    pub heard_unix: u64,
}

/// Could a station hold this call? Spots carry false decodes ("E34XQXUYE9H", "89EARO/J8Z5": both
/// chased for five minutes, 2026-11-23), and a garbled call reads as a new entity.
///
/// Wider than WSJT-X's standard call on purpose, since a dropped real one could be the ATNO: one
/// part of the call must be a prefix of one to three characters with a letter in it (`3DA`, `3X`),
/// one to four digits (`TM100VERDUN`), and one to six letters. `KH6XX/W0` and `F/OM0RX/P` pass on
/// the part that does; what stands beside it is a prefix or a suffix, at most four characters
/// ("5T4LS/85PD6", an "ATNO" chased 2026-11-24).
pub fn spot_call_ok(call: &str) -> bool {
    let call = call.trim().to_ascii_uppercase();
    let alnum = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric());
    let full = |p: &str| {
        let head = p.trim_end_matches(|c: char| c.is_ascii_alphabetic());
        let letters = p.len() - head.len();
        // The digits before the suffix; the prefix keeps any of its own (`9A3SWO`, `E51ABC`).
        let prefix = head.trim_end_matches(|c: char| c.is_ascii_digit());
        let digits = head.len() - prefix.len();
        (1..=6).contains(&letters)
            && (1..=4).contains(&digits)
            && (1..=3).contains(&prefix.len())
            && prefix.bytes().any(|b| b.is_ascii_alphabetic())
    };
    let part = |p: &str| alnum(p) && (full(p) || p.len() <= 4);
    call.len() <= 14 && call.split('/').all(part) && call.split('/').any(full)
}

/// The FT mode a cluster spot names: the skimmer's own mode, else the spotter's comment
/// ("WKED FT8 STRONG INTO CO"), else the standard FT8 or FT4 frequency it is on. `None` = another
/// mode, or no telling.
pub fn spot_ft_mode(freq_mhz: f64, comment: &str, skimmer: Option<&str>) -> Option<&'static str> {
    const FT: [&str; 3] = ["FT8", "FT4", "FT2"];
    const FT4_DIALS: [f64; 8] = [
        3.575, 7.0475, 10.140, 14.080, 18.104, 21.140, 24.919, 28.180,
    ];
    const FT8_DIALS: [f64; 10] = [
        1.840, 3.573, 5.357, 7.074, 10.136, 14.074, 18.100, 21.074, 24.915, 28.074,
    ];
    if let Some(m) = skimmer {
        return FT.into_iter().find(|f| m.eq_ignore_ascii_case(f));
    }
    let named = comment
        .split(|c: char| !c.is_ascii_alphanumeric())
        .find_map(|w| FT.into_iter().find(|f| w.eq_ignore_ascii_case(f)));
    if named.is_some() {
        return named;
    }
    // A spot names the dial, or the signal up to 3 kHz above it. FT4's 3.575 MHz is inside FT8's
    // window there, so its exact dial comes first.
    if FT4_DIALS.iter().any(|d| (freq_mhz - d).abs() < 0.0003) {
        return Some("FT4");
    }
    let above = |d: &f64| (-0.0003..0.003).contains(&(freq_mhz - d));
    if FT8_DIALS.iter().any(above) {
        return Some("FT8");
    }
    FT4_DIALS.iter().any(above).then_some("FT4")
}

/// The tier a spot would put its station in, for chasing it to another band; `None` = not worth
/// leaving the band for (only ATNO, DXpeditions and DXCC slots move the radio). `call_new` = the
/// station itself is not worked on that band in that mode class: a DXpedition is worth that,
/// whatever its entity has.
pub fn spot_tier(f: &FeedEntry, call_new: bool) -> Option<Tier> {
    if f.new_entity {
        Some(Tier::Atno)
    } else if f.dxpedition && (f.new_slot || call_new) {
        Some(Tier::Dxpedition)
    } else if f.new_slot {
        Some(Tier::DxccNewSlot)
    } else if f.new_state {
        Some(Tier::UsState)
    } else {
        None
    }
}

/// Is this spot a DXpedition worth leaving the band for: a listed DXpedition whose entity is needed
/// there (a new entity, or new on that band or mode), or that is itself not worked on that band in
/// that mode class (`call_new`; operator, 2026-11-25: VK9XY spotted on FT8 and FT4 "should be always
/// prioritised", Christmas Island confirmed there or not).
pub fn is_hunt_spot(f: &FeedEntry, call_new: bool) -> bool {
    f.dxpedition && (f.new_entity || f.new_slot || call_new)
}

/// The least spacing between a DXpedition's audio frequency and ours when the autopilot calls it,
/// Hz (operator, 2026-11-22: "never call on the DXpedition frequency, a minimum 1200 higher").
pub const DXPED_SPLIT_HZ: f32 = 1200.0;
/// A DXpedition heard above this audio frequency is retuned under it before it is called, Hz
/// (operator, 2026-11-22: "usually a DXpedition should be within 500 Hz … if not, the software
/// needs to retune to fit the DXpedition under 500").
pub const DXPED_LOW_HZ: f32 = 500.0;
/// Where such a retune puts the DXpedition's signal, Hz: under [`DXPED_LOW_HZ`] with room for its
/// 50 Hz width and a little drift.
pub const DXPED_PLACE_HZ: f32 = 400.0;
/// Where a DXpedition is called, Hz of our audio (operator, 2026-11-25: "the DX station should be
/// tuned in 0-500 Hz, and we should be calling in 1000-2500").
pub const DXPED_CALL_LOW_HZ: f32 = 1000.0;
pub const DXPED_CALL_TOP_HZ: f32 = 2500.0;
/// The highest audio offset on the DXpedition's own dial a call is made on: many receivers' filters
/// end there (operator, 2026-11-25: "we can't go above the filter bandwidth").
pub const DXPED_RECEIVER_TOP_HZ: f32 = 2700.0;

/// How far up the dial moves to put a DXpedition heard at `dx_hz` at [`DXPED_PLACE_HZ`], Hz; 0 = it is
/// under [`DXPED_LOW_HZ`] already, or sits so high that a call from [`DXPED_CALL_LOW_HZ`] would be
/// outside its receiver (live run 2026-11-25: 3B8/E70A at 2489 Hz).
pub fn dxped_shift_hz(dx_hz: f32) -> f32 {
    if dx_hz <= DXPED_LOW_HZ {
        return 0.0;
    }
    let shift = ((dx_hz - DXPED_PLACE_HZ) / 10.0).round() * 10.0;
    if DXPED_RECEIVER_TOP_HZ - shift >= DXPED_CALL_LOW_HZ {
        shift
    } else {
        0.0
    }
}

/// The audio offsets a DXpedition now heard at `dx_hz` is called from, with our dial `shift_hz` above
/// its own: [`DXPED_CALL_LOW_HZ`] to [`DXPED_CALL_TOP_HZ`], at least 500 Hz above it and inside its
/// receiver; where none of that is left, from at least [`DXPED_SPLIT_HZ`] below it.
pub fn dxped_range(dx_hz: f32, shift_hz: f32) -> (f32, f32) {
    let low = DXPED_CALL_LOW_HZ.max(dx_hz + DXPED_LOW_HZ);
    let top = DXPED_CALL_TOP_HZ.min(DXPED_RECEIVER_TOP_HZ - shift_hz);
    if low <= top {
        (low, top)
    } else {
        (CLEAR_LOW_HZ, (dx_hz - DXPED_SPLIT_HZ).max(CLEAR_LOW_HZ))
    }
}

/// Where to transmit when calling it, from `tx_hz`: the current offset when it is in that range and
/// [`DXPED_SPLIT_HZ`] away, else that far away, as near as the range allows.
pub fn dxped_call_hz(dx_hz: f32, shift_hz: f32, tx_hz: f32) -> f32 {
    let (low, top) = dxped_range(dx_hz, shift_hz);
    if (low..=top).contains(&tx_hz) && (tx_hz - dx_hz).abs() >= DXPED_SPLIT_HZ {
        tx_hz
    } else if low > dx_hz {
        (dx_hz + DXPED_SPLIT_HZ).clamp(low, top)
    } else {
        top
    }
}

/// The transmit offsets the autopilot moves to for a clear frequency, Hz: inside the receive filter
/// of any radio.
pub const CLEAR_LOW_HZ: f32 = 400.0;
pub const CLEAR_TOP_HZ: f32 = 2600.0;
/// How many periods back a signal still counts as being there: the autopilot hears nothing in its
/// own period while it calls, so the last few calls' worth.
pub const CLEAR_LOOKBACK_SLOTS: u64 = 12;

/// A transmit offset clear of the signals at `used` (their lowest tones, Hz), each `width_hz` wide:
/// `want` itself when nothing overlaps it, else the nearest clear one between `low` and `top`. With
/// no clear one, the one whose nearest signal is farthest away (the nearest to `want` of equals).
pub fn clear_tx_hz(want: f32, used: &[f32], width_hz: f32, low: f32, top: f32) -> f32 {
    let gap = |hz: f32| {
        used.iter()
            .map(|u| (hz - u).abs())
            .fold(f32::INFINITY, f32::min)
    };
    if gap(want) >= width_hz || top < low {
        return want;
    }
    let steps = ((top - low) / 5.0) as usize;
    let mut spots: Vec<f32> = (0..=steps).map(|i| low + i as f32 * 5.0).collect();
    spots.sort_by(|a, b| (a - want).abs().total_cmp(&(b - want).abs()));
    if let Some(hz) = spots.iter().find(|hz| gap(**hz) >= width_hz) {
        return *hz;
    }
    let mut best = want;
    for hz in spots {
        if gap(hz) > gap(best) {
            best = hz;
        }
    }
    best
}

/// The dial to hunt a spot from. A spot inside the standard channel's passband (the channel dial
/// up to 3 kHz above it) keeps the standard dial; any other — a DXpedition on its own frequency —
/// gets the spot rounded down to the kHz, which puts its signal low in the passband whether the
/// spotter gave the dial (Fox/Hound announcements) or the signal itself.
pub fn hunt_dial(spot_mhz: f64, channel_dial_mhz: Option<f64>) -> f64 {
    if let Some(ch) = channel_dial_mhz {
        let up = spot_mhz - ch;
        if (0.0..0.003).contains(&up) {
            return ch;
        }
    }
    (spot_mhz * 1000.0 + 1e-6).floor() / 1000.0
}

/// Is a spot audible from `dial_mhz` (inside the 200–2900 Hz passband)?
pub fn in_passband(spot_mhz: f64, dial_mhz: f64) -> bool {
    let up = spot_mhz - dial_mhz;
    (0.0002..=0.0029).contains(&up) || (spot_mhz - dial_mhz).abs() < 1e-6
}

/// What the UI shows about Answer callers and the autopilot. Tokens only: the UI owns every word.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AutomationDto {
    /// Answer callers is switched on in Settings.
    pub answer_callers: bool,
    /// Stations calling the operator, newest last.
    pub callers: Vec<String>,
    /// Callers answered in a row so far.
    pub chain: u32,
    /// The autopilot is allowed (Settings ▸ Automation).
    pub autopilot_enabled: bool,
    /// The autopilot is running.
    pub autopilot_on: bool,
    /// `off`, `listening`, `calling`, `inQso`, `moving`, `paused`, `noSchedule`, `wrongMode`,
    /// `presenceDue`.
    pub status: String,
    /// One line on the last decision (data: calls, bands and tiers, joined by the engine).
    pub reason: String,
    pub band: Option<String>,
    pub mode: Option<String>,
    pub target: Option<String>,
    pub target_tier: Option<Tier>,
    /// Calls the target has left unanswered, and how many it may (absent: no target, it answered,
    /// or it is rare and has no limit).
    pub calls_made: Option<u32>,
    pub calls_max: Option<u32>,
    /// Seconds until the operator must confirm presence.
    pub presence_remaining_secs: u64,
    pub presence_minutes: u32,
    /// The next band change the schedule plans, `HH:MM` UTC.
    pub next_window: Option<String>,
    pub qsos_last_hour: u32,
    /// The recent decisions, newest last (`HH:MM:SS text`).
    pub log: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cand(call: &str, cs: Option<&str>, ds: Option<&str>, snr: i32) -> Candidate {
        Candidate {
            call: call.into(),
            message: format!("CQ {call} JN88"),
            snr,
            freq_hz: 1500.0,
            slot: 10,
            call_status: cs.map(String::from),
            dxcc_status: ds.map(String::from),
            is_cq: true,
            ending: false,
            to_me: false,
            dxpedition: false,
            new_reference: false,
            new_state: false,
            lotw: false,
            distance_km: None,
            continent: None,
            most_wanted: false,
            new_grid: false,
            slot_unconfirmed: false,
        }
    }

    /// Default settings with these rule switches.
    fn with(tiers: &AutopilotTiers) -> AutomationSettings {
        AutomationSettings {
            tiers: tiers.clone(),
            ..AutomationSettings::default()
        }
    }

    fn pick_call(list: &[Candidate], s: &AutomationSettings) -> Option<(String, Tier)> {
        pick(list, s, &HashMap::new(), 0, |_| false).map(|(c, t)| (c.call.clone(), t))
    }

    /// Operator, 2026-11-22: new DXCC first, the Club Log top 50 "a must", then DXpeditions,
    /// unconfirmed DXCC, unconfirmed band slots, US states, POTA/SOTA/IOTA, grids, then any new call.
    #[test]
    fn the_default_order_is_the_operators() {
        let s = AutomationSettings::default();
        let mut mw = cand("P5A", Some("New Call"), Some("C/New Band"), -20);
        mw.most_wanted = true;
        let slot = cand("ZS1A", Some("New Call"), Some("C/New Band"), 10);
        assert_eq!(
            pick_call(&[slot.clone(), mw.clone()], &s).unwrap(),
            ("P5A".into(), Tier::MostWanted),
            "a top-50 entity beats a plain new slot"
        );
        let mut grid = cand("DL1A", Some("New Call"), Some("Confirmed"), 20);
        grid.new_grid = true;
        let plain = cand("DL2A", Some("New Call"), Some("Confirmed"), 25);
        assert_eq!(
            pick_call(&[plain.clone(), grid.clone()], &s).unwrap(),
            ("DL1A".into(), Tier::Grid),
            "a new grid beats a plain new call"
        );
        assert_eq!(
            pick_call(&[grid, slot.clone()], &s).unwrap().1,
            Tier::DxccNewSlot,
            "a band slot beats a grid"
        );
        // An entity worked here but never confirmed: a new call there may confirm it.
        let unconf = cand("TR8A", Some("New Call"), Some("Worked"), -5);
        assert_eq!(
            pick_call(&[slot, unconf], &s).unwrap(),
            ("TR8A".into(), Tier::DxccUnconfirmed)
        );
        // Confirmed elsewhere, worked here but not confirmed here: the band-slot rule.
        let mut here = cand("JA1A", Some("New Call"), Some("Confirmed"), 0);
        here.slot_unconfirmed = true;
        assert_eq!(pick_call(&[plain, here], &s).unwrap().1, Tier::DxccNewSlot);
    }

    /// The operator's own order wins, and an order missing rules (an older file) is made whole.
    #[test]
    fn the_operator_can_reorder_the_rules() {
        let mut grid = cand("DL1A", Some("New Call"), Some("Confirmed"), 0);
        grid.new_grid = true;
        let slot = cand("ZS1A", Some("New Call"), Some("C/New Band"), 0);
        // The settings page writes the whole list: here with Grid moved to the top.
        let s = AutomationSettings {
            priority: std::iter::once(Tier::Grid)
                .chain(
                    DEFAULT_PRIORITY
                        .iter()
                        .copied()
                        .filter(|t| *t != Tier::Grid),
                )
                .collect(),
            ..AutomationSettings::default()
        }
        .sanitized();
        assert_eq!(s.priority[0], Tier::Grid);
        assert_eq!(
            pick_call(&[slot.clone(), grid.clone()], &s).unwrap().1,
            Tier::Grid
        );
        // CONTROL: the default order puts the slot first.
        let d = AutomationSettings::default();
        assert_eq!(pick_call(&[slot, grid], &d).unwrap().1, Tier::DxccNewSlot);
        // An older file, written before Most Wanted and Grid existed: both come in at their
        // default places, and the operator's own order of the rest is kept.
        let old = [
            Tier::Dxpedition,
            Tier::Atno,
            Tier::DxccUnconfirmed,
            Tier::DxccNewSlot,
            Tier::UsState,
            Tier::Reference,
            Tier::NewCall,
        ];
        assert_eq!(
            sanitized_priority(&old),
            vec![
                Tier::Dxpedition,
                Tier::Atno,
                Tier::MostWanted,
                Tier::DxccUnconfirmed,
                Tier::DxccNewSlot,
                Tier::UsState,
                Tier::Reference,
                Tier::Grid,
                Tier::NewCall,
            ]
        );
        // A station calling the operator is never in the list, and an unknown name is dropped.
        let raw: AutomationSettings =
            serde_json::from_str(r#"{"priority":["callingMe","grid","fromTheFuture","atno"]}"#)
                .unwrap();
        let p = raw.sanitized().priority;
        assert!(!p.contains(&Tier::CallingMe));
        assert_eq!(p.len(), DEFAULT_PRIORITY.len(), "{p:?}");
        let at = |t: Tier| p.iter().position(|x| *x == t);
        assert!(
            at(Tier::Grid) < at(Tier::Atno),
            "grid-before-ATNO is kept: {p:?}"
        );
    }

    /// Within a rule, a station outside Europe comes before a louder European one — by continent,
    /// whatever the distance; an ATNO in Europe is its own stronger rule.
    #[test]
    fn outside_europe_first_within_a_rule() {
        let mut eu = cand("OH0A", Some("New Call"), Some("Confirmed"), 20);
        eu.continent = Some("EU".into());
        let mut af = cand("CN8A", Some("New Call"), Some("Confirmed"), -20);
        af.continent = Some("AF".into());
        af.distance_km = Some(2200.0); // nearer than 3000 km, still outside Europe
        let s = AutomationSettings::default();
        assert_eq!(pick_call(&[eu.clone(), af.clone()], &s).unwrap().0, "CN8A");
        let off = AutomationSettings {
            prefer_outside_eu: false,
            ..AutomationSettings::default()
        };
        assert_eq!(
            pick_call(&[eu.clone(), af.clone()], &off).unwrap().0,
            "OH0A",
            "CONTROL"
        );
        let mut atno = cand("1A0A", Some("New Call"), Some("ATNO"), -24);
        atno.continent = Some("EU".into());
        assert_eq!(pick_call(&[af, atno], &s).unwrap().0, "1A0A");
    }

    #[test]
    fn dx_is_called_before_a_louder_nearby_station_of_the_same_tier() {
        let mut near = cand("SP1ZMO", Some("New Call"), Some("Confirmed"), 15);
        near.distance_km = grid_km("JN88", "JO73");
        let mut far = cand("PY2XX", Some("New Call"), Some("Confirmed"), -15);
        far.distance_km = grid_km("JN88", "GG66");
        let list = vec![near, far];
        let (c, _) = pick(
            &list,
            &AutomationSettings::default(),
            &HashMap::new(),
            0,
            |_| false,
        )
        .unwrap();
        assert_eq!(c.call, "PY2XX");
        // A stronger tier still wins over distance.
        let mut atno_near = cand("Z60A", Some("New Call"), Some("ATNO"), 0);
        atno_near.distance_km = Some(900.0);
        let list = vec![list[1].clone(), atno_near];
        let (c, _) = pick(
            &list,
            &AutomationSettings::default(),
            &HashMap::new(),
            0,
            |_| false,
        )
        .unwrap();
        assert_eq!(c.call, "Z60A");
    }

    #[test]
    fn a_dxpedition_sits_under_500_hz_and_is_called_from_1000_to_2500_in_its_receiver() {
        // Low already: the dial stays, and it is called 1200 Hz above, or where we are.
        assert_eq!(dxped_shift_hz(450.0), 0.0);
        assert_eq!(dxped_range(450.0, 0.0), (1000.0, 2500.0));
        assert_eq!(dxped_call_hz(450.0, 0.0, 450.0), 1650.0);
        assert_eq!(
            dxped_call_hz(450.0, 0.0, 2200.0),
            2200.0,
            "left where it is"
        );
        assert_eq!(dxped_call_hz(450.0, 0.0, 2700.0), 1650.0, "not above 2500");
        // Brought down, and called inside its own receiver (VK9XY on 10 m FT4, 2026-11-25).
        assert_eq!(dxped_shift_hz(1500.0), 1100.0);
        assert_eq!(dxped_call_hz(400.0, 1100.0, 1375.0), 1600.0);
        assert_eq!(dxped_shift_hz(1837.0), 1440.0);
        assert_eq!(dxped_range(397.0, 1440.0), (1000.0, 1260.0));
        assert_eq!(dxped_call_hz(397.0, 1440.0, 1375.0), 1260.0);
        // So high that no call from 1000 Hz is in its receiver (3B8/E70A): left there, from below.
        assert_eq!(dxped_shift_hz(2489.0), 0.0);
        assert_eq!(dxped_range(2489.0, 0.0), (400.0, 1289.0));
        assert_eq!(dxped_call_hz(2489.0, 0.0, 1375.0), 1289.0);
        assert_eq!(dxped_call_hz(2489.0, 0.0, 900.0), 900.0, "already there");
    }

    #[test]
    fn a_clear_transmit_offset_is_the_nearest_one_no_signal_overlaps() {
        let clear = |want, used: &[f32]| clear_tx_hz(want, used, 50.0, 400.0, 2600.0);
        assert_eq!(clear(1375.0, &[]), 1375.0);
        assert_eq!(
            clear(1375.0, &[1325.0, 1425.0]),
            1375.0,
            "just clear of both"
        );
        // DK2WH called JT1BZ on 1374 Hz (live run 2026-11-24): the nearest clear one.
        assert_eq!(clear(1375.0, &[1374.0]), 1425.0);
        assert_eq!(clear(1375.0, &[1374.0, 1430.0]), 1320.0);
        // Never outside the range, however near.
        assert_eq!(clear(420.0, &[430.0]), 480.0);
        // A full band: the widest gap.
        let full: Vec<f32> = (0..60)
            .map(|i| 380.0 + 40.0 * i as f32)
            .filter(|hz| *hz != 1780.0)
            .collect();
        assert_eq!(clear_tx_hz(1375.0, &full, 55.0, 400.0, 2600.0), 1780.0);
    }

    #[test]
    fn a_spots_ft_mode_comes_from_the_skimmer_the_comment_or_the_frequency() {
        // Spots of 24 November 2026.
        assert_eq!(
            spot_ft_mode(7.056, "WKED FT8 STRONG INTO CO", None),
            Some("FT8")
        );
        assert_eq!(
            spot_ft_mode(21.140, "FT4 +02dB from OH29 2115Hz", None),
            Some("FT4")
        );
        assert_eq!(spot_ft_mode(14.080, "", Some("FT4")), Some("FT4"));
        assert_eq!(spot_ft_mode(14.0252, "", Some("CW")), None);
        assert_eq!(spot_ft_mode(10.1362, "tnx", None), Some("FT8"));
        assert_eq!(spot_ft_mode(3.575, "", None), Some("FT4"));
        assert_eq!(spot_ft_mode(3.5742, "", None), Some("FT8"));
        assert_eq!(spot_ft_mode(21.1412, "", None), Some("FT4"));
        assert_eq!(spot_ft_mode(14.240, "59 in OM", None), None);
    }

    #[test]
    fn locator_distances_are_about_right() {
        // JN88 (Bratislava) to FN31 (Connecticut) is about 6900 km; JN88 to JO73 about 600.
        let d = grid_km("JN88", "FN31").unwrap();
        assert!((6500.0..7300.0).contains(&d), "{d}");
        let d = grid_km("JN88", "JO73").unwrap();
        assert!((400.0..800.0).contains(&d), "{d}");
        assert!(grid_km("JN88", "R73").is_none());
        assert!(grid_km("JN88", "ZZ00").is_none());
    }

    #[test]
    fn presence_interval_is_held_to_2_to_999_minutes() {
        let mut s = AutomationSettings::default();
        assert_eq!(s.presence_minutes(), 10);
        s.presence_check_min = 0;
        assert_eq!(s.presence_minutes(), 2);
        s.presence_check_min = 1;
        assert_eq!(s.presence_minutes(), 2);
        s.presence_check_min = 5000;
        assert_eq!(s.presence_minutes(), 999);
        s.presence_check_min = 45;
        assert_eq!(s.sanitized().presence_check_min, 45);
    }

    #[test]
    fn everything_is_off_by_default() {
        let s = AutomationSettings::default();
        assert!(!s.answer_callers);
        assert!(!s.autopilot_enabled);
    }

    #[test]
    fn a_settings_file_without_the_section_reads_the_defaults() {
        let s: AutomationSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(s, AutomationSettings::default());
        let s: AutomationSettings =
            serde_json::from_str(r#"{"presenceCheckMin":30,"answerCallers":true}"#).unwrap();
        assert_eq!(s.presence_check_min, 30);
        assert!(s.answer_callers);
        assert_eq!(s.answer_chain_limit, 3);
    }

    #[test]
    fn the_schedule_wraps_past_midnight() {
        let s = AutomationSettings::default().schedule;
        assert_eq!(
            window_at(&s, parse_hhmm("12:00").unwrap()).unwrap().start,
            "07:00"
        );
        assert_eq!(
            window_at(&s, parse_hhmm("23:30").unwrap()).unwrap().start,
            "20:00"
        );
        // Before the first window of the day, the last one (from yesterday) is still in force.
        assert_eq!(
            window_at(&s, parse_hhmm("03:00").unwrap()).unwrap().start,
            "20:00"
        );
        assert!(window_at(&[], 600).is_none());
    }

    #[test]
    fn the_patrol_goes_round_the_windows_bands() {
        let s = AutomationSettings::default().schedule;
        let p = patrol_at(&s, parse_hhmm("12:00").unwrap()).unwrap();
        let after = |band: &str| p.after(band, "FT8")[0].band.clone();
        assert_eq!(after("20m"), "17m");
        assert_eq!(after("10m"), "20m");
        assert_eq!(after("40m"), "20m", "not on the patrol: from its first");
        assert!(!p.many_modes());
    }

    /// Operator, 2026-11-24: "it can work both FT4 and FT8 in the same time switching between bands
    /// and modes". A window with both modes, or two windows with the same start, give every band in
    /// each mode; a window that starts later takes over as before.
    #[test]
    fn a_patrol_of_two_modes_visits_every_band_in_both() {
        let w = |start: &str, bands: &[&str], mode: &str| AutopilotWindow {
            start: start.into(),
            bands: bands.iter().map(|b| b.to_string()).collect(),
            mode: mode.into(),
        };
        let stops = |s: &[AutopilotWindow], at: &str| -> Vec<String> {
            patrol_at(s, parse_hhmm(at).unwrap())
                .unwrap()
                .stops()
                .iter()
                .map(|s| format!("{} {}", s.band, s.mode))
                .collect()
        };
        let both = ["20m FT8", "20m FT4", "17m FT8", "17m FT4"];
        let one = [w("07:00", &["20m", "17m"], "ft8 + ft4")];
        assert_eq!(stops(&one, "12:00"), both);
        let two = [
            w("07:00", &["20m", "17m"], "FT8"),
            w("07:00", &["20m", "17m"], "FT4"),
        ];
        assert_eq!(stops(&two, "12:00"), both);
        // Different bands in each.
        let mixed = [
            w("07:00", &["20m", "15m"], "FT8"),
            w("07:00", &["20m"], "FT4"),
        ];
        assert_eq!(stops(&mixed, "12:00"), ["20m FT8", "20m FT4", "15m FT8"]);
        // A later start still takes over alone.
        let later = [w("07:00", &["20m"], "FT8"), w("09:00", &["20m"], "FT4")];
        assert_eq!(stops(&later, "08:00"), ["20m FT8"]);
        assert_eq!(stops(&later, "12:00"), ["20m FT4"]);
        let p = patrol_at(&two, 720).unwrap();
        assert!(p.many_modes() && p.has("17M", "ft4") && p.has_mode("FT4") && !p.has("10m", "FT8"));
        let s = AutomationSettings {
            schedule: vec![
                w("07:00", &["20m"], " ft8+CW+ft4 "),
                w("08:00", &["20m"], "CW"),
            ],
            ..AutomationSettings::default()
        }
        .sanitized();
        assert_eq!(s.schedule.len(), 1);
        assert_eq!(s.schedule[0].mode, "FT8+FT4");
    }

    #[test]
    fn sanitizing_drops_bad_windows_and_modes() {
        let mut s = AutomationSettings::default();
        s.schedule.push(AutopilotWindow {
            start: "25:00".into(),
            bands: vec!["20m".into()],
            mode: "FT8".into(),
        });
        s.schedule.push(AutopilotWindow {
            start: "10:00".into(),
            bands: vec!["20m".into()],
            mode: "CW".into(),
        });
        s.schedule.push(AutopilotWindow {
            start: "11:00".into(),
            bands: vec![" ".into()],
            mode: "ft4".into(),
        });
        let s = s.sanitized();
        assert_eq!(s.schedule.len(), 2);
    }

    #[test]
    fn tiers_follow_the_accepted_order() {
        let mut me = cand("A1", Some("Worked"), Some("Confirmed"), 0);
        me.to_me = true;
        assert_eq!(tier_of(&me), Some(Tier::CallingMe));
        assert_eq!(
            tier_of(&cand("A1", Some("New Call"), Some("ATNO"), 0)),
            Some(Tier::Atno)
        );
        let mut dx = cand("A1", Some("New Band"), Some("Confirmed"), 0);
        dx.dxpedition = true;
        assert_eq!(tier_of(&dx), Some(Tier::Dxpedition));
        assert_eq!(
            tier_of(&cand("A1", Some("New Call"), Some("W/New Band"), 0)),
            Some(Tier::DxccUnconfirmed)
        );
        assert_eq!(
            tier_of(&cand("A1", Some("New Call"), Some("C/New Mode"), 0)),
            Some(Tier::DxccNewSlot)
        );
        let mut park = cand("A1", Some("New Call"), Some("Confirmed"), 0);
        park.new_reference = true;
        assert_eq!(tier_of(&park), Some(Tier::Reference));
        assert_eq!(
            tier_of(&cand("A1", Some("New Band+Mode"), Some("Confirmed"), 0)),
            Some(Tier::NewCall)
        );
        assert_eq!(
            tier_of(&cand("A1", Some("Worked"), Some("Worked"), 0)),
            None
        );
        assert_eq!(
            tier_of(&cand("A1", Some("Confirmed"), Some("Confirmed"), 0)),
            None
        );
        assert_eq!(tier_of(&cand("A1", None, None, 0)), None);
    }

    #[test]
    fn the_strongest_tier_wins_over_a_stronger_signal() {
        let list = vec![
            cand("NEW1", Some("New Call"), Some("Confirmed"), 20),
            cand("RARE", Some("New Call"), Some("ATNO"), -20),
        ];
        let (c, t) = pick(
            &list,
            &AutomationSettings::default(),
            &HashMap::new(),
            0,
            |_| false,
        )
        .unwrap();
        assert_eq!((c.call.as_str(), t), ("RARE", Tier::Atno));
    }

    #[test]
    fn within_a_tier_the_stronger_signal_wins() {
        let list = vec![
            cand("WEAK", Some("New Call"), Some("Confirmed"), -18),
            cand("LOUD", Some("New Call"), Some("Confirmed"), 5),
        ];
        let (c, _) = pick(
            &list,
            &AutomationSettings::default(),
            &HashMap::new(),
            0,
            |_| false,
        )
        .unwrap();
        assert_eq!(c.call, "LOUD");
    }

    #[test]
    fn a_switched_off_tier_and_a_non_cq_are_not_called() {
        let tiers = AutopilotTiers {
            new_call: false,
            ..AutopilotTiers::default()
        };
        let mut not_cq = cand("TALK", Some("New Call"), Some("ATNO"), 0);
        not_cq.is_cq = false;
        let list = vec![cand("NEW1", Some("New Call"), Some("Confirmed"), 0), not_cq];
        assert!(pick(&list, &with(&tiers), &HashMap::new(), 0, |_| false).is_none());
    }

    /// Live run 2026-11-24 (xlog.27): XE2X, a needed 80 m slot, was called from one contact to the
    /// next and sent no CQ. A needed station ending a contact is free; an ordinary one, or one with
    /// only a new grid, still waits for its CQ.
    #[test]
    fn a_needed_station_ending_a_contact_is_called_and_an_ordinary_one_is_not() {
        let s = AutomationSettings::default();
        let ending = |call: &str, ds: &str| Candidate {
            is_cq: false,
            ending: true,
            ..cand(call, Some("New Call"), Some(ds), -15)
        };
        let xe2x = ending("XE2X", "C/New Band");
        let plain = ending("DL1A", "Confirmed");
        let mut grid = ending("DL2A", "Confirmed");
        grid.new_grid = true;
        assert!(pick_call(&[plain.clone(), grid], &s).is_none());
        assert_eq!(
            pick_call(&[plain, xe2x], &s).unwrap(),
            ("XE2X".into(), Tier::DxccNewSlot)
        );
    }

    #[test]
    fn the_retry_ladder_waits_then_gives_up() {
        let mut a = HashMap::new();
        a.insert(
            "RARE".to_string(),
            Attempts {
                failures: 1,
                last_unix: 1000,
                ..Default::default()
            },
        );
        let list = vec![cand("RARE", Some("New Call"), Some("ATNO"), 0)];
        assert!(pick(
            &list,
            &AutomationSettings::default(),
            &a,
            1000 + 599,
            |_| false
        )
        .is_none());
        assert!(pick(
            &list,
            &AutomationSettings::default(),
            &a,
            1000 + 600,
            |_| false
        )
        .is_some());
        a.insert(
            "RARE".to_string(),
            Attempts {
                failures: 4,
                last_unix: 0,
                ..Default::default()
            },
        );
        assert!(pick(
            &list,
            &AutomationSettings::default(),
            &a,
            10_000_000,
            |_| false
        )
        .is_none());
    }

    #[test]
    fn a_skipped_call_is_passed_over() {
        let list = vec![
            cand("DONE", Some("New Call"), Some("ATNO"), 0),
            cand("NEXT", Some("New Call"), Some("Confirmed"), 0),
        ];
        let (c, _) = pick(
            &list,
            &AutomationSettings::default(),
            &HashMap::new(),
            0,
            |c| c.call == "DONE",
        )
        .unwrap();
        assert_eq!(c.call, "NEXT");
    }

    #[test]
    fn a_needed_state_ranks_below_dxcc_and_is_off_by_default() {
        let mut ny = cand("W2ABC", Some("Worked"), Some("Confirmed"), 0);
        ny.new_state = true;
        assert_eq!(tier_of(&ny), Some(Tier::UsState));
        let list = vec![ny.clone()];
        assert!(
            pick(
                &list,
                &AutomationSettings::default(),
                &HashMap::new(),
                0,
                |_| false
            )
            .is_none(),
            "off until the operator switches it on"
        );
        let tiers = AutopilotTiers {
            us_state: true,
            ..AutopilotTiers::default()
        };
        assert!(pick(&list, &with(&tiers), &HashMap::new(), 0, |_| false).is_some());
        let slot = cand("ZZ1", Some("New Call"), Some("C/New Band"), -20);
        let list = vec![ny, slot];
        let (c, _) = pick(&list, &with(&tiers), &HashMap::new(), 0, |_| false).unwrap();
        assert_eq!(c.call, "ZZ1", "a DXCC slot outranks a state");
    }

    #[test]
    fn the_wanted_states_list_narrows_the_chase() {
        let mut s = AutomationSettings::default();
        assert!(s.wants_state(Some("VT")), "empty list: every state");
        assert!(s.wants_state(None));
        s.wanted_states = vec![" vt ".into(), "ND".into(), "".into()];
        let s = s.sanitized();
        assert_eq!(s.wanted_states, vec!["ND".to_string(), "VT".to_string()]);
        assert!(s.wants_state(Some("vt")));
        assert!(!s.wants_state(Some("CA")));
        assert!(
            !s.wants_state(None),
            "an unplaced station is not a wanted state"
        );
        let f = FeedEntry {
            new_state: true,
            ..FeedEntry::default()
        };
        assert_eq!(spot_tier(&f, true), Some(Tier::UsState));
    }

    #[test]
    fn a_switched_off_tier_does_not_hide_a_switched_on_one() {
        let mut both = cand("KH8Z", Some("New Call"), Some("ATNO"), 0);
        both.new_state = true;
        let tiers = AutopilotTiers {
            atno: false,
            us_state: true,
            ..AutopilotTiers::default()
        };
        assert_eq!(best_allowed_tier(&both, &with(&tiers)), Some(Tier::UsState));
        assert_eq!(tier_of(&both), Some(Tier::Atno));
    }

    #[test]
    fn a_dxpedition_keeps_its_own_frequency() {
        // Inside the FT8 channel's passband: the standard dial.
        assert_eq!(hunt_dial(14.0755, Some(14.074)), 14.074);
        // Its own frequency (14.090 Fox/Hound): that kHz.
        assert!((hunt_dial(14.090, Some(14.074)) - 14.090).abs() < 1e-9);
        assert!((hunt_dial(14.0905, Some(14.074)) - 14.090).abs() < 1e-9);
        assert!((hunt_dial(7.056, None) - 7.056).abs() < 1e-9);
        assert!(in_passband(14.0755, 14.074));
        assert!(!in_passband(14.090, 14.074));
        let mut f = FeedEntry {
            dxpedition: true,
            ..FeedEntry::default()
        };
        assert!(!is_hunt_spot(&f, false), "worked everywhere: not hunted");
        assert!(is_hunt_spot(&f, true), "itself not worked there");
        assert_eq!(spot_tier(&f, true), Some(Tier::Dxpedition));
        f.new_slot = true;
        assert!(is_hunt_spot(&f, false));
    }

    #[test]
    fn only_acceptable_directed_cqs_are_answered() {
        assert!(answerable_cq(""));
        assert!(answerable_cq("pota"));
        assert!(answerable_cq("EU"));
        assert!(!answerable_cq("NA"));
        assert!(!answerable_cq("DX"));
        assert!(!answerable_cq("JA"));
    }

    #[test]
    fn the_presence_clock_counts_down() {
        let mut p = PresenceClock::default();
        p.ack(1000);
        assert_eq!(p.remaining_secs(1000, 10), 600);
        assert!(!p.expired(1599, 10));
        assert!(p.expired(1600, 10));
        p.ack(1600);
        assert!(!p.expired(1700, 2));
    }

    #[test]
    fn the_hourly_count_forgets_after_an_hour() {
        let mut h = HourlyCount::default();
        h.note(0);
        h.note(100);
        assert_eq!(h.count(200), 2);
        assert_eq!(h.count(3600), 1);
        assert_eq!(h.count(3700), 0);
    }

    #[test]
    fn only_a_possible_callsign_is_a_spot() {
        for c in [
            "3DA0RU",
            "3XY1T",
            "VK9XY",
            "9A3SWO",
            "E51ABC",
            "1A0C",
            "4U1ITU",
            "H40GC",
            "J88DR",
            "TM100VERDUN",
            "R2026NY",
            "II7MGBR",
            "W1A",
            "KH6XX/W0",
            "F/OM0RX/P",
            "vp8/g4abc",
            "3B9FR",
            "T88XX",
            "S01WS",
            "3D2AG/R",
            "BV9P",
        ] {
            assert!(spot_call_ok(c), "{c}");
        }
        for c in [
            "",
            "E34XQXUYE9H",
            "89EARO/J8Z5",
            "5T4LS/85PD6",
            "KH6XX.W0",
            "<...>",
            "CQ",
            "73",
            "JN88",
            "W1/",
        ] {
            assert!(!spot_call_ok(c), "{c}");
        }
    }
}
