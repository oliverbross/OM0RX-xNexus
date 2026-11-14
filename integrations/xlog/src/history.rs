// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Explicit bounded call history reads from the logbook owner.
use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryContact {
    pub contact_id: String,
    pub callsign: String,
    pub band: String,
    pub mode: String,
    pub started_at_unix_ms: u64,
    pub frequency_hz: Option<u64>,
    pub rst_sent: String,
    pub rst_received: String,
    pub grid: String,
    pub comment: String,
    pub contact_revision: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryCell {
    pub band: String,
    pub mode: String,
    pub worked: bool,
    pub confirmed: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct History {
    pub protocol_version: u32,
    pub server_instance: String,
    pub workspace_id: String,
    pub callsign: String,
    pub generated_at_unix_ms: u64,
    pub contacts: Vec<HistoryContact>,
    pub matrix: Vec<HistoryCell>,
}
fn call(value: &str) -> bool {
    bounded(value, 32)
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'/' || b == b'-')
}
fn optional(value: &str, max: usize) -> bool {
    value.is_empty() || bounded(value, max)
}
impl History {
    fn validate(&self, hello: &Hello, callsign: &str) -> Result<(), Error> {
        if self.protocol_version != 1 {
            return Err(Error::Incompatible);
        }
        if self.server_instance != hello.server_instance || self.workspace_id != hello.workspace_id
        {
            return Err(Error::IdentityChanged);
        }
        let now = now_ms()?;
        if self.generated_at_unix_ms > now.saturating_add(2000)
            || now.saturating_sub(self.generated_at_unix_ms) > 60_000
        {
            return Err(Error::Stale);
        }
        if self.callsign != callsign || self.contacts.len() > 25 || self.matrix.len() > 256 {
            return Err(Error::Invalid);
        }
        let mut ids = HashSet::new();
        for c in &self.contacts {
            if !commands::valid_request_id(&c.contact_id)
                || !ids.insert(&c.contact_id)
                || !call(&c.callsign)
                || !bounded(&c.band, 32)
                || !bounded(&c.mode, 32)
                || c.started_at_unix_ms > 4_102_444_800_000
                || c.frequency_hz.is_some_and(|v| !frequency(v))
                || !optional(&c.rst_sent, 32)
                || !optional(&c.rst_received, 32)
                || !optional(&c.grid, 32)
                || c.comment.len() > 4096
                || c.comment
                    .chars()
                    .any(|v| v.is_control() && !matches!(v, '\r' | '\n' | '\t'))
                || c.contact_revision
                    .is_some_and(|v| v == 0 || v > 9_007_199_254_740_990)
            {
                return Err(Error::Invalid);
            }
        }
        let mut cells = HashSet::new();
        if self.matrix.iter().any(|c| {
            !bounded(&c.band, 32) || !bounded(&c.mode, 32) || !cells.insert((&c.band, &c.mode))
        }) {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
impl BridgeClient {
    /// On-demand read only. Missing revision never grants permission to edit a contact.
    pub fn history(&mut self, callsign: &str) -> Result<History, Error> {
        if !call(callsign) {
            return Err(Error::Invalid);
        }
        let callsign = callsign.to_ascii_uppercase();
        let result = (|| {
            let hello = self.hello.as_ref().ok_or(Error::Disconnected)?;
            if !hello.capabilities.iter().any(|c| c == "logbook.read") {
                return Err(Error::Capability);
            }
            let history: History = self.get(
                &format!("/xlog/v1/history?callsign={}", callsign.replace('/', "%2F")),
                256 * 1024,
            )?;
            history.validate(hello, &callsign)?;
            Ok(history)
        })();
        if matches!(
            result,
            Err(Error::IdentityChanged | Error::Http(401 | 403) | Error::Incompatible)
        ) {
            self.disconnect();
        }
        result
    }
}
