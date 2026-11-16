// SPDX-License-Identifier: GPL-3.0-only
use super::{wire::*, *};
use sha2::{Digest, Sha256};

pub(super) fn uuid(s: &str) -> bool {
    crate::commands::valid_request_id(s)
        && !s.bytes().any(|b| b.is_ascii_uppercase())
        && s != "00000000-0000-0000-0000-000000000000"
}
pub(super) fn binding(b: &Binding) -> bool {
    [
        &b.server_instance,
        &b.workspace_id,
        &b.context_id,
        &b.clock_epoch,
    ]
    .iter()
    .all(|s| uuid(s))
}
pub(super) fn reason(s: &Option<String>) -> bool {
    s.as_ref()
        .is_none_or(|s| !s.is_empty() && s.len() <= 512 && !s.chars().any(char::is_control))
}
pub(super) fn digest(bytes: &[u8]) -> ([u8; 32], String) {
    let value: [u8; 32] = Sha256::digest(bytes).into();
    let text = value.iter().map(|b| format!("{b:02x}")).collect();
    (value, text)
}
pub(super) fn context(c: &Context, hello: &Hello, now: u64) -> Result<Duration, Error> {
    if c.binding.server_instance != hello.server_instance
        || c.binding.workspace_id != hello.workspace_id
    {
        return Err(Error::IdentityChanged);
    }
    let abs = f32::from_bits(c.max_abs_sample_f32_bits);
    if c.schema_version != 1
        || !binding(&c.binding)
        || !reason(&c.reason)
        || c.ready == c.reason.is_some()
        || c.ready && (!c.local_enabled || c.identity.is_none() || c.route.is_none())
        || c.max_age_ms == 0
        || c.max_age_ms > 1000
        || c.generated_at_unix_ms > SAFE
        || c.generated_at_unix_ms > now.saturating_add(50)
        || c.heartbeat_interval_ms != 200
        || c.heartbeat_timeout_ms != 1000
        || c.max_prepare_bytes == 0
        || c.max_prepare_bytes > MAX_PREPARE
        || c.min_offset_hz >= c.max_offset_hz
        || c.max_offset_hz >= 24_000
        || !abs.is_finite()
        || abs <= 0.0
        || abs > 0.95
    {
        return Err(Error::Invalid);
    }
    if let Some(i) = &c.identity {
        let valid_call = !i.mycall.is_empty()
            && i.mycall.len() <= 64
            && i.mycall
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'/')
            && i.mycall.bytes().any(|b| b.is_ascii_uppercase())
            && i.mycall.bytes().any(|b| b.is_ascii_digit())
            && !i.mycall.starts_with('/')
            && !i.mycall.ends_with('/')
            && !i.mycall.contains("//");
        let grid = i.mygrid.as_bytes();
        let valid_grid = matches!(grid.len(), 4 | 6 | 8)
            && grid.iter().enumerate().all(|(n, b)| match n {
                0 | 1 => (b'A'..=b'R').contains(b),
                4 | 5 => (b'A'..=b'X').contains(b),
                _ => b.is_ascii_digit(),
            });
        if !valid_call
            || !valid_grid
            || !uuid(&i.identity_epoch)
            || i.worker_generation == 0
            || i.worker_generation > SAFE
            || i.settings_revision > SAFE
            || i.identity_revision > SAFE
        {
            return Err(Error::Invalid);
        }
    }
    if let Some(r) = &c.route {
        if !uuid(&r.route_epoch)
            || !bounded(&r.radio_id, 128)
            || !bounded(&r.receiver_id, 256)
            || !bounded(&r.device_id, 1024)
            || !bounded(&r.radio_mode, 32)
            || r.transmit_slice.len() != 1
            || !r.transmit_slice.bytes().all(|b| (b'A'..=b'H').contains(&b))
            || r.sample_rate_hz != 48_000
            || r.channels != 2
            || r.selected_channels.is_empty()
            || r.selected_channels.len() > 2
            || r.selected_channels.iter().any(|ch| *ch >= r.channels)
            || r.selected_channels.len() == 2 && r.selected_channels[0] == r.selected_channels[1]
            || !frequency(r.frequency_hz)
        {
            return Err(Error::Invalid);
        }
    }
    let remaining = c
        .max_age_ms
        .saturating_sub(now.saturating_sub(c.generated_at_unix_ms));
    if remaining == 0 {
        return Err(Error::Stale);
    }
    Ok(Duration::from_millis(remaining))
}

pub(super) fn final_pcm(
    c: &Context,
    d: &Prepare,
    pcm: &[u8],
    client_id: &str,
) -> Result<(), Error> {
    let m = &d.manifest;
    let period = m.mode.period_us();
    let end = d
        .slot_start_unix_us
        .checked_add(period)
        .ok_or(Error::Invalid)?;
    if d.schema_version != 2
        || d.binding != c.binding
        || d.client_id != client_id
        || !uuid(&d.operation_id)
        || !uuid(&d.event_id)
        || end > SAFE
        || !d.slot_start_unix_us.is_multiple_of(period)
        || d.latest_start_unix_us < d.slot_start_unix_us
        || d.latest_start_unix_us >= end
        || d.latest_start_unix_us - d.slot_start_unix_us > 50_000
    {
        return Err(Error::Invalid);
    }
    let offset = f32::from_bits(m.offset_hz_f32_bits);
    let gain = f32::from_bits(m.gain_f32_bits);
    if m.schema_version != 2
        || m.message_form != "wsjt77"
        || m.format.sample_rate_hz != 48_000
        || m.format.channels != 1
        || m.format.encoding != "f32le"
        || m.frames != m.mode.frames()
        || m.pcm_byte_count != m.frames * 4
        || pcm.len() != m.pcm_byte_count as usize
        || m.period_duration_us != period
        || m.waveform_frames > m.frames
        || m.waveform_frames == 0
        || m.support_start_frame >= m.waveform_frames
        || m.group_delay_frames != 256
        || m.filter_id != "kaiser-513-fc6000-beta10-x4-candidate1"
        || m.coefficients_sha256
            != "b3b266e2346da0272cdb1910943a6ba8020d17a56e55a7b0eb0f12cd2c73c764"
        || !gain.is_finite()
        || gain <= 0.0
        || gain > 0.8
        || !offset.is_finite()
        || offset < c.min_offset_hz as f32
        || offset > c.max_offset_hz as f32
        || m.message.is_empty()
        || m.message.len() > 37
        || m.message.trim() != m.message
        || !m.message.bytes().all(|b| (0x20..=0x7e).contains(&b))
        || m.message.contains("<...>")
        || m.message.starts_with('@')
    {
        return Err(Error::Invalid);
    }
    let max_abs = f32::from_bits(c.max_abs_sample_f32_bits);
    let mut has_signal = false;
    for (i, sample) in pcm.chunks_exact(4).enumerate() {
        let value = f32::from_le_bytes(sample.try_into().map_err(|_| Error::Invalid)?);
        if !value.is_finite()
            || value.abs() > max_abs
            || (i < m.support_start_frame as usize || i >= m.waveform_frames as usize)
                && sample != [0, 0, 0, 0]
        {
            return Err(Error::Invalid);
        }
        has_signal |= value != 0.0;
    }
    if !has_signal || digest(pcm).1 != m.pcm_sha256 {
        return Err(Error::Invalid);
    }
    let mut hash = Sha256::new();
    hash.update(b"xnexus-final-period-v2\0");
    hash.update([match m.mode {
        Mode::FT8 => 8,
        Mode::FT4 => 4,
    }]);
    hash.update(m.format.sample_rate_hz.to_le_bytes());
    hash.update(m.format.channels.to_le_bytes());
    hash.update(b"f32le\0");
    hash.update(m.offset_hz_f32_bits.to_le_bytes());
    hash.update((m.message.len() as u32).to_le_bytes());
    hash.update(m.message.as_bytes());
    hash.update((pcm.len() as u64).to_le_bytes());
    hash.update(pcm);
    if format!("{:x}", hash.finalize()) != m.content_sha256 {
        return Err(Error::Invalid);
    }
    Ok(())
}

pub(super) fn receipt(
    r: &Receipt,
    key: &OperationKey,
    previous: Option<&Receipt>,
) -> Result<(), Error> {
    if r.schema_version != 1
        || r.server_instance != key.binding.server_instance
        || r.workspace_id != key.binding.workspace_id
        || r.context_id != key.binding.context_id
        || r.clock_epoch != key.binding.clock_epoch
        || r.operation_id != key.operation_id
        || r.body_sha256 != key.body_sha256
        || r.event_id != key.origin_event_id
    {
        return Err(Error::IdentityChanged);
    }
    if !reason(&r.reason)
        || r.actual_start_unix_us.is_some_and(|v| v == 0 || v > SAFE)
        || matches!((r.actual_start_unix_us,key.slot_window),(Some(start),Some((lo,hi))) if start<lo || start>hi)
        || r.release_observed_unix_us
            .is_some_and(|v| v == 0 || v > SAFE)
        || matches!((r.actual_start_unix_us,r.release_observed_unix_us),(Some(a),Some(b)) if b<a)
    {
        return Err(Error::Invalid);
    }
    let consistent = match (r.phase, r.outcome) {
        (Phase::Prepared, Outcome::NoStart) | (Phase::Terminal, Outcome::NoStart) => {
            r.actual_start_unix_us.is_none() && r.release_observed_unix_us.is_none()
        }
        (Phase::Starting, Outcome::Uncertain) => {
            r.actual_start_unix_us.is_none() && r.release_observed_unix_us.is_none()
        }
        (Phase::Active, Outcome::Uncertain) => {
            r.actual_start_unix_us.is_some() && r.release_observed_unix_us.is_none()
        }
        (Phase::Terminal, Outcome::ReleaseObserved) => r.release_observed_unix_us.is_some(),
        (Phase::Terminal, Outcome::Uncertain) => r.release_observed_unix_us.is_none(),
        _ => false,
    };
    if !consistent {
        return Err(Error::Invalid);
    }
    if let Some(old) = previous {
        if r.phase < old.phase
            || old.actual_start_unix_us.is_some()
                && r.actual_start_unix_us != old.actual_start_unix_us
            || old.release_observed_unix_us.is_some()
                && r.release_observed_unix_us != old.release_observed_unix_us
            || old.phase == Phase::Terminal && old.outcome != Outcome::Uncertain && r != old
        {
            return Err(Error::Revision);
        }
        if old.phase == Phase::Terminal
            && old.outcome == Outcome::Uncertain
            && r.outcome == Outcome::NoStart
        {
            return Err(Error::Revision);
        }
    }
    Ok(())
}
