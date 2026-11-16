//! Native original-Station coordinator. No Engine, clocks, sockets, devices or log.
//! Only the authenticated owner adapter may implement OwnerEvents. This module
//! has no production fake-start constructor, serde receipt or transmit backend.
use crate::xlog_decode::{DecodeContext, DecoderMode, NativeDecodedPeriod};
use crate::xlog_ft_encode;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tempo_core::{
    message::{self, Msg},
    modes::ModeKind,
    qso::{State, Station},
};
use xnexus_xlog_rx::OwnerIdentity;

const MAX_ROWS: usize = 256;
const MAX_EVENTS: u32 = 4096;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Context {
    pub connection_token: String,
    pub station_identity_epoch: String,
    /// Exact commissioned owner descriptor, never derived from RX route.
    pub transmit_context: String,
    pub clock_epoch: String,
    pub receive: DecodeContext,
    pub owner: OwnerIdentity,
}
impl Context {
    fn mode(&self) -> ModeKind {
        match self.receive.mode {
            DecoderMode::Ft8 => ModeKind::Ft8,
            DecoderMode::Ft4 => ModeKind::Ft4,
        }
    }
    fn period_us(&self) -> u64 {
        self.receive.mode.period_ms() * 1000
    }
    fn valid(&self) -> bool {
        [
            &self.connection_token,
            &self.station_identity_epoch,
            &self.transmit_context,
            &self.clock_epoch,
        ]
        .iter()
        .all(|s| !s.is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control))
            && self.receive.stream_id == self.owner.capture_id
            && self.receive.epoch == self.owner.epoch
            && self.receive.radio_id == self.owner.radio_id
            && self.receive.receiver_id == self.owner.receiver_id
            && self.receive.mapping_generation == self.owner.mapping_generation
            && self.receive.dial_hz == self.owner.frequency_hz
    }
}
#[derive(Clone)]
pub(crate) struct Settings {
    pub call: String,
    pub grid: String,
    pub tx_even: bool,
    pub auto_sequence: bool,
    pub cq_call_cap: Option<u32>,
    pub call_cap: Option<u32>,
    pub cq_pause: Duration,
    pub watchdog: Duration,
    pub disable_after_73: bool,
}
pub(crate) enum Intent {
    Cq,
    CqDirected(String),
    Work {
        call: String,
        heard: Option<(Msg, i32)>,
        prefer_rrr: bool,
        skip_tx1: bool,
    },
    Enable(bool),
    Parity(bool),
    AutoSequence(bool),
    Resend,
    Override(String),
    FreeText(String),
    Stop,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Invalid,
    Disarmed,
    Expired,
    Busy,
    WrongContext,
    OldPeriod,
    WrongParity,
    SlotUsed,
    NoMessage,
    BudgetSpent,
    WrongOperation,
    Exhausted,
    Unknown,
    Encoder(xlog_ft_encode::Error),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Halt {
    Stop,
    Context,
    Refused,
    Unknown,
    Expired,
    Exhausted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Key {
    pub context: Context,
    pub qso_epoch: u64,
    pub intent: u64,
    pub slot_start_us: u64,
    pub latest_start_us: u64,
}
#[derive(Clone)]
pub(crate) struct Plan {
    key: Key,
    text: String,
    mode: ModeKind,
    deadline: Instant,
    receipt_deadline: Instant,
}
impl Plan {
    pub(crate) fn key(&self) -> &Key {
        &self.key
    }
    pub(crate) fn deadline(&self) -> Instant {
        self.deadline
    }
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
    pub(crate) fn prepare(&self, offset: f32) -> Result<xlog_ft_encode::Prepared, Error> {
        xlog_ft_encode::prepare(self.mode, &self.text, offset, self.deadline)
            .map_err(Error::Encoder)
    }
}
/// Exact binding returned by the owner adapter after it has checked the complete
/// approved body against the plan and final PCM. A digest alone proves no message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Operation {
    pub key: Key,
    pub id: String,
    pub body_digest: [u8; 32],
}
pub(crate) struct Started {
    pub event_id: String,
    pub started_us: u64,
}
pub(crate) enum OwnerState {
    Pending,
    Started(Started),
    NoStart,
    ObservedReleased,
    Refused,
    Unknown,
}
/// Trusted native integration seam, NOT an IPC implementation. The future SDK
/// adapter must verify session/token, exact whole body, current owner/clock and
/// immutable actual-start/event identity. HTTP ACK is ONLY Pending. No shipping
/// implementation exists yet; tests implement a controlled port under cfg(test).
pub(crate) trait OwnerEvents {
    fn state(&self, exact: &Operation, now: Instant) -> Result<OwnerState, Error>;
}

struct StopState {
    armed: bool,
    cancelled: bool,
    release: Option<Arc<Operation>>,
    started: bool,
}
/// Independent small admission/release cell. Never held across decode, encoding,
/// HTTP, owner proof verification or JSON. Stop does not need the coordinator lock.
#[derive(Clone)]
pub(crate) struct StopHandle(Arc<Mutex<StopState>>);
impl StopHandle {
    pub(crate) fn stop(&self) -> Result<Option<Arc<Operation>>, Error> {
        let mut s = self.0.lock().map_err(|_| Error::Unknown)?;
        s.armed = false;
        s.cancelled = true;
        Ok(s.release.clone())
    }
    pub(crate) fn release(&self) -> Result<Option<Arc<Operation>>, Error> {
        Ok(self.0.lock().map_err(|_| Error::Unknown)?.release.clone())
    }
}
struct Pending {
    plan: Plan,
    frozen: Station,
    started: Option<(String, u64)>,
}
pub(crate) struct View {
    pub state: State,
    pub dx_call: Option<String>,
    pub dx_grid: Option<String>,
    pub rx_report: Option<i32>,
    pub cq_running: bool,
    pub stalled: bool,
    pub qso_epoch: u64,
    pub pending_text: Option<String>,
    pub tx_count: u32,
    pub armed: bool,
    pub auto_sequence: bool,
    pub tx_even: bool,
    pub keyed: bool,
    pub done: bool,
    pub halt: Option<Halt>,
    pub release: Option<Arc<Operation>>,
}
#[derive(Clone)]
pub(crate) struct QsoFacts {
    pub epoch: u64,
    pub call: String,
    pub grid: String,
    pub received: Option<i32>,
    pub started_us: u64,
}
pub(crate) struct Coordinator {
    context: Context,
    settings: Settings,
    station: Station,
    stop: StopHandle,
    authority_deadline: Instant,
    operator_deadline: Instant,
    qso_epoch: u64,
    intent: u64,
    events: u32,
    seen_period: Option<u64>,
    reserved_until: u64,
    pending: Option<Pending>,
    cq_run: bool,
    cq_direction: String,
    cq_resume: Option<Instant>,
    halt: Option<Halt>,
    transcript: Vec<String>,
    finished: Option<QsoFacts>,
    qso_started_us: u64,
}
impl Coordinator {
    pub(crate) fn new(
        context: Context,
        settings: Settings,
        authority_deadline: Instant,
        now: Instant,
    ) -> Result<Self, Error> {
        if !context.valid()
            || !valid_call(&settings.call)
            || !valid_grid(&settings.grid)
            || settings.watchdog.is_zero()
            || settings.watchdog > Duration::from_secs(3600)
            || settings.cq_pause > Duration::from_secs(3600)
            || authority_deadline <= now
        {
            return Err(Error::Invalid);
        }
        let station = Station::monitoring(&settings.call, &settings.grid);
        Ok(Self {
            context,
            settings,
            station,
            stop: StopHandle(Arc::new(Mutex::new(StopState {
                armed: false,
                cancelled: true,
                release: None,
                started: false,
            }))),
            authority_deadline,
            operator_deadline: now,
            qso_epoch: 0,
            intent: 0,
            events: 0,
            seen_period: None,
            reserved_until: 0,
            pending: None,
            cq_run: false,
            cq_direction: String::new(),
            cq_resume: None,
            halt: Some(Halt::Stop),
            transcript: Vec::new(),
            finished: None,
            qso_started_us: 0,
        })
    }
    pub(crate) fn stop_handle(&self) -> StopHandle {
        self.stop.clone()
    }
    pub(crate) fn context(&self) -> &Context {
        &self.context
    }
    pub(crate) fn view(&self) -> Result<View, Error> {
        let s = self.stop.0.lock().map_err(|_| Error::Unknown)?;
        Ok(View {
            state: self.station.state,
            dx_call: self.station.dxcall.clone(),
            dx_grid: self.station.dxgrid.clone(),
            rx_report: self.station.rx_report,
            cq_running: self.cq_run,
            stalled: self.station.stalled(),
            qso_epoch: self.qso_epoch,
            pending_text: self.station.pending_text(),
            tx_count: self.station.tx_count,
            armed: s.armed,
            auto_sequence: self.settings.auto_sequence,
            tx_even: self.settings.tx_even,
            keyed: self.station.keyed_on_air,
            done: self.station.done(),
            halt: self.halt,
            release: s.release.clone(),
        })
    }
    pub(crate) fn log_facts(&self) -> Option<QsoFacts> {
        if self.station.keyed_on_air {
            if let Some(call) = &self.station.dxcall {
                return Some(QsoFacts {
                    epoch: self.qso_epoch,
                    call: call.clone(),
                    grid: self.station.dxgrid.clone().unwrap_or_default(),
                    received: self.station.rx_report,
                    started_us: self.qso_started_us,
                });
            }
        }
        self.finished.clone()
    }
    pub(crate) fn manual_target(
        &mut self,
        call: &str,
        grid: Option<&str>,
        now: Instant,
    ) -> Result<(), Error> {
        if grid.is_some_and(|g| !g.is_empty() && !valid_grid(g)) {
            return Err(Error::Invalid);
        }
        self.apply(
            Intent::Work {
                call: call.into(),
                heard: None,
                prefer_rrr: false,
                skip_tx1: false,
            },
            now,
        )?;
        if let Some(grid) = grid.filter(|g| !g.is_empty()) {
            self.station.dxgrid = Some(grid.into());
        }
        Ok(())
    }
    pub(crate) fn retire(&mut self, reason: Halt) -> Result<Option<Arc<Operation>>, Error> {
        let release = self.stop.stop()?;
        self.halt = Some(reason);
        // A pending exact binding remains in the Stop cell until owner proof of
        // release/no-start. Old queued start facts cannot mutate the Station.
        self.pending = None;
        Ok(release)
    }
    /// Same-context authenticated observation may renew admission, never a
    /// pending operation's captured deadline. Any changed coordinate retires.
    pub(crate) fn refresh(
        &mut self,
        context: &Context,
        deadline: Instant,
        now: Instant,
    ) -> Result<(), Error> {
        if context != &self.context {
            self.retire(Halt::Context)?;
            return Err(Error::WrongContext);
        }
        if deadline <= now {
            self.retire(Halt::Expired)?;
            return Err(Error::Expired);
        }
        self.authority_deadline = deadline;
        Ok(())
    }
    fn check_event(&mut self, now: Instant) -> Result<(), Error> {
        if now >= self.authority_deadline
            || now >= self.operator_deadline
            || self
                .pending
                .as_ref()
                .is_some_and(|p| p.started.is_none() && now >= p.plan.receipt_deadline)
        {
            self.retire(Halt::Expired)?;
            return Err(Error::Expired);
        }
        if self.events == MAX_EVENTS {
            self.retire(Halt::Exhausted)?;
            return Err(Error::Exhausted);
        }
        self.events += 1;
        Ok(())
    }
    fn check(&mut self, now: Instant) -> Result<(), Error> {
        self.check_event(now)?;
        if !self.stop.0.lock().map_err(|_| Error::Unknown)?.armed {
            return Err(Error::Disarmed);
        }
        Ok(())
    }
    fn drain_transcript(&mut self) {
        self.transcript.extend(
            self.station
                .transcript
                .drain(..)
                .map(|s| s.chars().take(512).collect::<String>()),
        );
        if self.transcript.len() > 32 {
            self.transcript.drain(..self.transcript.len() - 32);
        }
    }
    pub(crate) fn apply(&mut self, intent: Intent, now: Instant) -> Result<(), Error> {
        if matches!(intent, Intent::Enable(false)) {
            self.stop.0.lock().map_err(|_| Error::Unknown)?.armed = false;
            return Ok(());
        }
        if matches!(intent, Intent::Stop) {
            self.retire(Halt::Stop)?;
            return Ok(());
        }
        if now >= self.authority_deadline {
            self.retire(Halt::Expired)?;
            return Err(Error::Expired);
        }
        if self.pending.is_some() || self.stop.release()?.is_some() {
            return Err(Error::Busy);
        }
        let arms = !matches!(intent, Intent::Parity(_) | Intent::AutoSequence(_));
        let next = self.qso_epoch.checked_add(1).ok_or(Error::Exhausted)?;
        let intent = match intent {
            Intent::CqDirected(direction) => {
                if direction.is_empty()
                    || direction.len() > 8
                    || !direction
                        .bytes()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
                {
                    return Err(Error::Invalid);
                }
                let text = format!(
                    "CQ {} {} {}",
                    direction,
                    self.settings.call,
                    &self.settings.grid[..4]
                );
                xlog_ft_encode::validate_text(self.context.mode(), &text)
                    .map_err(Error::Encoder)?;
                self.cq_direction = direction;
                Intent::CqDirected(String::new())
            }
            Intent::Cq => {
                self.cq_direction.clear();
                Intent::Cq
            }
            other => other,
        };
        match intent {
            Intent::Cq | Intent::CqDirected(_) => {
                self.station = Station::calling_cq(&self.settings.call, &self.settings.grid);
                if !self.cq_direction.is_empty() {
                    self.station.override_next(Msg::Cq {
                        de: self.settings.call.clone(),
                        grid: self.settings.grid[..4].into(),
                        dir: self.cq_direction.clone(),
                    });
                }
                self.cq_run = true;
                self.qso_epoch = next;
                self.qso_started_us = 0;
            }
            Intent::Work {
                call,
                heard,
                prefer_rrr,
                skip_tx1,
            } => {
                if !valid_call(&call)
                    || heard.as_ref().is_some_and(|(m, snr)| {
                        m.to_text().len() > 128 || !(-50..=49).contains(snr)
                    })
                {
                    return Err(Error::Invalid);
                }
                self.station = Station::start(
                    &self.settings.call,
                    &self.settings.grid,
                    &call,
                    heard.as_ref().map(|(m, snr)| (m, *snr)),
                    prefer_rrr,
                    skip_tx1,
                );
                self.cq_run = false;
                self.qso_epoch = next;
                self.qso_started_us = 0;
            }
            Intent::Parity(even) => {
                self.settings.tx_even = even;
            }
            Intent::AutoSequence(on) => {
                self.settings.auto_sequence = on;
            }
            Intent::Resend => self.station.resend(),
            Intent::Override(text) => {
                xlog_ft_encode::validate_text(self.context.mode(), &text)
                    .map_err(Error::Encoder)?;
                self.station.override_next(self.station.tx_row(&text));
            }
            Intent::FreeText(text) => {
                if text.len() > 13 {
                    return Err(Error::Invalid);
                }
                xlog_ft_encode::validate_text(self.context.mode(), &text)
                    .map_err(Error::Encoder)?;
                self.station.override_next(Msg::Other(text));
            }
            Intent::Enable(true) => {}
            Intent::Stop | Intent::Enable(false) => unreachable!(),
        }
        self.station.cq_call_cap = self.settings.cq_call_cap;
        self.station.call_cap = self.settings.call_cap;
        self.station.report_follows_latest_decode = true;
        self.operator_deadline = now
            .checked_add(self.settings.watchdog)
            .ok_or(Error::Exhausted)?;
        // Parity/Auto edits do not arm a stopped station. CQ/Work/Enable/Resend/
        // message selection are explicit send intents in the original controls.
        if arms {
            let mut s = self.stop.0.lock().map_err(|_| Error::Unknown)?;
            s.armed = true;
            s.cancelled = false;
        }
        self.events = 0;
        self.halt = None;
        self.cq_resume = None;
        self.drain_transcript();
        Ok(())
    }
    /// Borrow actual accepted records before UI conversion. A busy period is
    /// consumed/dropped, not replayed after encoding or an owner response.
    pub(crate) fn observe(
        &mut self,
        period: &NativeDecodedPeriod,
        now: Instant,
    ) -> Result<(), Error> {
        if now >= self.authority_deadline {
            self.retire(Halt::Expired)?;
            return Err(Error::Expired);
        }
        if self.events == MAX_EVENTS {
            self.retire(Halt::Exhausted)?;
            return Err(Error::Exhausted);
        }
        self.events += 1;
        if period.context != self.context.receive || period.owner != self.context.owner {
            self.retire(Halt::Context)?;
            return Err(Error::WrongContext);
        }
        if period.host_period_start_unix_us % self.context.period_us() != 0
            || period.rows.len() > MAX_ROWS
            || period.rows.iter().any(|r| {
                r.mode != Some(self.context.mode())
                    || r.message.len() > 128
                    || !r.sync.is_finite()
                    || !r.dt.is_finite()
                    || !r.freq.is_finite()
                    || !r.qual.is_finite()
            })
        {
            return Err(Error::Invalid);
        }
        if self
            .seen_period
            .is_some_and(|old| period.host_period_start_unix_us <= old)
        {
            return Err(Error::OldPeriod);
        }
        self.seen_period = Some(period.host_period_start_unix_us);
        if self.pending.is_some() {
            return Err(Error::Busy);
        }
        let prior_dx = self.station.dxcall.clone();
        if self.settings.auto_sequence {
            self.station.observe(&period.rows);
        }
        if prior_dx.is_none() && self.station.dxcall.is_some() {
            self.qso_epoch = self.qso_epoch.checked_add(1).ok_or(Error::Exhausted)?;
            self.qso_started_us = period.host_period_start_unix_us;
        }
        if self.station.done() && self.station.keyed_on_air {
            self.finished = self.log_facts();
        }
        if self.cq_run && self.station.done() && self.station.pending.is_none() {
            self.station = Station::calling_cq(&self.settings.call, &self.settings.grid);
            if !self.cq_direction.is_empty() {
                self.station.override_next(Msg::Cq {
                    de: self.settings.call.clone(),
                    grid: self.settings.grid[..4].into(),
                    dir: self.cq_direction.clone(),
                });
            }
            self.station.cq_call_cap = self.settings.cq_call_cap;
            self.station.call_cap = self.settings.call_cap;
            self.station.report_follows_latest_decode = true;
        }
        self.drain_transcript();
        Ok(())
    }
    /// Caller supplies a *current owner* slot/clock observation. This performs
    /// no sleeps or scheduling and never catches up a missed period.
    pub(crate) fn plan(
        &mut self,
        slot_start_us: u64,
        latest_start_us: u64,
        wall_now_us: u64,
        now: Instant,
    ) -> Result<Plan, Error> {
        self.check(now)?;
        if self.pending.is_some() || self.stop.release()?.is_some() {
            return Err(Error::Busy);
        }
        let duration = self.context.period_us();
        let end = slot_start_us.checked_add(duration).ok_or(Error::Invalid)?;
        if slot_start_us == 0
            || slot_start_us % duration != 0
            || latest_start_us < slot_start_us
            || latest_start_us >= end
            || wall_now_us > latest_start_us
        {
            return Err(Error::Invalid);
        }
        if (slot_start_us / duration) % 2 != u64::from(!self.settings.tx_even) {
            return Err(Error::WrongParity);
        }
        if slot_start_us < self.reserved_until {
            return Err(Error::SlotUsed);
        }
        if self.station.stalled()
            && self.station.state == State::CallingCq
            && !self.settings.cq_pause.is_zero()
        {
            let until = *self.cq_resume.get_or_insert(now + self.settings.cq_pause);
            if now < until {
                return Err(Error::BudgetSpent);
            }
            self.station.resume_cq_run();
            self.cq_resume = None;
        }
        let (message, _) = self
            .station
            .outgoing_rv()
            .ok_or(if self.station.stalled() {
                Error::BudgetSpent
            } else {
                Error::NoMessage
            })?;
        let text = message.to_text();
        xlog_ft_encode::validate_text(self.context.mode(), &text).map_err(Error::Encoder)?;
        self.intent = self.intent.checked_add(1).ok_or(Error::Exhausted)?;
        self.reserved_until = end;
        let deadline = now
            .checked_add(Duration::from_micros(latest_start_us - wall_now_us))
            .ok_or(Error::Invalid)?
            .min(self.operator_deadline);
        if deadline <= now {
            return Err(Error::Expired);
        }
        let receipt_deadline = now
            .checked_add(Duration::from_micros(end - wall_now_us))
            .ok_or(Error::Invalid)?
            .min(self.operator_deadline);
        let plan = Plan {
            key: Key {
                context: self.context.clone(),
                qso_epoch: self.qso_epoch,
                intent: self.intent,
                slot_start_us,
                latest_start_us,
            },
            text,
            mode: self.context.mode(),
            deadline,
            receipt_deadline,
        };
        self.pending = Some(Pending {
            plan: plan.clone(),
            frozen: self.station.clone(),
            started: None,
        });
        Ok(plan)
    }
    /// SDK adapter has verified the owner's exact body/final PCM binding. This
    /// function additionally prevents a stale preparation attaching to new intent.
    pub(crate) fn bind(
        &mut self,
        plan: &Plan,
        operation: Operation,
        now: Instant,
    ) -> Result<(), Error> {
        self.check(now)?;
        let p = self.pending.as_ref().ok_or(Error::WrongOperation)?;
        if now >= p.plan.deadline {
            self.retire(Halt::Expired)?;
            return Err(Error::Expired);
        }
        if p.plan.key != plan.key
            || operation.key != plan.key
            || operation.id.is_empty()
            || operation.id.len() > 128
            || operation.body_digest == [0; 32]
        {
            return Err(Error::WrongOperation);
        }
        let mut s = self.stop.0.lock().map_err(|_| Error::Unknown)?;
        if !s.armed {
            return Err(Error::Disarmed);
        }
        if s.release.is_some() {
            return Err(Error::Busy);
        }
        s.release = Some(Arc::new(operation));
        s.started = false;
        Ok(())
    }
    /// Verification happens outside the independent Stop lock. Its result is
    /// re-fenced under that lock before Station accounting or release retirement.
    pub(crate) fn owner_event(
        &mut self,
        port: &impl OwnerEvents,
        now: Instant,
    ) -> Result<(), Error> {
        let operation = self.stop.release()?.ok_or(Error::WrongOperation)?;
        let event = match port.state(&operation, now) {
            Ok(event) => event,
            Err(error) => {
                self.retire(Halt::Unknown)?;
                return Err(error);
            }
        };
        if matches!(event, OwnerState::NoStart | OwnerState::ObservedReleased) {
            let mut s = self.stop.0.lock().map_err(|_| Error::Unknown)?;
            if s.release.as_deref() != Some(operation.as_ref()) {
                return Err(Error::WrongOperation);
            }
            if matches!(event, OwnerState::NoStart) && s.started {
                return Err(Error::WrongOperation);
            }
            if self.pending.as_ref().is_some_and(|p| p.started.is_none()) {
                s.armed = false;
                self.halt = Some(Halt::Refused);
            }
            s.release = None;
            s.started = false;
            self.pending = None;
            return Ok(());
        }
        self.check_event(now)?;
        match event {
            OwnerState::Pending => Ok(()),
            OwnerState::Refused | OwnerState::Unknown => {
                self.retire(if matches!(event, OwnerState::Unknown) {
                    Halt::Unknown
                } else {
                    Halt::Refused
                })?;
                Ok(())
            }
            OwnerState::Started(start) => {
                let p = self.pending.as_mut().ok_or(Error::WrongOperation)?;
                if let Some((id, time)) = &p.started {
                    return if id == &start.event_id && *time == start.started_us {
                        Ok(())
                    } else {
                        Err(Error::WrongOperation)
                    };
                }
                if start.event_id.is_empty()
                    || start.event_id.len() > 128
                    || start.started_us < p.plan.key.slot_start_us
                    || start.started_us > p.plan.key.latest_start_us
                {
                    return Err(Error::WrongOperation);
                }
                let mut s = self.stop.0.lock().map_err(|_| Error::Unknown)?;
                if s.cancelled || s.release.as_deref() != Some(operation.as_ref()) {
                    return Err(Error::Disarmed);
                }
                p.frozen.after_tx();
                self.station = p.frozen.clone();
                p.started = Some((start.event_id, start.started_us));
                s.started = true;
                if self.settings.disable_after_73 && !self.cq_run && self.station.closing_sent() {
                    s.armed = false;
                }
                drop(s);
                self.drain_transcript();
                Ok(())
            }
            OwnerState::NoStart | OwnerState::ObservedReleased => unreachable!(),
        }
    }
}
fn valid_call(s: &str) -> bool {
    (3..=32).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'/')
        && message::looks_like_call(s)
}
fn valid_grid(s: &str) -> bool {
    let b = s.as_bytes();
    [4, 6, 8].contains(&b.len())
        && b.iter().enumerate().all(|(i, c)| match i {
            0 | 1 => (b'A'..=b'R').contains(c),
            2 | 3 | 6 | 7 => c.is_ascii_digit(),
            _ => (b'A'..=b'X').contains(c),
        })
}

#[cfg(test)]
#[path = "xlog_ft_runtime_tests.rs"]
mod tests;
