// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Explicit bounded call history reads from the logbook owner.
use super::*;
mod read_model;
pub use read_model::{HistoryClient, HistoryQuery, HistorySummary, ReadLease, Recent};

#[derive(Debug, Clone)]
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
    pub submode: Option<String>,
    pub exact_mode: Option<String>,
    pub details_complete: Option<bool>,
    revision_present: bool,
    frequency_present: bool,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logbook_revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contacts_limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contacts_complete: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrix_complete: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_summary: Option<HistorySummary>,
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
fn validate_envelope(
    protocol_version: u32,
    server_instance: &str,
    workspace_id: &str,
    generated_at_unix_ms: u64,
    hello: &Hello,
    now: u64,
) -> Result<(), Error> {
    if protocol_version != 1 {
        return Err(Error::Incompatible);
    }
    if server_instance != hello.server_instance || workspace_id != hello.workspace_id {
        return Err(Error::IdentityChanged);
    }
    if generated_at_unix_ms > now.saturating_add(2000)
        || now.saturating_sub(generated_at_unix_ms) > 60_000
    {
        return Err(Error::Stale);
    }
    Ok(())
}
fn validate_contacts(contacts: &[HistoryContact], limit: usize) -> Result<(), Error> {
    if contacts.len() > limit {
        return Err(Error::Invalid);
    }
    let mut ids = HashSet::new();
    for c in contacts {
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
        c.validate_details(false)?;
    }
    Ok(())
}
impl History {
    fn validate(&self, hello: &Hello, callsign: &str) -> Result<(), Error> {
        self.validate_at(hello, callsign, now_ms()?)
    }
    fn validate_at(&self, hello: &Hello, callsign: &str, now: u64) -> Result<(), Error> {
        validate_envelope(
            self.protocol_version,
            &self.server_instance,
            &self.workspace_id,
            self.generated_at_unix_ms,
            hello,
            now,
        )?;
        if self.callsign != callsign || self.matrix.len() > 256 {
            return Err(Error::Invalid);
        }
        validate_contacts(&self.contacts, 25)?;
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
            let hello = self.hello().ok_or(Error::Disconnected)?;
            if !hello.capabilities.iter().any(|c| c == "logbook.read") {
                return Err(Error::Capability);
            }
            let history: History = self.get(
                &format!("/xlog/v1/history?callsign={}", callsign.replace('/', "%2F")),
                256 * 1024,
            )?;
            history.validate(hello, &callsign)?;
            history.validate_read_model(hello, &HistoryQuery::new(&callsign, None, None)?)?;
            self.hello().ok_or(Error::Disconnected)?;
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
