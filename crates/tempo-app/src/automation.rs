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
    /// `"FT8"`, `"FT4"` or `"FT2"`.
    pub mode: String,
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
    /// Time budget for an ATNO or DXpedition, seconds.
    pub rare_budget_secs: u32,
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
    /// The rules, strongest first: the operator's own order ([`DEFAULT_PRIORITY`] until changed).
    /// Each rule is switched on or off in [`Self::tiers`]. A name this build does not know (a
    /// newer build's rule) is skipped rather than failing the whole settings file.
    #[serde(deserialize_with = "de_priority")]
    pub priority: Vec<Tier>,
    /// Within a rule, call a station outside Europe before a European one (operator, 2026-11-22:
    /// "always prioritising DX from outside of Europe unless the EU station is an all-time new
    /// one" — an ATNO is its own, stronger rule).
    pub prefer_outside_eu: bool,
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
            rare_budget_secs: 900,
            max_qso_per_hour: 60,
            tiers: AutopilotTiers::default(),
            chase_spots: true,
            wanted_states: Vec::new(),
            hunt_dxpeditions: true,
            hunt_hound: true,
            priority: DEFAULT_PRIORITY.to_vec(),
            prefer_outside_eu: true,
        }
    }
}

impl AutomationSettings {
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
        // A DXpedition or ATNO is called for at least ten minutes (operator, 2026-11-22: a
        // pileup is rarely broken in five to eight calls).
        self.rare_budget_secs = self.rare_budget_secs.clamp(600, 10800);
        self.max_qso_per_hour = self.max_qso_per_hour.clamp(1, 300);
        self.schedule
            .retain(|w| parse_hhmm(&w.start).is_some() && is_auto_mode(&w.mode));
        for w in &mut self.schedule {
            w.bands.retain(|b| !b.trim().is_empty());
            w.mode = w.mode.trim().to_uppercase();
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

/// The band to patrol after `current` in a window: the next in its list, wrapping, or its first
/// when `current` is not in it.
pub fn next_band<'a>(w: &'a AutopilotWindow, current: Option<&str>) -> Option<&'a str> {
    let i = current.and_then(|c| w.bands.iter().position(|b| b.eq_ignore_ascii_case(c)));
    match i {
        Some(i) => w.bands.get((i + 1) % w.bands.len()),
        None => w.bands.first(),
    }
    .map(String::as_str)
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
    /// When the last attempt ended, Unix seconds.
    pub last_unix: u64,
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
        .filter(|c| c.to_me || c.is_cq)
        .filter(|c| !skip(c))
        .filter_map(|c| best_allowed_tier(c, s).map(|t| (c, t)))
        .filter(|(c, _)| may_retry(attempts.get(&c.call.to_uppercase()), now_unix))
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
}

/// Could a station hold this call? Spots carry false decodes ("E34XQXUYE9H", "89EARO/J8Z5": both
/// chased for five minutes, 2026-11-23), and a garbled call reads as a new entity.
///
/// Wider than WSJT-X's standard call on purpose, since a dropped real one could be the ATNO: one
/// part of the call must be a prefix of one to three characters with a letter in it (`3DA`, `3X`),
/// one to four digits (`TM100VERDUN`), and one to six letters. `KH6XX/W0` and `F/OM0RX/P` pass on
/// the part that does.
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
    call.len() <= 14 && call.split('/').all(alnum) && call.split('/').any(full)
}

/// The tier a spot would put its station in, for chasing it to another band; `None` = not worth
/// leaving the band for (only ATNO, DXpeditions and DXCC slots move the radio).
pub fn spot_tier(f: &FeedEntry) -> Option<Tier> {
    if f.new_entity {
        Some(Tier::Atno)
    } else if f.dxpedition && f.new_slot {
        Some(Tier::Dxpedition)
    } else if f.new_slot {
        Some(Tier::DxccNewSlot)
    } else if f.new_state {
        Some(Tier::UsState)
    } else {
        None
    }
}

/// Is this spot a DXpedition worth leaving the band for: a listed DXpedition, needed (a new
/// entity, or new on that band or mode)?
pub fn is_hunt_spot(f: &FeedEntry) -> bool {
    f.dxpedition && (f.new_entity || f.new_slot)
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
/// The highest audio offset the autopilot calls a DXpedition from: an FT8 signal is 50 Hz wide
/// and many radios' transmit passband ends near 3 kHz.
pub const DXPED_SPLIT_TOP_HZ: f32 = 2900.0;
/// Where to transmit when calling a DXpedition heard at `dx_hz`, from `tx_hz`: the current offset
/// when it is already at least [`DXPED_SPLIT_HZ`] above (and inside the passband), else exactly
/// that far above. Always above, never below (operator, 2026-11-22: "a DXpedition is usually
/// within 500 Hz, so we can only go above; those are the rules"): one heard so high that 1200 Hz
/// above does not fit is called from the top of the passband, still above it.
pub fn dxped_split_hz(dx_hz: f32, tx_hz: f32) -> f32 {
    let up = dx_hz + DXPED_SPLIT_HZ;
    if tx_hz >= up && tx_hz <= DXPED_SPLIT_TOP_HZ {
        tx_hz
    } else {
        up.min(DXPED_SPLIT_TOP_HZ)
    }
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
    fn a_dxpedition_is_always_called_from_above_never_on_or_below_it() {
        // The usual case: a DXpedition low in the passband, called 1200 Hz above.
        assert_eq!(dxped_split_hz(450.0, 450.0), 1650.0);
        assert_eq!(dxped_split_hz(450.0, 300.0), 1650.0, "never below");
        // Already far enough above, and inside the passband: left where it is.
        assert_eq!(dxped_split_hz(450.0, 2200.0), 2200.0);
        // Too close above: moved out to the full spacing.
        assert_eq!(dxped_split_hz(1500.0, 2000.0), 2700.0);
        // Too high to fit 1200 above: the top of the passband, still above, never below.
        assert_eq!(dxped_split_hz(2000.0, 2000.0), DXPED_SPLIT_TOP_HZ);
        assert_eq!(dxped_split_hz(2000.0, 500.0), DXPED_SPLIT_TOP_HZ);
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
    fn bands_rotate_in_order_and_wrap() {
        let w = &AutomationSettings::default().schedule[0];
        assert_eq!(next_band(w, Some("20m")), Some("17m"));
        assert_eq!(next_band(w, Some("10m")), Some("20m"));
        assert_eq!(next_band(w, Some("40m")), Some("20m"));
        assert_eq!(next_band(w, None), Some("20m"));
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

    #[test]
    fn the_retry_ladder_waits_then_gives_up() {
        let mut a = HashMap::new();
        a.insert(
            "RARE".to_string(),
            Attempts {
                failures: 1,
                last_unix: 1000,
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
        assert_eq!(spot_tier(&f), Some(Tier::UsState));
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
        assert!(!is_hunt_spot(&f), "worked everywhere: not hunted");
        f.new_slot = true;
        assert!(is_hunt_spot(&f));
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
