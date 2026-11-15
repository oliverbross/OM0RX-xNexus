// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Unwired schema-1 contact-detail codec. A decoded value is not a current read
//! lease or permission to edit. Transport/session/owner authority is separate.
//! Raw retained facts remain display-only; never infer award or upload state.
use crate::{commands::valid_request_id, outbox::QsoDraft, Error};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const MAX_DETAIL_BYTES: usize = 120 * 1024;
const SAFE: i64 = 9_007_199_254_740_990;

fn required_nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldState {
    Value,
    Empty,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, bound(deserialize = "T: Deserialize<'de>"))]
pub struct Field<T> {
    pub state: FieldState,
    #[serde(deserialize_with = "required_nullable")]
    pub value: Option<T>,
    #[serde(deserialize_with = "required_nullable")]
    pub reason: Option<String>,
}

trait Fact {
    fn valid_fact(&self) -> bool;
}
impl Fact for String {
    fn valid_fact(&self) -> bool {
        !self.is_empty() && self.len() <= 4096
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotesText(pub String);
impl Fact for NotesText {
    fn valid_fact(&self) -> bool {
        !self.0.is_empty() && self.0.len() <= 16 * 1024
    }
}
impl Fact for i64 {
    fn valid_fact(&self) -> bool {
        (-SAFE..=SAFE).contains(self)
    }
}
impl Fact for f64 {
    fn valid_fact(&self) -> bool {
        self.is_finite() && self.abs() <= SAFE as f64
    }
}
impl Fact for bool {
    fn valid_fact(&self) -> bool {
        true
    }
}
trait Validate {
    fn validate(&self) -> Result<(), Error>;
}
fn reason(value: &str) -> bool {
    crate::bounded(value, 4096)
}
impl<T: Fact> Validate for Field<T> {
    fn validate(&self) -> Result<(), Error> {
        let valid = match self.state {
            FieldState::Value => {
                self.reason.is_none() && self.value.as_ref().is_some_and(Fact::valid_fact)
            }
            FieldState::Empty => self.value.is_none() && self.reason.is_none(),
            FieldState::Unavailable => {
                self.value.is_none() && self.reason.as_deref().is_some_and(reason)
            }
        };
        if valid {
            Ok(())
        } else {
            Err(Error::Invalid)
        }
    }
}
pub type Text = Field<String>;
pub type Integer = Field<i64>;
pub type Number = Field<f64>;

macro_rules! group {
    ($name:ident { $($field:ident: $ty:ty),+ $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $(pub $field: $ty),+ }
        impl Validate for $name {
            fn validate(&self) -> Result<(), Error> {
                $(self.$field.validate()?;)+
                Ok(())
            }
        }
    }
}
group!(Contact {
    callsign: Text, band: Text, mode: Text, submode: Text,
    started_at_unix_ms: Integer, ended_at_unix_ms: Integer,
    stored_time_unit: Text, original_start_precision: Text, original_end_precision: Text,
    raw_qso_date: Text, raw_time_on: Text, raw_qso_date_off: Text, raw_time_off: Text,
    rst_sent: Text, rst_received: Text, grid: Text, comment: Text, notes: Field<NotesText>,
});
group!(Radio {
    frequency_hz: Integer,
    receive_frequency_hz: Integer,
    receive_band: Text,
    tx_power_raw: Text,
    tx_power_w: Number,
    propagation_mode: Text,
    antenna_path: Text,
    satellite_name: Text,
});
group!(Station {
    name: Text,
    qth: Text,
    state: Text,
    county: Text,
    country: Text,
    dxcc: Integer,
    continent: Text,
    cq_zone: Text,
    itu_zone: Text,
});
group!(OwnStation {
    operator_call: Text,
    callsign: Text,
    grid: Text,
    recorded_rig: Text,
    radio_label: Text,
    station_profile_id: Integer,
    station_location: Text,
});
group!(ReferenceSide {
    pota: Text,
    sota: Text,
    wwff: Text,
    iota: Text,
    sig: Text,
    sig_info: Text
});
group!(References {
    their: ReferenceSide,
    my: ReferenceSide,
    dok: Text
});
group!(Confirmation {
    source: Text,
    evidence_kind: Text,
    sent_status: Text,
    sent_date: Text,
    sent_via: Text,
    received_status: Text,
    received_date: Text,
    received_via: Text,
    match_age_seconds: Number,
});
group!(Confirmations {
    paper: Confirmation,
    lotw: Confirmation,
    eqsl: Confirmation,
    qrz: Confirmation
});
group!(Awards {
    credit_granted: Text,
    credit_submitted: Text,
    authority: Text
});
group!(Upload {
    source: Text, state: Text, receipt_revision: Integer, last_attempt_at_unix_ms: Integer,
    is_current_revision: Field<bool>,
});
group!(Uploads {
    lotw: Upload,
    eqsl: Upload,
    qrz: Upload,
    clublog: Upload
});
group!(Contest {
    contest_id: Text,
    stx: Text,
    srx: Text,
    stx_string: Text,
    srx_string: Text
});
group!(Fields {
    contact: Contact,
    radio: Radio,
    station: Station,
    own_station: OwnStation,
    references: References,
    confirmations: Confirmations,
    awards: Awards,
    uploads: Uploads,
    contest: Contest,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Found,
    Deleted,
    NotFound,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditableField {
    StartedAtUnixMs,
    FrequencyHz,
    RstSent,
    RstReceived,
    Grid,
    Comment,
}
const EDIT_FIELDS: [EditableField; 6] = [
    EditableField::StartedAtUnixMs,
    EditableField::FrequencyHz,
    EditableField::RstSent,
    EditableField::RstReceived,
    EditableField::Grid,
    EditableField::Comment,
];

// Separate wire type makes frequency presence mandatory; legacy QsoDraft itself
// deliberately permits omitted nullable frequency in other existing APIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditBase {
    pub contact_id: String,
    pub callsign: String,
    pub band: String,
    pub mode: String,
    pub started_at_unix_ms: u64,
    #[serde(deserialize_with = "required_nullable")]
    pub frequency_hz: Option<u64>,
    pub rst_sent: String,
    pub rst_received: String,
    pub grid: String,
    pub comment: String,
}
impl EditBase {
    fn valid_owner_core(&self) -> bool {
        // Wire admissibility plus the retained form required for a lossless
        // owner roundtrip. No country/callsign policy or mode alias inference.
        (3..=32).contains(&self.callsign.len())
            && self
                .callsign
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || matches!(b, b'/' | b'-'))
            && (1..=16).contains(&self.band.len())
            && self
                .band
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.')
            && (1..=16).contains(&self.mode.len())
            && self
                .mode
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
            && valid_grid(&self.grid)
    }
    fn draft(&self) -> QsoDraft {
        QsoDraft {
            contact_id: self.contact_id.clone(),
            callsign: self.callsign.clone(),
            band: self.band.clone(),
            mode: self.mode.clone(),
            started_at_unix_ms: self.started_at_unix_ms,
            frequency_hz: self.frequency_hz,
            rst_sent: self.rst_sent.clone(),
            rst_received: self.rst_received.clone(),
            grid: self.grid.clone(),
            comment: self.comment.clone(),
        }
    }
}
fn valid_grid(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    let bytes = value.as_bytes();
    matches!(bytes.len(), 4 | 6 | 8)
        && bytes.iter().enumerate().all(|(i, b)| match i {
            0 | 1 => (b'A'..=b'R').contains(b),
            4 | 5 => (b'A'..=b'X').contains(b),
            _ => b.is_ascii_digit(),
        })
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Editability {
    pub allowed: bool,
    pub reason: String,
    pub allowed_fields: Vec<EditableField>,
    #[serde(deserialize_with = "required_nullable")]
    pub base: Option<EditBase>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetailData {
    pub schema_version: u32,
    pub contact_id: String,
    pub status: Status,
    #[serde(deserialize_with = "required_nullable")]
    pub reason: Option<String>,
    pub logbook_revision: i64,
    #[serde(deserialize_with = "required_nullable")]
    pub contact_revision: Option<i64>,
    #[serde(deserialize_with = "required_nullable")]
    pub fields: Option<Fields>,
    pub editability: Editability,
    pub locate: Text,
}

fn text_matches(cell: &Text, value: &str) -> bool {
    if value.is_empty() {
        cell.state == FieldState::Empty
    } else {
        cell.state == FieldState::Value && cell.value.as_deref() == Some(value)
    }
}
fn integer_matches(cell: &Integer, value: Option<u64>) -> bool {
    match value {
        None => cell.state == FieldState::Empty,
        Some(n) => cell.state == FieldState::Value && i64::try_from(n).ok() == cell.value,
    }
}
impl DetailData {
    fn validate(&self) -> Result<(), Error> {
        if self.schema_version != 1
            || !valid_request_id(&self.contact_id)
            || self.contact_id.bytes().any(|b| b.is_ascii_uppercase())
            || !(0..=SAFE).contains(&self.logbook_revision)
            || self
                .contact_revision
                .is_some_and(|v| !(0..=SAFE).contains(&v))
            || self.reason.as_deref().is_some_and(|v| !reason(v))
            || !reason(&self.editability.reason)
        {
            return Err(Error::Invalid);
        }
        self.locate.validate()?;
        if let Some(fields) = &self.fields {
            fields.validate()?;
        }
        if self.status == Status::Found {
            if self.fields.is_none() || self.contact_revision.is_none() || self.reason.is_some() {
                return Err(Error::Invalid);
            }
        } else if self.fields.is_some() || self.editability.allowed {
            return Err(Error::Invalid);
        }
        let edit = &self.editability;
        if !edit.allowed {
            if edit.base.is_some() || !edit.allowed_fields.is_empty() {
                return Err(Error::Invalid);
            }
        } else {
            if self.contact_revision.is_none_or(|n| n <= 0)
                || edit.reason != "complete_core"
                || edit.allowed_fields.len() != EDIT_FIELDS.len()
                || !EDIT_FIELDS.iter().all(|f| edit.allowed_fields.contains(f))
            {
                return Err(Error::Invalid);
            }
            let base = edit.base.as_ref().ok_or(Error::Invalid)?;
            base.draft().validate().map_err(|_| Error::Invalid)?;
            if !base.valid_owner_core() {
                return Err(Error::Invalid);
            }
            if !base.contact_id.eq_ignore_ascii_case(&self.contact_id) {
                return Err(Error::Invalid);
            }
            let fields = self.fields.as_ref().ok_or(Error::Invalid)?;
            let c = &fields.contact;
            if !text_matches(&c.callsign, &base.callsign)
                || !text_matches(&c.band, &base.band)
                || !text_matches(&c.mode, &base.mode)
                || !text_matches(&c.rst_sent, &base.rst_sent)
                || !text_matches(&c.rst_received, &base.rst_received)
                || !text_matches(&c.grid, &base.grid)
                || !text_matches(&c.comment, &base.comment)
                || !integer_matches(&c.started_at_unix_ms, Some(base.started_at_unix_ms))
                || !integer_matches(&fields.radio.frequency_hz, base.frequency_hz)
            {
                return Err(Error::Invalid);
            }
        }
        if serde_json::to_vec(self).map_err(|_| Error::Invalid)?.len() > MAX_DETAIL_BYTES {
            return Err(Error::TooLarge);
        }
        Ok(())
    }
}

/// Validated owner projection only. It carries no HTTP/session/edit authority.
#[derive(Debug, Clone, PartialEq)]
pub struct ContactDetail {
    data: DetailData,
}
impl ContactDetail {
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_DETAIL_BYTES {
            return Err(Error::TooLarge);
        }
        let data: DetailData = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
        data.validate()?;
        Ok(Self { data })
    }
    pub fn data(&self) -> &DetailData {
        &self.data
    }
    /// Still requires a current owner lease/control policy before any mutation.
    pub fn editable_base(&self) -> Option<QsoDraft> {
        self.data.editability.base.as_ref().map(EditBase::draft)
    }
}
impl Serialize for ContactDetail {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.data.validate().map_err(serde::ser::Error::custom)?;
        self.data.serialize(s)
    }
}
impl<'de> Deserialize<'de> for ContactDetail {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let data = DetailData::deserialize(d)?;
        data.validate().map_err(serde::de::Error::custom)?;
        Ok(Self { data })
    }
}

#[cfg(test)]
mod tests;

/// Authenticated selected-contact reads; no native/UI wiring.
pub mod client;
