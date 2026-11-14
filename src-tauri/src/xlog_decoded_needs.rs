// SPDX-License-Identifier: GPL-3.0-only
//! Literal sender identity and revocable owner facts. No scoring or band inference.
//! Lookup admits 32 distinct call/context pairs per second: never attempted
//! first (newest ties), then least recently attempted. Every retained key gets
//! a turn even when earlier facts expire before the next batch completes.
//! At most 200 rows/cache entries survive, while owner leases last at most two
//! seconds. Older visible rows may therefore remain Unknown: retaining a row
//! does not promise continuous classification, and cache hits never renew TTL.
use super::Mode;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tempo_core::message::Msg;
use xnexus_xlog_client::audio::AudioDescriptor;
use xnexus_xlog_client::needs::{DecoderMode, NeedsAuthority, NeedsBatch, NeedsResult};
use xnexus_xlog_client::NeedStatus;

// Strictly more than the one-second HTTP deadline. Deferral does not
// consume the one-second HTTP cadence or mutate any existing fact's expiry.
pub fn query_has_budget(remaining_ms: u64) -> bool {
    remaining_ms > 1000
}
/// One fallback admission for each newer owner observation, without inventing
/// an authority counter or renewing the SDK's original monotonic deadline.
#[derive(Default)]
pub struct Observation(Mutex<(u64, bool)>);
impl Observation {
    pub fn observe(&self, generated_at: u64) {
        if let Ok(mut value) = self.0.lock() {
            if generated_at > value.0 {
                *value = (generated_at, true);
            }
        }
    }
    pub fn admit(&self, remaining_ms: u64) -> bool {
        if remaining_ms == 0 {
            return false;
        }
        let Ok(mut value) = self.0.lock() else {
            return false;
        };
        if query_has_budget(remaining_ms) || value.1 {
            value.1 = false;
            true
        } else {
            false
        }
    }
}
pub struct Cadence {
    next: Instant,
}
impl Cadence {
    pub fn new(now: Instant) -> Self {
        Self { next: now }
    }
    pub fn ready(&self, now: Instant) -> bool {
        now >= self.next
    }
    pub fn attempted(&mut self, started: Instant) {
        self.next = started + Duration::from_secs(1);
    }
    pub fn wait(&self, now: Instant) -> Duration {
        let until = self.next.saturating_duration_since(now);
        if until.is_zero() {
            Duration::from_millis(100)
        } else {
            until.min(Duration::from_millis(100))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Context {
    pub worker_instance: String,
    pub worker_epoch: u64,
    pub server_instance: String,
    pub workspace_id: String,
    pub route: AudioDescriptor,
    pub decoder_mode: Mode,
}
impl Context {
    pub fn decoder_mode(&self) -> DecoderMode {
        match self.decoder_mode {
            Mode::FT8 => DecoderMode::FT8,
            Mode::FT4 => DecoderMode::FT4,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct View {
    pub status: NeedStatus,
    pub score: Option<i32>,
    pub reason_code: Option<String>,
    pub reasons: Vec<String>,
    pub owner: Option<NeedsResult>,
    pub authority: Option<NeedsAuthority>,
    pub generated_at_unix_ms: Option<u64>,
    pub max_age_ms: Option<u64>,
    /// Native monotonic remaining lifetime. UI deadlines start BEFORE its IPC request.
    pub remaining_valid_ms: u64,
}
impl Default for View {
    fn default() -> Self {
        Self::unknown("native_unclassified")
    }
}
impl View {
    pub fn unknown(reason: &str) -> Self {
        Self {
            status: NeedStatus::Unknown,
            score: None,
            reason_code: Some(reason.into()),
            reasons: vec![],
            owner: None,
            authority: None,
            generated_at_unix_ms: None,
            max_age_ms: None,
            remaining_valid_ms: 0,
        }
    }
}

#[derive(Clone)]
pub struct Fact {
    batch: Arc<NeedsBatch>,
    query_id: String,
}
impl Fact {
    pub fn new(batch: Arc<NeedsBatch>, query_id: String) -> Option<Self> {
        let fact = Self { batch, query_id };
        fact.view().map(|_| fact)
    }
    pub fn view(&self) -> Option<View> {
        let remaining_valid_ms = self.batch.remaining_valid_ms();
        if remaining_valid_ms == 0 {
            return None;
        }
        let metadata = self.batch.metadata().ok()?;
        let result = self
            .batch
            .results()
            .ok()?
            .iter()
            .find(|r| r.query_id == self.query_id)?;
        Some(View {
            status: result.need.status,
            score: result.need.score,
            reason_code: result.need.reason_code.clone(),
            reasons: result.need.reasons.clone(),
            owner: Some(result.clone()),
            authority: metadata.authority.clone(),
            generated_at_unix_ms: Some(metadata.generated_at_unix_ms),
            max_age_ms: Some(metadata.max_age_ms),
            remaining_valid_ms,
        })
    }
    pub fn is_fresh(&self) -> bool {
        self.batch.is_fresh()
    }
    pub fn matches(&self, candidate: &Candidate) -> bool {
        self.batch.results().ok().is_some_and(|results| {
            results.iter().any(|row| {
                row.query_id == self.query_id
                    && row.normalized_callsign == candidate.callsign
                    && row.dial_frequency_hz == candidate.context.route.frequency_hz
                    && row.decoder_mode == candidate.context.decoder_mode()
            })
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub decode_id: String,
    pub callsign: String,
    pub context: Context,
}
impl Candidate {
    pub fn same_lookup(&self, other: &Self) -> bool {
        self.callsign == other.callsign && self.context == other.context
    }
}
#[derive(Default)]
pub struct Cache(Vec<(Candidate, Fact)>);

/// Oldest attempt first. No wall clock/counter or TTL extension is involved.
#[derive(Default)]
pub struct Scheduler(Vec<Candidate>);
impl Scheduler {
    pub fn order(&mut self, retained: &[Candidate], eligible: &mut [Candidate]) {
        self.0
            .retain(|old| retained.iter().any(|row| old.same_lookup(row)));
        // Stable sort preserves newest-row order among never-attempted keys.
        eligible.sort_by_key(|row| {
            self.0
                .iter()
                .position(|old| old.same_lookup(row))
                .map(|n| n + 1)
                .unwrap_or(0)
        });
    }
    pub fn attempted(&mut self, keys: &[Candidate]) {
        for key in keys {
            self.0.retain(|old| !old.same_lookup(key));
            self.0.push(key.clone());
        }
        let extra = self.0.len().saturating_sub(200);
        self.0.drain(..extra);
    }
}
impl Cache {
    pub fn find(&mut self, candidate: &Candidate) -> Option<Fact> {
        self.0.retain(|(_, fact)| fact.is_fresh());
        self.0
            .iter()
            .find(|(old, _)| old.same_lookup(candidate))
            .map(|(_, fact)| fact.clone())
    }
    pub fn insert(&mut self, candidate: Candidate, fact: Fact) {
        self.0
            .retain(|(old, fact)| !old.same_lookup(&candidate) && fact.is_fresh());
        self.0.push((candidate, fact));
        let extra = self.0.len().saturating_sub(200);
        self.0.drain(..extra);
    }
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

pub fn literal_sender(message: &str) -> Option<String> {
    if !message.is_ascii() || message.len() > 512 || message.bytes().any(|b| b.is_ascii_control()) {
        return None;
    }
    let canonical = message
        .split_whitespace()
        .map(str::to_ascii_uppercase)
        .collect::<Vec<_>>()
        .join(" ");
    let parsed = Msg::parse(&canonical);
    // Some tolerant CQ parser arms consume a suffix from malformed free text.
    // A complete recognized form must round-trip, not merely end in a grid.
    if matches!(parsed, Msg::Other(_))
        || (matches!(parsed, Msg::Cq { .. }) && parsed.to_text() != canonical)
    {
        return None;
    }
    let sender = parsed.sender()?;
    let bytes = sender.as_bytes();
    let report = (2..=3).contains(&bytes.len())
        && bytes[0].is_ascii_digit()
        && bytes.iter().all(|b| b.is_ascii_digit() || *b == b'N');
    if !(3..=32).contains(&bytes.len())
        || !bytes
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'/')
        || !bytes.iter().any(u8::is_ascii_uppercase)
        || !bytes.iter().any(u8::is_ascii_digit)
        || sender.starts_with('/')
        || sender.ends_with('/')
        || sender.contains("//")
        || report
        || matches!(sender, "RR73" | "RRR" | "R73")
    {
        return None;
    }
    Some(sender.to_owned())
}

pub fn decode_id(worker: &str, epoch: u64, period: u64, ordinal: u64) -> Option<String> {
    let id = format!("{worker}_e{epoch}_p{period}_n{ordinal}");
    (id.len() <= 128 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')).then_some(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn low_budget_deferrals_escape_one_second_owner_poll_phase_lock() {
        let start = Instant::now();
        for age in [900u64, 990] {
            let initial = age + 501;
            let mut cadence = Cadence::new(start + Duration::from_millis(initial));
            assert!(!query_has_budget(2000 - initial));
            // Deferral does not advance cadence. Main poll wakes immediately
            // after observing the next snapshot, including1ms local overhead.
            let wake = age + 1001;
            assert!(cadence.ready(start + Duration::from_millis(wake)));
            assert!(query_has_budget(3000 - wake));
            cadence.attempted(start + Duration::from_millis(wake));
            let next_owner_reply = 2055; // next1Hz owner turn plus55ms query work
            assert!(
                next_owner_reply < 3000,
                "original captured expiry remains fixed"
            );
            assert!(next_owner_reply - wake < 1000);
            // A slightly early next wake waits only to admission, not another100ms.
            assert_eq!(
                cadence.wait(start + Duration::from_millis(wake + 999)),
                Duration::from_millis(1)
            );
        }
        assert!(!query_has_budget(1000));
        assert!(query_has_budget(1001));
    }
    #[test]
    fn observation_fallback_is_once_per_new_snapshot_and_never_revives_expired_authority() {
        let observation = Observation::default();
        assert!(!observation.admit(600), "idle insufficient budget defers");
        observation.observe(1000);
        assert!(!observation.admit(0), "expired authority never queries");
        assert!(
            observation.admit(600),
            "pending wake survives deferral until real dispatch"
        );
        assert!(!observation.admit(600));
        observation.observe(1000);
        observation.observe(999);
        assert!(
            !observation.admit(600),
            "same/older metadata cannot create fallback attempts"
        );
        observation.observe(2000);
        assert!(
            observation.admit(10),
            "fresh near-boundary snapshot gets one bounded attempt"
        );
        assert!(!observation.admit(10));
        assert!(
            observation.admit(1001),
            "idle healthy budget remains usable"
        );
        observation.observe(u64::MAX);
        assert!(observation.admit(10));
        observation.observe(1);
        assert!(!observation.admit(10), "no counter wrap or authority reuse");
    }
    #[test]
    fn immutable_id_is_bounded_even_at_counter_limits() {
        let worker = format!("w{:x}_{:x}_{:x}", u32::MAX, u64::MAX, u64::MAX);
        let id = decode_id(&worker, u64::MAX, u64::MAX, u64::MAX).unwrap();
        assert!(id.len() <= 128);
        assert_ne!(
            id,
            decode_id(&worker, u64::MAX, u64::MAX, u64::MAX - 1).unwrap()
        );
        assert!(decode_id(&"x".repeat(129), 1, 1, 1).is_none());
        assert!(decode_id("bad/route", 1, 1, 1).is_none());
    }
    #[test]
    fn literal_calls_preserve_portable_and_special_identity_without_hash_borrowing() {
        for (message, expected) in [
            ("CQ K1ABC FN42", "K1ABC"),
            ("CQ EA1/PE5X", "EA1/PE5X"),
            ("CQ II7MGBR", "II7MGBR"),
            ("CQ EN3SUKR", "EN3SUKR"),
            ("CQ GB50 FN42", "GB50"),
            ("CQ 9A2024 JN75", "9A2024"),
            ("CQ K1ABC/MM FN42", "K1ABC/MM"),
            ("CQ K1ABC/AM FN42", "K1ABC/AM"),
            ("<K1ABC> W9XYZ -10", "W9XYZ"),
            ("W9XYZ K1ABC -40", "K1ABC"),
            ("CQ DX K1ABC FN42", "K1ABC"),
            (" cq   k1abc fn42 ", "K1ABC"),
        ] {
            assert_eq!(
                literal_sender(message).as_deref(),
                Some(expected),
                "{message}"
            );
        }
    }
    #[test]
    fn malformed_text_reports_and_bracketed_senders_never_become_calls() {
        for message in [
            "CQ <...>",
            "CQ <K1ABC> FN42",
            "W9XYZ <K1ABC> -10",
            "HELLO WORLD",
            "CQ 599 FN42",
            "CQ 5NN FN42",
            "CQ 59N FN42",
            "CQ RR73 FN42",
            "CQ R73 FN42",
            "CQ TOO MANY WORDS K1ABC FN42",
            "CQ K1ABC/ FN42",
            "CQ /K1ABC FN42",
            "CQ K1ABC//P FN42",
            "CQ K1ABC FN42\n",
        ] {
            assert_eq!(literal_sender(message), None, "{message}");
        }
    }
}
