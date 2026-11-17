// SPDX-License-Identifier: GPL-3.0-only
//! Native original FT controls. Network/output authority belongs to xLog.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) enum Mode {
    FT8,
    FT4,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Action {
    Cq {
        #[serde(default)]
        direction: Option<String>,
    },
    Work {
        decode_id: String,
    },
    Enable {
        enabled: bool,
    },
    Parity {
        even: bool,
    },
    AutoSequence {
        enabled: bool,
    },
    AutoCycle {
        enabled: bool,
    },
    Resend,
    Override {
        message: String,
        call: Option<String>,
        grid: Option<String>,
    },
    FreeText {
        message: String,
    },
    Offset {
        hz: f32,
    },
    Drive {
        gain: f32,
    },
    Hold {
        enabled: bool,
    },
    SkipTx1 {
        enabled: bool,
    },
    SearchAndPounce {
        enabled: bool,
    },
    Clear,
    Stop,
    Tune {
        enabled: bool,
    },
}
#[derive(Clone, Debug, Serialize)]
pub(crate) struct FtView {
    pub connection_token: String,
    pub available: bool,
    pub armed: bool,
    pub tuning: bool,
    pub tx_cycle_auto: bool,
    pub phase: String,
    pub reason: String,
    pub mode: Option<Mode>,
    pub tx_even: bool,
    pub auto_sequence: bool,
    pub hold: bool,
    pub skip_tx1: bool,
    pub search_and_pounce: bool,
    pub offset_hz: f32,
    pub gain: f32,
    pub pending_message: Option<String>,
    pub tx_count: u32,
    pub keyed: bool,
    pub done: bool,
    pub dx_call: Option<String>,
    pub operation_id: Option<String>,
    pub release_pending: bool,
    pub qso_state: String,
    pub dx_grid: Option<String>,
    pub rx_report: Option<i32>,
    pub cq_running: bool,
    pub stalled: bool,
    pub watchdog_stopped: bool,
    pub remaining_valid_ms: u64,
}
use crate::{xlog_decode::NativeDecodedPeriod, xlog_ft_output, xlog_ft_runtime as kernel};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, Weak,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tempo_core::message::{fmt_report, Msg};
use xnexus_xlog_client::{
    audio::AudioBatch,
    outbox::QsoDraft,
    tx::{wire, ContextLease, PreparedOperation, TxClient, ValidatedReceipt},
};
static OPERATION_HELD: AtomicBool = AtomicBool::new(false);
struct Reservation(bool);
impl Reservation {
    fn acquire() -> Result<Self, String> {
        OPERATION_HELD
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "Another exact release remains pending".to_string())?;
        Ok(Self(true))
    }
    fn published(mut self) {
        self.0 = false;
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if self.0 {
            OPERATION_HELD.store(false, Ordering::Release);
        }
    }
}
static IDS: AtomicU64 = AtomicU64::new(1);
fn id() -> Result<String, String> {
    let n = IDS
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| "FT identity exhausted")?;
    let t = wall_us()?;
    Ok(format!("ft-{:x}-{t:x}-{n:x}", std::process::id()))
}
fn uuid() -> Result<String, String> {
    let mut h = sha2::Sha256::new();
    use sha2::Digest;
    h.update(id()?);
    let b = h.finalize();
    Ok(format!("{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-4{:01x}{:02x}-8{:01x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",b[0],b[1],b[2],b[3],b[4],b[5],b[6]&15,b[7],b[8]&15,b[9],b[10],b[11],b[12],b[13],b[14],b[15]))
}
fn wall_us() -> Result<u64, String> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "Host clock unavailable")?
            .as_micros(),
    )
    .map_err(|_| "Host clock overflow".into())
}
fn lock<T>(m: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, String> {
    m.lock().map_err(|_| "FT runtime state unavailable".into())
}
fn error(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}
#[derive(Clone)]
struct Receive {
    period: Arc<NativeDecodedPeriod>,
    ids: Vec<Option<String>>,
    authority: Arc<AudioBatch>,
    epoch: u64,
}
#[derive(Clone)]
pub(crate) struct Feed(Weak<Inner>);
impl Feed {
    /// Called only after RX's post-DSP freshness/epoch check, before UI projection.
    /// One replaceable mailbox; no waiting, no Station or HTTP on decoder thread.
    pub(crate) fn publish(
        &self,
        period: Arc<NativeDecodedPeriod>,
        ids: Vec<Option<String>>,
        authority: Arc<AudioBatch>,
    ) {
        if let Some(i) = self.0.upgrade() {
            if i.connected.load(Ordering::Acquire) {
                if let Ok(mut q) = i.inbox.try_lock() {
                    *q = Some(Receive {
                        period,
                        ids,
                        authority,
                        epoch: i.epoch.load(Ordering::Acquire),
                    });
                }
            }
        }
    }
    pub(crate) fn touch(&self, batch: Arc<AudioBatch>) {
        if let Some(i) = self.0.upgrade() {
            if let Ok(mut a) = i.rx_authority.try_lock() {
                *a = Some(batch);
            }
        }
    }
    pub(crate) fn retire(&self) {
        if let Some(i) = self.0.upgrade() {
            i.request_stop();
            if i.rx_epoch
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
                .is_err()
            {
                i.connected.store(false, Ordering::Release);
            }
        }
    }
}
#[derive(Clone)]
struct Config {
    cycle_auto: bool,
    even: bool,
    auto: bool,
    hold: bool,
    skip: bool,
    sp: bool,
    offset: f32,
    gain: f32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            cycle_auto: true,
            even: true,
            auto: true,
            hold: false,
            skip: false,
            sp: false,
            offset: 1500.0,
            gain: 0.5,
        }
    }
}
struct SavedLog {
    facts: kernel::QsoFacts,
    key: kernel::Context,
    sent: Option<i32>,
    started_ms: u64,
}
struct Runtime {
    coordinator: Option<kernel::Coordinator>,
    key: Option<kernel::Context>,
    receive: Option<Receive>,
    // Selection-only original records; never replayed into the sequencer.
    retained: std::collections::VecDeque<(String, Arc<NativeDecodedPeriod>, usize)>,
    context: Option<Arc<ContextLease>>,
    config: Config,
    reason: String,
    phase: String,
    rx_epoch: u64,
    dx_started_ms: u64,
    log_id: Option<String>,
    request_id: Option<String>,
    sent: Option<i32>,
    sent_epoch: Option<u64>,
    logged_epoch: Option<String>,
    saved_log: Option<SavedLog>,
    completed: Option<QsoDraft>,
    last_terminal: Option<xnexus_xlog_client::tx::OperationKey>,
}
impl Runtime {
    fn retain_receive(&mut self, r: &Receive) {
        for (index, id) in r.ids.iter().enumerate() {
            if let Some(id) = id {
                if index < r.period.rows.len() && !self.retained.iter().any(|(old, _, _)| old == id)
                {
                    self.retained
                        .push_back((id.clone(), r.period.clone(), index));
                    while self.retained.len() > 200 {
                        self.retained.pop_front();
                    }
                }
            }
        }
    }
    fn retain_log(&mut self) {
        if let (Some(facts), Some(key)) = (
            self.coordinator.as_ref().and_then(|c| c.log_facts()),
            self.key.clone(),
        ) {
            let sent = if self.sent_epoch == Some(facts.epoch) {
                self.sent
            } else {
                None
            };
            // A sample-clock repair cannot create a second manual Log identity
            // for the same Station/QSO. Keep its original saved key.
            let key = self
                .saved_log
                .as_ref()
                .filter(|old| old.facts.epoch == facts.epoch && old.key.same_station(&key))
                .map(|old| old.key.clone())
                .unwrap_or(key);
            self.saved_log = Some(SavedLog {
                facts,
                key,
                sent,
                started_ms: self.dx_started_ms,
            });
        }
    }
}
struct Active {
    stop_client: Arc<xnexus_xlog_client::tx::TxStopClient>,
    query_ready: bool,
    prepare_error: Option<String>,
    operation: Arc<PreparedOperation>,
    native: Option<kernel::Operation>,
    stop_event: String,
    start_delivered: bool,
    terminal: bool,
}
struct Inner {
    client: Arc<TxClient>,
    client_id: String,
    context_lane: Mutex<()>,
    token: String,
    connected: AtomicBool,
    epoch: AtomicU64,
    rx_epoch: AtomicU64,
    stop_requested: AtomicBool,
    future_disarmed: AtomicBool,
    stop: Mutex<Option<kernel::StopHandle>>,
    state: Mutex<Runtime>,
    active: Mutex<Option<Active>>,
    inbox: Mutex<Option<Receive>>,
    rx_authority: Mutex<Option<Arc<AudioBatch>>>,
    preparing: AtomicBool,
    acting: AtomicBool,
}
pub(crate) struct Host {
    inner: Arc<Inner>,
}
impl Drop for Host {
    fn drop(&mut self) {
        self.revoke();
    }
}
impl Inner {
    fn request_stop(&self) {
        // Admission and Stop share only this short cell, never HTTP or state.
        // A late action must not overwrite a newer urgent Stop latch.
        let guard = self.stop.lock();
        if self
            .epoch
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
            .is_err()
        {
            self.connected.store(false, Ordering::Release);
        }
        self.stop_requested.store(true, Ordering::Release);
        if let Ok(s) = guard {
            if let Some(s) = s.as_ref() {
                let _ = s.stop();
            }
        } else {
            self.connected.store(false, Ordering::Release);
        }
    }
    fn admit(&self, epoch: u64) -> Result<(), String> {
        let _guard = lock(&self.stop)?;
        if epoch != self.epoch.load(Ordering::Acquire) || !self.connected.load(Ordering::Acquire) {
            return Err("FT intent cancelled".into());
        }
        self.future_disarmed.store(false, Ordering::Release);
        self.stop_requested.store(false, Ordering::Release);
        Ok(())
    }
    fn fail(&self, reason: String) {
        self.request_stop();
        if let Ok(mut s) = self.state.lock() {
            s.reason = reason;
            s.phase = "stopped".into();
        }
    }
    fn current(&self) -> Result<Arc<ContextLease>, String> {
        if !self.connected.load(Ordering::Acquire) {
            return Err("FT connection retired".into());
        }
        let c = lock(&self.state)?
            .context
            .clone()
            .ok_or("Waiting for xLog TX context")?;
        if !c.is_fresh() {
            return Err("xLog TX context expired".into());
        }
        Ok(c)
    }
    // Called under context_lane by an explicit operator action. A cached view
    // is not authority for refusing an operator intent: read the owner now.
    fn action_context(&self) -> Result<Arc<ContextLease>, String> {
        if !self.connected.load(Ordering::Acquire) {
            return Err("FT connection retired".into());
        }
        let c = Arc::new(self.client.context().map_err(error)?);
        let mut s = lock(&self.state)?;
        if s.context.as_ref().is_some_and(|old| {
            old.context().binding != c.context().binding
                || old.context().identity != c.context().identity
                || old.context().route != c.context().route
        }) {
            self.request_stop();
            s.retain_log();
            s.coordinator = None;
            s.key = None;
            s.receive = None;
            s.retained.clear();
        }
        s.context = Some(c.clone());
        Ok(c)
    }
}
impl Host {
    pub(crate) fn attach(
        client: &xnexus_xlog_client::BridgeClient,
        token: String,
    ) -> Result<Self, String> {
        let client_id = uuid()?;
        let client = client.tx_handle(&client_id).map_err(error)?;
        Self::start(client, client_id, token)
    }
    fn start(client: TxClient, client_id: String, token: String) -> Result<Self, String> {
        let i = allocate(client, client_id, token);
        let heartbeat = i.clone();
        std::thread::Builder::new()
            .name("xlog-ft-heartbeat".into())
            .spawn(move || heartbeat_loop(heartbeat))
            .map_err(error)?;
        let control = i.clone();
        if let Err(e) = std::thread::Builder::new()
            .name("xlog-ft-control".into())
            .spawn(move || control_loop(control))
        {
            i.connected.store(false, Ordering::Release);
            return Err(error(e));
        }
        let release = i.clone();
        if let Err(e) = std::thread::Builder::new()
            .name("xlog-ft-release".into())
            .spawn(move || release_loop(release))
        {
            i.connected.store(false, Ordering::Release);
            return Err(error(e));
        }
        Ok(Self { inner: i })
    }
    pub(crate) fn token(&self) -> &str {
        &self.inner.token
    }
    pub(crate) fn has_release(&self) -> bool {
        lock(&self.inner.active).map_or(true, |a| a.is_some())
    }
    pub(crate) fn feed(&self) -> Feed {
        Feed(Arc::downgrade(&self.inner))
    }
    pub(crate) fn revoke(&self) {
        self.inner.request_stop();
        self.inner.connected.store(false, Ordering::Release);
    }
    pub(crate) fn stop(&self) {
        self.inner.request_stop();
    }
    pub(crate) fn disarm_future(&self) -> Result<(), String> {
        {
            let _guard = lock(&self.inner.stop)?;
            if self
                .inner
                .epoch
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
                .is_err()
            {
                self.inner.connected.store(false, Ordering::Release);
                return Err("FT intent epoch exhausted".into());
            }
            self.inner.future_disarmed.store(true, Ordering::Release);
        }
        if let Some(c) = lock(&self.inner.state)?.coordinator.as_mut() {
            c.apply(kernel::Intent::Enable(false), Instant::now())
                .map_err(error)?;
        }
        Ok(())
    }
    pub(crate) fn view(&self) -> Result<FtView, String> {
        let s = lock(&self.inner.state)?;
        let v = s
            .coordinator
            .as_ref()
            .map(|c| c.view())
            .transpose()
            .map_err(error)?;
        let active = lock(&self.inner.active)?;
        let op = active
            .as_ref()
            .map(|a| a.operation.key().operation_id.clone());
        let remain = s
            .context
            .as_ref()
            .map_or(0, |c| c.remaining_valid_ms().min(1000));
        let available = self.inner.connected.load(Ordering::Acquire)
            && remain > 0
            && s.context
                .as_ref()
                .is_some_and(|c| c.context().ready && c.context().local_enabled);
        Ok(FtView {
            connection_token: self.inner.token.clone(),
            available,
            tuning: active.as_ref().is_some_and(|a| a.native.is_none()),
            tx_cycle_auto: s.config.cycle_auto,
            armed: available && v.as_ref().is_some_and(|v| v.armed),
            phase: s.phase.clone(),
            reason: if let Some(reason) = active.as_ref().and_then(|a| a.prepare_error.clone()) {
                reason
            } else if available {
                s.reason.clone()
            } else {
                s.context
                    .as_ref()
                    .and_then(|c| c.context().reason.clone())
                    .unwrap_or_else(|| s.reason.clone())
            },
            mode: s.receive.as_ref().map(|r| match r.period.context.mode {
                crate::xlog_decode::DecoderMode::Ft8 => Mode::FT8,
                _ => Mode::FT4,
            }),
            tx_even: s.config.even,
            auto_sequence: s.config.auto,
            hold: s.config.hold,
            skip_tx1: s.config.skip,
            search_and_pounce: s.config.sp,
            offset_hz: s.config.offset,
            gain: s.config.gain,
            pending_message: v.as_ref().and_then(|v| v.pending_text.clone()),
            tx_count: v.as_ref().map_or(0, |v| v.tx_count),
            keyed: v.as_ref().is_some_and(|v| v.keyed),
            done: v.as_ref().is_some_and(|v| v.done),
            dx_call: v
                .as_ref()
                .and_then(|v| v.dx_call.clone())
                .or_else(|| s.saved_log.as_ref().map(|q| q.facts.call.clone())),
            operation_id: op.clone(),
            release_pending: op.is_some(),
            qso_state: v
                .as_ref()
                .map_or("Listening".into(), |v| format!("{:?}", v.state)),
            dx_grid: v.as_ref().and_then(|v| v.dx_grid.clone()),
            rx_report: v.as_ref().and_then(|v| v.rx_report),
            cq_running: v.as_ref().is_some_and(|v| v.cq_running),
            stalled: v.as_ref().is_some_and(|v| v.stalled),
            watchdog_stopped: v
                .as_ref()
                .is_some_and(|v| v.halt == Some(kernel::Halt::Expired)),
            remaining_valid_ms: if available { remain } else { 0 },
        })
    }
    fn retune(&self, gain: f32) -> Result<FtView, String> {
        let binding = self.inner.current()?.context().binding.clone();
        lock(&self.inner.state)?.config.gain = gain;
        self.inner.future_disarmed.store(true, Ordering::Release);
        self.inner.request_stop();
        let epoch = self.inner.epoch.load(Ordering::Acquire);
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if epoch != self.inner.epoch.load(Ordering::Acquire)
                || !self.inner.connected.load(Ordering::Acquire)
            {
                return Err("Tune drive restart cancelled; exact release retained".into());
            }
            if lock(&self.inner.active)?.is_none() {
                break;
            }
            if Instant::now() >= deadline {
                return Err("Tune release not observed; no replacement started".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        {
            let _lane = lock(&self.inner.context_lane)?;
            let c = self.inner.current()?;
            if c.context().binding != binding {
                return Err("Tune context changed".into());
            }
            let c = self
                .inner
                .client
                .enable(&c, &uuid()?, true)
                .map_err(error)?;
            if !c.is_fresh() || !c.context().client_enabled || c.context().binding != binding {
                return Err("Tune re-enable refused".into());
            }
            lock(&self.inner.state)?.context = Some(Arc::new(c));
            self.inner.admit(epoch)?;
        }
        start_tune(&self.inner, epoch)?;
        self.view()
    }
    pub(crate) fn action(&self, action: Action) -> Result<FtView, String> {
        if matches!(action, Action::Enable { enabled: false }) {
            self.disarm_future()?;
            return self.view();
        }
        if matches!(action,Action::Drive{gain} if gain==0.0) {
            self.stop();
            lock(&self.inner.state)?.config.gain = 0.0;
            return self.view();
        }
        if matches!(
            action,
            Action::Stop | Action::Tune { enabled: false } | Action::Clear
        ) {
            self.stop();
            let mut s = lock(&self.inner.state)?;
            if matches!(action, Action::Clear) {
                s.retain_log();
                s.coordinator = None;
                s.key = None;
            }
            drop(s);
            return self.view();
        }
        if self.inner.acting.swap(true, Ordering::AcqRel) {
            return Err("FT action already in progress".into());
        }
        struct Acting<'a>(&'a AtomicBool);
        impl Drop for Acting<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release)
            }
        }
        let _acting = Acting(&self.inner.acting);
        if let Action::Drive { gain } = action {
            if !gain.is_finite() || gain <= 0.0 || gain > 0.8 {
                return Err("Invalid Tune drive".into());
            }
            if lock(&self.inner.active)?
                .as_ref()
                .is_some_and(|a| a.native.is_none())
            {
                return self.retune(gain);
            }
        }
        let epoch = self.inner.epoch.load(Ordering::Acquire);
        let arms = matches!(
            action,
            Action::Cq { .. }
                | Action::Work { .. }
                | Action::Enable { enabled: true }
                | Action::Resend
                | Action::Override { .. }
                | Action::FreeText { .. }
                | Action::Tune { enabled: true }
        );
        if arms && lock(&self.inner.state)?.config.gain == 0.0 {
            return Err("Drive is zero; choose a positive gain before enabling TX".into());
        }
        if arms {
            let _lane = lock(&self.inner.context_lane)?;
            let lease = self.inner.action_context()?;
            let enabled = self
                .inner
                .client
                .enable(&lease, &uuid()?, true)
                .map_err(error)?;
            if !enabled.is_fresh() || !enabled.context().client_enabled {
                return Err("xLog did not enable this TX client".into());
            }
            if epoch != self.inner.epoch.load(Ordering::Acquire) {
                return Err("FT intent cancelled".into());
            }
            lock(&self.inner.state)?.context = Some(Arc::new(enabled));
            // Publish the requested latch before releasing the context command lane.
            // Heartbeat must not undo an enable between HTTP and local intent admission.
            self.inner.future_disarmed.store(false, Ordering::Release);
        } else {
            let _lane = lock(&self.inner.context_lane)?;
            self.inner.action_context()?;
        }
        if matches!(action, Action::Tune { enabled: true }) {
            start_tune(&self.inner, epoch)?;
            return self.view();
        }
        let mut s = lock(&self.inner.state)?;
        if epoch != self.inner.epoch.load(Ordering::Acquire) {
            return Err("FT intent cancelled".into());
        }
        if lock(&self.inner.active)?.is_some() || self.inner.preparing.load(Ordering::Acquire) {
            return Err("Stop the current operation before changing its intent".into());
        }
        if let (Some(key), Some(current)) = (s.key.clone(), s.context.clone()) {
            let remaining = current.remaining_valid_ms();
            if remaining == 0 {
                return Err("Owner TX context expired".into());
            }
            if let Some(c) = s.coordinator.as_mut() {
                c.refresh(
                    &key,
                    Instant::now() + Duration::from_millis(remaining),
                    Instant::now(),
                )
                .map_err(error)?;
            }
        }
        let intent = match action {
            Action::Offset { hz } => {
                if !hz.is_finite() || !(100.0..=4900.0).contains(&hz) {
                    return Err("Offset outside qualified FT passband".into());
                }
                s.config.offset = hz;
                None
            }
            Action::Drive { gain } => {
                if !gain.is_finite() || gain <= 0.0 || gain > 0.8 {
                    return Err("Drive gain must be greater than0 and at most0.8".into());
                }
                s.config.gain = gain;
                None
            }
            Action::Hold { enabled } => {
                s.config.hold = enabled;
                None
            }
            Action::SkipTx1 { enabled } => {
                s.config.skip = enabled;
                None
            }
            Action::SearchAndPounce { enabled } => {
                s.config.sp = enabled;
                if enabled
                    && s.coordinator
                        .as_ref()
                        .is_some_and(|c| c.view().is_ok_and(|v| v.cq_running))
                {
                    self.inner.request_stop();
                }
                None
            }
            Action::AutoCycle { enabled } => {
                s.config.cycle_auto = enabled;
                None
            }
            Action::Parity { even } => {
                s.config.cycle_auto = false;
                s.config.even = even;
                Some(kernel::Intent::Parity(even))
            }
            Action::AutoSequence { enabled } => {
                s.config.auto = enabled;
                Some(kernel::Intent::AutoSequence(enabled))
            }
            Action::Cq { direction } => {
                if s.config.sp {
                    return Err("Leave S&P before calling CQ".into());
                }
                Some(direction.map_or(kernel::Intent::Cq, kernel::Intent::CqDirected))
            }
            Action::Work { decode_id } => {
                let r = s
                    .receive
                    .as_ref()
                    .ok_or("No current native decoded period")?;
                if !current_work_route(&self.inner, r) {
                    return Err("Selected decode authority expired".into());
                }
                if s.rx_epoch != self.inner.rx_epoch.load(Ordering::Acquire) {
                    return Err("Selected receive context retired".into());
                }
                let (_, period, n) = s
                    .retained
                    .iter()
                    .find(|(id, _, _)| id == &decode_id)
                    .ok_or("Selected decode is no longer retained")?;
                let period = period.clone();
                let current = s.key.as_ref().ok_or("No current station context")?;
                let mut selected = current.clone();
                selected.receive = period.context.clone();
                selected.owner = period.owner.clone();
                if !current.same_station(&selected) {
                    return Err("Selected decode context changed".into());
                }
                let row = period.rows.get(*n).ok_or("Selected decode unavailable")?;
                let msg = Msg::parse(&row.message);
                let snr = row.snr;
                let freq = row.freq;
                let period_us = period.context.mode.period_ms() * 1000;
                let period = period.host_period_start_unix_us;
                let call = msg
                    .sender()
                    .ok_or("Selected decode has no explicit sender")?
                    .to_owned();
                if !s.config.hold {
                    if !freq.is_finite() || !(100.0..=4900.0).contains(&freq) {
                        return Err("Selected audio offset is outside qualified TX range".into());
                    }
                    s.config.offset = freq;
                }
                if s.config.cycle_auto {
                    let even = period / period_us % 2 != 0;
                    s.config.even = even;
                    if let Some(c) = s.coordinator.as_mut() {
                        c.apply(kernel::Intent::Parity(even), Instant::now())
                            .map_err(error)?;
                    }
                }
                Some(kernel::Intent::Work {
                    call,
                    heard: Some((msg, snr)),
                    prefer_rrr: false,
                    skip_tx1: s.config.skip,
                })
            }
            Action::Enable { enabled } => Some(kernel::Intent::Enable(enabled)),
            Action::Resend => Some(kernel::Intent::Resend),
            Action::Override {
                message,
                call,
                grid,
            } => {
                if let Some(call) = call {
                    s.coordinator
                        .as_mut()
                        .ok_or("Wait for current receive context")?
                        .manual_target(&call, grid.as_deref(), Instant::now())
                        .map_err(error)?;
                    s.log_id = Some(uuid()?);
                    s.request_id = Some(uuid()?);
                    s.dx_started_ms = wall_us()? / 1000;
                    s.sent = None;
                    s.sent_epoch = None;
                    s.saved_log = None;
                    s.completed = None;
                }
                Some(kernel::Intent::Override(message))
            }
            Action::FreeText { message } => Some(kernel::Intent::FreeText(message)),
            _ => None,
        };
        if let Some(intent) = intent {
            let newqso = matches!(
                intent,
                kernel::Intent::Cq | kernel::Intent::CqDirected(_) | kernel::Intent::Work { .. }
            );
            s.coordinator
                .as_mut()
                .ok_or("Wait for one complete receive period")?
                .apply(intent, Instant::now())
                .map_err(error)?;
            if newqso {
                s.log_id = Some(uuid()?);
                s.request_id = Some(uuid()?);
                s.dx_started_ms = wall_us()? / 1000;
                s.sent = None;
                s.completed = None;
                s.saved_log = None;
                s.sent_epoch = None;
            }
        }
        if arms {
            self.inner.admit(epoch)?;
            s.reason.clear();
            s.phase = "armed".into();
        }
        drop(s);
        self.view()
    }
}
fn allocate(client: TxClient, client_id: String, token: String) -> Arc<Inner> {
    Arc::new(Inner {
        client: Arc::new(client),
        client_id,
        context_lane: Mutex::new(()),
        token,
        connected: AtomicBool::new(true),
        epoch: AtomicU64::new(1),
        rx_epoch: AtomicU64::new(1),
        stop_requested: AtomicBool::new(false),
        future_disarmed: AtomicBool::new(true),
        stop: Mutex::new(None),
        active: Mutex::new(None),
        inbox: Mutex::new(None),
        rx_authority: Mutex::new(None),
        preparing: AtomicBool::new(false),
        acting: AtomicBool::new(false),
        state: Mutex::new(Runtime {
            coordinator: None,
            key: None,
            receive: None,
            retained: std::collections::VecDeque::new(),
            context: None,
            config: Config::default(),
            reason: "Waiting for xLog TX context and a complete receive period".into(),
            phase: "off".into(),
            rx_epoch: 1,
            dx_started_ms: 0,
            log_id: None,
            request_id: None,
            sent: None,
            sent_epoch: None,
            logged_epoch: None,
            saved_log: None,
            completed: None,
            last_terminal: None,
        }),
    })
}
fn heartbeat_loop(i: Arc<Inner>) {
    while i.connected.load(Ordering::Acquire) {
        let before = Instant::now();
        let Ok(_lane) = i.context_lane.try_lock() else {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        };
        let previous = lock(&i.state).ok().and_then(|s| s.context.clone());
        let result = match previous.as_ref() {
            Some(c)
                if c.is_fresh()
                    && c.context().client_enabled
                    && (i.stop_requested.load(Ordering::Acquire)
                        || i.future_disarmed.load(Ordering::Acquire)) =>
            {
                uuid().and_then(|e| i.client.enable(c, &e, false).map_err(error))
            }
            Some(c)
                if c.is_fresh()
                    && (c.context().client_enabled
                        || lock(&i.active).is_ok_and(|a| a.is_some())) =>
            {
                i.client.heartbeat(c).map_err(error)
            }
            _ => i.client.context().map_err(error),
        };
        match result {
            Ok(c) => {
                let changed = previous.as_ref().is_some_and(|old| {
                    old.context().binding != c.context().binding
                        || old.context().identity != c.context().identity
                        || old.context().route != c.context().route
                });
                if changed
                    || !c.context().ready
                    || !c.context().local_enabled
                    || previous.as_ref().is_some_and(|old| {
                        old.context().client_enabled
                            && !c.context().client_enabled
                            && !i.future_disarmed.load(Ordering::Acquire)
                    })
                {
                    i.request_stop();
                }
                if let Ok(mut s) = i.state.lock() {
                    if changed {
                        s.retain_log();
                        s.coordinator = None;
                        s.key = None;
                        s.receive = None;
                        s.retained.clear();
                        s.reason = "Owner TX context changed; explicit intent required".into();
                    }
                    if !changed {
                        if let (Some(key), Some(coordinator)) =
                            (s.key.clone(), s.coordinator.as_mut())
                        {
                            let _ = coordinator.refresh(
                                &key,
                                Instant::now() + Duration::from_millis(c.remaining_valid_ms()),
                                Instant::now(),
                            );
                        }
                    }
                    s.context = Some(Arc::new(c));
                }
            }
            Err(e) => i.fail(format!("TX context unavailable: {e}")),
        }
        drop(_lane);
        let delay = Duration::from_millis(200).saturating_sub(before.elapsed());
        std::thread::sleep(delay);
    }
}
fn kernel_context(i: &Inner, c: &ContextLease, r: &Receive) -> Result<kernel::Context, String> {
    let c = c.context();
    let identity = c
        .identity
        .as_ref()
        .ok_or("Saved owner identity unavailable")?;
    let route = c
        .route
        .as_ref()
        .ok_or("Commissioned TX route unavailable")?;
    if c.binding.server_instance != r.period.owner.server_instance
        || c.binding.workspace_id != r.period.owner.workspace_id
        || route.radio_id != r.period.owner.radio_id
        || route.receiver_id != r.period.owner.receiver_id
        || route.frequency_hz != r.period.owner.frequency_hz
    {
        return Err("Receive and commissioned TX contexts differ".into());
    }
    Ok(kernel::Context {
        connection_token: i.token.clone(),
        station_identity_epoch: format!(
            "{}:{}:{}",
            identity.identity_epoch, identity.identity_revision, identity.settings_revision
        ),
        transmit_context: c.binding.context_id.clone(),
        clock_epoch: c.binding.clock_epoch.clone(),
        receive: r.period.context.clone(),
        owner: r.period.owner.clone(),
    })
}
fn consume(i: &Arc<Inner>) -> Result<(), String> {
    // enable/heartbeat may invalidate the old SDK lease before its replacement
    // is published. Skip this observation, without dropping the queued period
    // or cancelling intent. This never blocks the independent Stop/release lane.
    let Ok(_lane) = i.context_lane.try_lock() else {
        return Ok(());
    };
    let incoming = lock(&i.inbox)?.take();
    let now = Instant::now();
    let mut s = lock(&i.state)?;
    if s.receive.as_ref().is_some_and(|r| !same_rx_route(i, r)) {
        i.request_stop();
        s.retain_log();
        s.coordinator = None;
        s.key = None;
        s.receive = None;
        s.retained.clear();
    }
    let epoch = i.rx_epoch.load(Ordering::Acquire);
    if epoch != s.rx_epoch {
        s.retain_log();
        s.coordinator = None;
        s.key = None;
        s.receive = None;
        s.retained.clear();
        s.rx_epoch = epoch;
    }
    let Some(c) = s.context.clone().filter(|c| c.is_fresh()) else {
        i.request_stop();
        return Ok(());
    };
    if let Some(r) = incoming {
        if r.epoch != i.epoch.load(Ordering::Acquire) || !r.authority.is_fresh() {
            return Ok(());
        }
        let key = kernel_context(i, &c, &r)?;
        if s.key
            .as_ref()
            .is_some_and(|old| old != &key && old.same_station(&key))
        {
            if let Some(coordinator) = s.coordinator.as_mut() {
                coordinator.repair_receive_stream(&key).map_err(error)?;
                s.key = Some(key.clone());
            }
        }
        if s.key.as_ref() != Some(&key) {
            if s.coordinator.is_some() {
                i.request_stop();
            }
            let ident = c
                .context()
                .identity
                .as_ref()
                .ok_or("Owner identity missing")?;
            let settings = kernel::Settings {
                call: ident.mycall.clone(),
                grid: ident.mygrid.clone(),
                tx_even: s.config.even,
                auto_sequence: s.config.auto,
                cq_call_cap: None,
                call_cap: Some(8),
                cq_pause: Duration::ZERO,
                watchdog: Duration::from_secs(360),
                disable_after_73: true,
            };
            let coordinator = kernel::Coordinator::new(
                key.clone(),
                settings,
                now + Duration::from_millis(c.remaining_valid_ms()),
                now,
            )
            .map_err(error)?;
            *lock(&i.stop)? = Some(coordinator.stop_handle());
            s.coordinator = Some(coordinator);
            s.key = Some(key);
            s.retained.clear();
        }
        if let Some(coordinator) = s.coordinator.as_mut() {
            match coordinator.observe(&r.period, now) {
                Ok(()) | Err(kernel::Error::Busy | kernel::Error::OldPeriod) => {}
                Err(e) => return Err(error(e)),
            }
        }
        s.retain_receive(&r);
        s.receive = Some(r);
        s.retain_log();
    }
    if let (Some(key), Some(coordinator)) = (s.key.clone(), s.coordinator.as_mut()) {
        coordinator
            .refresh(
                &key,
                now + Duration::from_millis(c.remaining_valid_ms()),
                now,
            )
            .map_err(error)?;
    }
    Ok(())
}
struct CachedEvent(kernel::OwnerState);
impl kernel::OwnerEvents for CachedEvent {
    fn state(
        &self,
        _: &kernel::Operation,
        _: Instant,
    ) -> Result<kernel::OwnerState, kernel::Error> {
        Ok(match &self.0 {
            kernel::OwnerState::Started(s) => kernel::OwnerState::Started(kernel::Started {
                event_id: s.event_id.clone(),
                started_us: s.started_us,
            }),
            kernel::OwnerState::Pending => kernel::OwnerState::Pending,
            kernel::OwnerState::NoStart => kernel::OwnerState::NoStart,
            kernel::OwnerState::ObservedReleased => kernel::OwnerState::ObservedReleased,
            kernel::OwnerState::Refused => kernel::OwnerState::Refused,
            kernel::OwnerState::Unknown => kernel::OwnerState::Unknown,
        })
    }
}
/// ONLY sealed SDK proof reaches this function. A public wire Receipt is never
/// accepted as authority; terminal-with-start accounts start before release.
fn apply_proof(i: &Inner, proof: ValidatedReceipt) -> Result<(), String> {
    let mut s = lock(&i.state)?;
    let mut active = lock(&i.active)?;
    if s.last_terminal.as_ref() == Some(proof.key()) {
        // A retained sealed observation may race a newer status/Stop completion.
        // It cannot retire or advance a replacement operation.
        return Ok(());
    }
    let a = active.as_mut().ok_or("No retained operation")?;
    if proof.key() != a.operation.key() {
        return Err("Owner receipt identity mismatch".into());
    }
    let r = proof.receipt();
    let terminal = matches!(r.phase, wire::Phase::Terminal);
    if a.native.is_some() {
        if let Some(c) = s.coordinator.as_mut() {
            if !a.start_delivered {
                if let Some(at) = r.actual_start_unix_us {
                    // Cancelled/context-retired coordinators cannot advance from late facts.
                    if i.connected.load(Ordering::Acquire)
                        && !i.stop_requested.load(Ordering::Acquire)
                    {
                        c.owner_event(
                            &CachedEvent(kernel::OwnerState::Started(kernel::Started {
                                event_id: r.event_id.clone(),
                                started_us: at,
                            })),
                            Instant::now(),
                        )
                        .map_err(error)?;
                    }
                    a.start_delivered = true;
                }
            }
            if terminal {
                match r.outcome {
                    wire::Outcome::NoStart => {
                        c.owner_event(&CachedEvent(kernel::OwnerState::NoStart), Instant::now())
                            .map_err(error)?;
                    }
                    wire::Outcome::ReleaseObserved => {
                        c.owner_event(
                            &CachedEvent(kernel::OwnerState::ObservedReleased),
                            Instant::now(),
                        )
                        .map_err(error)?;
                    }
                    wire::Outcome::Uncertain => {
                        c.retire(kernel::Halt::Unknown).map_err(error)?;
                    }
                }
            }
        }
    }
    if r.actual_start_unix_us.is_some() {
        if let Some(native) = a.native.as_ref() {
            if let Some(c) = s.coordinator.as_ref() {
                let v = c.view().map_err(error)?;
                if v.keyed {
                    if let Some(text) = a
                        .operation
                        .descriptor()
                        .map(|d| d.manifest.message.as_str())
                    {
                        if let Msg::Report { snr, .. } | Msg::RReport { snr, .. } = Msg::parse(text)
                        {
                            s.sent = Some(snr);
                            s.sent_epoch = Some(native.key.qso_epoch);
                        }
                    }
                    let _ = native;
                }
            }
        }
    }
    s.retain_log();
    s.phase = format!("{:?}", r.phase).to_lowercase();
    s.reason = r.reason.clone().unwrap_or_default();
    if terminal
        && matches!(
            r.outcome,
            wire::Outcome::NoStart | wire::Outcome::ReleaseObserved
        )
    {
        a.terminal = true;
        s.last_terminal = Some(proof.key().clone());
        *active = None;
        OPERATION_HELD.store(false, Ordering::Release);
    } else if terminal {
        s.reason = "Owner release is uncertain; exact Stop remains pending".into();
        drop(active);
        drop(s);
        // Unknown release cancels a user drive-change restart as well as scheduling.
        i.request_stop();
    }
    Ok(())
}
fn control_loop(i: Arc<Inner>) {
    while i.connected.load(Ordering::Acquire) {
        if let Err(e) = consume(&i) {
            i.fail(e)
        }
        let op = lock(&i.active).ok().and_then(|a| {
            a.as_ref()
                .filter(|a| a.query_ready)
                .map(|a| a.operation.clone())
        });
        if let Some(op) = op {
            match i.client.status(&op) {
                Ok(Some(p)) => {
                    if let Err(e) = apply_proof(&i, p) {
                        i.fail(e)
                    }
                }
                Ok(None) => i.fail("Owner receipt missing; exact release remains pending".into()),
                Err(e) => i.fail(format!("TX receipt unavailable: {e:?}")),
            }
        }
        if !i.stop_requested.load(Ordering::Acquire) && !i.preparing.load(Ordering::Acquire) {
            if let Err(e) = maybe_prepare(&i) {
                i.fail(e)
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
fn release_loop(i: Arc<Inner>) {
    loop {
        if i.stop_requested.load(Ordering::Acquire) {
            let op = lock(&i.active).ok().and_then(|a| {
                a.as_ref()
                    .map(|a| (a.stop_client.clone(), a.stop_event.clone()))
            });
            if let Some((stop, event)) = op {
                match stop.stop(&event) {
                    Ok(p) => {
                        let _ = apply_proof(&i, p);
                    }
                    Err(e) => {
                        if let Ok(mut s) = i.state.lock() {
                            s.reason = format!("Exact Stop unresolved: {e:?}");
                        }
                    }
                }
            }
        }
        if !i.connected.load(Ordering::Acquire)
            && lock(&i.active).is_ok_and(|a| a.is_none())
            && !i.preparing.load(Ordering::Acquire)
            && !i.acting.load(Ordering::Acquire)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}
fn maybe_prepare(i: &Arc<Inner>) -> Result<(), String> {
    if OPERATION_HELD.load(Ordering::Acquire)
        || i.future_disarmed.load(Ordering::Acquire)
        || i.acting.load(Ordering::Acquire)
    {
        return Ok(());
    }
    let now = Instant::now();
    let wall = wall_us()?;
    let mut s = lock(&i.state)?;
    let Some(c) = s
        .context
        .clone()
        .filter(|c| c.is_fresh() && c.context().ready && c.context().client_enabled)
    else {
        return Ok(());
    };
    let cfg = s.config.clone();
    let Some(coordinator) = s.coordinator.as_mut() else {
        return Ok(());
    };
    let v = coordinator.view().map_err(error)?;
    if !v.armed {
        return Ok(());
    }
    let period = match s.key.as_ref().ok_or("Missing native context")?.receive.mode {
        crate::xlog_decode::DecoderMode::Ft8 => 15_000_000,
        _ => 7_500_000,
    };
    let mut slot = (wall / period + 1) * period;
    if slot / period % 2 != u64::from(!cfg.even) {
        slot += period;
    }
    if slot - wall > 10_000_000 || slot - wall < 2_000_000 {
        return Ok(());
    }
    let latest = slot + 50_000;
    let plan = match s
        .coordinator
        .as_mut()
        .unwrap()
        .plan(slot, latest, wall, now)
    {
        Ok(p) => p,
        Err(
            kernel::Error::WrongParity
            | kernel::Error::SlotUsed
            | kernel::Error::BudgetSpent
            | kernel::Error::NoMessage
            | kernel::Error::Busy,
        ) => return Ok(()),
        Err(e) => return Err(error(e)),
    };
    if i.preparing.swap(true, Ordering::AcqRel) {
        return Ok(());
    }
    let epoch = i.epoch.load(Ordering::Acquire);
    drop(s);
    let worker = i.clone();
    if let Err(e) = std::thread::Builder::new()
        .name("xlog-ft-prepare".into())
        .spawn(move || {
            let result = prepare_operation(&worker, plan, c, cfg, epoch);
            if let Err(e) = result {
                worker.fail(e)
            }
            worker.preparing.store(false, Ordering::Release);
        })
    {
        i.preparing.store(false, Ordering::Release);
        return Err(error(e));
    }
    Ok(())
}
fn prepare_operation(
    i: &Inner,
    plan: kernel::Plan,
    _captured: Arc<ContextLease>,
    cfg: Config,
    epoch: u64,
) -> Result<(), String> {
    let result = prepare_operation_inner(i, &plan, cfg, epoch);
    if result.is_err() {
        if let Some(c) = lock(&i.state)?.coordinator.as_mut() {
            c.discard_unbound(&plan).map_err(error)?;
        }
    }
    result
}
fn prepare_operation_inner(
    i: &Inner,
    plan: &kernel::Plan,
    cfg: Config,
    epoch: u64,
) -> Result<(), String> {
    let source = plan.prepare(cfg.offset).map_err(error)?;
    let output = xlog_ft_output::prepare(&source, cfg.gain, plan.deadline()).map_err(error)?;
    if epoch != i.epoch.load(Ordering::Acquire) {
        return Err("Preparation cancelled".into());
    }
    let c = i
        .current()
        .map_err(|e| format!("Fresh owner context: {e}"))?;
    if c.context().binding.context_id != plan.key().context.transmit_context
        || c.context().binding.clock_epoch != plan.key().context.clock_epoch
    {
        return Err("Preparation owner changed".into());
    }
    let descriptor = wire::Prepare {
        schema_version: 2,
        binding: c.context().binding.clone(),
        client_id: i.client_id.clone(),
        operation_id: uuid()?,
        event_id: uuid()?,
        slot_start_unix_us: plan.key().slot_start_us,
        latest_start_unix_us: plan.key().latest_start_us,
        manifest: serde_json::from_slice(output.manifest()).map_err(error)?,
    };
    let before_build_ms = c.remaining_valid_ms();
    let prepared = Arc::new(
        i.client
            .build_prepared(&c, descriptor, output.pcm())
            .map_err(|e| format!("SDK body validation: {e:?}; admission {before_build_ms}ms"))?,
    );
    let admission = {
        let _lane = lock(&i.context_lane)?;
        let current = i.action_context()?;
        if epoch != i.epoch.load(Ordering::Acquire) {
            return Err("Preparation owner changed before dispatch".into());
        }
        let mut s = lock(&i.state)?;
        let key = s.key.clone().ok_or("Coordinator retired")?;
        s.coordinator
            .as_mut()
            .ok_or("Coordinator retired")?
            .refresh(
                &key,
                Instant::now() + Duration::from_millis(current.remaining_valid_ms()),
                Instant::now(),
            )
            .map_err(error)?;
        current
    }; // Never hold the heartbeat lane during prepare HTTP or release.
    publish_and_prepare(
        i,
        &plan,
        prepared.clone(),
        epoch,
        before_build_ms,
        &admission,
    )?;
    if epoch != i.epoch.load(Ordering::Acquire) {
        return Err("Prepared operation cancelled; release pending".into());
    }
    if wall_us()?
        .checked_add(500_000)
        .is_none_or(|t| t > plan.key().slot_start_us)
    {
        return Err("Prepared too late to commit safely".into());
    }
    let c = i.current()?;
    let receipt = i
        .client
        .commit(&c, &prepared, &uuid()?)
        .map_err(|e| format!("SDK commit: {e:?}; remaining {}ms", c.remaining_valid_ms()))?;
    apply_proof(i, receipt)
}
// Actual production publication/dispatch boundary, also exercised by the fake-owner regression.
fn publish_and_prepare(
    i: &Inner,
    plan: &kernel::Plan,
    prepared: Arc<PreparedOperation>,
    epoch: u64,
    before_build_ms: u64,
    admission: &ContextLease,
) -> Result<(), String> {
    let stop_client = Arc::new(i.client.stop_handle(&prepared).map_err(error)?);
    let stop_event = uuid()?;
    let native = kernel::Operation {
        key: plan.key().clone(),
        id: prepared.key().operation_id.clone(),
        body_digest: prepared.key().body_digest,
    };
    let reservation = Reservation::acquire()?;
    {
        let mut s = lock(&i.state)?;
        if epoch != i.epoch.load(Ordering::Acquire) {
            OPERATION_HELD.store(false, Ordering::Release);
            return Err("Preparation cancelled".into());
        }
        if let Err(e) = s.coordinator.as_mut().ok_or("Coordinator retired")?.bind(
            plan,
            native.clone(),
            Instant::now(),
        ) {
            OPERATION_HELD.store(false, Ordering::Release);
            return Err(error(e));
        }
        *lock(&i.active)? = Some(Active {
            stop_client,
            query_ready: false,
            prepare_error: None,
            operation: prepared.clone(),
            native: Some(native),
            stop_event,
            start_delivered: false,
            terminal: false,
        });
    }
    reservation.published();
    let before_dispatch_ms = admission.remaining_valid_ms();
    let result = i.client.prepare_with_context(admission, &prepared);
    let receipt = match result {
        Ok(receipt) => {
            if let Some(a) = lock(&i.active)?.as_mut() {
                if a.operation.key() == prepared.key() {
                    a.query_ready = true;
                }
            }
            receipt
        }
        Err(e) => {
            let reason = format!("SDK prepare dispatch: {e:?}; body admission {before_build_ms}ms, dispatch {before_dispatch_ms}ms");
            // The exclusive initial send has returned. Only a false conservative
            // SDK dispatch marker permits local cancellation; transport uncertainty
            // retains the exact operation and its independent Stop path.
            let mut s = lock(&i.state)?;
            let mut active = lock(&i.active)?;
            if let Some(a) = active
                .as_mut()
                .filter(|a| a.operation.key() == prepared.key())
            {
                a.prepare_error = Some(reason.clone());
                if !prepared.initial_dispatch_attempted() {
                    if let (Some(c), Some(native)) = (s.coordinator.as_mut(), a.native.as_ref()) {
                        if c.stop_handle().release().map_err(error)?.as_deref() == Some(native) {
                            c.discard_not_dispatched(native).map_err(error)?;
                        }
                    }
                    *active = None;
                    OPERATION_HELD.store(false, Ordering::Release);
                } else {
                    a.query_ready = true;
                }
            }
            s.reason = reason.clone();
            return Err(reason);
        }
    };
    apply_proof(i, receipt)?;
    Ok(())
}
fn start_tune(i: &Arc<Inner>, epoch: u64) -> Result<(), String> {
    let c = i.current()?;
    let (hz, gain) = {
        let mut s = lock(&i.state)?;
        if let Some(c) = s.coordinator.as_mut() {
            c.apply(kernel::Intent::Enable(false), Instant::now())
                .map_err(error)?;
        }
        (s.config.offset, s.config.gain)
    };
    let tune = wire::Tune {
        binding: c.context().binding.clone(),
        client_id: i.client_id.clone(),
        operation_id: uuid()?,
        event_id: uuid()?,
        tone_hz_f32_bits: hz.to_bits(),
        gain_f32_bits: gain.to_bits(),
        duration_ms: 30_000,
    };
    let op = Arc::new(i.client.build_tune(&c, tune).map_err(error)?);
    let stop_client = Arc::new(i.client.stop_handle(&op).map_err(error)?);
    let stop_event = uuid()?;
    let reservation = Reservation::acquire()?;
    let admission = lock(&i.stop)?;
    if epoch != i.epoch.load(Ordering::Acquire) || !i.connected.load(Ordering::Acquire) {
        return Err("Tune cancelled".into());
    }
    *lock(&i.active)? = Some(Active {
        stop_client,
        query_ready: false,
        prepare_error: None,
        operation: op.clone(),
        native: None,
        stop_event,
        start_delivered: false,
        terminal: false,
    });
    reservation.published();
    i.future_disarmed.store(false, Ordering::Release);
    i.stop_requested.store(false, Ordering::Release);
    drop(admission);
    if epoch != i.epoch.load(Ordering::Acquire) {
        return Err("Tune cancelled before dispatch; exact release retained".into());
    }
    let result = i.client.tune(&op);
    if let Some(a) = lock(&i.active)?.as_mut() {
        a.query_ready = true;
    }
    match result {
        Ok(p) => apply_proof(i, p),
        Err(e) => {
            i.fail(format!("Tune outcome unknown: {e:?}"));
            Err(error(e))
        }
    }
}

// An explicit Work gesture selects retained message data, not a new decoded
// period. A repaired sample stream must still belong to the same physical route.
fn current_work_route(i: &Inner, r: &Receive) -> bool {
    lock(&i.rx_authority).is_ok_and(|a| {
        a.as_ref()
            .is_some_and(|a| a.is_fresh() && rx_route_matches(a, r, false))
    })
}
// A temporary PCM lease gap cannot erase the original Station/QSO. Admission
// of Work still needs fresh physical authority; new periods retain their own
// exact stream/epoch admission in the decoder and SDK.
fn same_rx_route(i: &Inner, r: &Receive) -> bool {
    lock(&i.rx_authority).is_ok_and(|a| a.as_ref().is_some_and(|a| rx_route_matches(a, r, false)))
}
fn rx_route_matches(a: &AudioBatch, r: &Receive, strict_stream: bool) -> bool {
    a.descriptor.as_ref().is_some_and(|d| {
        (!strict_stream
            || (d.capture_id == r.period.owner.capture_id
                && d.epoch == r.period.owner.epoch
                && d.mapping_generation == r.period.owner.mapping_generation))
            && d.radio_id == r.period.owner.radio_id
            && d.receiver_id == r.period.owner.receiver_id
            && d.frequency_hz == r.period.owner.frequency_hz
            && d.mode == r.period.owner.radio_mode
            && d.device_id == r.period.owner.device_id
            && d.sample_rate == r.period.owner.sample_rate
            && d.source_channels == u16::from(r.period.owner.source_channels)
            && d.selected_channel == u16::from(r.period.owner.selected_channel)
            && d.slice == r.period.owner.slice
            && d.dax_channel == r.period.owner.dax_channel
    })
}
impl Host {
    pub(crate) fn log_draft(&self) -> Result<(String, QsoDraft, String), String> {
        let mut s = lock(&self.inner.state)?;
        s.retain_log();
        let saved = s
            .saved_log
            .as_ref()
            .ok_or("No current selected QSO to log")?;
        let scope = format!(
            "{}:{}:{}",
            saved.key.transmit_context, saved.key.receive.epoch, saved.facts.epoch
        );
        if s.logged_epoch.as_ref() == Some(&scope) {
            if let Some(q) = &s.completed {
                return Ok((
                    s.request_id.clone().ok_or("Missing QSO request identity")?,
                    q.clone(),
                    saved.key.owner.workspace_id.clone(),
                ));
            }
        }
        let hz = saved.key.receive.dial_hz;
        let mode = match saved.key.receive.mode {
            crate::xlog_decode::DecoderMode::Ft8 => "FT8",
            _ => "FT4",
        };
        let band = tempo_core::logbook::query::band_key("", hz as f64 / 1_000_000.0)
            .ok_or("Owner frequency has no supported log band")?
            .to_ascii_lowercase();
        let workspace = saved.key.owner.workspace_id.clone();
        let q = QsoDraft {
            contact_id: uuid()?,
            callsign: saved.facts.call.clone(),
            band,
            mode: mode.into(),
            started_at_unix_ms: if saved.facts.started_us > 0 {
                saved.facts.started_us / 1000
            } else {
                saved.started_ms
            },
            frequency_hz: Some(hz),
            rst_sent: saved.sent.map(fmt_report).unwrap_or_default(),
            rst_received: saved.facts.received.map(fmt_report).unwrap_or_default(),
            grid: saved.facts.grid.clone(),
            comment: String::new(),
        };
        q.validate().map_err(error)?;
        let request = uuid()?;
        s.logged_epoch = Some(scope);
        s.request_id = Some(request.clone());
        s.log_id = Some(q.contact_id.clone());
        s.completed = Some(q.clone());
        Ok((request, q, workspace))
    }
}

#[cfg(test)]
#[path = "xlog_ft_host_tests.rs"]
mod tests;
