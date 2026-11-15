// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Passive bearing IPC lane. Selection is retired BEFORE context lookup, so a
//! busy main poll or invalid new selection cannot leave older geometry current.
//! No main-client/global/Stop lock is retained over HTTP or serialization.
use serde::{ser::Error as _, Serialize, Serializer};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use xnexus_xlog_client::{
    bearing::client::{
        BearingClient, BearingContext, BearingLease, BearingReadError, BearingReadFence,
        BearingReadRequest,
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
struct Selection {
    sequence: u64,
    live: AtomicBool,
}
impl Selection {
    fn check(&self) -> Result<(), String> {
        if self.sequence != 0 && self.live.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err("Bearing selection changed".into())
        }
    }
}
#[derive(Default)]
struct Selections {
    sequence: u64,
    current: Option<Arc<Selection>>,
}
struct Permit(Arc<AtomicBool>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub(super) struct Lane {
    client: Mutex<Option<BearingClient>>,
    sdk_fence: Option<BearingReadFence>,
    scope: Arc<Scope>,
    selections: Mutex<Selections>,
    busy: Arc<AtomicBool>,
}
pub(super) struct Prepared {
    request: BearingReadRequest,
    scope: Arc<Scope>,
    selection: Arc<Selection>,
    permit: Permit,
}
impl Lane {
    pub(super) fn new(client: &BridgeClient, connection_token: &str) -> Result<Self, String> {
        let handle = match client.bearing_handle(connection_token) {
            Ok(handle) => Some(handle),
            Err(Error::Capability) => None,
            Err(error) => return Err(error.to_string()),
        };
        let sdk_fence = handle.as_ref().map(BearingClient::fence);
        Ok(Self {
            client: Mutex::new(handle),
            sdk_fence,
            scope: Arc::new(Scope {
                token: connection_token.into(),
                live: AtomicBool::new(true),
            }),
            selections: Mutex::new(Selections::default()),
            busy: Arc::new(AtomicBool::new(false)),
        })
    }
    fn select(&self, expected: &str) -> Result<Arc<Selection>, String> {
        self.scope.check(expected)?;
        let mut state = self.selections.lock().map_err(|_| {
            self.revoke();
            "xLog bearing selection unavailable"
        })?;
        if let Some(old) = state.current.take() {
            old.live.store(false, Ordering::Release);
        }
        let Some(sequence) = state.sequence.checked_add(1) else {
            self.revoke();
            return Err("xLog bearing selection exhausted".into());
        };
        state.sequence = sequence;
        let selection = Arc::new(Selection {
            sequence,
            live: AtomicBool::new(true),
        });
        state.current = Some(selection.clone());
        self.scope.check(expected)?;
        Ok(selection)
    }
    /// Synchronous pre-schedule admission. Context closure MUST be nonblocking;
    /// the command obtains only SDK context through the ordinary client's try_lock.
    pub(super) fn capture(
        &self,
        expected: &str,
        callsign: &str,
        context: impl FnOnce() -> Result<BearingContext, String>,
    ) -> Result<Prepared, String> {
        let selection = self.select(expected)?;
        let fence = self
            .sdk_fence
            .as_ref()
            .ok_or("xLog bearing is unsupported")?;
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "xLog bearing is busy")?;
        let permit = Permit(self.busy.clone());
        // Failed context lookup or SDK validation still leaves prior native facts retired.
        let context = context()?;
        self.scope.check(expected)?;
        selection.check()?;
        let request = fence
            .capture(expected, callsign, context)
            .map_err(|error| error.to_string())?;
        self.scope.check(expected)?;
        selection.check()?;
        Ok(Prepared {
            request,
            scope: self.scope.clone(),
            selection,
            permit,
        })
    }
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
            selection,
            permit,
        } = prepared;
        let _permit = permit;
        if !Arc::ptr_eq(&scope, &self.scope) {
            return Err("Connection changed".into());
        }
        self.scope.check(&scope.token)?;
        selection.check()?;
        let mut client = self
            .client
            .try_lock()
            .map_err(|_| "xLog bearing is busy or unavailable")?;
        self.scope.check(&scope.token)?;
        selection.check()?;
        let lease = client
            .as_mut()
            .ok_or("xLog bearing is unsupported")?
            .read(&request)
            .map_err(|error| match error {
                BearingReadError::Unavailable(code) => format!("xLog bearing unavailable ({code})"),
                BearingReadError::ContextChanged => "xLog bearing station context changed".into(),
                BearingReadError::Bridge(error) => error.to_string(),
            })?;
        self.scope.check(&scope.token)?;
        selection.check()?;
        lease.bearing().map_err(|error| error.to_string())?;
        Ok(Response {
            lease,
            scope,
            selection,
        })
    }
}
impl Drop for Lane {
    fn drop(&mut self) {
        self.revoke();
    }
}
pub(super) struct Response {
    lease: BearingLease,
    scope: Arc<Scope>,
    selection: Arc<Selection>,
}
impl Serialize for Response {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.scope
            .check(&self.scope.token)
            .map_err(S::Error::custom)?;
        self.selection.check().map_err(S::Error::custom)?;
        let result = self.lease.serialize(serializer);
        self.scope
            .check(&self.scope.token)
            .map_err(S::Error::custom)?;
        self.selection.check().map_err(S::Error::custom)?;
        result
    }
}
#[cfg(test)]
#[path = "xlog_bearing_read_tests.rs"]
mod tests;
