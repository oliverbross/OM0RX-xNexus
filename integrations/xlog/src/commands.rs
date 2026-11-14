// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Explicit, identity-bound receive/control actions. No transmit path exists.
//! The independent stop transport never takes the poll client's mutex.
use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Command {
    #[serde(rename = "station.tune")]
    StationTune {
        radio_id: String,
        frequency_hz: u64,
        mode: String,
    },
    #[serde(rename = "rotor.point")]
    RotorPoint { rotor_id: String, azimuth_deg: f64 },
    #[serde(rename = "rotor.stop")]
    RotorStop { rotor_id: String },
    #[serde(rename = "logbook.create")]
    LogbookCreate { qso: crate::outbox::QsoDraft },
    #[serde(rename = "logbook.update")]
    LogbookUpdate {
        qso: crate::outbox::QsoDraft,
        expected_contact_revision: u64,
    },
}
impl Command {
    pub fn capability(&self) -> &'static str {
        match self {
            Self::StationTune { .. } => "station.tune",
            Self::RotorPoint { .. } => "rotor.point",
            Self::RotorStop { .. } => "rotor.stop",
            Self::LogbookCreate { .. } | Self::LogbookUpdate { .. } => "logbook.write",
        }
    }
    fn validate(&self) -> Result<(), Error> {
        let valid = match self {
            Self::StationTune {
                radio_id,
                frequency_hz,
                mode,
            } => bounded(radio_id, 128) && frequency(*frequency_hz) && bounded(mode, 32),
            Self::RotorPoint {
                rotor_id,
                azimuth_deg,
            } => {
                bounded(rotor_id, 128)
                    && azimuth_deg.is_finite()
                    && (0.0..360.0).contains(azimuth_deg)
            }
            Self::RotorStop { rotor_id } => bounded(rotor_id, 128),
            Self::LogbookCreate { qso } => qso.validate().is_ok(),
            Self::LogbookUpdate {
                qso,
                expected_contact_revision,
            } => {
                qso.validate().is_ok()
                    && (1..=9_007_199_254_740_990).contains(expected_contact_revision)
            }
        };
        if valid {
            Ok(())
        } else {
            Err(Error::Invalid)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Accepted,
    Applied,
    Observed,
    Rejected,
    Uncertain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandReceipt {
    pub request_id: String,
    pub server_instance: String,
    pub workspace_id: String,
    pub status: CommandStatus,
    pub message: Option<String>,
    #[serde(default)]
    pub contact_id: Option<String>,
    #[serde(default)]
    pub contact_revision: Option<u64>,
}
impl CommandReceipt {
    fn validate(&self, hello: &Hello, request_id: &str) -> Result<(), Error> {
        if self.request_id != request_id
            || self.server_instance != hello.server_instance
            || self.workspace_id != hello.workspace_id
        {
            return Err(Error::IdentityChanged);
        }
        if self.message.as_ref().is_some_and(|s| !bounded(s, 512))
            || self
                .contact_id
                .as_ref()
                .is_some_and(|s| !valid_request_id(s))
            || self
                .contact_revision
                .is_some_and(|v| v > 9_007_199_254_740_990)
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct Envelope<'a> {
    request_id: &'a str,
    server_instance: &'a str,
    workspace_id: &'a str,
    expected_revision: u64,
    command: &'a Command,
}

pub(crate) fn valid_request_id(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
fn send(
    http: &Client,
    endpoint: &str,
    hello: &Hello,
    revision: u64,
    request_id: &str,
    command: &Command,
) -> Result<CommandReceipt, Error> {
    if !valid_request_id(request_id) {
        return Err(Error::Invalid);
    }
    command.validate()?;
    if !hello.capabilities.iter().any(|s| s == command.capability()) {
        return Err(Error::Capability);
    }
    let body = serde_json::to_vec(&Envelope {
        request_id,
        server_instance: &hello.server_instance,
        workspace_id: &hello.workspace_id,
        expected_revision: revision,
        command,
    })
    .map_err(|_| Error::Invalid)?;
    let response = http
        .post(format!("{endpoint}/xlog/v1/command"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .map_err(|_| Error::OutcomeUnknown)?;
    // Once sent, a lost/malformed reply cannot establish whether the owner acted.
    let receipt: CommandReceipt = read_response(response, 65536).map_err(|error| match error {
        Error::Http(401 | 403) => error,
        _ => Error::OutcomeUnknown,
    })?;
    receipt
        .validate(hello, request_id)
        .map_err(|_| Error::OutcomeUnknown)?;
    Ok(receipt)
}

/// Copy this at connection time; keep it outside the ordinary poll-client mutex.
/// A revoked token or changed workspace still fails at xLog's owner boundary.
#[derive(Clone)]
pub struct StopClient {
    http: Client,
    endpoint: String,
    hello: Hello,
}
impl StopClient {
    pub fn stop_rotor(&self, request_id: &str, rotor_id: &str) -> Result<CommandReceipt, Error> {
        send(
            &self.http,
            &self.endpoint,
            &self.hello,
            0,
            request_id,
            &Command::RotorStop {
                rotor_id: rotor_id.into(),
            },
        )
    }
}

impl BridgeClient {
    pub fn stop_handle(&self) -> Result<StopClient, Error> {
        let hello = self.hello.as_ref().ok_or(Error::Disconnected)?;
        if !hello.capabilities.iter().any(|c| c == "rotor.stop") {
            return Err(Error::Capability);
        }
        Ok(StopClient {
            http: self.http.clone(),
            endpoint: self.endpoint.clone(),
            hello: hello.clone(),
        })
    }
    pub fn command(&mut self, request_id: &str, command: Command) -> Result<CommandReceipt, Error> {
        if let Command::RotorStop { rotor_id } = &command {
            return self.stop_handle()?.stop_rotor(request_id, rotor_id);
        }
        command.validate()?;
        let snapshot = self.snapshot()?;
        match &command {
            Command::StationTune { radio_id, .. }
                if !snapshot.station.connected
                    || snapshot.station.radio_id.as_ref() != Some(radio_id)
                    || snapshot.station.transmitting != Some(false) =>
            {
                return Err(Error::Invalid)
            }
            Command::RotorPoint { rotor_id, .. }
                if !snapshot.rotor.connected
                    || snapshot.rotor.rotor_id.as_ref() != Some(rotor_id) =>
            {
                return Err(Error::Invalid)
            }
            _ => {}
        }
        let revision = snapshot.revision;
        let hello = self.hello.as_ref().ok_or(Error::Disconnected)?;
        let result = send(
            &self.http,
            &self.endpoint,
            hello,
            revision,
            request_id,
            &command,
        );
        // A new observation is required after any sent action. Never reuse stale pre-action state.
        self.observation = None;
        if matches!(result, Err(Error::Http(401 | 403))) {
            self.disconnect();
        }
        result
    }
    pub fn receipt(&mut self, request_id: &str) -> Result<Option<CommandReceipt>, Error> {
        if !valid_request_id(request_id) {
            return Err(Error::Invalid);
        }
        let result = (|| {
            let hello = self.hello.as_ref().ok_or(Error::Disconnected)?;
            let receipt: CommandReceipt =
                match self.get(&format!("/xlog/v1/receipt/{request_id}"), 65536) {
                    Ok(receipt) => receipt,
                    Err(Error::Http(404)) => return Ok(None),
                    Err(error) => return Err(error),
                };
            receipt.validate(hello, request_id)?;
            Ok(Some(receipt))
        })();
        if matches!(result, Err(Error::IdentityChanged | Error::Http(401 | 403))) {
            self.disconnect();
        }
        result
    }
}
