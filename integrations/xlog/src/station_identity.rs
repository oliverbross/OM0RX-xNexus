// SPDX-License-Identifier: GPL-3.0-only
//! Saved owner identity for display only. Validity is the enclosing Snapshot's
//! session/revision/freshness contract; this is never a transmission grant.
use crate::Error;
use serde::{Deserialize, Serialize};

const MAX_SAFE: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentitySource {
    SavedStationSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityState {
    Current,
    Incomplete,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatingModule {
    NormalLogging,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct IdentityBinding {
    pub workspace_generation: String,
    pub worker_generation: u64,
    pub settings_revision: u64,
    pub identity_epoch: String,
    pub identity_revision: u64,
    pub operating_module: OperatingModule,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct StationIdentity {
    pub schema_version: u32,
    pub source: IdentitySource,
    pub state: IdentityState,
    pub mycall: Option<String>,
    pub mygrid: Option<String>,
    pub binding: Option<IdentityBinding>,
    pub transmit_authority: bool,
}

impl StationIdentity {
    pub(crate) fn validate(&self, workspace: &str) -> Result<(), Error> {
        if self.schema_version != 1
            || self.transmit_authority
            || self.mycall.as_deref().is_some_and(|call| !valid_call(call))
            || self.mygrid.as_deref().is_some_and(|grid| !valid_grid(grid))
        {
            return Err(Error::Invalid);
        }
        let has_both = self.mycall.is_some() && self.mygrid.is_some();
        match self.state {
            IdentityState::Unavailable => {
                if self.mycall.is_some() || self.mygrid.is_some() || self.binding.is_some() {
                    return Err(Error::Invalid);
                }
            }
            IdentityState::Current | IdentityState::Incomplete => {
                if (self.state == IdentityState::Current) != has_both {
                    return Err(Error::Invalid);
                }
                let binding = self.binding.as_ref().ok_or(Error::Invalid)?;
                if binding.workspace_generation != workspace
                    || !non_nil_uuid(&binding.workspace_generation)
                    || !non_nil_uuid(&binding.identity_epoch)
                    || !(1..=MAX_SAFE).contains(&binding.worker_generation)
                    || binding.settings_revision > MAX_SAFE
                    || binding.identity_revision > MAX_SAFE
                {
                    return Err(Error::Invalid);
                }
            }
        }
        Ok(())
    }
}

fn valid_call(call: &str) -> bool {
    !call.is_empty()
        && call.len() <= 64
        && call
            .bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'/')
        && call.bytes().any(|c| c.is_ascii_uppercase())
        && call.bytes().any(|c| c.is_ascii_digit())
        && !call.starts_with('/')
        && !call.ends_with('/')
        && !call.contains("//")
}

fn valid_grid(grid: &str) -> bool {
    let bytes = grid.as_bytes();
    matches!(bytes.len(), 4 | 6 | 8)
        && bytes.iter().enumerate().all(|(i, b)| match i {
            0 | 1 => (b'A'..=b'R').contains(b),
            4 | 5 => (b'A'..=b'X').contains(b),
            _ => b.is_ascii_digit(),
        })
}

// Match the owner's UUID parser formats without adding a dependency. Identity
// equality remains exact string equality, not UUID alias normalization.
fn non_nil_uuid(value: &str) -> bool {
    let value = if let Some(v) = value.strip_prefix("urn:uuid:") {
        if v.len() != 36 {
            return false;
        }
        v
    } else if let Some(v) = value.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
        if v.len() != 36 {
            return false;
        }
        v
    } else {
        value
    };
    let bytes = value.as_bytes();
    (bytes.len() == 32 || bytes.len() == 36)
        && bytes.iter().enumerate().all(|(i, c)| {
            if bytes.len() == 36 && matches!(i, 8 | 13 | 18 | 23) {
                *c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
        && bytes.iter().any(|c| c.is_ascii_hexdigit() && *c != b'0')
}

#[cfg(test)]
mod tests;
