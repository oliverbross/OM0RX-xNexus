// SPDX-License-Identifier: GPL-3.0-only
//! Authenticated owner-mediated digital TX. No encoder, scheduler or device access.
//! A receipt is historical evidence, not permission to dispatch another operation.
use crate::{bounded, frequency, now_ms, BridgeClient, Client, Error, Hello};
use serde::de::DeserializeOwned;
use std::{
    io::Read,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

mod validate;
pub mod wire;
const SAFE: u64 = 9_007_199_254_740_991;
const MAX_PREPARE: usize = 4 * 1024 * 1024;
const SMALL: usize = 16 * 1024;

#[derive(Clone)]
pub struct ContextLease {
    context: wire::Context,
    expires: Instant,
    live: Arc<AtomicBool>,
    session: Arc<AtomicBool>,
    issuer: Arc<()>,
}
impl ContextLease {
    pub fn context(&self) -> &wire::Context {
        &self.context
    }
    pub fn remaining_valid_ms(&self) -> u64 {
        if !self.live.load(Ordering::Acquire) || !self.session.load(Ordering::Acquire) {
            return 0;
        }
        self.expires
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(1000) as u64
    }
    pub fn is_fresh(&self) -> bool {
        self.remaining_valid_ms() > 0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationKey {
    pub binding: wire::Binding,
    pub client_id: String,
    pub operation_id: String,
    pub origin_event_id: String,
    pub body_sha256: String,
    pub body_digest: [u8; 32],
    pub slot_window: Option<(u64, u64)>,
}
/// This type has no public constructor or deserializer. Only validated HTTP replies create it.
#[derive(Clone, Debug)]
pub struct ValidatedReceipt {
    key: OperationKey,
    receipt: wire::Receipt,
}
impl ValidatedReceipt {
    pub fn key(&self) -> &OperationKey {
        &self.key
    }
    pub fn receipt(&self) -> &wire::Receipt {
        &self.receipt
    }
}
struct OperationState {
    key: OperationKey,
    body: Vec<u8>,
    descriptor: Option<wire::Prepare>,
    context: ContextLease,
    dispatched: AtomicBool,
    committed: AtomicBool,
    stopped: AtomicBool,
    receipt: Mutex<Option<ValidatedReceipt>>,
}
#[derive(Clone)]
pub struct PreparedOperation(Arc<OperationState>);
impl PreparedOperation {
    pub fn key(&self) -> &OperationKey {
        &self.0.key
    }
    pub fn descriptor(&self) -> Option<&wire::Prepare> {
        self.0.descriptor.as_ref()
    }
    pub fn body(&self) -> &[u8] {
        &self.0.body
    }
    pub fn cached_receipt(&self) -> Option<ValidatedReceipt> {
        self.0.receipt.lock().ok().and_then(|r| r.clone())
    }
}
struct Authority {
    current: Option<ContextLease>,
    ticket: Arc<()>,
}
impl Authority {
    fn retire(&mut self) {
        if let Some(c) = &self.current {
            c.live.store(false, Ordering::Release);
        }
        self.ticket = Arc::new(());
    }
}
#[derive(Clone)]
struct Transport {
    http: Client,
    endpoint: String,
    session: Arc<AtomicBool>,
}
impl Transport {
    fn request<T: DeserializeOwned>(
        &self,
        path: &str,
        body: Option<(&[u8], bool)>,
        prepare: bool,
    ) -> Result<T, Error> {
        if !self.session.load(Ordering::Acquire) {
            return Err(Error::Disconnected);
        }
        let url = format!("{}/xlog/v1/tx/{path}", self.endpoint);
        let req = if let Some((bytes, binary)) = body {
            self.http
                .post(url)
                .header(
                    "Content-Type",
                    if binary {
                        "application/octet-stream"
                    } else {
                        "application/json"
                    },
                )
                .body(bytes.to_vec())
        } else {
            self.http.get(url)
        };
        let response = req
            .timeout(Duration::from_secs(if prepare { 2 } else { 1 }))
            .send()
            .map_err(|_| Error::Transport)?;
        let status = response.status().as_u16();
        if matches!(status, 401 | 403) {
            self.session.store(false, Ordering::Release);
            return Err(Error::Disconnected);
        }
        if status != 200 {
            return Err(Error::Http(status));
        }
        if response.content_length().is_some_and(|n| n > SMALL as u64) {
            return Err(Error::TooLarge);
        }
        let mut bytes = Vec::new();
        response
            .take(SMALL as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Transport)?;
        if bytes.len() > SMALL {
            return Err(Error::TooLarge);
        }
        if !self.session.load(Ordering::Acquire) {
            return Err(Error::Disconnected);
        }
        serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)
    }
}
pub struct TxClient {
    transport: Transport,
    hello: Hello,
    client_id: String,
    issuer: Arc<()>,
    authority: Arc<Mutex<Authority>>,
    context_busy: AtomicBool,
    ordinary_busy: AtomicBool,
}
impl Drop for TxClient {
    fn drop(&mut self) {
        if let Ok(mut s) = self.authority.lock() {
            s.retire();
        }
    }
}
impl BridgeClient {
    pub fn tx_handle(&self, client_id: &str) -> Result<TxClient, Error> {
        let hello = self.hello().ok_or(Error::Disconnected)?;
        if !hello.capabilities.iter().any(|c| c == "digital.tx") {
            return Err(Error::Capability);
        }
        if !validate::uuid(client_id)
            || !validate::uuid(&hello.server_instance)
            || !validate::uuid(&hello.workspace_id)
        {
            return Err(Error::Invalid);
        }
        Ok(TxClient {
            transport: Transport {
                http: self.http.clone(),
                endpoint: self.endpoint.clone(),
                session: self.audio_session.clone(),
            },
            hello: hello.clone(),
            client_id: client_id.into(),
            issuer: Arc::new(()),
            authority: Arc::new(Mutex::new(Authority {
                current: None,
                ticket: Arc::new(()),
            })),
            context_busy: AtomicBool::new(false),
            ordinary_busy: AtomicBool::new(false),
        })
    }
}
struct Permit<'a>(&'a AtomicBool);
impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl TxClient {
    fn owns(&self, c: &ContextLease) -> bool {
        Arc::ptr_eq(&self.issuer, &c.issuer) && Arc::ptr_eq(&self.transport.session, &c.session)
    }
    fn admission(&self, c: &ContextLease, enabled: bool) -> Result<(), Error> {
        if !self.owns(c) || !c.is_fresh() {
            return Err(Error::Stale);
        }
        if !c.context.ready || !c.context.local_enabled || enabled && !c.context.client_enabled {
            return Err(Error::Stale);
        }
        Ok(())
    }
    fn observe(&self, path: &str, body: Option<Vec<u8>>) -> Result<ContextLease, Error> {
        if self
            .context_busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(Error::Stale);
        }
        let _permit = Permit(&self.context_busy);
        let ticket = self
            .authority
            .lock()
            .map_err(|_| Error::Disconnected)?
            .ticket
            .clone();
        let started = Instant::now();
        // Owner generation can legitimately occur after request admission.
        // Check future dates at receipt, but never move the read-start deadline.
        let result = (|| {
            let context: wire::Context =
                self.transport
                    .request(path, body.as_deref().map(|b| (b, false)), false)?;
            let received = Instant::now();
            let budget = validate::context(&context, &self.hello, now_ms()?)?;
            let mut expires = started
                .checked_add(Duration::from_millis(context.max_age_ms))
                .ok_or(Error::Stale)?
                .min(received.checked_add(budget).ok_or(Error::Stale)?);
            let mut state = self.authority.lock().map_err(|_| Error::Disconnected)?;
            if !Arc::ptr_eq(&ticket, &state.ticket) {
                return Err(Error::Stale);
            }
            let mut live = Arc::new(AtomicBool::new(true));
            if let Some(old) = &state.current {
                let same_scope = old.context.binding == context.binding
                    && old.context.identity == context.identity
                    && old.context.route == context.route;
                if same_scope && context.generated_at_unix_ms < old.context.generated_at_unix_ms {
                    return Err(Error::Revision);
                }
                if same_scope && context.generated_at_unix_ms == old.context.generated_at_unix_ms {
                    expires = expires.min(old.expires);
                }
                if same_scope
                    && old.context.ready == context.ready
                    && old.context.local_enabled == context.local_enabled
                    && old.context.client_enabled == context.client_enabled
                    && old.live.load(Ordering::Acquire)
                {
                    live = old.live.clone();
                } else {
                    old.live.store(false, Ordering::Release);
                }
                // A read of the same old observation cannot re-enable an explicitly retired grant.
                if body.is_none()
                    && same_scope
                    && context.generated_at_unix_ms == old.context.generated_at_unix_ms
                    && !old.live.load(Ordering::Acquire)
                {
                    return Err(Error::Stale);
                }
            }
            let lease = ContextLease {
                context,
                expires,
                live,
                session: self.transport.session.clone(),
                issuer: self.issuer.clone(),
            };
            if !lease.is_fresh() {
                return Err(Error::Stale);
            }
            state.current = Some(lease.clone());
            Ok(lease)
        })();
        if let Err(error) = result {
            if error == Error::IdentityChanged {
                self.transport.session.store(false, Ordering::Release);
            }
            if let Ok(mut state) = self.authority.lock() {
                if Arc::ptr_eq(&ticket, &state.ticket) {
                    state.retire();
                }
            }
        }
        result
    }
    pub fn context(&self) -> Result<ContextLease, Error> {
        self.observe("context", None)
    }
    pub fn enable(
        &self,
        c: &ContextLease,
        event_id: &str,
        enabled: bool,
    ) -> Result<ContextLease, Error> {
        if !self.owns(c) || !validate::uuid(event_id) {
            return Err(Error::Invalid);
        }
        if enabled {
            self.admission(c, false)?;
        } else {
            self.authority
                .lock()
                .map_err(|_| Error::Disconnected)?
                .retire();
        }
        let body = serde_json::to_vec(&wire::Enable {
            binding: c.context.binding.clone(),
            client_id: self.client_id.clone(),
            event_id: event_id.into(),
            enabled,
        })
        .map_err(|_| Error::Invalid)?;
        self.observe("enable", Some(body))
            .map_err(|_| Error::OutcomeUnknown)
    }
    pub fn heartbeat(&self, c: &ContextLease) -> Result<ContextLease, Error> {
        if !self.owns(c) || !c.is_fresh() {
            return Err(Error::Stale);
        }
        let body = serde_json::to_vec(&wire::Heartbeat {
            binding: c.context.binding.clone(),
            client_id: self.client_id.clone(),
        })
        .map_err(|_| Error::Invalid)?;
        self.observe("heartbeat", Some(body))
    }
    pub fn build_prepared(
        &self,
        c: &ContextLease,
        d: wire::Prepare,
        pcm: &[u8],
    ) -> Result<PreparedOperation, Error> {
        self.admission(c, true)?;
        validate::final_pcm(&c.context, &d, pcm, &self.client_id)?;
        let json = serde_json::to_vec(&d).map_err(|_| Error::Invalid)?;
        if json.is_empty() || json.len() > SMALL {
            return Err(Error::TooLarge);
        }
        let length = 4usize
            .checked_add(json.len())
            .and_then(|n| n.checked_add(pcm.len()))
            .ok_or(Error::TooLarge)?;
        if length > MAX_PREPARE || length > c.context.max_prepare_bytes {
            return Err(Error::TooLarge);
        }
        let mut body = Vec::with_capacity(length);
        body.extend_from_slice(&(json.len() as u32).to_le_bytes());
        body.extend(json);
        body.extend_from_slice(pcm);
        let key = self.key(
            &d.binding,
            &d.operation_id,
            &d.event_id,
            &body,
            Some((d.slot_start_unix_us, d.latest_start_unix_us)),
        );
        Ok(self.operation(c, key, body, Some(d)))
    }
    pub fn build_tune(&self, c: &ContextLease, d: wire::Tune) -> Result<PreparedOperation, Error> {
        self.admission(c, true)?;
        let hz = f32::from_bits(d.tone_hz_f32_bits);
        let gain = f32::from_bits(d.gain_f32_bits);
        if !gain.is_finite()
            || gain <= 0.0
            || gain > 0.8
            || d.binding != c.context.binding
            || d.client_id != self.client_id
            || !validate::uuid(&d.operation_id)
            || !validate::uuid(&d.event_id)
            || !(1..=30_000).contains(&d.duration_ms)
            || !hz.is_finite()
            || hz < c.context.min_offset_hz as f32
            || hz > c.context.max_offset_hz as f32
        {
            return Err(Error::Invalid);
        }
        let body = serde_json::to_vec(&d).map_err(|_| Error::Invalid)?;
        if body.len() > SMALL {
            return Err(Error::TooLarge);
        }
        Ok(self.operation(
            c,
            self.key(&d.binding, &d.operation_id, &d.event_id, &body, None),
            body,
            None,
        ))
    }
    fn key(
        &self,
        b: &wire::Binding,
        id: &str,
        event: &str,
        body: &[u8],
        slot_window: Option<(u64, u64)>,
    ) -> OperationKey {
        let (body_digest, body_sha256) = validate::digest(body);
        OperationKey {
            binding: b.clone(),
            client_id: self.client_id.clone(),
            operation_id: id.into(),
            origin_event_id: event.into(),
            body_sha256,
            body_digest,
            slot_window,
        }
    }
    fn operation(
        &self,
        c: &ContextLease,
        key: OperationKey,
        body: Vec<u8>,
        descriptor: Option<wire::Prepare>,
    ) -> PreparedOperation {
        PreparedOperation(Arc::new(OperationState {
            key,
            body,
            descriptor,
            context: c.clone(),
            dispatched: AtomicBool::new(false),
            committed: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            receipt: Mutex::new(None),
        }))
    }
    fn check_operation(&self, op: &PreparedOperation) -> Result<(), Error> {
        if !self.owns(&op.0.context) {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    fn send_initial(
        &self,
        op: &PreparedOperation,
        binary: bool,
    ) -> Result<ValidatedReceipt, Error> {
        self.check_operation(op)?;
        self.admission(&op.0.context, true)?;
        if self
            .ordinary_busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(Error::Stale);
        }
        let _permit = Permit(&self.ordinary_busy);
        if op.0.descriptor.is_some() != binary {
            return Err(Error::Invalid);
        }
        if op.0.stopped.load(Ordering::Acquire)
            || op
                .0
                .dispatched
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return Err(Error::OutcomeUnknown);
        }
        let receipt = self
            .transport
            .request(
                if binary { "prepare" } else { "tune" },
                Some((&op.0.body, binary)),
                binary,
            )
            .map_err(|_| Error::OutcomeUnknown)?;
        accept(&self.transport, op, receipt).map_err(|_| Error::OutcomeUnknown)
    }
    pub fn prepare(&self, op: &PreparedOperation) -> Result<ValidatedReceipt, Error> {
        self.send_initial(op, true)
    }
    pub fn tune(&self, op: &PreparedOperation) -> Result<ValidatedReceipt, Error> {
        self.send_initial(op, false)
    }
    pub fn commit(
        &self,
        c: &ContextLease,
        op: &PreparedOperation,
        event_id: &str,
    ) -> Result<ValidatedReceipt, Error> {
        self.check_operation(op)?;
        self.admission(c, true)?;
        if self
            .ordinary_busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(Error::Stale);
        }
        let _permit = Permit(&self.ordinary_busy);
        if (c.context.binding != op.key().binding
            || c.context.identity != op.0.context.context.identity
            || c.context.route != op.0.context.context.route)
            || !validate::uuid(event_id)
            || event_id == op.key().origin_event_id
            || op.0.descriptor.is_none()
        {
            return Err(Error::Invalid);
        }
        let proof = op.cached_receipt().ok_or(Error::OutcomeUnknown)?;
        if proof.receipt.phase != wire::Phase::Prepared
            || op.0.stopped.load(Ordering::Acquire)
            || op
                .0
                .committed
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return Err(Error::OutcomeUnknown);
        }
        let body = operation_body(op, event_id)?;
        let receipt = self
            .transport
            .request("commit", Some((&body, false)), false)
            .map_err(|_| Error::OutcomeUnknown)?;
        accept(&self.transport, op, receipt).map_err(|_| Error::OutcomeUnknown)
    }
    pub fn status(&self, op: &PreparedOperation) -> Result<Option<ValidatedReceipt>, Error> {
        self.check_operation(op)?;
        let status: wire::Status = self.transport.request(
            &format!("status?operation_id={}", op.key().operation_id),
            None,
            false,
        )?;
        if status.schema_version != 1
            || status.server_instance != op.key().binding.server_instance
            || status.workspace_id != op.key().binding.workspace_id
            || status.operation_id != op.key().operation_id
        {
            self.transport.session.store(false, Ordering::Release);
            return Err(Error::IdentityChanged);
        }
        status
            .receipt
            .map(|r| accept(&self.transport, op, r))
            .transpose()
    }
    pub fn stop_handle(&self, op: &PreparedOperation) -> Result<TxStopClient, Error> {
        self.check_operation(op)?;
        Ok(TxStopClient {
            // Release only: retain original authenticated endpoint independently
            // from ordinary observation/session retirement, like rotor Stop.
            // The owner still authenticates and checks the immutable operation.
            transport: Transport {
                http: self.transport.http.clone(),
                endpoint: self.transport.endpoint.clone(),
                session: Arc::new(AtomicBool::new(true)),
            },
            authority: self.authority.clone(),
            operation: op.clone(),
        })
    }
}
fn operation_body(op: &PreparedOperation, event_id: &str) -> Result<Vec<u8>, Error> {
    serde_json::to_vec(&wire::Operation {
        binding: op.key().binding.clone(),
        client_id: op.key().client_id.clone(),
        operation_id: op.key().operation_id.clone(),
        body_sha256: op.key().body_sha256.clone(),
        event_id: event_id.into(),
    })
    .map_err(|_| Error::Invalid)
}
fn accept(
    transport: &Transport,
    op: &PreparedOperation,
    receipt: wire::Receipt,
) -> Result<ValidatedReceipt, Error> {
    if !transport.session.load(Ordering::Acquire) {
        return Err(Error::Disconnected);
    }
    let mut current = op.0.receipt.lock().map_err(|_| Error::Disconnected)?;
    if let Err(e) = validate::receipt(&receipt, op.key(), current.as_ref().map(|r| &r.receipt)) {
        if e == Error::IdentityChanged {
            transport.session.store(false, Ordering::Release);
        }
        return Err(e);
    }
    let proof = ValidatedReceipt {
        key: op.key().clone(),
        receipt,
    };
    *current = Some(proof.clone());
    Ok(proof)
}
/// Independent exact-operation release path. No context refresh or ordinary-client lock.
pub struct TxStopClient {
    transport: Transport,
    authority: Arc<Mutex<Authority>>,
    operation: PreparedOperation,
}
impl TxStopClient {
    pub fn stop(&self, event_id: &str) -> Result<ValidatedReceipt, Error> {
        if !validate::uuid(event_id) || event_id == self.operation.key().origin_event_id {
            return Err(Error::Invalid);
        }
        self.operation.0.stopped.store(true, Ordering::Release);
        if let Ok(mut s) = self.authority.lock() {
            s.retire();
        }
        let body = operation_body(&self.operation, event_id)?;
        let receipt = self
            .transport
            .request("stop", Some((&body, false)), false)
            .map_err(|_| Error::OutcomeUnknown)?;
        accept(&self.transport, &self.operation, receipt).map_err(|_| Error::OutcomeUnknown)
    }
}

#[cfg(test)]
mod tests;
