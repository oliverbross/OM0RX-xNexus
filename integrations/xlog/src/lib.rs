// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Native observation and receive/control client for the xLog companion bridge.
//!
//! xLog is the authority. This crate neither opens a log database nor starts
//! hardware, feeds, a Nexus engine, or a transmitter. A failed or stale poll
//! removes usable state. Reconnection is explicit and replaces session identity.
//! HTTP is limited to an explicitly configured IPv4 loopback endpoint, with no
//! redirects/proxies. Error messages never include payloads, URLs or credentials.
//! Controls require explicit advertised capabilities. No transmission API exists.
//! Availability depends on the companion bridge in the paired xLog build.

use reqwest::{blocking::Client, header, redirect::Policy, Url};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt,
    io::Read,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_AGE_MS: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Endpoint,
    Token,
    Transport,
    Http(u16),
    TooLarge,
    Invalid,
    Incompatible,
    IdentityChanged,
    Revision,
    Stale,
    Disconnected,
    Capability,
    OutcomeUnknown,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Endpoint => "Expected http://127.0.0.1:<port> with no path or credentials",
            Self::Token => "Invalid pairing token",
            Self::Transport => "xLog bridge unavailable or timed out",
            Self::Http(_) => "xLog bridge refused the request",
            Self::TooLarge => "xLog bridge response exceeds the size limit",
            Self::Invalid => "Invalid xLog bridge response",
            Self::Incompatible => "Incompatible xLog bridge protocol or capability",
            Self::IdentityChanged => "xLog session or workspace changed; reconnect explicitly",
            Self::Revision => "xLog snapshot revision is inconsistent",
            Self::Stale => "xLog observations are stale",
            Self::Disconnected => "Connect to xLog before requesting observations",
            Self::Capability => "xLog has not granted this capability",
            Self::OutcomeUnknown => {
                "The action outcome is unknown; check its receipt before retrying"
            }
        })
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Hello {
    pub application: String,
    pub protocol_version: u32,
    pub server_instance: String,
    pub workspace_id: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Station {
    pub connected: bool,
    pub radio_id: Option<String>,
    pub frequency_hz: Option<u64>,
    pub mode: Option<String>,
    pub transmitting: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Rotor {
    pub connected: bool,
    pub rotor_id: Option<String>,
    pub azimuth_deg: Option<f64>,
    pub moving: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedStatus {
    Needed,
    Worked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Need {
    pub status: NeedStatus,
    pub score: i32,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Spot {
    pub id: String,
    pub callsign: String,
    pub frequency_hz: u64,
    pub mode: String,
    pub observed_at_unix_ms: u64,
    pub source: String,
    pub need: Need,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub adif: Option<u32>,
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    #[serde(default)]
    pub band: Option<String>,
    #[serde(default)]
    pub call_status: Option<String>,
    #[serde(default)]
    pub dxcc_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Snapshot {
    pub protocol_version: u32,
    pub server_instance: String,
    pub workspace_id: String,
    pub revision: u64,
    pub generated_at_unix_ms: u64,
    pub max_age_ms: u64,
    pub station: Station,
    pub rotor: Rotor,
    pub spots: Vec<Spot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs: Option<needs::NeedsMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub station_identity: Option<StationIdentity>,
}

pub mod audio;
pub mod bearing;
pub mod contact_detail;
mod commands;
pub mod needs;
mod station_identity;
pub use commands::{Command, CommandReceipt, CommandStatus, StopClient};
pub use station_identity::{
    IdentityBinding, IdentitySource, IdentityState, OperatingModule, StationIdentity,
};
mod history;
pub mod outbox;
pub use history::{
    History, HistoryCell, HistoryClient, HistoryContact, HistoryQuery, HistorySummary, ReadLease,
    Recent,
};

fn bounded(text: &str, max: usize) -> bool {
    !text.trim().is_empty() && text.len() <= max && !text.chars().any(char::is_control)
}
fn frequency(hz: u64) -> bool {
    (1..=1_000_000_000_000).contains(&hz)
}
fn now_ms() -> Result<u64, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Stale)?
        .as_millis()
        .try_into()
        .map_err(|_| Error::Stale)
}

impl Snapshot {
    fn validate(&self, hello: &Hello, now: u64) -> Result<Duration, Error> {
        if self.protocol_version != 1 {
            return Err(Error::Incompatible);
        }
        if self.server_instance != hello.server_instance || self.workspace_id != hello.workspace_id
        {
            return Err(Error::IdentityChanged);
        }
        if let Some(identity) = &self.station_identity {
            identity.validate(&self.workspace_id)?;
        }
        if !(1..=MAX_AGE_MS).contains(&self.max_age_ms)
            || self.generated_at_unix_ms > now.saturating_add(2000)
        {
            return Err(Error::Stale);
        }
        let age = now.saturating_sub(self.generated_at_unix_ms);
        if age >= self.max_age_ms {
            return Err(Error::Stale);
        }
        if self
            .station
            .radio_id
            .as_ref()
            .is_some_and(|s| !bounded(s, 128))
            || (self.station.connected && self.station.radio_id.is_none())
            || self.station.frequency_hz.is_some_and(|v| !frequency(v))
            || self.station.mode.as_ref().is_some_and(|s| !bounded(s, 32))
            || self
                .rotor
                .rotor_id
                .as_ref()
                .is_some_and(|s| !bounded(s, 128))
            || (self.rotor.connected && self.rotor.rotor_id.is_none())
            || self
                .rotor
                .azimuth_deg
                .is_some_and(|v| !v.is_finite() || !(0.0..360.0).contains(&v))
            || self.spots.len() > 4096
        {
            return Err(Error::Invalid);
        }
        let mut ids = HashSet::new();
        for spot in &self.spots {
            if !bounded(&spot.id, 128)
                || !ids.insert(&spot.id)
                || !bounded(&spot.callsign, 32)
                || !spot
                    .callsign
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'/' || b == b'-')
                || !frequency(spot.frequency_hz)
                || !bounded(&spot.mode, 32)
                || !bounded(&spot.source, 256)
                || spot.country.as_ref().is_some_and(|s| !bounded(s, 128))
                || spot.band.as_ref().is_some_and(|s| !bounded(s, 32))
                || spot.call_status.as_ref().is_some_and(|s| !bounded(s, 64))
                || spot.dxcc_status.as_ref().is_some_and(|s| !bounded(s, 64))
                || spot
                    .latitude
                    .is_some_and(|v| !v.is_finite() || !(-90.0..=90.0).contains(&v))
                || spot
                    .longitude
                    .is_some_and(|v| !v.is_finite() || !(-180.0..=180.0).contains(&v))
                || spot.observed_at_unix_ms > self.generated_at_unix_ms.saturating_add(2000)
                || spot.need.reasons.len() > 32
                || spot.need.reasons.iter().any(|s| !bounded(s, 256))
                || (spot.need.status == NeedStatus::Needed && spot.need.reasons.is_empty())
            {
                return Err(Error::Invalid);
            }
        }
        Ok(Duration::from_millis(self.max_age_ms - age))
    }

    /// Preserve xLog's score; unknown is never interpreted as needed.
    /// Freshness is enforced by BridgeClient::snapshot before this is exposed.
    pub fn needed(&self) -> Vec<&Spot> {
        let mut spots: Vec<_> = self
            .spots
            .iter()
            .filter(|s| s.need.status == NeedStatus::Needed)
            .collect();
        spots.sort_by(|a, b| {
            b.need
                .score
                .cmp(&a.need.score)
                .then_with(|| a.callsign.cmp(&b.callsign))
                .then_with(|| a.id.cmp(&b.id))
        });
        spots
    }
}

struct Observation {
    snapshot: Snapshot,
    received: Instant,
    remaining: Duration,
}

/// Intentionally no Debug implementation: the HTTP client contains authorization.
pub struct BridgeClient {
    endpoint: String,
    http: Client,
    hello: Option<Hello>,
    observation: Option<Observation>,
    audio_session: Arc<AtomicBool>,
    needs_authority: Arc<std::sync::Mutex<needs::AuthorityState>>,
}

impl Drop for BridgeClient {
    fn drop(&mut self) {
        self.audio_session.store(false, Ordering::Release);
    }
}

impl BridgeClient {
    pub fn new(endpoint: &str, token: &str) -> Result<Self, Error> {
        // Check the raw form too: URL normalization must not admit an alternate host spelling.
        let port = endpoint
            .strip_prefix("http://127.0.0.1:")
            .ok_or(Error::Endpoint)?;
        if port.is_empty()
            || !port.bytes().all(|b| b.is_ascii_digit())
            || port.parse::<u16>().ok().filter(|p| *p != 0).is_none()
        {
            return Err(Error::Endpoint);
        }
        let parsed = Url::parse(endpoint).map_err(|_| Error::Endpoint)?;
        if parsed.host_str() != Some("127.0.0.1") {
            return Err(Error::Endpoint);
        }
        if !(32..=4096).contains(&token.len()) || !token.bytes().all(|b| (33..=126).contains(&b)) {
            return Err(Error::Token);
        }
        let mut auth =
            header::HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| Error::Token)?;
        auth.set_sensitive(true);
        let mut headers = header::HeaderMap::new();
        headers.insert(header::AUTHORIZATION, auth);
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        let http = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(3))
            .default_headers(headers)
            .build()
            .map_err(|_| Error::Transport)?;
        Ok(Self {
            endpoint: endpoint.into(),
            http,
            hello: None,
            observation: None,
            audio_session: Arc::new(AtomicBool::new(false)),
            needs_authority: Arc::new(std::sync::Mutex::new(needs::AuthorityState::default())),
        })
    }

    fn get<T: for<'de> Deserialize<'de>>(&self, path: &str, max: usize) -> Result<T, Error> {
        let response = self
            .http
            .get(format!("{}{path}", self.endpoint))
            .send()
            .map_err(|_| Error::Transport)?;
        read_response(response, max)
    }

    pub fn hello(&self) -> Option<&Hello> {
        self.audio_session
            .load(Ordering::Acquire)
            .then_some(())
            .and(self.hello.as_ref())
    }

    /// Explicitly establishes a new identity. Failure leaves no prior session usable.
    pub fn connect(&mut self) -> Result<&Hello, Error> {
        self.disconnect();
        let hello: Hello = self.get("/xlog/v1/hello", 65536)?;
        if hello.application != "OM0RX-xLog"
            || hello.protocol_version != 1
            || !hello.capabilities.iter().any(|s| s == "snapshot.read")
        {
            return Err(Error::Incompatible);
        }
        if !bounded(&hello.server_instance, 128)
            || !bounded(&hello.workspace_id, 128)
            || hello.capabilities.len() > 64
            || hello.capabilities.iter().any(|s| !bounded(s, 64))
        {
            return Err(Error::Invalid);
        }
        self.hello = Some(hello);
        self.audio_session = Arc::new(AtomicBool::new(true));
        self.needs_authority = Arc::new(std::sync::Mutex::new(needs::AuthorityState::default()));
        Ok(self.hello.as_ref().expect("stored hello"))
    }

    /// No automatic reconnect/retry: a failed request invalidates session authority.
    pub fn poll(&mut self) -> Result<&Snapshot, Error> {
        let result = (|| {
            let hello = self.hello().ok_or(Error::Disconnected)?;
            let snapshot: Snapshot = self.get("/xlog/v1/snapshot", MAX_BYTES)?;
            let remaining = snapshot.validate(hello, now_ms()?)?;
            let received = Instant::now();
            if let Some(old) = &self.observation {
                if snapshot.revision < old.snapshot.revision {
                    return Err(Error::Revision);
                }
                if snapshot.revision == old.snapshot.revision {
                    // Optional classification has its own authority and lease.
                    // Its metadata cannot change the meaning of a base revision.
                    let mut comparable_base = snapshot.clone();
                    comparable_base.needs = old.snapshot.needs.clone();
                    if comparable_base != old.snapshot {
                        return Err(Error::Revision);
                    }
                    return Ok(Observation {
                        snapshot,
                        received,
                        remaining: remaining
                            .min(old.remaining.saturating_sub(old.received.elapsed())),
                    });
                }
            }
            Ok(Observation {
                snapshot,
                received,
                remaining,
            })
        })();
        match result {
            Ok(observation) if !observation.remaining.is_zero() => {
                // Optional missing metadata clears only needs authority. Other
                // observation and receive capabilities remain usable.
                let metadata = observation.snapshot.needs.as_ref();
                let update = self
                    .needs_authority
                    .lock()
                    .map_err(|_| Error::Disconnected)
                    .and_then(|mut state| state.observe(metadata, now_ms()?));
                if update.is_err() {
                    // Needs has its own short freshness window. Expired,
                    // backwards or invalid optional metadata must not retire
                    // otherwise fresh station/audio/control observations.
                    self.invalidate_needs();
                }
                self.observation = Some(observation)
            }
            Ok(_) => {
                self.disconnect();
                return Err(Error::Stale);
            }
            Err(error) => {
                self.disconnect();
                return Err(error);
            }
        }
        self.snapshot()
    }

    pub fn snapshot(&self) -> Result<&Snapshot, Error> {
        if !self.audio_session.load(Ordering::Acquire) {
            return Err(Error::Disconnected);
        }
        let observation = self.observation.as_ref().ok_or(Error::Disconnected)?;
        if observation.received.elapsed() >= observation.remaining {
            return Err(Error::Stale);
        }
        Ok(&observation.snapshot)
    }
    pub fn disconnect(&mut self) {
        self.audio_session.store(false, Ordering::Release);
        self.invalidate_needs();
        self.hello = None;
        self.observation = None;
    }

    /// Clear needs facts immediately for a local pending/uncertain write fence.
    /// Only a subsequent fresh owner snapshot can restore query authority.
    pub fn invalidate_needs(&self) {
        self.needs_authority
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .invalidate();
    }
}

fn read_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::blocking::Response,
    max: usize,
) -> Result<T, Error> {
    if response.status().as_u16() != 200 {
        return Err(Error::Http(response.status().as_u16()));
    }
    if response.content_length().is_some_and(|n| n > max as u64) {
        return Err(Error::TooLarge);
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
        return Err(Error::Invalid);
    }
    let mut body = Vec::new();
    response
        .take(max as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|_| Error::Transport)?;
    if body.len() > max {
        return Err(Error::TooLarge);
    }
    serde_json::from_slice(&body).map_err(|_| Error::Invalid)
}

#[cfg(test)]
mod desktop_contract_tests;
#[cfg(test)]
mod tests;
