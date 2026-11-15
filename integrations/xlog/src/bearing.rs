// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Unwired schema-1 passive bearing body. Decoding grants neither freshness,
//! motion nor transmission authority. The future host envelope/lease is separate.
//! No resolver, coordinate conversion or reciprocal-bearing calculation lives here.
use crate::{commands::valid_request_id, Error};
use serde::{Deserialize, Deserializer, Serialize};

pub mod client;

pub const MAX_BEARING_BYTES: usize = 16 * 1024;
const SAFE: i64 = 9_007_199_254_740_990;

fn required_nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Available,
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    OriginUnavailable,
    RecordTooLarge,
    InvalidRecord,
    LogbookUnavailable,
    ReferenceUnavailable,
    CountryAmbiguous,
    CountryKnownNoEntity,
    CountryInvalid,
    CountryMissingDate,
    CountryUnknown,
    TargetCoordinatesUnavailable,
    ReferenceMetadataInvalid,
    Coincident,
    Antipodal,
    InvalidGeometry,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginSource {
    SavedStationSettings,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetSource {
    ExactLogbookGrid,
    CtyEntityApproximation,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Precision {
    HistoricalGridCellCenter,
    EntityApproximation,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CountryStatus {
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub source: OriginSource,
    pub grid: String,
    pub latitude: f64,
    pub longitude: f64,
}
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub source: TargetSource,
    #[serde(deserialize_with = "required_nullable")]
    pub grid: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub precision: Precision,
    #[serde(deserialize_with = "required_nullable")]
    pub contact_id: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub contact_revision: Option<i64>,
    #[serde(deserialize_with = "required_nullable")]
    pub contact_started_at_unix_ms: Option<i64>,
    #[serde(deserialize_with = "required_nullable")]
    pub reference_revision: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub matched_reference_revision: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub matched_rule: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub country_status: Option<CountryStatus>,
}
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Geometry {
    pub short_path_deg: f64,
    pub long_path_deg: f64,
    pub distance_km: f64,
}
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BearingData {
    pub schema_version: u32,
    pub requested_callsign: String,
    pub normalized_callsign: String,
    pub status: Status,
    #[serde(deserialize_with = "required_nullable")]
    pub reason: Option<Reason>,
    #[serde(deserialize_with = "required_nullable")]
    pub origin: Option<Origin>,
    #[serde(deserialize_with = "required_nullable")]
    pub target: Option<Target>,
    #[serde(deserialize_with = "required_nullable")]
    pub geometry: Option<Geometry>,
    pub logbook_revision: i64,
    #[serde(deserialize_with = "required_nullable")]
    pub reference_revision: Option<String>,
    pub motion_authority: bool,
    pub transmit_authority: bool,
}

fn valid_call(call: &str) -> bool {
    (3..=32).contains(&call.len())
        && call.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'/')
        && call.bytes().any(|c| c.is_ascii_alphabetic())
        && call.bytes().any(|c| c.is_ascii_digit())
        && !call.starts_with('/')
        && !call.ends_with('/')
        && !call.contains("//")
}
fn valid_grid(grid: &str) -> bool {
    matches!(grid.len(), 4 | 6 | 8)
        && grid.bytes().enumerate().all(|(i, c)| match i {
            0 | 1 => (b'A'..=b'R').contains(&c),
            4 | 5 => (b'A'..=b'X').contains(&c),
            _ => c.is_ascii_digit(),
        })
}
fn coordinates(lat: f64, lon: f64) -> bool {
    lat.is_finite()
        && lon.is_finite()
        && (-90.0..=90.0).contains(&lat)
        && (-180.0..=180.0).contains(&lon)
}
fn reference(value: &str) -> bool {
    !value.is_empty() && value.len() <= 2048
}

impl Target {
    fn valid(&self, log_revision: i64) -> bool {
        if !coordinates(self.latitude, self.longitude) {
            return false;
        }
        match self.source {
            TargetSource::ExactLogbookGrid => {
                self.precision == Precision::HistoricalGridCellCenter
                    && self.grid.as_deref().is_some_and(valid_grid)
                    && self.contact_id.as_deref().is_some_and(|id| {
                        valid_request_id(id) && !id.bytes().any(|b| b.is_ascii_uppercase())
                    })
                    && self
                        .contact_revision
                        .is_some_and(|v| (0..=log_revision).contains(&v))
                    && self
                        .contact_started_at_unix_ms
                        .is_some_and(|v| (-SAFE..=SAFE).contains(&v))
                    && self.reference_revision.is_none()
                    && self.matched_reference_revision.is_none()
                    && self.matched_rule.is_none()
                    && self.country_status.is_none()
            }
            TargetSource::CtyEntityApproximation => {
                self.precision == Precision::EntityApproximation
                    && self.grid.is_none()
                    && self.contact_id.is_none()
                    && self.contact_revision.is_none()
                    && self.contact_started_at_unix_ms.is_none()
                    && self.reference_revision.as_deref().is_some_and(reference)
                    && self
                        .matched_reference_revision
                        .as_ref()
                        .is_none_or(|v| v.len() <= 2048)
                    && self.matched_rule.as_ref().is_none_or(|v| v.len() <= 128)
                    && self.country_status == Some(CountryStatus::Resolved)
            }
        }
    }
}
impl BearingData {
    fn validate(&self, requested: &str) -> Result<(), Error> {
        if self.schema_version != 1
            || self.motion_authority
            || self.transmit_authority
            || !valid_call(requested)
            || self.requested_callsign != requested
            || self.normalized_callsign != requested.to_ascii_uppercase()
            || !(0..=SAFE).contains(&self.logbook_revision)
            || self
                .origin
                .as_ref()
                .is_some_and(|v| !valid_grid(&v.grid) || !coordinates(v.latitude, v.longitude))
            || self
                .target
                .as_ref()
                .is_some_and(|v| !v.valid(self.logbook_revision))
            || self.reference_revision
                != self
                    .target
                    .as_ref()
                    .and_then(|v| v.reference_revision.clone())
        {
            return Err(Error::Invalid);
        }
        let shape = match (self.status, self.reason) {
            (Status::Available, None) => {
                self.origin.is_some() && self.target.is_some() && self.geometry.is_some()
            }
            (Status::Unavailable, Some(Reason::OriginUnavailable)) => {
                self.origin.is_none() && self.target.is_none() && self.geometry.is_none()
            }
            (
                Status::Unavailable,
                Some(Reason::Coincident | Reason::Antipodal | Reason::InvalidGeometry),
            ) => self.origin.is_some() && self.target.is_some() && self.geometry.is_none(),
            (Status::Unavailable, Some(_)) => {
                self.origin.is_some() && self.target.is_none() && self.geometry.is_none()
            }
            _ => false,
        };
        if !shape
            || self.geometry.as_ref().is_some_and(|v| {
                !v.short_path_deg.is_finite()
                    || !(0.0..360.0).contains(&v.short_path_deg)
                    || !v.long_path_deg.is_finite()
                    || !(0.0..360.0).contains(&v.long_path_deg)
                    || !v.distance_km.is_finite()
                    || v.distance_km < 0.0
            })
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

/// Validated data, not a current read lease or permission to move the rotor.
#[derive(Debug, Clone, PartialEq)]
pub struct Bearing {
    data: BearingData,
}
impl Bearing {
    pub fn decode(bytes: &[u8], exact_requested_callsign: &str) -> Result<Self, Error> {
        if bytes.len() > MAX_BEARING_BYTES {
            return Err(Error::TooLarge);
        }
        let data: BearingData = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
        data.validate(exact_requested_callsign)?;
        Ok(Self { data })
    }
    pub fn data(&self) -> &BearingData {
        &self.data
    }
}

#[cfg(test)]
mod tests;
