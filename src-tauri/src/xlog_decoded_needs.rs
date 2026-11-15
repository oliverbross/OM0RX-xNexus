// SPDX-License-Identifier: GPL-3.0-only
//! Literal sender identity and revocable owner facts. No scoring or band inference.
//! Lookup admits at most32 distinct call/context pairs per second: never attempted
//! first (newest ties), then least recently attempted. Every retained key gets
//! a turn even when earlier facts expire before the next batch completes.
//! One pending group retries cache misses for at most6 SDK invocations or6s;
//! ordinary snapshot expiry defers it without resetting those bounds.
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
        // A sub-millisecond remainder is already unavailable to view/attach.
        // Do not return an unrenderable cache hit instead of scheduling a lookup.
        self.batch.remaining_valid_ms() > 0
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
/// A single immutable lookup group survives transient owner cache misses.
/// Bounds count SDK query invocations (which may fail before HTTP dispatch),
/// never idle deferrals; facts retain their independent original SDK leases.
struct Pending {
    keys: Vec<Candidate>,
    first_attempt: Instant,
    attempts: u8,
}
impl Pending {
    fn exhausted(&self, now: Instant) -> bool {
        self.attempts >= 6
            || now.saturating_duration_since(self.first_attempt) >= Duration::from_secs(6)
    }
}
#[derive(Default)]
pub struct LookupSchedule {
    scheduler: Scheduler,
    pending: Option<Pending>,
}
impl LookupSchedule {
    pub fn order(&mut self, retained: &[Candidate], eligible: &mut Vec<Candidate>, now: Instant) {
        if let Some(group) = &mut self.pending {
            // Never borrow a trimmed row's request ID for a newer duplicate.
            group.keys.retain(|key| retained.contains(key));
            if group.keys.is_empty() || group.exhausted(now) {
                self.cancel(true);
            }
        }
        self.scheduler.order(retained, eligible);
        if let Some(group) = &self.pending {
            eligible.retain(|row| group.keys.iter().any(|key| key.same_lookup(row)));
        }
    }
    pub fn query_key(&self, row: &Candidate) -> Candidate {
        self.pending
            .as_ref()
            .and_then(|group| group.keys.iter().find(|key| key.same_lookup(row)))
            .unwrap_or(row)
            .clone()
    }
    pub fn attempted(&mut self, keys: &[Candidate], started: Instant) {
        let group = self.pending.get_or_insert_with(|| {
            debug_assert!(!keys.is_empty() && keys.len() <= 32);
            Pending {
                keys: keys.to_vec(),
                first_attempt: started,
                attempts: 0,
            }
        });
        group.attempts = group.attempts.saturating_add(1);
    }
    pub fn original_survives(&self, candidate: &Candidate, retained: &[Candidate]) -> bool {
        self.pending.as_ref().is_some_and(|group| {
            group
                .keys
                .iter()
                .any(|key| key.same_lookup(candidate) && retained.contains(key))
        })
    }
    pub fn complete(&mut self, useful: bool, now: Instant) {
        if useful
            || self
                .pending
                .as_ref()
                .is_some_and(|group| group.exhausted(now))
        {
            self.cancel(true);
        }
    }
    pub fn cancel(&mut self, advance: bool) {
        if let Some(group) = self.pending.take() {
            if advance {
                self.scheduler.attempted(&group.keys);
            }
        }
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
    fn virtual_rows(count: usize) -> Vec<Candidate> {
        let route: AudioDescriptor = serde_json::from_value(serde_json::json!({
            "capture_id":"00000000-0000-4000-8000-000000000001","epoch":"00000000-0000-4000-8000-000000000002",
            "radio_id":"radio1","receiver_id":"fixture/slice-a","mapping_generation":1,"device_id":"fixture-dax",
            "slice":"A","dax_channel":1,"frequency_hz":14074000,"mode":"DIGU","source_channels":2,"selected_channel":0,
            "sample_rate":48000,"sample_format":"s16le","timestamp_basis":"windows_endpoint_capture",
            "remote_rf_latency_us":null,"utc_sync_accuracy_us":null
        })).unwrap();
        (0..count)
            .map(|i| Candidate {
                decode_id: format!("row_{i}"),
                callsign: format!("K{i}AA"),
                context: Context {
                    worker_instance: "worker".into(),
                    worker_epoch: 1,
                    server_instance: "owner".into(),
                    workspace_id: "workspace".into(),
                    route: route.clone(),
                    decoder_mode: Mode::FT8,
                },
            })
            .collect()
    }

    #[test]
    fn pending_groups_reach_every_row_with_independent_poll_phases_and_delays() {
        // Real scheduling primitives, virtual transport/owner: 1738ms owner
        // cycles; metadata initially1852ms old; independent main-poll phase,
        // 1000ms wait AFTER each100/218ms response; independent lookup phase.
        // Original two-second cache + captured authority deadlines never renew.
        // This models scheduling, not authenticated SDK acceptance (separate tests).
        struct Flight {
            keys: Vec<Candidate>,
            arrival: i64,
            response: i64,
            authority_deadline: i64,
            cached_generation: Option<Option<i64>>,
        }
        let mut covered_gap_after_cache = false;
        for count in [179, 200] {
            for snapshot_phase in [0i64, 311, 827] {
                for lookup_phase in [0i64, 149, 733] {
                    for delay in [100i64, 218] {
                        let retained = virtual_rows(count);
                        let start = Instant::now();
                        let mut schedule = LookupSchedule::default();
                        let observation = Observation::default();
                        observation.observe(1_000_000 - 1852);
                        let mut cadence =
                            Cadence::new(start + Duration::from_millis(lookup_phase as u64));
                        let mut owner_generated = -1852i64;
                        let mut observed_generated = owner_generated;
                        let mut next_poll = 709i64;
                        let mut snapshot_start = snapshot_phase;
                        let mut snapshot_flight: Option<(i64, i64)> = None;
                        let mut queued: Option<Vec<Candidate>> = None;
                        let mut cache = std::collections::HashMap::<String, i64>::new();
                        let mut known = std::collections::HashSet::new();
                        let mut previous_attempt = None;
                        let mut flight: Option<Flight> = None;
                        let mut pending_gap_after_cache = false;
                        for now in 0i64..60000 {
                            if now == next_poll {
                                owner_generated = now - 40;
                                if let Some(keys) = queued.take() {
                                    assert!(keys.len() <= 32);
                                    for key in keys {
                                        cache.insert(key.callsign, owner_generated);
                                    }
                                }
                                next_poll += 1738;
                            }
                            if now == snapshot_start {
                                snapshot_flight = Some((now + delay, owner_generated));
                                snapshot_start = i64::MAX;
                            }
                            let mut observation_wake = false;
                            if snapshot_flight.is_some_and(|(arrives, _)| arrives == now) {
                                observed_generated = snapshot_flight.take().unwrap().1;
                                if now - observed_generated < 2000 {
                                    observation.observe((1_000_000 + observed_generated) as u64);
                                    observation_wake = true;
                                }
                                snapshot_start = now + 1000;
                            }
                            if let Some(request) =
                                flight.as_mut().filter(|request| request.arrival == now)
                            {
                                let hit = request
                                    .keys
                                    .iter()
                                    .map(|key| cache.get(&key.callsign).copied())
                                    .collect::<Option<Vec<_>>>()
                                    .filter(|times| {
                                        times.iter().all(|generated| now - generated < 2000)
                                    })
                                    .and_then(|times| times.into_iter().min());
                                request.cached_generation = Some(hit);
                                if hit.is_none() && queued.is_none() {
                                    queued = Some(request.keys.clone());
                                }
                            }
                            if flight
                                .as_ref()
                                .is_some_and(|request| request.response == now)
                            {
                                let request = flight.take().unwrap();
                                let useful = now < request.authority_deadline
                                    && request
                                        .cached_generation
                                        .flatten()
                                        .is_some_and(|generated| now - generated < 2000);
                                if useful {
                                    for key in request.keys {
                                        known.insert(key.callsign);
                                    }
                                }
                                schedule
                                    .complete(useful, start + Duration::from_millis(now as u64));
                            }
                            if known.len() == count {
                                break;
                            }
                            if flight.is_some() || (!observation_wake && now % 100 != 0) {
                                continue;
                            }
                            let instant = start + Duration::from_millis(now as u64);
                            let mut eligible = retained.clone();
                            schedule.order(&retained, &mut eligible, instant);
                            let remaining = (2000 - (now - observed_generated)).max(0) as u64;
                            if remaining == 0 {
                                if schedule.pending.as_ref().is_some_and(|group| {
                                    group.keys.iter().all(|key| {
                                        cache
                                            .get(&key.callsign)
                                            .is_some_and(|generated| now - generated < 2000)
                                    })
                                }) {
                                    pending_gap_after_cache = true;
                                }
                                // Ordinary authority expiry defers without dropping
                                // the group or resetting its original residence bound.
                                continue;
                            }
                            if !cadence.ready(instant) || !observation.admit(remaining) {
                                continue;
                            }
                            let keys: Vec<_> = eligible
                                .iter()
                                .take(32)
                                .map(|row| schedule.query_key(row))
                                .collect();
                            if keys.is_empty() {
                                continue;
                            }
                            if let Some(previous) = previous_attempt {
                                assert!(now - previous >= 1000);
                            }
                            previous_attempt = Some(now);
                            cadence.attempted(instant);
                            schedule.attempted(&keys, instant);
                            flight = Some(Flight {
                                keys,
                                arrival: now + delay / 2,
                                response: now + delay,
                                authority_deadline: observed_generated + 2000,
                                cached_generation: None,
                            });
                        }
                        assert_eq!(known.len(),count,"snapshot_phase={snapshot_phase} lookup_phase={lookup_phase} delay={delay}");
                        covered_gap_after_cache |= pending_gap_after_cache;
                    }
                }
            }
        }
        assert!(
            covered_gap_after_cache,
            "phase matrix must exercise cache fill before authority gap and fresh observation"
        );
    }

    #[test]
    fn cached_pending_group_survives_authority_gap_without_renewing_residence() {
        let retained = virtual_rows(179);
        let start = Instant::now();
        let mut schedule = LookupSchedule::default();
        schedule.attempted(&retained[..32], start);
        // Owner fills at1738ms. The captured observation expires at1852ms;
        // next delayed main snapshot arrives2069ms, before next query can run.
        for millis in [1738, 1852, 1952, 2069] {
            let mut eligible = retained.clone();
            schedule.order(
                &retained,
                &mut eligible,
                start + Duration::from_millis(millis),
            );
            assert_eq!(eligible, retained[..32]);
            assert_eq!(schedule.pending.as_ref().unwrap().attempts, 1);
            assert_eq!(schedule.pending.as_ref().unwrap().first_attempt, start);
        }
        // A new query may now consume ORIGINAL still-fresh owner facts.
        schedule.attempted(&retained[..32], start + Duration::from_millis(2069));
        schedule.complete(true, start + Duration::from_millis(2287));
        let mut eligible = retained.clone();
        schedule.order(
            &retained,
            &mut eligible,
            start + Duration::from_millis(2287),
        );
        assert_eq!(eligible[0], retained[32]);
    }

    #[test]
    fn pending_failure_advances_fairness_after_six_invocations_or_six_seconds() {
        let retained = virtual_rows(200);
        let start = Instant::now();
        let mut schedule = LookupSchedule::default();
        let mut seen = std::collections::HashSet::new();
        let mut groups = Vec::<Vec<String>>::new();
        for second in 0..42u64 {
            let now = start + Duration::from_secs(second);
            let mut eligible = retained.clone();
            schedule.order(&retained, &mut eligible, now);
            let keys: Vec<_> = eligible.into_iter().take(32).collect();
            let ids: Vec<_> = keys.iter().map(|key| key.decode_id.clone()).collect();
            if second % 6 == 0 {
                groups.push(ids.clone());
            }
            assert_eq!(ids, *groups.last().unwrap());
            for key in &keys {
                seen.insert(key.decode_id.clone());
            }
            schedule.attempted(&keys, now);
            schedule.complete(false, now);
        }
        assert_eq!(seen.len(), 200);
        // Idle deferral does not count attempts, but cannot hold the group forever.
        let mut schedule = LookupSchedule::default();
        schedule.attempted(&retained[..32], start);
        let mut eligible = retained.clone();
        schedule.order(
            &retained,
            &mut eligible,
            start + Duration::from_millis(5999),
        );
        assert_eq!(eligible[0], retained[0]);
        schedule.order(&retained, &mut eligible, start + Duration::from_secs(6));
        // Recreate full eligible list, as the real worker does each iteration.
        let mut eligible = retained.clone();
        schedule.order(&retained, &mut eligible, start + Duration::from_secs(6));
        assert_eq!(eligible[0], retained[32]);
    }

    #[test]
    fn pending_identity_trim_route_and_revocation_never_rebind_old_queries() {
        let retained = virtual_rows(3);
        let now = Instant::now();
        let mut schedule = LookupSchedule::default();
        schedule.attempted(&retained[..2], now);
        let mut duplicate = retained[0].clone();
        duplicate.decode_id = "new_duplicate".into();
        let survivors = vec![duplicate.clone(), retained[1].clone(), retained[2].clone()];
        assert!(!schedule.original_survives(&duplicate, &survivors));
        let mut eligible = survivors.clone();
        schedule.order(&survivors, &mut eligible, now);
        assert_eq!(eligible, vec![retained[1].clone()]);
        for changed in 0..7 {
            let mut replacement = retained[0].clone();
            match changed {
                0 => replacement.context.worker_epoch += 1,
                1 => replacement.context.server_instance = "new_owner".into(),
                2 => replacement.context.workspace_id = "new_workspace".into(),
                3 => replacement.context.decoder_mode = Mode::FT4,
                4 => replacement.context.route.epoch = "new_epoch".into(),
                5 => replacement.context.route.frequency_hz += 1000,
                _ => replacement.context.route.mapping_generation += 1,
            }
            let mut schedule = LookupSchedule::default();
            schedule.attempted(&retained[..1], now);
            assert!(!schedule.original_survives(&replacement, std::slice::from_ref(&replacement)));
            let mut eligible = vec![replacement.clone()];
            schedule.order(std::slice::from_ref(&replacement), &mut eligible, now);
            assert_eq!(schedule.query_key(&replacement), replacement);
            assert!(schedule.pending.is_none());
        }
        // Authority cancellation advances only the already attempted group.
        schedule.cancel(true);
        assert!(schedule.pending.is_none());
        let mut eligible = retained.clone();
        schedule.order(&retained, &mut eligible, now);
        assert_eq!(eligible[0], retained[0]); // removed first key was never rebound
        schedule.attempted(&retained[..1], now);
        schedule.cancel(false); // write fence discards pending without a query
        assert!(schedule.pending.is_none());
    }

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
