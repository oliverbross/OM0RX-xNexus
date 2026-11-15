// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Independent selected-contact reads. No native commands, fallback, or mutation.
use super::{ContactDetail, Status};
use crate::{now_ms, BridgeClient, Error, Hello};
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

pub const MAX_ENVELOPE_BYTES: usize = 128 * 1024;
const SAFE: u64 = 9_007_199_254_740_990;
const LEASE_MS: u64 = 5_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub protocol_version: u32,
    pub server_instance: String,
    pub workspace_id: String,
    pub worker_generation: u64,
    pub generation: String,
    pub generated_at_unix_ms: u64,
    pub max_age_ms: u64,
    pub detail: ContactDetail,
}
fn canonical_id(id: &str) -> bool {
    crate::commands::valid_request_id(id) && !id.bytes().any(|b| b.is_ascii_uppercase())
}
fn stable_code(code: &str) -> bool {
    (1..=128).contains(&code.len())
        && code
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
impl Envelope {
    fn validate(
        &self,
        hello: &Hello,
        request: &ContactReadRequest,
        receipt_wall: u64,
        status: u16,
    ) -> Result<(), Error> {
        if self.protocol_version != 1 {
            return Err(Error::Incompatible);
        }
        if self.server_instance != hello.server_instance
            || self.workspace_id != hello.workspace_id
            || self.generation != hello.workspace_id
        {
            return Err(Error::IdentityChanged);
        }
        if !(1..=SAFE).contains(&self.worker_generation)
            || self.generated_at_unix_ms > SAFE
            || self.max_age_ms != LEASE_MS
            || self.detail.data().contact_id != request.contact_id
        {
            return Err(Error::Invalid);
        }
        endpoint_policy(&self.detail)?;
        match (status, self.detail.data().status) {
            (200, Status::Found | Status::Deleted | Status::NotFound) => (),
            (503, Status::Unavailable)
                if self
                    .detail
                    .data()
                    .reason
                    .as_deref()
                    .is_some_and(stable_code) => {}
            _ => return Err(Error::Invalid),
        }
        if receipt_wall < request.started_wall
            || self.generated_at_unix_ms < request.started_wall
            || self.generated_at_unix_ms > receipt_wall
            || receipt_wall.saturating_sub(self.generated_at_unix_ms) >= LEASE_MS
            || request.started.elapsed() >= Duration::from_millis(LEASE_MS)
        {
            return Err(Error::Stale);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContactReadError {
    Bridge(Error),
    /// Bounded stable owner code. Never a missing contact or empty successful read.
    Unavailable(String),
}
impl From<Error> for ContactReadError {
    fn from(error: Error) -> Self {
        Self::Bridge(error)
    }
}
impl std::fmt::Display for ContactReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bridge(error) => error.fmt(f),
            Self::Unavailable(_) => f.write_str("xLog contact detail is unavailable"),
        }
    }
}
impl std::error::Error for ContactReadError {}

/// Caller retains this fence and revokes it synchronously on local connection
/// retirement, without acquiring the client/HTTP lock. It is not a pairing token.
#[derive(Clone)]
pub struct ContactReadFence(Arc<Fence>);
struct Fence {
    token: String,
    live: AtomicBool,
    requests: Mutex<RequestEpoch>,
}
struct RequestEpoch {
    sequence: u64,
    live: Arc<AtomicBool>,
}
impl ContactReadFence {
    pub fn revoke(&self) {
        self.0.live.store(false, Ordering::Release);
    }
    /// Captures selected-contact authority without holding the HTTP client lock.
    /// Every capture retires the preceding request and all of its retained facts.
    pub fn capture(
        &self,
        expected_token: &str,
        contact_id: &str,
    ) -> Result<ContactReadRequest, Error> {
        self.capture_clock(expected_token, contact_id, now_ms)
    }
    // Private deterministic seam; no system-clock mutation or global test state.
    fn capture_clock(
        &self,
        expected_token: &str,
        contact_id: &str,
        wall_clock: impl FnOnce() -> Result<u64, Error>,
    ) -> Result<ContactReadRequest, Error> {
        let started = Instant::now();
        self.check(expected_token)?;
        let mut epoch = self.0.requests.lock().map_err(|_| Error::Disconnected)?;
        epoch.live.store(false, Ordering::Release);
        let started_wall = wall_clock()?;
        if !canonical_id(contact_id) || started_wall > SAFE {
            return Err(Error::Invalid);
        }
        let Some(sequence) = epoch.sequence.checked_add(1) else {
            self.revoke();
            return Err(Error::Disconnected);
        };
        self.check(expected_token)?;
        epoch.sequence = sequence;
        epoch.live = Arc::new(AtomicBool::new(true));
        Ok(ContactReadRequest {
            contact_id: contact_id.into(),
            token: expected_token.into(),
            fence: self.clone(),
            started,
            started_wall,
            request_live: epoch.live.clone(),
            sequence,
        })
    }
    fn check(&self, token: &str) -> Result<(), Error> {
        if self.0.live.load(Ordering::Acquire) && token == self.0.token {
            Ok(())
        } else {
            Err(Error::Disconnected)
        }
    }
}
/// Capture before scheduling the blocking read; its monotonic clock cannot be
/// reset by queueing, IPC, a delayed reply, or reuse of the request.
pub struct ContactReadRequest {
    contact_id: String,
    token: String,
    fence: ContactReadFence,
    started: Instant,
    started_wall: u64,
    request_live: Arc<AtomicBool>,
    sequence: u64,
}
impl ContactReadRequest {
    fn check(&self) -> Result<(), Error> {
        self.fence.check(&self.token)?;
        if self.request_live.load(Ordering::Acquire) && self.sequence != 0 {
            Ok(())
        } else {
            Err(Error::Disconnected)
        }
    }
}

/// Retains paired-session and local connection authority through serialization.
/// Consumers must also compare the selected contact and connection token, then
/// apply the remaining duration to their PRE-invoke monotonic time.
pub struct ContactLease {
    payload: Envelope,
    token: String,
    session: Arc<AtomicBool>,
    fence: ContactReadFence,
    facts: Arc<AtomicBool>,
    request_live: Arc<AtomicBool>,
    expires: Instant,
    receipt_wall: u64,
}
impl ContactLease {
    fn check(&self) -> Result<(), Error> {
        self.check_at(now_ms(), Instant::now())
    }
    // Failure latches ONLY this operation's facts. A later read has a distinct
    // Arc and cannot be retired by a delayed old response/serializer callback.
    fn check_at(&self, wall: Result<u64, Error>, now: Instant) -> Result<(), Error> {
        let result = (|| {
            self.fence.check(&self.token)?;
            if !self.session.load(Ordering::Acquire)
                || !self.facts.load(Ordering::Acquire)
                || !self.request_live.load(Ordering::Acquire)
            {
                return Err(Error::Disconnected);
            }
            let wall = wall?;
            if wall < self.receipt_wall
                || wall < self.payload.generated_at_unix_ms
                || wall.saturating_sub(self.payload.generated_at_unix_ms) >= LEASE_MS
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
    pub fn remaining_valid_ms(&self) -> u64 {
        let wall = now_ms();
        let now = Instant::now();
        if self.check_at(wall, now).is_err() {
            return 0;
        }
        let Ok(wall) = wall else {
            return 0;
        };
        self.expires
            .saturating_duration_since(now)
            .as_millis()
            .min(u128::from(LEASE_MS.saturating_sub(
                wall.saturating_sub(self.payload.generated_at_unix_ms),
            ))) as u64
    }
    pub fn value(&self) -> Result<&Envelope, Error> {
        self.check()?;
        Ok(&self.payload)
    }
}
impl Serialize for ContactLease {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.check().map_err(S::Error::custom)?;
        let mut out = serializer.serialize_struct("ContactRead", 3)?;
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

/// Not Debug: the inherited HTTP client retains native pairing credentials.
pub struct ContactDetailClient {
    http: Client,
    endpoint: String,
    hello: Hello,
    session: Arc<AtomicBool>,
    fence: ContactReadFence,
    facts: Arc<AtomicBool>,
    worker: Option<u64>,
    revision: Option<u64>,
    contact_revision: Option<(String, i64)>,
    last: Option<(u64, Instant)>,
}
impl BridgeClient {
    pub fn contact_detail_handle(
        &self,
        connection_token: &str,
    ) -> Result<ContactDetailClient, Error> {
        let hello = self.hello().ok_or(Error::Disconnected)?;
        if !hello
            .capabilities
            .iter()
            .any(|c| c == "logbook.contact.read")
        {
            return Err(Error::Capability);
        }
        if !crate::bounded(connection_token, 128) {
            return Err(Error::Invalid);
        }
        Ok(ContactDetailClient {
            http: self.http.clone(),
            endpoint: self.endpoint.clone(),
            hello: hello.clone(),
            session: self.audio_session.clone(),
            fence: ContactReadFence(Arc::new(Fence {
                token: connection_token.into(),
                live: AtomicBool::new(true),
                requests: Mutex::new(RequestEpoch {
                    sequence: 0,
                    live: Arc::new(AtomicBool::new(false)),
                }),
            })),
            facts: Arc::new(AtomicBool::new(true)),
            worker: None,
            revision: None,
            contact_revision: None,
            last: None,
        })
    }
}
impl Drop for ContactDetailClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}
impl ContactDetailClient {
    pub fn fence(&self) -> ContactReadFence {
        self.fence.clone()
    }
    pub fn disconnect(&self) {
        self.fence.revoke();
        self.facts.store(false, Ordering::Release);
    }
    fn active(&self) -> Result<(), Error> {
        self.fence.check(&self.fence.0.token)?;
        if self.session.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err(Error::Disconnected)
        }
    }
    pub fn capture(
        &self,
        expected_token: &str,
        contact_id: &str,
    ) -> Result<ContactReadRequest, Error> {
        self.active()?;
        self.fence.capture(expected_token, contact_id)
    }

    fn retire(&mut self) {
        self.facts.store(false, Ordering::Release);
        self.facts = Arc::new(AtomicBool::new(true));
    }
    pub fn read(&mut self, request: &ContactReadRequest) -> Result<ContactLease, ContactReadError> {
        // An attempted refresh never leaves previously displayed facts current.
        self.retire();
        let result = self.read_inner(request);
        if let Err(error) = &result {
            self.facts.store(false, Ordering::Release);
            if matches!(
                error,
                ContactReadError::Bridge(
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
        request: &ContactReadRequest,
    ) -> Result<ContactLease, ContactReadError> {
        self.active()?;
        if !Arc::ptr_eq(&self.fence.0, &request.fence.0) {
            return Err(Error::IdentityChanged.into());
        }
        request.check()?;
        if request.started.elapsed() >= Duration::from_millis(LEASE_MS)
            || now_ms()? < request.started_wall
        {
            return Err(Error::Stale.into());
        }
        let response = self
            .http
            .get(format!(
                "{}/xlog/v1/logbook/contacts/{}",
                self.endpoint, request.contact_id
            ))
            .timeout(
                Duration::from_millis(LEASE_MS)
                    .saturating_sub(request.started.elapsed())
                    .min(Duration::from_secs(3)),
            )
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
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !content_type
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
                return Err(ContactReadError::Unavailable(error.error));
            }
        }
        let envelope: Envelope = serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)?;
        let receipt_wall = now_ms()?;
        envelope.validate(&self.hello, request, receipt_wall, status)?;
        if self.worker.is_some_and(|v| v != envelope.worker_generation) {
            self.disconnect();
            return Err(Error::IdentityChanged.into());
        }
        let revision = envelope.detail.data().logbook_revision as u64;
        if self.revision.is_some_and(|old| revision < old) {
            return Err(Error::Revision.into());
        }
        if let (Some((old_id, old_revision)), Some(new_revision)) = (
            &self.contact_revision,
            envelope.detail.data().contact_revision,
        ) {
            if old_id == &request.contact_id && new_revision < *old_revision {
                return Err(Error::Revision.into());
            }
        }
        let expires = deadline(
            request,
            envelope.generated_at_unix_ms,
            receipt_wall,
            self.last,
        )?;
        self.active()?;
        let lease = ContactLease {
            token: request.token.clone(),
            session: self.session.clone(),
            fence: self.fence.clone(),
            facts: self.facts.clone(),
            request_live: request.request_live.clone(),
            expires,
            receipt_wall,
            payload: envelope,
        };
        lease.check()?;
        self.contact_revision = lease
            .payload
            .detail
            .data()
            .contact_revision
            .map(|r| (request.contact_id.clone(), r));
        self.worker = Some(lease.payload.worker_generation);
        self.revision = Some(revision);
        self.last = Some((lease.payload.generated_at_unix_ms, expires));
        Ok(lease)
    }
}

// The endpoint imposes additional policy on a retained DTO. Keep this separate:
// the general codec deliberately preserves broader read-only Swift facts.
fn endpoint_policy(detail: &ContactDetail) -> Result<(), Error> {
    use super::{FieldState, EDIT_FIELDS};
    let data = detail.data();
    if data
        .contact_revision
        .is_some_and(|r| r > data.logbook_revision)
        || (data.status == Status::Deleted && data.contact_revision.is_none())
        || (data.status == Status::NotFound && data.contact_revision.is_some())
        || (matches!(data.status, Status::Deleted | Status::NotFound) && data.reason.is_some())
        || (data.status == Status::Unavailable && !data.reason.as_deref().is_some_and(stable_code))
        || !stable_code(&data.editability.reason)
        || (data.editability.allowed && data.editability.allowed_fields != EDIT_FIELDS)
        || data.locate.state != FieldState::Unavailable
        || data.locate.reason.as_deref() != Some("paged_logbook_not_supported")
    {
        return Err(Error::Invalid);
    }
    if let Some(fields) = &data.fields {
        if fields.radio.tx_power_w.value.is_some_and(|n| n < 0.0) {
            return Err(Error::Invalid);
        }
        for (name, group) in [
            ("paper", &fields.confirmations.paper),
            ("lotw", &fields.confirmations.lotw),
            ("eqsl", &fields.confirmations.eqsl),
            ("qrz", &fields.confirmations.qrz),
        ] {
            if group.source.value.as_deref() != Some(name)
                || group.match_age_seconds.value.is_some_and(|n| n < 0.0)
            {
                return Err(Error::Invalid);
            }
        }
        for (name, group) in [
            ("lotw", &fields.uploads.lotw),
            ("eqsl", &fields.uploads.eqsl),
            ("qrz", &fields.uploads.qrz),
            ("clublog", &fields.uploads.clublog),
        ] {
            if group.source.value.as_deref() != Some(name) {
                return Err(Error::Invalid);
            }
        }
    }
    fn reasons(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(o) => {
                if o.get("state").and_then(serde_json::Value::as_str) == Some("unavailable")
                    && !o
                        .get("reason")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(stable_code)
                {
                    return false;
                }
                o.values().all(reasons)
            }
            serde_json::Value::Array(a) => a.iter().all(reasons),
            _ => true,
        }
    }
    if !reasons(&serde_json::to_value(detail).map_err(|_| Error::Invalid)?) {
        return Err(Error::Invalid);
    }
    Ok(())
}

fn deadline(
    request: &ContactReadRequest,
    generated: u64,
    receipt_wall: u64,
    last: Option<(u64, Instant)>,
) -> Result<Instant, Error> {
    let mut expires = (request.started + Duration::from_millis(LEASE_MS)).min(
        Instant::now()
            + Duration::from_millis(
                LEASE_MS.saturating_sub(receipt_wall.saturating_sub(generated)),
            ),
    );
    if let Some((stamp, previous)) = last {
        if generated < stamp {
            return Err(Error::Stale);
        }
        if generated == stamp {
            expires = expires.min(previous);
        }
    }
    Ok(expires)
}

#[cfg(test)]
mod tests;
