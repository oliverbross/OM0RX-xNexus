// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Independent passive bearing reads, bounded by the ORIGINAL admitted snapshot.
//! A bearing lease conveys no rotor or transmission authority.
use super::{Bearing, BearingData, Status, MAX_BEARING_BYTES};
use crate::{now_ms, BridgeClient, Error, Hello, Snapshot};
use reqwest::{blocking::Client, header};
use serde::{ser::Error as _, ser::SerializeStruct, Deserialize, Serialize, Serializer};
use std::{
    io::Read,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

pub const MAX_ENVELOPE_BYTES: usize = 20 * 1024;
const SAFE: u64 = 9_007_199_254_740_990;
const LEASE_MS: u64 = 5_000;

// Deliberately stricter than the older shared StationIdentity display parser.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    workspace_generation: String,
    worker_generation: u64,
    settings_revision: u64,
    identity_epoch: String,
    identity_revision: u64,
    operating_module: crate::station_identity::OperatingModule,
}
fn canonical_uuid(id: &str) -> bool {
    crate::commands::valid_request_id(id)
        && !id.bytes().any(|b| b.is_ascii_uppercase())
        && id.bytes().any(|b| b.is_ascii_hexdigit() && b != b'0')
}
impl Binding {
    fn valid(&self, workspace: &str, worker: u64) -> bool {
        canonical_uuid(&self.workspace_generation)
            && self.workspace_generation == workspace
            && canonical_uuid(&self.identity_epoch)
            && (1..=SAFE).contains(&self.worker_generation)
            && self.worker_generation == worker
            && self.settings_revision <= SAFE
            && self.identity_revision <= SAFE
    }
}
#[derive(Clone, PartialEq, Eq)]
struct OriginIdentity {
    binding: Binding,
    grid: String,
}
impl OriginIdentity {
    fn from_snapshot(snapshot: &Snapshot) -> Result<Self, Error> {
        use crate::station_identity::{IdentitySource, IdentityState};
        let identity = snapshot.station_identity.as_ref().ok_or(Error::Stale)?;
        identity.validate(&snapshot.workspace_id)?;
        if identity.source != IdentitySource::SavedStationSettings
            || !matches!(
                identity.state,
                IdentityState::Current | IdentityState::Incomplete
            )
            || identity.transmit_authority
        {
            return Err(Error::Invalid);
        }
        let grid = identity
            .mygrid
            .as_ref()
            .filter(|v| super::valid_grid(v))
            .ok_or(Error::Stale)?
            .clone();
        let source = identity.binding.as_ref().ok_or(Error::Stale)?;
        let binding = Binding {
            workspace_generation: source.workspace_generation.clone(),
            worker_generation: source.worker_generation,
            settings_revision: source.settings_revision,
            identity_epoch: source.identity_epoch.clone(),
            identity_revision: source.identity_revision,
            operating_module: source.operating_module,
        };
        if !binding.valid(&snapshot.workspace_id, source.worker_generation) {
            return Err(Error::Invalid);
        }
        Ok(Self { binding, grid })
    }
}
#[derive(Default)]
pub(crate) struct AuthorityState {
    current: Option<(OriginIdentity, Arc<AtomicBool>)>,
}
impl AuthorityState {
    pub(crate) fn clear(&mut self) {
        if let Some((_, live)) = self.current.take() {
            live.store(false, Ordering::Release);
        }
    }
    pub(crate) fn observe(&mut self, snapshot: &Snapshot) {
        let Ok(identity) = OriginIdentity::from_snapshot(snapshot) else {
            self.clear();
            return;
        };
        if self
            .current
            .as_ref()
            .is_some_and(|(old, _)| old == &identity)
        {
            return;
        }
        self.clear();
        self.current = Some((identity, Arc::new(AtomicBool::new(true))));
    }
    fn retire_if_current(&mut self, captured: &Arc<AtomicBool>) {
        captured.store(false, Ordering::Release);
        if self
            .current
            .as_ref()
            .is_some_and(|(_, live)| Arc::ptr_eq(live, captured))
        {
            self.clear();
        }
    }
}
/// Opaque immutable authority captured only from the SDK's admitted observation.
/// A subsequent identical poll cannot extend this context's original deadline.
pub struct BearingContext {
    identity: OriginIdentity,
    authority: Arc<Mutex<AuthorityState>>,
    live: Arc<AtomicBool>,
    session: Arc<AtomicBool>,
    deadline: Instant,
}
impl BearingContext {
    fn check(&self) -> Result<(), Error> {
        if !self.live.load(Ordering::Acquire) || !self.session.load(Ordering::Acquire) {
            return Err(Error::Disconnected);
        }
        if Instant::now() >= self.deadline {
            return Err(Error::Stale);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    protocol_version: u32,
    server_instance: String,
    workspace_id: String,
    worker_generation: u64,
    station_identity_binding: Binding,
    generation: String,
    generated_at_unix_ms: u64,
    max_age_ms: u64,
    bearing: BearingData,
}
impl Envelope {
    fn validate(
        &self,
        hello: &Hello,
        request: &BearingReadRequest,
        wall: u64,
        status: u16,
    ) -> Result<(), BearingReadError> {
        if self.protocol_version != 1 {
            return Err(Error::Incompatible.into());
        }
        if self.server_instance != hello.server_instance
            || self.workspace_id != hello.workspace_id
            || self.generation != hello.workspace_id
        {
            return Err(Error::IdentityChanged.into());
        }
        if !canonical_uuid(&self.server_instance)
            || !canonical_uuid(&self.workspace_id)
            || !self
                .station_identity_binding
                .valid(&self.workspace_id, self.worker_generation)
            || self.generated_at_unix_ms > SAFE
            || self.max_age_ms != LEASE_MS
        {
            return Err(Error::Invalid.into());
        }
        if self.station_identity_binding != request.context.identity.binding {
            return Err(BearingReadError::ContextChanged);
        }
        let bytes = serde_json::to_vec(&self.bearing).map_err(|_| Error::Invalid)?;
        if bytes.len() > MAX_BEARING_BYTES {
            return Err(Error::TooLarge.into());
        }
        Bearing::decode(&bytes, &request.callsign)?;
        if self.bearing.origin.as_ref().map(|o| o.grid.as_str())
            != Some(request.context.identity.grid.as_str())
        {
            return Err(BearingReadError::ContextChanged);
        }
        if !matches!(
            (status, self.bearing.status),
            (200, Status::Available) | (503, Status::Unavailable)
        ) {
            return Err(Error::Invalid.into());
        }
        if wall < request.started_wall
            || self.generated_at_unix_ms < request.started_wall
            || self.generated_at_unix_ms > wall
            || wall - self.generated_at_unix_ms >= LEASE_MS
        {
            return Err(Error::Stale.into());
        }
        request.clock(wall, Instant::now()).map_err(Into::into)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BearingReadError {
    Bridge(Error),
    /// Saved station settings changed; retires only passive bearing context.
    ContextChanged,
    Unavailable(String),
}
impl From<Error> for BearingReadError {
    fn from(value: Error) -> Self {
        Self::Bridge(value)
    }
}
impl std::fmt::Display for BearingReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bridge(e) => e.fmt(f),
            Self::ContextChanged => f.write_str("xLog bearing station context changed"),
            Self::Unavailable(_) => f.write_str("xLog bearing is unavailable"),
        }
    }
}
impl std::error::Error for BearingReadError {}
fn stable_code(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

#[derive(Clone)]
pub struct BearingReadFence(Arc<Fence>);
struct Fence {
    token: String,
    live: AtomicBool,
    session: Arc<AtomicBool>,
    authority: Arc<Mutex<AuthorityState>>,
    requests: Mutex<RequestEpoch>,
}
struct RequestEpoch {
    sequence: u64,
    live: Arc<AtomicBool>,
}
impl BearingReadFence {
    pub fn revoke(&self) {
        self.0.live.store(false, Ordering::Release);
    }
    fn check(&self, token: &str) -> Result<(), Error> {
        if token == self.0.token
            && self.0.live.load(Ordering::Acquire)
            && self.0.session.load(Ordering::Acquire)
        {
            Ok(())
        } else {
            Err(Error::Disconnected)
        }
    }
    /// Called before scheduling; captures request identity and retires prior facts.
    pub fn capture(
        &self,
        expected_token: &str,
        callsign: &str,
        context: BearingContext,
    ) -> Result<BearingReadRequest, Error> {
        self.capture_clock(expected_token, callsign, context, now_ms)
    }
    fn capture_clock(
        &self,
        token: &str,
        callsign: &str,
        context: BearingContext,
        clock: impl FnOnce() -> Result<u64, Error>,
    ) -> Result<BearingReadRequest, Error> {
        let started = Instant::now();
        self.check(token)?;
        let mut epoch = self.0.requests.lock().map_err(|_| Error::Disconnected)?;
        epoch.live.store(false, Ordering::Release);
        let wall = clock()?;
        if !super::valid_call(callsign) || wall > SAFE {
            return Err(Error::Invalid);
        }
        if !Arc::ptr_eq(&self.0.authority, &context.authority)
            || !Arc::ptr_eq(&self.0.session, &context.session)
        {
            return Err(Error::IdentityChanged);
        }
        context.check()?;
        let Some(sequence) = epoch.sequence.checked_add(1) else {
            self.revoke();
            return Err(Error::Disconnected);
        };
        self.check(token)?;
        epoch.sequence = sequence;
        epoch.live = Arc::new(AtomicBool::new(true));
        Ok(BearingReadRequest {
            callsign: callsign.into(),
            token: token.into(),
            fence: self.clone(),
            context,
            started,
            started_wall: wall,
            sequence,
            request_live: epoch.live.clone(),
        })
    }
}
pub struct BearingReadRequest {
    callsign: String,
    token: String,
    fence: BearingReadFence,
    context: BearingContext,
    started: Instant,
    started_wall: u64,
    sequence: u64,
    request_live: Arc<AtomicBool>,
}
impl BearingReadRequest {
    fn check(&self) -> Result<(), Error> {
        self.fence.check(&self.token)?;
        self.context.check()?;
        if self.sequence == 0 || !self.request_live.load(Ordering::Acquire) {
            return Err(Error::Disconnected);
        }
        Ok(())
    }
    fn clock(&self, wall: u64, now: Instant) -> Result<(), Error> {
        let Some(elapsed) = now.checked_duration_since(self.started) else {
            return Err(Error::Stale);
        };
        if wall > SAFE
            || wall < self.started_wall
            || wall - self.started_wall >= LEASE_MS
            || elapsed >= Duration::from_millis(LEASE_MS)
            || (wall - self.started_wall).abs_diff(elapsed.as_millis() as u64) > 250
        {
            return Err(Error::Stale);
        }
        Ok(())
    }
}
pub struct BearingLease {
    payload: Envelope,
    token: String,
    session: Arc<AtomicBool>,
    fence: BearingReadFence,
    facts: Arc<AtomicBool>,
    request_live: Arc<AtomicBool>,
    identity_live: Arc<AtomicBool>,
    expires: Instant,
    started: Instant,
    started_wall: u64,
    receipt_wall: u64,
}
impl BearingLease {
    fn check_at(&self, wall: Result<u64, Error>, now: Instant) -> Result<(), Error> {
        let result = (|| {
            self.fence.check(&self.token)?;
            if !self.session.load(Ordering::Acquire)
                || !self.facts.load(Ordering::Acquire)
                || !self.request_live.load(Ordering::Acquire)
                || !self.identity_live.load(Ordering::Acquire)
            {
                return Err(Error::Disconnected);
            }
            let wall = wall?;
            let elapsed = now
                .checked_duration_since(self.started)
                .ok_or(Error::Stale)?;
            if wall > SAFE
                || wall < self.receipt_wall
                || wall < self.started_wall
                || wall < self.payload.generated_at_unix_ms
                || wall - self.started_wall >= LEASE_MS
                || wall - self.payload.generated_at_unix_ms >= LEASE_MS
                || (wall - self.started_wall).abs_diff(elapsed.as_millis() as u64) > 250
                || now >= self.expires
            {
                return Err(Error::Stale);
            }
            Ok(())
        })();
        if result.is_err() {
            self.facts.store(false, Ordering::Release);
        }
        result
    }
    fn check(&self) -> Result<(), Error> {
        self.check_at(now_ms(), Instant::now())
    }
    pub fn remaining_valid_ms(&self) -> u64 {
        let now = Instant::now();
        let wall = now_ms();
        if self.check_at(wall, now).is_err() {
            return 0;
        }
        let Ok(wall) = wall else {
            return 0;
        };
        self.expires
            .saturating_duration_since(now)
            .as_millis()
            .min(u128::from(
                LEASE_MS.saturating_sub(wall.saturating_sub(self.started_wall)),
            ))
            .min(u128::from(LEASE_MS.saturating_sub(
                wall.saturating_sub(self.payload.generated_at_unix_ms),
            ))) as u64
    }
    pub fn bearing(&self) -> Result<&BearingData, Error> {
        self.check()?;
        Ok(&self.payload.bearing)
    }
}
impl Serialize for BearingLease {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.check().map_err(S::Error::custom)?;
        let mut out = serializer.serialize_struct("BearingRead", 3)?;
        out.serialize_field("connection_token", &self.token)?;
        out.serialize_field("payload", &self.payload)?;
        self.check().map_err(S::Error::custom)?;
        let remaining = self.remaining_valid_ms();
        if remaining == 0 {
            return Err(S::Error::custom(Error::Stale));
        }
        out.serialize_field("remaining_valid_ms", &remaining)?;
        self.check().map_err(S::Error::custom)?;
        let result = out.end();
        self.check().map_err(S::Error::custom)?;
        result
    }
}
/// Not Clone or Debug. One blocking reader, independent of the main poll/Stop lane.
pub struct BearingClient {
    http: Client,
    endpoint: String,
    hello: Hello,
    session: Arc<AtomicBool>,
    fence: BearingReadFence,
    facts: Arc<AtomicBool>,
    revision: Option<i64>,
    last: Option<(u64, Instant)>,
}
impl BridgeClient {
    pub fn bearing_context(&self) -> Result<BearingContext, Error> {
        let hello = self.hello().ok_or(Error::Disconnected)?;
        if !hello.capabilities.iter().any(|s| s == "bearing.read") {
            return Err(Error::Capability);
        }
        self.snapshot()?;
        let observation = self.observation.as_ref().ok_or(Error::Disconnected)?;
        let identity = OriginIdentity::from_snapshot(&observation.snapshot)?;
        let state = self
            .bearing_authority
            .lock()
            .map_err(|_| Error::Disconnected)?;
        let (current, live) = state.current.as_ref().ok_or(Error::Stale)?;
        if current != &identity {
            return Err(Error::IdentityChanged);
        }
        let context = BearingContext {
            identity,
            authority: self.bearing_authority.clone(),
            live: live.clone(),
            session: self.audio_session.clone(),
            deadline: observation.received + observation.remaining,
        };
        context.check()?;
        Ok(context)
    }
    pub fn bearing_handle(&self, connection_token: &str) -> Result<BearingClient, Error> {
        let hello = self.hello().ok_or(Error::Disconnected)?;
        if !hello.capabilities.iter().any(|s| s == "bearing.read") {
            return Err(Error::Capability);
        }
        if !crate::bounded(connection_token, 128)
            || !canonical_uuid(&hello.server_instance)
            || !canonical_uuid(&hello.workspace_id)
        {
            return Err(Error::Invalid);
        }
        Ok(BearingClient {
            http: self.http.clone(),
            endpoint: self.endpoint.clone(),
            hello: hello.clone(),
            session: self.audio_session.clone(),
            fence: BearingReadFence(Arc::new(Fence {
                token: connection_token.into(),
                live: AtomicBool::new(true),
                session: self.audio_session.clone(),
                authority: self.bearing_authority.clone(),
                requests: Mutex::new(RequestEpoch {
                    sequence: 0,
                    live: Arc::new(AtomicBool::new(false)),
                }),
            })),
            facts: Arc::new(AtomicBool::new(true)),
            revision: None,
            last: None,
        })
    }
}
impl Drop for BearingClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}
impl BearingClient {
    pub fn fence(&self) -> BearingReadFence {
        self.fence.clone()
    }
    pub fn disconnect(&self) {
        self.fence.revoke();
        self.facts.store(false, Ordering::Release);
    }
    fn active(&self) -> Result<(), Error> {
        self.fence.check(&self.fence.0.token)
    }
    pub fn read(&mut self, request: &BearingReadRequest) -> Result<BearingLease, BearingReadError> {
        // A retired queued request cannot erase a newer request's retained facts.
        self.active()?;
        request.check()?;
        if !Arc::ptr_eq(&self.fence.0, &request.fence.0) {
            return Err(Error::IdentityChanged.into());
        }
        self.facts.store(false, Ordering::Release);
        self.facts = Arc::new(AtomicBool::new(true));
        let result = self.read_inner(request);
        if let Err(error) = &result {
            self.facts.store(false, Ordering::Release);
            if matches!(error, BearingReadError::ContextChanged) {
                request
                    .context
                    .authority
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .retire_if_current(&request.context.live);
            }
            if matches!(
                error,
                BearingReadError::Bridge(
                    Error::IdentityChanged | Error::Incompatible | Error::Http(401 | 403)
                )
            ) {
                self.session.store(false, Ordering::Release);
                self.disconnect();
            }
        }
        result
    }
    fn read_inner(
        &mut self,
        request: &BearingReadRequest,
    ) -> Result<BearingLease, BearingReadError> {
        request.clock(now_ms()?, Instant::now())?;
        let budget = (request.started + Duration::from_millis(LEASE_MS))
            .min(request.context.deadline)
            .saturating_duration_since(Instant::now());
        if budget.is_zero() {
            return Err(Error::Stale.into());
        }
        let response = self
            .http
            .get(format!(
                "{}/xlog/v1/bearing?callsign={}",
                self.endpoint,
                request.callsign.replace('/', "%2F")
            ))
            .timeout(budget)
            .send()
            .map_err(|_| Error::Transport)?;
        let status = response.status().as_u16();
        if !matches!(status, 200 | 503) {
            return Err(Error::Http(status).into());
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_ENVELOPE_BYTES as u64)
        {
            return Err(Error::TooLarge.into());
        }
        if !response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .eq_ignore_ascii_case("application/json")
        {
            return Err(Error::Invalid.into());
        }
        let mut bytes = Vec::new();
        response
            .take(MAX_ENVELOPE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Transport)?;
        if bytes.len() > MAX_ENVELOPE_BYTES {
            return Err(Error::TooLarge.into());
        }
        self.active()?;
        request.check()?;
        request.clock(now_ms()?, Instant::now())?;
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Unavailable {
            error: String,
        }
        if status == 503 {
            if let Ok(error) = serde_json::from_slice::<Unavailable>(&bytes) {
                if !stable_code(&error.error) {
                    return Err(Error::Invalid.into());
                }
                return Err(BearingReadError::Unavailable(error.error));
            }
        }
        let envelope: Envelope = serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)?;
        let wall = now_ms()?;
        envelope.validate(&self.hello, request, wall, status)?;
        if self
            .revision
            .is_some_and(|r| envelope.bearing.logbook_revision < r)
        {
            return Err(Error::Revision.into());
        }
        let expires = deadline(
            request,
            envelope.generated_at_unix_ms,
            wall,
            Instant::now(),
            self.last,
        )?;
        self.active()?;
        request.check()?;
        let lease = BearingLease {
            payload: envelope,
            token: request.token.clone(),
            session: self.session.clone(),
            fence: self.fence.clone(),
            facts: self.facts.clone(),
            request_live: request.request_live.clone(),
            identity_live: request.context.live.clone(),
            expires,
            started: request.started,
            started_wall: request.started_wall,
            receipt_wall: wall,
        };
        lease.check()?;
        self.revision = Some(lease.payload.bearing.logbook_revision);
        self.last = Some((lease.payload.generated_at_unix_ms, expires));
        Ok(lease)
    }
}

fn deadline(
    request: &BearingReadRequest,
    generated: u64,
    wall: u64,
    now: Instant,
    last: Option<(u64, Instant)>,
) -> Result<Instant, Error> {
    let mut expires = request
        .context
        .deadline
        .min(request.started + Duration::from_millis(LEASE_MS))
        .min(now + Duration::from_millis(LEASE_MS.saturating_sub(wall.saturating_sub(generated))));
    if let Some((stamp, old_expiry)) = last {
        if generated < stamp {
            return Err(Error::Stale);
        }
        if generated == stamp {
            expires = expires.min(old_expiry);
        }
    }
    Ok(expires)
}

#[cfg(test)]
mod tests;
