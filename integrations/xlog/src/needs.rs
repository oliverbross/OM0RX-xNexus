// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Optional owner-only classification. No contact export, scoring, hardware or TX.
//! Keep this handle on a dedicated bounded lookup worker, outside PCM/Stop locks.
//! Returned facts have revocable session, snapshot-authority and monotonic leases.
use super::*;
use std::collections::HashMap;
use std::sync::Mutex;

const SAFE: u64 = 9_007_199_254_740_990;
const REQUEST_BYTES: usize = 16 * 1024;
const RESPONSE_BYTES: usize = 64 * 1024;

fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}
fn opaque(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && s.bytes().all(|b| (0x20..=0x7e).contains(&b))
}
fn text_bound(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}
fn query_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}
fn normalized_call(s: &str) -> bool {
    (3..=32).contains(&s.len())
        && !matches!(s, "RR73" | "RRR" | "R73" | "73")
        && !(s.len() <= 3
            && s.as_bytes()[0].is_ascii_digit()
            && s.bytes().all(|b| b.is_ascii_digit() || b == b'N'))
        && s.bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'/')
        && s.bytes().any(|b| b.is_ascii_uppercase())
        && s.bytes().any(|b| b.is_ascii_digit())
        && !s.starts_with('/')
        && !s.ends_with('/')
        && !s.contains("//")
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecoderMode {
    FT8,
    FT4,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NeedsQuery {
    pub query_id: String,
    pub callsign: String,
    pub dial_frequency_hz: u64,
    pub mode: DecoderMode,
}
impl NeedsQuery {
    fn validate(&self) -> bool {
        query_id(&self.query_id)
            && normalized_call(&self.callsign)
            && frequency(self.dial_frequency_hz)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Readiness {
    Ready,
    Unavailable,
    Refreshing,
    Uncertain,
    Stale,
}
impl Readiness {
    fn code(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Unavailable => "unavailable",
            Self::Refreshing => "refreshing",
            Self::Uncertain => "uncertain",
            Self::Stale => "stale",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedsAuthority {
    pub logbook_revision: u64,
    pub classification_revision: u64,
    pub country_reference_revision: String,
    pub policy_revision: String,
}
impl NeedsAuthority {
    fn valid(&self) -> bool {
        self.logbook_revision <= SAFE
            && self.classification_revision <= SAFE
            && opaque(&self.country_reference_revision, 512)
            && opaque(&self.policy_revision, 64)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedsMetadata {
    pub generated_at_unix_ms: u64,
    pub max_age_ms: u64,
    pub readiness: Readiness,
    #[serde(deserialize_with = "required_option")]
    pub authority: Option<NeedsAuthority>,
}
impl NeedsMetadata {
    fn lifetime(&self, now: u64) -> Result<Duration, Error> {
        if !(1..=2000).contains(&self.max_age_ms)
            || self.generated_at_unix_ms > now.saturating_add(50)
            || now.saturating_sub(self.generated_at_unix_ms) >= self.max_age_ms
        {
            return Err(Error::Stale);
        }
        if (self.readiness == Readiness::Ready) != self.authority.is_some()
            || self.authority.as_ref().is_some_and(|a| !a.valid())
        {
            return Err(Error::Invalid);
        }
        Ok(Duration::from_millis(
            self.max_age_ms - now.saturating_sub(self.generated_at_unix_ms),
        ))
    }
    fn same_authority(&self, other: &Self) -> bool {
        self.readiness == other.readiness && self.authority == other.authority
    }
}
struct AuthorityObservation {
    metadata: NeedsMetadata,
    expires: Instant,
    live: Arc<AtomicBool>,
}
#[derive(Default)]
pub(crate) struct AuthorityState {
    current: Option<AuthorityObservation>,
    last_timestamp: Option<(u64, Instant)>,
}
#[derive(Clone)]
struct AuthorityLease {
    metadata: NeedsMetadata,
    expires: Instant,
    live: Arc<AtomicBool>,
}
impl AuthorityLease {
    fn fresh(&self) -> bool {
        self.live.load(Ordering::Acquire) && Instant::now() < self.expires
    }
}
impl AuthorityState {
    pub(crate) fn invalidate(&mut self) {
        if let Some(old) = self.current.take() {
            old.live.store(false, Ordering::Release);
        }
    }
    pub(crate) fn observe(
        &mut self,
        metadata: Option<&NeedsMetadata>,
        now: u64,
    ) -> Result<(), Error> {
        let Some(metadata) = metadata else {
            self.invalidate();
            return Ok(());
        };
        let duration = match metadata.lifetime(now) {
            Ok(d) => d,
            Err(e) => {
                self.invalidate();
                return Err(e);
            }
        };
        let mut expires = Instant::now() + duration;
        if let Some((generated, deadline)) = self.last_timestamp {
            if metadata.generated_at_unix_ms < generated {
                self.invalidate();
                return Err(Error::Revision);
            }
            if metadata.generated_at_unix_ms == generated {
                expires = expires.min(deadline);
            }
        }
        self.last_timestamp = Some((metadata.generated_at_unix_ms, expires));
        if Instant::now() >= expires {
            self.invalidate();
            return Err(Error::Stale);
        }
        if let Some(old) = self.current.as_mut() {
            if metadata.same_authority(&old.metadata)
                && old.live.load(Ordering::Acquire)
                && Instant::now() < old.expires
            {
                if metadata.generated_at_unix_ms < old.metadata.generated_at_unix_ms {
                    self.invalidate();
                    return Err(Error::Revision);
                }
                if metadata.generated_at_unix_ms == old.metadata.generated_at_unix_ms {
                    expires = expires.min(old.expires);
                }
                old.metadata = metadata.clone();
                old.expires = expires;
                return Ok(());
            }
        }
        self.invalidate();
        self.current = Some(AuthorityObservation {
            metadata: metadata.clone(),
            expires,
            live: Arc::new(AtomicBool::new(true)),
        });
        Ok(())
    }
    fn lease(&self) -> Result<AuthorityLease, Error> {
        let current = self.current.as_ref().ok_or(Error::Stale)?;
        let lease = AuthorityLease {
            metadata: current.metadata.clone(),
            expires: current.expires,
            live: current.live.clone(),
        };
        if !lease.fresh() {
            return Err(Error::Stale);
        }
        Ok(lease)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallStatus {
    #[serde(rename = "New Call")]
    NewCall,
    #[serde(rename = "New Band")]
    NewBand,
    #[serde(rename = "New Mode")]
    NewMode,
    #[serde(rename = "New Band+Mode")]
    NewBandMode,
    Worked,
    Confirmed,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DxccStatus {
    #[serde(rename = "ATNO")]
    Atno,
    #[serde(rename = "C/New Band")]
    ConfirmedNewBand,
    #[serde(rename = "C/New Mode")]
    ConfirmedNewMode,
    #[serde(rename = "C/New Band+Mode")]
    ConfirmedNewBandMode,
    #[serde(rename = "W/New Band")]
    WorkedNewBand,
    #[serde(rename = "W/New Mode")]
    WorkedNewMode,
    #[serde(rename = "W/New Band+Mode")]
    WorkedNewBandMode,
    Worked,
    Confirmed,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CountryResolution {
    Resolved,
    KnownNoEntity,
    Ambiguous,
    Unknown,
    Invalid,
    MissingDate,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryNeed {
    pub status: NeedStatus,
    #[serde(deserialize_with = "required_option")]
    pub score: Option<i32>,
    #[serde(deserialize_with = "required_option")]
    pub reason_code: Option<String>,
    pub reasons: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedsResult {
    pub query_id: String,
    pub normalized_callsign: String,
    pub dial_frequency_hz: u64,
    pub decoder_mode: DecoderMode,
    pub classification_mode: String,
    #[serde(deserialize_with = "required_option")]
    pub band: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub country: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub adif: Option<u32>,
    #[serde(deserialize_with = "required_option")]
    pub country_resolution: Option<CountryResolution>,
    pub call_status: CallStatus,
    pub dxcc_status: DxccStatus,
    pub need: QueryNeed,
}
impl NeedsResult {
    fn validate(&self, query: &NeedsQuery, readiness: Readiness) -> Result<(), Error> {
        if self.query_id != query.query_id
            || self.normalized_callsign != query.callsign
            || self.dial_frequency_hz != query.dial_frequency_hz
            || self.decoder_mode != query.mode
            || self.classification_mode != "DIGI"
            || self.band.as_ref().is_some_and(|s| !text_bound(s, 32))
            || self.country.as_ref().is_some_and(|s| !text_bound(s, 128))
            || self.adif.is_some_and(|a| !(1..=9999).contains(&a))
            || self.need.reasons.len() > 30
            || self.need.reasons.iter().any(|s| !text_bound(s, 256))
        {
            return Err(Error::Invalid);
        }
        if readiness != Readiness::Ready {
            if self.band.is_some()
                || self.country.is_some()
                || self.adif.is_some()
                || self.country_resolution.is_some()
                || self.call_status != CallStatus::Unknown
                || self.dxcc_status != DxccStatus::Unknown
                || self.need.status != NeedStatus::Unknown
                || self.need.score.is_some()
                || !self.need.reasons.is_empty()
                || self.need.reason_code.as_deref() != Some(readiness.code())
            {
                return Err(Error::Invalid);
            }
        } else if self.need.status == NeedStatus::Unknown {
            if self.need.score.is_some()
                || !matches!(
                    self.need.reason_code.as_deref(),
                    Some(
                        "country_ambiguous"
                            | "country_unknown"
                            | "known_no_entity"
                            | "unsupported_band"
                    )
                )
            {
                return Err(Error::Invalid);
            }
            match self.need.reason_code.as_deref() {
                Some("known_no_entity" | "country_ambiguous" | "country_unknown") => {
                    let resolution_ok = match self.need.reason_code.as_deref() {
                        Some("known_no_entity") => {
                            self.country_resolution == Some(CountryResolution::KnownNoEntity)
                        }
                        Some("country_ambiguous") => {
                            self.country_resolution == Some(CountryResolution::Ambiguous)
                        }
                        _ => matches!(
                            self.country_resolution,
                            Some(
                                CountryResolution::Unknown
                                    | CountryResolution::Invalid
                                    | CountryResolution::MissingDate
                            )
                        ),
                    };
                    if self.country.is_some()
                        || self.adif.is_some()
                        || self.dxcc_status != DxccStatus::Unknown
                        || !resolution_ok
                    {
                        return Err(Error::Invalid);
                    }
                }
                Some("unsupported_band") if self.band.is_some() => return Err(Error::Invalid),
                _ => {}
            }
        } else if self.need.score.is_none() || self.need.reason_code.is_some() {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
#[derive(Deserialize)]
struct WireResponse {
    protocol_version: u32,
    server_instance: String,
    workspace_id: String,
    generated_at_unix_ms: u64,
    max_age_ms: u64,
    readiness: Readiness,
    #[serde(deserialize_with = "required_option")]
    authority: Option<NeedsAuthority>,
    results: Vec<NeedsResult>,
}
pub struct NeedsBatch {
    metadata: NeedsMetadata,
    results: Vec<NeedsResult>,
    expires: Instant,
    authority: AuthorityLease,
    session: Arc<AtomicBool>,
    facts: Arc<AtomicBool>,
}
impl NeedsBatch {
    pub fn is_fresh(&self) -> bool {
        self.session.load(Ordering::Acquire)
            && self.facts.load(Ordering::Acquire)
            && self.authority.fresh()
            && Instant::now() < self.expires
    }
    /// Conservative lifetime for a consumer's monotonic display deadline.
    /// Capture the consumer clock before IPC; polling never renews this batch.
    pub fn remaining_valid_ms(&self) -> u64 {
        if !self.is_fresh() {
            return 0;
        }
        self.expires
            .min(self.authority.expires)
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(2000) as u64
    }
    pub fn results(&self) -> Result<&[NeedsResult], Error> {
        if !self.is_fresh() {
            return Err(Error::Stale);
        }
        Ok(&self.results)
    }
    pub fn metadata(&self) -> Result<&NeedsMetadata, Error> {
        if !self.is_fresh() {
            return Err(Error::Stale);
        }
        Ok(&self.metadata)
    }
}
/// Native credentials only; intentionally neither Clone, Debug nor Serialize.
pub struct NeedsClient {
    http: Client,
    endpoint: String,
    hello: Hello,
    session: Arc<AtomicBool>,
    authority: Arc<Mutex<AuthorityState>>,
    facts: Arc<AtomicBool>,
    connected: bool,
    last_response: Option<(u64, Instant)>,
}
impl BridgeClient {
    pub fn needs_handle(&self) -> Result<NeedsClient, Error> {
        let hello = self.hello().ok_or(Error::Disconnected)?;
        if !hello.capabilities.iter().any(|s| s == "logbook.classify") {
            return Err(Error::Capability);
        }
        Ok(NeedsClient {
            http: self.http.clone(),
            endpoint: self.endpoint.clone(),
            hello: hello.clone(),
            session: self.audio_session.clone(),
            authority: self.needs_authority.clone(),
            facts: Arc::new(AtomicBool::new(true)),
            connected: true,
            last_response: None,
        })
    }
}
impl NeedsClient {
    pub fn disconnect(&mut self) {
        self.connected = false;
        self.facts.store(false, Ordering::Release);
    }
    /// One synchronous bounded read. No mutex is held during the request, no retry.
    pub fn query(&mut self, queries: &[NeedsQuery]) -> Result<NeedsBatch, Error> {
        let result = self.query_once(queries);
        if let Err(error) = &result {
            self.facts.store(false, Ordering::Release);
            self.facts = Arc::new(AtomicBool::new(true));
            if matches!(
                error,
                Error::IdentityChanged | Error::Incompatible | Error::Http(401 | 403)
            ) {
                self.session.store(false, Ordering::Release);
                if let Ok(mut state) = self.authority.lock() {
                    state.invalidate();
                }
                self.disconnect();
            } else if matches!(error, Error::Http(404)) {
                self.disconnect();
            }
        }
        result
    }
    fn query_once(&mut self, queries: &[NeedsQuery]) -> Result<NeedsBatch, Error> {
        if !self.connected || !self.session.load(Ordering::Acquire) {
            return Err(Error::Disconnected);
        }
        let mut ids = HashSet::new();
        if queries.is_empty()
            || queries.len() > 32
            || queries
                .iter()
                .any(|q| !q.validate() || !ids.insert(&q.query_id))
        {
            return Err(Error::Invalid);
        }
        #[derive(Serialize)]
        struct Request<'a> {
            queries: &'a [NeedsQuery],
        }
        let body = serde_json::to_vec(&Request { queries }).map_err(|_| Error::Invalid)?;
        if body.len() > REQUEST_BYTES {
            return Err(Error::TooLarge);
        }
        let lease = self
            .authority
            .lock()
            .map_err(|_| Error::Disconnected)?
            .lease()?;
        let response = self
            .http
            .post(format!("{}/xlog/v1/needs/query", self.endpoint))
            .header(header::CONTENT_TYPE, "application/json")
            .body(body)
            .timeout(Duration::from_secs(1))
            .send()
            .map_err(|_| Error::Transport)?;
        let wire: WireResponse = read_response(response, RESPONSE_BYTES)?;
        self.accept(wire, queries, lease, now_ms()?)
    }
    fn accept(
        &mut self,
        wire: WireResponse,
        queries: &[NeedsQuery],
        lease: AuthorityLease,
        now: u64,
    ) -> Result<NeedsBatch, Error> {
        if !self.session.load(Ordering::Acquire) {
            return Err(Error::Disconnected);
        }
        if wire.protocol_version != 1 {
            return Err(Error::Incompatible);
        }
        if wire.server_instance != self.hello.server_instance
            || wire.workspace_id != self.hello.workspace_id
        {
            return Err(Error::IdentityChanged);
        }
        let metadata = NeedsMetadata {
            generated_at_unix_ms: wire.generated_at_unix_ms,
            max_age_ms: wire.max_age_ms,
            readiness: wire.readiness,
            authority: wire.authority,
        };
        let duration = metadata.lifetime(now)?;
        let mut expires = Instant::now() + duration;
        if let Some((generated, deadline)) = self.last_response {
            if metadata.generated_at_unix_ms < generated {
                return Err(Error::Stale);
            }
            if metadata.generated_at_unix_ms == generated {
                expires = expires.min(deadline);
            }
        }
        if Instant::now() >= expires {
            return Err(Error::Stale);
        }
        if !lease.fresh() || !metadata.same_authority(&lease.metadata) {
            lease.live.store(false, Ordering::Release);
            return Err(Error::Stale);
        }
        if wire.results.len() != queries.len() {
            return Err(Error::Invalid);
        }
        let requests: HashMap<_, _> = queries.iter().map(|q| (q.query_id.as_str(), q)).collect();
        let mut seen = HashSet::new();
        for row in &wire.results {
            if !seen.insert(&row.query_id) {
                return Err(Error::Invalid);
            }
            row.validate(
                requests.get(row.query_id.as_str()).ok_or(Error::Invalid)?,
                metadata.readiness,
            )?;
        }
        self.last_response = Some((metadata.generated_at_unix_ms, expires));
        Ok(NeedsBatch {
            metadata,
            results: wire.results,
            expires,
            authority: lease,
            session: self.session.clone(),
            facts: self.facts.clone(),
        })
    }
}
impl Drop for NeedsClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}

#[cfg(test)]
mod tests;
