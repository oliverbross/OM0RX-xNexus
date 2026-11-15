// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Selected-contact IPC lane. No station/RX/Stop/global lock during HTTP or
//! serialization. Keep the SDK lease intact until the IPC serializer finishes.
use serde::{ser::Error as _, Serialize, Serializer};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use xnexus_xlog_client::{
    contact_detail::client::{
        ContactDetailClient, ContactLease, ContactReadError, ContactReadFence, ContactReadRequest,
    },
    BridgeClient, Error,
};

struct Scope {
    token: String,
    live: AtomicBool,
}
impl Scope {
    fn check(&self, expected: &str) -> Result<(), String> {
        if self.live.load(Ordering::Acquire) && self.token == expected {
            Ok(())
        } else {
            Err("Connection changed".into())
        }
    }
}
struct Permit(Arc<AtomicBool>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub(super) struct Lane {
    client: Mutex<Option<ContactDetailClient>>,
    sdk_fence: Option<ContactReadFence>,
    scope: Arc<Scope>,
    busy: Arc<AtomicBool>,
}
pub(super) struct Prepared {
    request: ContactReadRequest,
    scope: Arc<Scope>,
    permit: Permit,
}
impl Lane {
    pub(super) fn new(client: &BridgeClient, connection_token: &str) -> Result<Self, String> {
        let handle = match client.contact_detail_handle(connection_token) {
            Ok(handle) => Some(handle),
            Err(Error::Capability) => None,
            Err(error) => return Err(error.to_string()),
        };
        let sdk_fence = handle.as_ref().map(ContactDetailClient::fence);
        Ok(Self {
            client: Mutex::new(handle),
            sdk_fence,
            scope: Arc::new(Scope {
                token: connection_token.into(),
                live: AtomicBool::new(true),
            }),
            busy: Arc::new(AtomicBool::new(false)),
        })
    }
    /// No HTTP/client lock. Called before scheduling to bind selection and time.
    /// Even a busy new selection retires the previous selection's SDK authority.
    pub(super) fn capture(&self, expected: &str, contact_id: &str) -> Result<Prepared, String> {
        self.scope.check(expected)?;
        let request = self
            .sdk_fence
            .as_ref()
            .ok_or("xLog contact detail is unsupported")?
            .capture(expected, contact_id)
            .map_err(|error| error.to_string())?;
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "xLog contact detail is busy")?;
        let permit = Permit(self.busy.clone());
        self.scope.check(expected)?;
        Ok(Prepared {
            request,
            scope: self.scope.clone(),
            permit,
        })
    }
    /// Called by connection retirement synchronously; never waits for HTTP.
    pub(super) fn revoke(&self) {
        self.scope.live.store(false, Ordering::Release);
        if let Some(fence) = &self.sdk_fence {
            fence.revoke();
        }
    }
    pub(super) fn read(&self, prepared: Prepared) -> Result<Response, String> {
        let Prepared {
            request,
            scope,
            permit,
        } = prepared;
        let _permit = permit;
        if !Arc::ptr_eq(&scope, &self.scope) {
            return Err("Connection changed".into());
        }
        self.scope.check(&scope.token)?;
        let mut client = self
            .client
            .try_lock()
            .map_err(|_| "xLog contact detail is busy or unavailable")?;
        self.scope.check(&scope.token)?;
        let lease = client
            .as_mut()
            .ok_or("xLog contact detail is unsupported")?
            .read(&request)
            .map_err(|error| match error {
                ContactReadError::Unavailable(code) => {
                    format!("xLog contact detail unavailable ({code})")
                }
                ContactReadError::Bridge(error) => error.to_string(),
            })?;
        self.scope.check(&scope.token)?;
        lease.value().map_err(|error| error.to_string())?;
        Ok(Response { lease, scope })
    }
}
impl Drop for Lane {
    fn drop(&mut self) {
        self.revoke();
    }
}

pub(super) struct Response {
    lease: ContactLease,
    scope: Arc<Scope>,
}
impl Serialize for Response {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.scope
            .check(&self.scope.token)
            .map_err(S::Error::custom)?;
        // Delegate to the retained SDK lease; never clone a bare Envelope.
        let result = self.lease.serialize(serializer);
        self.scope
            .check(&self.scope.token)
            .map_err(S::Error::custom)?;
        result
    }
}

#[cfg(test)]
#[path = "xlog_contact_detail_read_tests.rs"]
mod tests;
