// SPDX-License-Identifier: GPL-3.0-only
//! OFFLINE ONLY. No owner authentication, scheduling, device, log, Engine or IPC.
//! Fake start/release facts are explicit test inputs, never physical evidence.
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tempo_core::{
    message::Msg,
    modes::{Decode, ModeKind},
    qso::{State, Station},
};
use xnexus_tx_prepare::{prepare, Format, Limits, PreparedPeriod};

const MAX_ROWS: usize = 256;
const MAX_TEXT: usize = 128;
const MAX_DIAGNOSTICS: usize = 32;
const MAX_SESSION_EVENTS: u32 = 4096;
const MAX_BODY: usize = 40_000;
const MAX_MANIFEST: usize = 16_384;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    UnsupportedMode,
    Disarmed,
    Exhausted,
    BusyDropped,
    DuplicatePeriod,
    ConflictingPeriod,
    OldPeriod,
    WrongContext,
    SlotUsed,
    NoMessage,
    BudgetSpent,
    ReleaseOutstanding,
    WrongOperation,
    Expired,
    Preparation(xnexus_tx_prepare::Error),
}

fn period_us(mode: ModeKind) -> Result<u64, Error> {
    match mode {
        ModeKind::Ft8 => Ok(15_000_000),
        ModeKind::Ft4 => Ok(7_500_000),
        _ => Err(Error::UnsupportedMode),
    }
}
fn bounded(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}

/// Opaque OFFLINE context: its strings are equality coordinates, not credentials
/// or proof of an authorized TX route. In particular RX origin grants no TX.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub owner_session: String,
    pub receive_origin: String,
    pub transmit_context: String,
    pub clock_epoch: String,
    pub mode: ModeKind,
}
impl Context {
    fn validate(&self) -> Result<(), Error> {
        period_us(self.mode)?;
        if ![
            &self.owner_session,
            &self.receive_origin,
            &self.transmit_context,
            &self.clock_epoch,
        ]
        .iter()
        .all(|s| bounded(s, 1024))
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Epoch {
    pub context: u64,
    pub qso: u64,
}

/// Caller supplies the actual complete decode batch, not retained display rows.
/// No missing sync/AP/quality values are reconstructed here.
pub struct CompleteDecodeBatch {
    pub epoch: Epoch,
    pub context: Context,
    pub period_start_us: u64,
    pub rows: Vec<Decode>,
}
impl CompleteDecodeBatch {
    fn fingerprint(&self) -> Result<[u8; 32], Error> {
        self.context.validate()?;
        if self.period_start_us == 0
            || self.period_start_us % period_us(self.context.mode)? != 0
            || self.rows.len() > MAX_ROWS
        {
            return Err(Error::InvalidInput);
        }
        let mut h = Sha256::new();
        // Local duplicate detection only, NOT an owner protocol/body digest.
        for row in &self.rows {
            if row.mode != Some(self.context.mode)
                || !bounded(&row.message, MAX_TEXT)
                || ![row.sync, row.dt, row.freq, row.qual]
                    .iter()
                    .all(|v| v.is_finite())
                || row.nap != 0
                || row.rv.is_some()
                || row.raw.is_some()
            {
                return Err(Error::InvalidInput);
            }
            h.update((row.message.len() as u64).to_le_bytes());
            h.update(row.message.as_bytes());
            h.update(row.snr.to_le_bytes());
            for value in [row.sync, row.dt, row.freq, row.qual] {
                h.update(value.to_bits().to_le_bytes());
            }
        }
        Ok(h.finalize().into())
    }
}

/// Explicit operator intent; cannot inject arbitrary mutable Station state.
pub enum OperatorIntent {
    Monitor,
    Cq,
    Call {
        dxcall: String,
        context: Option<(Msg, i32)>,
        prefer_rrr: bool,
        skip_tx1: bool,
    },
}
pub struct OperatorSettings {
    pub mycall: String,
    pub mygrid: String,
    pub cq_call_cap: Option<u32>,
    pub call_cap: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Correlation {
    pub epoch: Epoch,
    pub intent: u64,
    pub context: Context,
    pub slot_start_us: u64,
    pub slot_duration_us: u64,
    pub latest_start_us: u64,
}

/// Entire owner body and manifest preserved independently of exact PCM. This
/// constructor performs SIZE/EQUALITY checks only. It neither parses nor
/// authenticates an owner Body. A real owner fixture must cross-check these
/// bytes; a future live SDK, not this prototype, would own that constructor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfflineOperation {
    correlation: Correlation,
    operation_id: String,
    owner_body: Vec<u8>,
    manifest: Vec<u8>,
    pcm: Vec<u8>,
}
impl OfflineOperation {
    pub fn from_offline_fixture(
        intent: &PreparedIntent,
        operation_id: String,
        owner_body: Vec<u8>,
        manifest: Vec<u8>,
        pcm: Vec<u8>,
    ) -> Result<Self, Error> {
        if !bounded(&operation_id, 128)
            || owner_body.is_empty()
            || owner_body.len() > MAX_BODY
            || manifest.is_empty()
            || manifest.len() > MAX_MANIFEST
            || pcm != intent.period.pcm()
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            correlation: intent.correlation.clone(),
            operation_id,
            owner_body,
            manifest,
            pcm,
        })
    }
    pub fn correlation(&self) -> &Correlation {
        &self.correlation
    }
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
    pub fn owner_body(&self) -> &[u8] {
        &self.owner_body
    }
    pub fn manifest(&self) -> &[u8] {
        &self.manifest
    }
    pub fn pcm(&self) -> &[u8] {
        &self.pcm
    }
}

#[derive(Clone, Debug)]
pub struct PreparedIntent {
    correlation: Correlation,
    period: Arc<PreparedPeriod>,
    message: Msg,
    rv: u8,
}
impl PreparedIntent {
    pub fn correlation(&self) -> &Correlation {
        &self.correlation
    }
    pub fn period(&self) -> &PreparedPeriod {
        &self.period
    }
    pub fn message(&self) -> &Msg {
        &self.message
    }
    /// Station accounting only. FT8/FT4 modulation never consumes HARQ RV.
    pub fn rv(&self) -> u8 {
        self.rv
    }
}
struct Pending {
    intent: PreparedIntent,
    frozen: Station,
    operation: Option<Arc<OfflineOperation>>,
}

/// No serde or transport constructor: this is intentionally a fake input.
pub struct FakeActualStart {
    pub operation: Arc<OfflineOperation>,
    pub started_us: u64,
    pub observed_us: u64,
}
#[derive(Clone, Copy, Debug)]
pub enum NonStart {
    Starting,
    Permit,
    Ack,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Halt {
    Stop,
    ContextRetired,
    Refused,
    Unknown,
    Expired,
    Exhausted,
    Preparation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub epoch: Epoch,
    pub armed: bool,
    pub state: Option<State>,
    pub pending_text: Option<String>,
    pub tx_count: u32,
    pub keyed: bool,
    pub closing_sent: bool,
    pub done: bool,
    pub release_outstanding: bool,
    pub halt_reason: Option<Halt>,
}

/// One offline session. No automatic retry, clock, persistence or work queue.
/// New process starts disarmed; cross-process same-slot durability belongs to
/// the actual owner's journal. Slot high-water survives ALL resets here.
pub struct Adapter {
    context: Option<Context>,
    epoch: Epoch,
    intent_counter: u64,
    station: Option<Station>,
    armed: bool,
    pending: Option<Pending>,
    release: Option<Arc<OfflineOperation>>,
    seen: Option<(u64, [u8; 32])>,
    reserved_until: u64,
    session_events: u32,
    diagnostics: Vec<String>,
    halt_reason: Option<Halt>,
}
impl Default for Adapter {
    fn default() -> Self {
        Self::new()
    }
}
impl Adapter {
    pub fn new() -> Self {
        Self {
            context: None,
            epoch: Epoch { context: 0, qso: 0 },
            intent_counter: 0,
            station: None,
            armed: false,
            pending: None,
            release: None,
            seen: None,
            reserved_until: 0,
            session_events: 0,
            diagnostics: Vec::new(),
            halt_reason: None,
        }
    }
    pub fn view(&self) -> View {
        let s = self.station.as_ref();
        View {
            epoch: self.epoch,
            armed: self.armed,
            state: s.map(|s| s.state),
            pending_text: s.and_then(Station::pending_text),
            tx_count: s.map_or(0, |s| s.tx_count),
            keyed: s.is_some_and(|s| s.keyed_on_air),
            closing_sent: s.is_some_and(Station::closing_sent),
            done: s.is_some_and(Station::done),
            release_outstanding: self.release_identity().is_some(),
            halt_reason: self.halt_reason,
        }
    }
    fn diagnostics(&mut self) {
        if let Some(s) = &mut self.station {
            for line in s.transcript.drain(..) {
                if self.diagnostics.len() == MAX_DIAGNOSTICS {
                    self.diagnostics.remove(0);
                }
                // Input limits bound upstream lines; retain a defensive byte cap
                // without splitting UTF-8 or modifying Station/QSO fields.
                let mut end = line.len().min(1024);
                while !line.is_char_boundary(end) {
                    end -= 1;
                }
                self.diagnostics.push(line[..end].to_string());
            }
        }
    }
    fn event(&mut self) -> Result<(), Error> {
        if !self.armed {
            return Err(Error::Disarmed);
        }
        if self.session_events >= MAX_SESSION_EVENTS {
            self.halt(Halt::Exhausted);
            return Err(Error::Exhausted);
        }
        self.session_events += 1;
        Ok(())
    }
    pub fn release_identity(&self) -> Option<Arc<OfflineOperation>> {
        self.pending
            .as_ref()
            .and_then(|p| p.operation.clone())
            .or_else(|| self.release.clone())
    }
    /// Retains the exact old identity even after the QSO/context is retired.
    pub fn halt(&mut self, reason: Halt) -> Option<Arc<OfflineOperation>> {
        if let Some(p) = self.pending.take() {
            if let Some(op) = p.operation {
                self.release = Some(op);
            }
        }
        self.armed = false;
        self.halt_reason = Some(reason);
        self.release.clone()
    }
    /// Explicit FAKE release completion, not an output command or hardware fact.
    pub fn acknowledge_fake_release(&mut self, operation: &OfflineOperation) -> Result<(), Error> {
        if self.pending.is_some() {
            return Err(Error::ReleaseOutstanding);
        }
        if self.release.as_deref() != Some(operation) {
            return Err(Error::WrongOperation);
        }
        self.release = None;
        Ok(())
    }
    pub fn begin(
        &mut self,
        context: Context,
        settings: OperatorSettings,
        intent: OperatorIntent,
    ) -> Result<Epoch, Error> {
        // Retire first; invalid/replacement operator input cannot leave old intent armed.
        self.halt(Halt::Stop);
        if self.release.is_some() {
            return Err(Error::ReleaseOutstanding);
        }
        context.validate()?;
        if !bounded(&settings.mycall, 32) || !bounded(&settings.mygrid, 8) {
            return Err(Error::InvalidInput);
        }
        let next_qso = self.epoch.qso.checked_add(1).ok_or(Error::Exhausted)?;
        let next_context = if self.context.as_ref() != Some(&context) {
            self.epoch.context.checked_add(1).ok_or(Error::Exhausted)?
        } else {
            self.epoch.context
        };
        let mut s = match intent {
            OperatorIntent::Monitor => Station::monitoring(&settings.mycall, &settings.mygrid),
            OperatorIntent::Cq => Station::calling_cq(&settings.mycall, &settings.mygrid),
            OperatorIntent::Call {
                dxcall,
                context,
                prefer_rrr,
                skip_tx1,
            } => {
                if !bounded(&dxcall, 32)
                    || context
                        .as_ref()
                        .is_some_and(|(m, _)| m.to_text().len() > MAX_TEXT)
                {
                    return Err(Error::InvalidInput);
                }
                Station::start(
                    &settings.mycall,
                    &settings.mygrid,
                    &dxcall,
                    context.as_ref().map(|(m, snr)| (m, *snr)),
                    prefer_rrr,
                    skip_tx1,
                )
            }
        };
        s.cq_call_cap = settings.cq_call_cap;
        s.call_cap = settings.call_cap;
        self.station = Some(s);
        self.context = Some(context);
        self.epoch = Epoch {
            context: next_context,
            qso: next_qso,
        };
        self.session_events = 0;
        self.armed = true;
        self.halt_reason = None;
        self.diagnostics();
        Ok(self.epoch)
    }
    pub fn observe(&mut self, batch: CompleteDecodeBatch) -> Result<(), Error> {
        self.event()?;
        if batch.epoch != self.epoch || self.context.as_ref() != Some(&batch.context) {
            return Err(Error::WrongContext);
        }
        let fingerprint = batch.fingerprint()?;
        if let Some((last, previous)) = self.seen {
            if batch.period_start_us < last {
                return Err(Error::OldPeriod);
            }
            if batch.period_start_us == last {
                return Err(if previous == fingerprint {
                    Error::DuplicatePeriod
                } else {
                    Error::ConflictingPeriod
                });
            }
        }
        // Busy means DROP, not defer. Even this rejected period is remembered.
        self.seen = Some((batch.period_start_us, fingerprint));
        if self.pending.is_some() {
            return Err(Error::BusyDropped);
        }
        self.station
            .as_mut()
            .ok_or(Error::Disarmed)?
            .observe(&batch.rows);
        self.diagnostics();
        Ok(())
    }
    pub fn prepare_slot(
        &mut self,
        slot_start_us: u64,
        latest_start_us: u64,
        format: Format,
        offset_hz: f32,
        limits: Limits,
    ) -> Result<PreparedIntent, Error> {
        self.event()?;
        if self.pending.is_some() {
            return Err(Error::ReleaseOutstanding);
        }
        if self.release.is_some() {
            return Err(Error::ReleaseOutstanding);
        }
        let context = self.context.clone().ok_or(Error::Disarmed)?;
        let duration = period_us(context.mode)?;
        let end = slot_start_us
            .checked_add(duration)
            .ok_or(Error::InvalidInput)?;
        if slot_start_us == 0
            || slot_start_us % duration != 0
            || latest_start_us < slot_start_us
            || latest_start_us >= end
        {
            return Err(Error::InvalidInput);
        }
        if slot_start_us < self.reserved_until {
            return Err(Error::SlotUsed);
        }
        let s = self.station.as_ref().ok_or(Error::Disarmed)?;
        let (message, rv) = s.outgoing_rv().ok_or(if s.stalled() {
            Error::BudgetSpent
        } else {
            Error::NoMessage
        })?;
        let frozen = s.clone();
        let Some(counter) = self.intent_counter.checked_add(1) else {
            self.halt(Halt::Exhausted);
            return Err(Error::Exhausted);
        };
        self.intent_counter = counter;
        self.reserved_until = end; // Failed encoding also consumes this slot.
        let period = match prepare(context.mode, &message.to_text(), format, offset_hz, limits) {
            Ok(p) => p,
            Err(e) => {
                self.halt(Halt::Preparation);
                return Err(Error::Preparation(e));
            }
        };
        let intent = PreparedIntent {
            correlation: Correlation {
                epoch: self.epoch,
                intent: counter,
                context,
                slot_start_us,
                slot_duration_us: duration,
                latest_start_us,
            },
            period: Arc::new(period),
            message,
            rv,
        };
        self.pending = Some(Pending {
            intent: intent.clone(),
            frozen,
            operation: None,
        });
        Ok(intent)
    }
    pub fn bind_offline(&mut self, operation: Arc<OfflineOperation>) -> Result<(), Error> {
        self.event()?;
        let pending = self.pending.as_mut().ok_or(Error::WrongOperation)?;
        if pending.operation.is_some()
            || operation.correlation != pending.intent.correlation
            || operation.pcm != pending.intent.period.pcm()
        {
            return Err(Error::WrongOperation);
        }
        pending.operation = Some(operation);
        Ok(())
    }
    pub fn non_start(
        &mut self,
        operation: &OfflineOperation,
        _event: NonStart,
    ) -> Result<(), Error> {
        self.event()?;
        if self.pending.as_ref().and_then(|p| p.operation.as_deref()) != Some(operation) {
            return Err(Error::WrongOperation);
        }
        Ok(())
    }
    pub fn fake_actual_start(&mut self, event: FakeActualStart) -> Result<(), Error> {
        self.event()?;
        let pending = self.pending.as_ref().ok_or(Error::WrongOperation)?;
        if pending.operation.as_deref() != Some(event.operation.as_ref()) {
            return Err(Error::WrongOperation);
        }
        let c = &pending.intent.correlation;
        if event.started_us < c.slot_start_us
            || event.started_us > c.latest_start_us
            || event.observed_us < event.started_us
            || event.observed_us > c.latest_start_us
        {
            self.halt(Halt::Expired);
            return Err(Error::Expired);
        }
        let mut pending = self.pending.take().ok_or(Error::WrongOperation)?;
        pending.frozen.after_tx();
        self.station = Some(pending.frozen);
        self.release = pending.operation;
        self.diagnostics();
        Ok(())
    }
    pub fn refuse_or_unknown(
        &mut self,
        operation: &OfflineOperation,
        unknown: bool,
    ) -> Result<Arc<OfflineOperation>, Error> {
        self.event()?;
        if self.release_identity().as_deref() != Some(operation) {
            return Err(Error::WrongOperation);
        }
        self.halt(if unknown {
            Halt::Unknown
        } else {
            Halt::Refused
        })
        .ok_or(Error::WrongOperation)
    }
    pub fn expire(&mut self, now_us: u64) -> Option<Arc<OfflineOperation>> {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| now_us > p.intent.correlation.latest_start_us)
        {
            self.halt(Halt::Expired)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
