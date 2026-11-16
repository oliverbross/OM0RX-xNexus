// SPDX-License-Identifier: GPL-3.0-only
//! Protocol DTOs only. These public values are NOT validated receipt proof.
//! Owner contract source SHA256:1208BDA0E4B86AB724E76C279C2B02E1258830ED0EE32336F7549E0BE898EC17.
use serde::{Deserialize, Serialize};

fn required<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub server_instance: String,
    pub workspace_id: String,
    pub context_id: String,
    pub clock_epoch: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    FT8,
    FT4,
}
impl Mode {
    pub fn frames(self) -> u32 {
        match self {
            Self::FT8 => 720_000,
            Self::FT4 => 360_000,
        }
    }
    pub fn period_us(self) -> u64 {
        match self {
            Self::FT8 => 15_000_000,
            Self::FT4 => 7_500_000,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Format {
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub encoding: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub mode: Mode,
    pub message_form: String,
    pub message: String,
    pub format: Format,
    pub offset_hz_f32_bits: u32,
    pub frames: u32,
    pub waveform_frames: u32,
    pub filter_id: String,
    pub coefficients_sha256: String,
    pub group_delay_frames: u32,
    pub support_start_frame: u32,
    pub gain_f32_bits: u32,
    pub pcm_byte_count: u32,
    pub period_duration_us: u64,
    pub pcm_sha256: String,
    pub content_sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prepare {
    pub schema_version: u32,
    pub binding: Binding,
    pub client_id: String,
    pub operation_id: String,
    pub event_id: String,
    pub slot_start_unix_us: u64,
    pub latest_start_unix_us: u64,
    pub manifest: Manifest,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enable {
    pub binding: Binding,
    pub client_id: String,
    pub event_id: String,
    pub enabled: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Heartbeat {
    pub binding: Binding,
    pub client_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub binding: Binding,
    pub client_id: String,
    pub operation_id: String,
    pub body_sha256: String,
    pub event_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tune {
    pub binding: Binding,
    pub client_id: String,
    pub operation_id: String,
    pub event_id: String,
    pub tone_hz_f32_bits: u32,
    pub gain_f32_bits: u32,
    pub duration_ms: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Prepared,
    Starting,
    Active,
    Terminal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    NoStart,
    ReleaseObserved,
    Uncertain,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema_version: u32,
    pub server_instance: String,
    pub workspace_id: String,
    pub operation_id: String,
    pub body_sha256: String,
    pub context_id: String,
    pub clock_epoch: String,
    pub event_id: String,
    pub phase: Phase,
    pub outcome: Outcome,
    #[serde(deserialize_with = "required")]
    pub actual_start_unix_us: Option<u64>,
    #[serde(deserialize_with = "required")]
    pub release_observed_unix_us: Option<u64>,
    #[serde(deserialize_with = "required")]
    pub reason: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputRoute {
    pub radio_id: String,
    pub receiver_id: String,
    pub transmit_slice: String,
    pub device_id: String,
    pub route_epoch: String,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub selected_channels: Vec<u16>,
    pub frequency_hz: u64,
    pub radio_mode: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub mycall: String,
    pub mygrid: String,
    pub worker_generation: u64,
    pub settings_revision: u64,
    pub identity_epoch: String,
    pub identity_revision: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub schema_version: u32,
    pub binding: Binding,
    pub ready: bool,
    #[serde(deserialize_with = "required")]
    pub reason: Option<String>,
    pub local_enabled: bool,
    pub client_enabled: bool,
    #[serde(deserialize_with = "required")]
    pub identity: Option<Identity>,
    #[serde(deserialize_with = "required")]
    pub route: Option<OutputRoute>,
    pub generated_at_unix_ms: u64,
    pub max_age_ms: u64,
    pub heartbeat_interval_ms: u64,
    pub heartbeat_timeout_ms: u64,
    pub max_prepare_bytes: usize,
    pub min_offset_hz: u32,
    pub max_offset_hz: u32,
    pub max_abs_sample_f32_bits: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub schema_version: u32,
    pub server_instance: String,
    pub workspace_id: String,
    pub operation_id: String,
    #[serde(deserialize_with = "required")]
    pub receipt: Option<Receipt>,
}
