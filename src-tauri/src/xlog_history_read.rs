// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Passive owner reads, independent of the station/RX/Stop locks. No local log.
//! Keep leases intact until serialization; the webview must still match the
//! connection/query and use its PRE-invoke monotonic time for the display limit.

use ring::rand::{SecureRandom, SystemRandom};
use serde::{ser::Error as _, ser::SerializeStruct, Serialize, Serializer};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use xnexus_xlog_client::{
    BridgeClient, Error, History, HistoryClient, HistoryQuery, ReadLease, Recent,
};

pub(super) struct Fence {
    token: String,
    live: AtomicBool,
}

impl Fence {
    fn new() -> Result<Arc<Self>, String> {
        // Existing shell dependency; an opaque per-connection nonce, not a
        // numeric generation that JavaScript could round or a persisted ID.
        let mut bytes = [0u8; 16];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| "Connection identity unavailable")?;
        Ok(Arc::new(Self {
            token: bytes.iter().map(|b| format!("{b:02x}")).collect(),
            live: AtomicBool::new(true),
        }))
    }

    pub(super) fn token(&self) -> &str {
        &self.token
    }

    pub(super) fn revoke(&self) {
        self.live.store(false, Ordering::Release);
    }

    fn check(&self, expected: &str) -> Result<(), String> {
        if self.live.load(Ordering::Acquire) && expected == self.token {
            Ok(())
        } else {
            Err("Connection changed".into())
        }
    }
}

pub(super) struct Lane {
    client: Mutex<Option<HistoryClient>>,
    fence: Arc<Fence>,
}

impl Lane {
    pub(super) fn new(client: &BridgeClient) -> Result<Self, String> {
        let read = match client.history_handle() {
            Ok(read) => Some(read),
            Err(Error::Capability) => None,
            Err(error) => return Err(error.to_string()),
        };
        Ok(Self {
            client: Mutex::new(read),
            fence: Fence::new()?,
        })
    }

    pub(super) fn token(&self) -> &str {
        self.fence.token()
    }

    /// Synchronous, never waits for an outstanding read or station command.
    pub(super) fn revoke(&self) {
        self.fence.revoke();
    }

    fn read<T>(
        &self,
        expected: &str,
        request: impl FnOnce(&mut HistoryClient) -> Result<ReadLease<T>, Error>,
    ) -> Result<Response<T>, String> {
        self.fence.check(expected)?;
        // Fail busy rather than growing a queue behind a slow database read.
        let mut handle = self
            .client
            .try_lock()
            .map_err(|_| "xLog history is busy or unavailable")?;
        self.fence.check(expected)?;
        let lease = request(handle.as_mut().ok_or("xLog history is unsupported")?)
            .map_err(|error| error.to_string())?;
        self.fence.check(expected)?;
        lease.value().map_err(|error| error.to_string())?;
        Ok(Response {
            lease,
            fence: Arc::clone(&self.fence),
        })
    }

    pub(super) fn history(
        &self,
        expected: &str,
        query: &HistoryQuery,
    ) -> Result<Response<History>, String> {
        self.read(expected, |client| client.history(query))
    }

    pub(super) fn recent(&self, expected: &str) -> Result<Response<Recent>, String> {
        self.read(expected, HistoryClient::recent)
    }
}

impl Drop for Lane {
    fn drop(&mut self) {
        self.revoke();
    }
}

pub(super) struct Response<T> {
    lease: ReadLease<T>,
    fence: Arc<Fence>,
}

impl<T: Serialize> Serialize for Response<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.fence
            .check(self.fence.token())
            .map_err(S::Error::custom)?;
        let payload = self.lease.value().map_err(S::Error::custom)?;
        let mut value = serializer.serialize_struct("HistoryRead", 3)?;
        value.serialize_field("payload", payload)?;
        value.serialize_field("connection_token", self.fence.token())?;
        // Account for payload serialization as well as HTTP time. A later
        // disconnect can still race IPC, hence the consumer's context fence.
        let remaining = self.lease.remaining_valid_ms();
        self.fence
            .check(self.fence.token())
            .map_err(S::Error::custom)?;
        if remaining == 0 {
            return Err(S::Error::custom("xLog history expired"));
        }
        value.serialize_field("remaining_valid_ms", &remaining)?;
        value.end()
    }
}

#[cfg(test)]
#[path = "xlog_history_read_tests.rs"]
mod tests;
