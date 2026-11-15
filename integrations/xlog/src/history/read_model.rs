// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Additive owner history/recent reads. Never derive full-log facts from pages.
use super::*;

const SAFE: u64 = 9_007_199_254_740_990;
const RESPONSE_BYTES: usize = 512 * 1024;
// Owner schema-1 lifetime, anchored before its database read, not serialization.
const READ_MAX_AGE_MS: u64 = 5_000;
const LEGACY_MAX_AGE_MS: u64 = 60_000;
const FUTURE_SKEW_MS: u64 = 2_000;

fn required_option<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(d)
}
fn present_nullable<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(d).map(Some)
}

impl<'de> Deserialize<'de> for HistoryContact {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            contact_id: String,
            callsign: String,
            band: String,
            mode: String,
            started_at_unix_ms: u64,
            #[serde(default, deserialize_with = "present_nullable")]
            frequency_hz: Option<Option<u64>>,
            rst_sent: String,
            rst_received: String,
            grid: String,
            comment: String,
            #[serde(default, deserialize_with = "present_nullable")]
            contact_revision: Option<Option<u64>>,
            #[serde(default)]
            submode: Option<String>,
            #[serde(default)]
            exact_mode: Option<String>,
            #[serde(default)]
            details_complete: Option<bool>,
        }
        let w = Wire::deserialize(d)?;
        Ok(Self {
            contact_id: w.contact_id,
            callsign: w.callsign,
            band: w.band,
            mode: w.mode,
            started_at_unix_ms: w.started_at_unix_ms,
            frequency_present: w.frequency_hz.is_some(),
            frequency_hz: w.frequency_hz.flatten(),
            rst_sent: w.rst_sent,
            rst_received: w.rst_received,
            grid: w.grid,
            comment: w.comment,
            revision_present: w.contact_revision.is_some(),
            contact_revision: w.contact_revision.flatten(),
            submode: w.submode,
            exact_mode: w.exact_mode,
            details_complete: w.details_complete,
        })
    }
}

impl Serialize for HistoryContact {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let count = 9
            + usize::from(self.frequency_present)
            + usize::from(self.revision_present)
            + usize::from(self.submode.is_some())
            + usize::from(self.exact_mode.is_some())
            + usize::from(self.details_complete.is_some());
        let mut wire = serializer.serialize_struct("HistoryContact", count)?;
        wire.serialize_field("contact_id", &self.contact_id)?;
        wire.serialize_field("callsign", &self.callsign)?;
        wire.serialize_field("band", &self.band)?;
        wire.serialize_field("mode", &self.mode)?;
        wire.serialize_field("started_at_unix_ms", &self.started_at_unix_ms)?;
        if self.frequency_present {
            wire.serialize_field("frequency_hz", &self.frequency_hz)?;
        }
        wire.serialize_field("rst_sent", &self.rst_sent)?;
        wire.serialize_field("rst_received", &self.rst_received)?;
        wire.serialize_field("grid", &self.grid)?;
        wire.serialize_field("comment", &self.comment)?;
        if self.revision_present {
            wire.serialize_field("contact_revision", &self.contact_revision)?;
        }
        if let Some(value) = &self.submode {
            wire.serialize_field("submode", value)?;
        }
        if let Some(value) = &self.exact_mode {
            wire.serialize_field("exact_mode", value)?;
        }
        if let Some(value) = self.details_complete {
            wire.serialize_field("details_complete", &value)?;
        }
        wire.end()
    }
}
impl PartialEq for HistoryContact {
    fn eq(&self, other: &Self) -> bool {
        // Presence bookkeeping is validation evidence, not a new contact fact.
        self.contact_id == other.contact_id
            && self.callsign == other.callsign
            && self.band == other.band
            && self.mode == other.mode
            && self.started_at_unix_ms == other.started_at_unix_ms
            && self.frequency_hz == other.frequency_hz
            && self.rst_sent == other.rst_sent
            && self.rst_received == other.rst_received
            && self.grid == other.grid
            && self.comment == other.comment
            && self.contact_revision == other.contact_revision
            && self.submode == other.submode
            && self.exact_mode == other.exact_mode
            && self.details_complete == other.details_complete
    }
}

/// Canonical request echo, not a client classification policy. The aliases are
/// the owner's SpotLogic.normalizedMode + CompanionHistoryProjection.exactMode.
fn exact_mode(value: &str) -> String {
    match value.trim().to_ascii_uppercase().as_str() {
        "USB" | "LSB" | "PHONE" | "PH" => "SSB".into(),
        "CWR" => "CW".into(),
        "RTTYR" => "RTTY".into(),
        value => value.into(),
    }
}
fn band_query(value: &str) -> bool {
    if value.len() > 16 {
        return false;
    }
    let Some(number) = value
        .strip_suffix("cm")
        .or_else(|| value.strip_suffix("mm"))
        .or_else(|| value.strip_suffix('m'))
    else {
        return false;
    };
    let mut parts = number.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    (1..=4).contains(&whole.len())
        && whole.bytes().all(|b| b.is_ascii_digit())
        && fraction
            .is_none_or(|v| (1..=3).contains(&v.len()) && v.bytes().all(|b| b.is_ascii_digit()))
        && parts.next().is_none()
}
fn mode_query(value: &str) -> bool {
    (1..=20).contains(&value.len())
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryQuery {
    callsign: String,
    band: Option<String>,
    exact_mode: Option<String>,
}
impl HistoryQuery {
    pub fn new(callsign: &str, band: Option<&str>, mode: Option<&str>) -> Result<Self, Error> {
        if !call(callsign) || (mode.is_some() && band.is_none()) {
            return Err(Error::Invalid);
        }
        let band = band.map(|v| v.trim().to_ascii_lowercase());
        let mode = mode.map(exact_mode);
        if band.as_ref().is_some_and(|v| !band_query(v))
            || mode.as_ref().is_some_and(|v| !mode_query(v))
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            callsign: callsign.to_ascii_uppercase(),
            band,
            exact_mode: mode,
        })
    }
    fn path(&self) -> String {
        let mut path = format!(
            "/xlog/v1/history?callsign={}",
            self.callsign.replace('/', "%2F")
        );
        if let Some(band) = &self.band {
            path.push_str(&format!("&band={band}"));
        }
        if let Some(mode) = &self.exact_mode {
            path.push_str(&format!("&mode={mode}"));
        }
        path
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistorySummary {
    pub schema_version: u32,
    pub scope: String,
    pub complete: bool,
    pub total: u64,
    pub worked_before: bool,
    pub confirmed_count: u64,
    pub confirmation_policy: String,
    pub confirmation_sources: Vec<String>,
    #[serde(deserialize_with = "required_option")]
    pub last_started_at_unix_ms: Option<u64>,
    pub bands: Vec<String>,
    pub bands_complete: bool,
    pub modes: Vec<String>,
    pub modes_complete: bool,
    #[serde(deserialize_with = "required_option")]
    pub requested_band: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub requested_exact_mode: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub requested_band_worked: Option<bool>,
    #[serde(deserialize_with = "required_option")]
    pub requested_band_exact_mode_worked: Option<bool>,
}
fn sorted_facet(values: &[String]) -> bool {
    values.len() <= 128
        && values.iter().all(|v| bounded(v, 32))
        && values.windows(2).all(|v| v[0] < v[1])
}
fn ordered_contacts(contacts: &[HistoryContact]) -> bool {
    contacts.windows(2).all(|v| {
        v[0].started_at_unix_ms > v[1].started_at_unix_ms
            || (v[0].started_at_unix_ms == v[1].started_at_unix_ms
                && v[0].contact_id.to_ascii_lowercase() < v[1].contact_id.to_ascii_lowercase())
    })
}
impl HistoryContact {
    pub(super) fn validate_details(&self, required: bool) -> Result<(), Error> {
        if !required
            && self.submode.is_none()
            && self.exact_mode.is_none()
            && self.details_complete.is_none()
        {
            return Ok(());
        }
        let (Some(submode), Some(exact), Some(complete)) =
            (&self.submode, &self.exact_mode, self.details_complete)
        else {
            return Err(Error::Invalid);
        };
        if !self.revision_present
            || !self.frequency_present
            || !optional(submode, 32)
            || !bounded(exact, 32)
            || *exact
                != exact_mode(if submode.trim().is_empty() {
                    &self.mode
                } else {
                    submode
                })
            || complete != self.contact_revision.is_some()
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
impl History {
    pub(super) fn validate_read_model(
        &self,
        hello: &Hello,
        query: &HistoryQuery,
    ) -> Result<Option<u64>, Error> {
        self.validate_read_model_at(hello, query, now_ms()?)
    }
    fn validate_read_model_at(
        &self,
        hello: &Hello,
        query: &HistoryQuery,
        now: u64,
    ) -> Result<Option<u64>, Error> {
        let any = self.generation.is_some()
            || self.logbook_revision.is_some()
            || self.max_age_ms.is_some()
            || self.contacts_limit.is_some()
            || self.contacts_complete.is_some()
            || self.matrix_complete.is_some()
            || self.history_summary.is_some();
        if !any {
            return Ok(None);
        }
        let (
            Some(generation),
            Some(revision),
            Some(age),
            Some(limit),
            Some(complete),
            Some(matrix_complete),
            Some(summary),
        ) = (
            &self.generation,
            self.logbook_revision,
            self.max_age_ms,
            self.contacts_limit,
            self.contacts_complete,
            self.matrix_complete,
            &self.history_summary,
        )
        else {
            return Err(Error::Invalid);
        };
        if generation != &hello.workspace_id {
            return Err(Error::IdentityChanged);
        }
        if revision > SAFE
            || age == 0
            || age > READ_MAX_AGE_MS
            || limit != 25
            || summary.schema_version != 1
            || summary.scope != "normal_log_base_callsign"
        {
            return Err(Error::Invalid);
        }
        if self.generated_at_unix_ms > now.saturating_add(FUTURE_SKEW_MS)
            || now.saturating_sub(self.generated_at_unix_ms) >= age
        {
            return Err(Error::Stale);
        }
        // A partial owner summary is unavailable, never an empty/new call.
        if !summary.complete {
            return Err(Error::Stale);
        }
        if summary.total > SAFE
            || summary.confirmed_count > summary.total
            || summary.worked_before != (summary.total != 0)
            || summary.confirmation_policy != "xlog_cluster_paper_or_lotw_v1"
            || summary.confirmation_sources != ["paper_qsl", "lotw"]
            || self.contacts.len() as u64 != summary.total.min(25)
            || complete != (summary.total <= 25)
            || summary.last_started_at_unix_ms
                != self.contacts.first().map(|c| c.started_at_unix_ms)
            || !ordered_contacts(&self.contacts)
            || (!matrix_complete && self.matrix.len() != 256)
            || self.matrix.len() as u64 > summary.total
            || (summary.total == 0 && !self.matrix.is_empty())
            || (summary.total != 0
                && (self.matrix.is_empty() || summary.bands.is_empty() || summary.modes.is_empty()))
            || self.matrix.iter().any(|cell| !cell.worked)
            || !sorted_facet(&summary.bands)
            || !sorted_facet(&summary.modes)
            || (!summary.bands_complete && summary.bands.len() != 128)
            || (!summary.modes_complete && summary.modes.len() != 128)
            || summary.bands.len() as u64 > summary.total
            || summary.modes.len() as u64 > summary.total
            || summary
                .bands
                .iter()
                .any(|v| *v != v.trim().to_ascii_lowercase())
            || summary.modes.iter().any(|v| *v != exact_mode(v))
            || summary.requested_band != query.band
            || summary.requested_exact_mode != query.exact_mode
            || summary.requested_band_worked.is_some() != query.band.is_some()
            || summary.requested_band_exact_mode_worked.is_some() != query.exact_mode.is_some()
            || (summary.requested_band_exact_mode_worked == Some(true)
                && summary.requested_band_worked != Some(true))
        {
            return Err(Error::Invalid);
        }
        if let Some(band) = &query.band {
            let present = summary.bands.contains(band);
            if (present && summary.requested_band_worked != Some(true))
                || (summary.bands_complete && summary.requested_band_worked != Some(present))
            {
                return Err(Error::Invalid);
            }
        }
        if let Some(mode) = &query.exact_mode {
            if summary.modes_complete
                && !summary.modes.contains(mode)
                && summary.requested_band_exact_mode_worked != Some(false)
            {
                return Err(Error::Invalid);
            }
        }
        for contact in &self.contacts {
            contact.validate_details(true)?;
            let band = contact.band.trim().to_ascii_lowercase();
            let mode = contact.exact_mode.as_ref().ok_or(Error::Invalid)?;
            let submode = contact.submode.as_deref().ok_or(Error::Invalid)?;
            let legacy_mode = if submode.is_empty() {
                &contact.mode
            } else {
                submode
            };
            if (summary.bands_complete && !summary.bands.contains(&band))
                || (summary.modes_complete && !summary.modes.contains(mode))
                || (query.band.as_ref() == Some(&band)
                    && summary.requested_band_worked != Some(true))
                || (query.band.as_ref() == Some(&band)
                    && query.exact_mode.as_ref() == Some(mode)
                    && summary.requested_band_exact_mode_worked != Some(true))
                || (matrix_complete
                    && !self
                        .matrix
                        .iter()
                        .any(|cell| cell.band == contact.band && cell.mode == legacy_mode))
            {
                return Err(Error::Invalid);
            }
        }
        Ok(Some(revision))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recent {
    pub protocol_version: u32,
    pub server_instance: String,
    pub workspace_id: String,
    pub schema_version: u32,
    pub generation: String,
    pub logbook_revision: u64,
    pub generated_at_unix_ms: u64,
    pub max_age_ms: u64,
    pub contacts: Vec<HistoryContact>,
    pub limit: u32,
    pub total: u64,
    pub complete: bool,
    pub ordering: String,
}
impl Recent {
    fn validate(&self, hello: &Hello) -> Result<(), Error> {
        self.validate_at(hello, now_ms()?)
    }
    fn validate_at(&self, hello: &Hello, now: u64) -> Result<(), Error> {
        if self.schema_version != 1 {
            return Err(Error::Invalid);
        }
        if self.generation != hello.workspace_id {
            return Err(Error::IdentityChanged);
        }
        if self.logbook_revision > SAFE
            || self.total > SAFE
            || self.limit != 5
            || self.contacts.len() as u64 != self.total.min(5)
            || self.complete != (self.total <= 5)
            || self.ordering != "qso_start_desc_contact_id_asc"
            || !ordered_contacts(&self.contacts)
            || self.max_age_ms == 0
            || self.max_age_ms > READ_MAX_AGE_MS
        {
            return Err(Error::Invalid);
        }
        validate_envelope(
            self.protocol_version,
            &self.server_instance,
            &self.workspace_id,
            self.generated_at_unix_ms,
            hello,
            now,
        )?;
        if now.saturating_sub(self.generated_at_unix_ms) >= self.max_age_ms {
            return Err(Error::Stale);
        }
        validate_contacts(&self.contacts, 5)?;
        for contact in &self.contacts {
            contact.validate_details(true)?;
        }
        Ok(())
    }
}

/// A response can be held only until its original deadline, or session/revision
/// retirement. IPC consumers capture request-start time and recheck full context.
pub struct ReadLease<T> {
    value: T,
    expires: Instant,
    session: Arc<AtomicBool>,
    revision: Arc<AtomicBool>,
}
impl<T> ReadLease<T> {
    pub fn remaining_valid_ms(&self) -> u64 {
        if !self.session.load(Ordering::Acquire) || !self.revision.load(Ordering::Acquire) {
            return 0;
        }
        self.expires
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(u128::from(LEGACY_MAX_AGE_MS)) as u64
    }
    pub fn value(&self) -> Result<&T, Error> {
        if self.remaining_valid_ms() == 0 {
            Err(Error::Stale)
        } else {
            Ok(&self.value)
        }
    }
}

/// Independent bounded passive reads, never the PCM/Stop owner or UI state lock.
/// Deliberately not Debug: the HTTP client contains native pairing credentials.
pub struct HistoryClient {
    http: Client,
    endpoint: String,
    hello: Hello,
    session: Arc<AtomicBool>,
    revision_live: Arc<AtomicBool>,
    revision: Option<u64>,
    last: Option<(u64, Instant)>,
    connected: bool,
}
impl BridgeClient {
    pub fn history_handle(&self) -> Result<HistoryClient, Error> {
        let hello = self.hello().ok_or(Error::Disconnected)?;
        if !hello.capabilities.iter().any(|c| c == "logbook.read") {
            return Err(Error::Capability);
        }
        Ok(HistoryClient {
            http: self.http.clone(),
            endpoint: self.endpoint.clone(),
            hello: hello.clone(),
            session: self.audio_session.clone(),
            revision_live: Arc::new(AtomicBool::new(true)),
            revision: None,
            last: None,
            connected: true,
        })
    }
}
impl HistoryClient {
    fn retire_facts(&mut self) {
        self.revision_live.store(false, Ordering::Release);
        self.revision_live = Arc::new(AtomicBool::new(true));
    }
    pub fn disconnect(&mut self) {
        self.connected = false;
        self.retire_facts();
        self.revision_live.store(false, Ordering::Release);
    }
    fn finish<T>(&mut self, result: Result<ReadLease<T>, Error>) -> Result<ReadLease<T>, Error> {
        if let Err(error) = &result {
            self.retire_facts();
            if matches!(
                error,
                Error::IdentityChanged | Error::Incompatible | Error::Http(401 | 403)
            ) {
                self.session.store(false, Ordering::Release);
            }
        }
        result
    }
    fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, Error> {
        if !self.connected
            || !self.session.load(Ordering::Acquire)
            || !self.revision_live.load(Ordering::Acquire)
        {
            return Err(Error::Disconnected);
        }
        let response = self
            .http
            .get(format!("{}{path}", self.endpoint))
            .send()
            .map_err(|_| Error::Transport)?;
        let value = read_response(response, RESPONSE_BYTES)?;
        if !self.session.load(Ordering::Acquire) {
            return Err(Error::Disconnected);
        }
        Ok(value)
    }
    fn lease<T>(
        &mut self,
        value: T,
        generated: u64,
        max_age: u64,
        revision: Option<u64>,
        request_started: Instant,
    ) -> Result<ReadLease<T>, Error> {
        let now = now_ms()?;
        if generated > now.saturating_add(FUTURE_SKEW_MS)
            || now.saturating_sub(generated) >= max_age
        {
            return Err(Error::Stale);
        }
        // A future-skewed host timestamp cannot grant fresh time spent waiting
        // on HTTP/database work. Both the wire age and full request elapsed count.
        let mut expires = (Instant::now()
            + Duration::from_millis(max_age - now.saturating_sub(generated)))
        .min(request_started + Duration::from_millis(max_age));
        if let Some((old_time, old_expiry)) = self.last {
            if generated < old_time {
                return Err(Error::Stale);
            }
            if generated == old_time {
                expires = expires.min(old_expiry);
            }
        }
        if let (Some(old), Some(new)) = (self.revision, revision) {
            if new < old {
                return Err(Error::Revision);
            }
        }
        if self.revision != revision {
            self.retire_facts();
        }
        if !self.session.load(Ordering::Acquire) || Instant::now() >= expires {
            return Err(Error::Stale);
        }
        self.revision = revision;
        self.last = Some((generated, expires));
        Ok(ReadLease {
            value,
            expires,
            session: self.session.clone(),
            revision: self.revision_live.clone(),
        })
    }
    pub fn history(&mut self, query: &HistoryQuery) -> Result<ReadLease<History>, Error> {
        let result = (|| {
            let started = Instant::now();
            let history: History = self.get(&query.path())?;
            history.validate(&self.hello, &query.callsign)?;
            let revision = history.validate_read_model(&self.hello, query)?;
            if revision.is_none() && (query.band.is_some() || query.exact_mode.is_some()) {
                return Err(Error::Capability);
            }
            let generated = history.generated_at_unix_ms;
            let age = history.max_age_ms.unwrap_or(LEGACY_MAX_AGE_MS);
            self.lease(history, generated, age, revision, started)
        })();
        self.finish(result)
    }
    /// Optional endpoint: HTTP404 means unavailable; never an empty logbook.
    pub fn recent(&mut self) -> Result<ReadLease<Recent>, Error> {
        let result = (|| {
            let started = Instant::now();
            let recent: Recent = self.get("/xlog/v1/recent")?;
            recent.validate(&self.hello)?;
            let (generated, age, revision) = (
                recent.generated_at_unix_ms,
                recent.max_age_ms,
                recent.logbook_revision,
            );
            self.lease(recent, generated, age, Some(revision), started)
        })();
        self.finish(result)
    }
}

#[cfg(test)]
mod tests;
