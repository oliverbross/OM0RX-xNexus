// SPDX-License-Identifier: GPL-3.0-only
// Acceptance-only normal library: owner cfg(test) modules are not compiled.
include!(concat!(env!("OUT_DIR"), "/owner_modules.rs"));

use digital_tx_owner::{Journal, Observation, Owner, Permit, Phase, Record, Transport};
use output_authority::{Authority, Reservation};
use prepared_period_adapter::{
    PreparedOperation, SyntheticContext, SyntheticObservation, SyntheticPowerPolicy,
};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tempo_core::{message::Msg, modes::ModeKind};
use xnexus_tx_prepare::{Encoding, Format, Limits};
use xnexus_tx_sequence::{
    Adapter, Context, FakeActualStart, Halt, NonStart, OfflineOperation, OperatorIntent,
    OperatorSettings, PreparedIntent,
};

const ID: &str = "10000000-0000-4000-8000-000000000001";
const SECOND_ID: &str = "10000000-0000-4000-8000-000000000002";
const SLOT: u64 = 30_000_000;
const CASES: [&str; 14] = [
    "ft8-cq",
    "ft8-grid",
    "ft8-report",
    "ft8-rreport",
    "ft8-rrr",
    "ft8-rr73",
    "ft8-73",
    "ft4-cq",
    "ft4-grid",
    "ft4-report",
    "ft4-rreport",
    "ft4-rrr",
    "ft4-rr73",
    "ft4-73",
];

#[derive(Default)]
struct MemoryJournal(Vec<Record>);
impl Journal for MemoryJournal {
    fn read(&mut self) -> Result<Vec<Record>, String> {
        Ok(self.0.clone())
    }
    fn append(&mut self, record: &Record) -> Result<(), String> {
        self.0.push(record.clone());
        Ok(())
    }
}
struct FakeStarted {
    reservation: Reservation,
    operation: Arc<PreparedOperation>,
    fake_start_us: u64,
}
struct FakeSink {
    operation: Arc<PreparedOperation>,
    observation: SyntheticObservation,
    started: Mutex<Vec<FakeStarted>>,
    released: Mutex<Vec<Reservation>>,
}
impl Transport for FakeSink {
    fn start(&self, exact: &Reservation, permit: &Permit) -> Result<(), String> {
        if exact.operation_id() != self.operation.body().operation_id
            || !permit.matches_prepared_body(self.operation.body())
            || !permit.begin_effect(self.observation.owner())
        {
            return Err("Exact immutable fake sink object retired or substituted".into());
        }
        // Explicit fake effect record retaining THE object given to this sink.
        // Not a timestamp/authority claim about any real output device.
        self.started.lock().unwrap().push(FakeStarted {
            reservation: exact.clone(),
            operation: self.operation.clone(),
            fake_start_us: self.observation.owner().now_us,
        });
        Ok(()) // Caller still sees only an ACK until it consumes the record.
    }
    fn mute_and_release(&self, exact: &Reservation) -> Result<(), String> {
        if exact.operation_id() != self.operation.body().operation_id {
            return Err("Foreign fake release".into());
        }
        self.released.lock().unwrap().push(exact.clone());
        Ok(())
    }
}
fn synthetic(mode: prepared_period::Mode) -> SyntheticContext {
    SyntheticContext {
        server_instance: ID.into(),
        workspace_generation: ID.into(),
        worker_generation: 7,
        settings_revision: 3,
        identity_epoch: ID.into(),
        identity_revision: 4,
        radio_session: ID.into(),
        selection_epoch: ID.into(),
        receive_slice: "A".into(),
        transmit_slice: "A".into(),
        dax_route_epoch: ID.into(),
        dax_device_id: "inert-no-device".into(),
        mode,
        band: "20m".into(),
        frequency_hz: 14_074_000,
        antenna: "ANT1".into(),
        power_policy: SyntheticPowerPolicy {
            epoch: ID.into(),
            radio_milliwatts: 1000,
            ceiling_milliwatts: 1000,
            amplifier_enabled: false,
            digital_allowed: true,
        },
        reservation_epoch: ID.into(),
        clock_epoch: ID.into(),
    }
}
fn owner_limits() -> prepared_period::Limits {
    prepared_period::Limits {
        max_manifest_bytes: 4096,
        max_message_bytes: 37,
        max_frames: 180000,
        max_pcm_bytes: 720000,
        min_offset_hz: 0.0,
        max_offset_hz: 5000.0,
        max_abs_sample: 1.0,
    }
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn corpus(case: &str) -> (Vec<u8>, Vec<u8>, String) {
    assert!(CASES.contains(&case));
    let root = std::env::var("XNEXUS_OFFLINE_CORPUS")
        .expect("Explicit pinned corpus required; never skip");
    let root = Path::new(&root);
    assert!(root.is_absolute());
    let inventory = std::fs::read(root.join("inventory.json")).unwrap();
    assert_eq!(
        sha(&inventory),
        "b77dae43d212848e65ca7d3116af47a361b35b42825c490b8c6b7b6fca9ba481"
    );
    let index: serde_json::Value = serde_json::from_slice(&inventory).unwrap();
    assert_eq!(index["case_count"], 14);
    let entries = index["cases"].as_array().unwrap();
    assert_eq!(entries.len(), 14);
    let item = entries.iter().find(|item| item["case"] == case).unwrap();
    let raw = std::fs::read(root.join(format!("{case}.json"))).unwrap();
    let pcm = std::fs::read(root.join(format!("{case}.f32le"))).unwrap();
    assert_eq!(sha(&raw), item["manifest_sha256"].as_str().unwrap());
    assert_eq!(sha(&pcm), item["pcm_sha256"].as_str().unwrap());
    (raw, pcm, item["body_sha256"].as_str().unwrap().to_owned())
}
struct Fixture {
    adapter: Adapter,
    intent: PreparedIntent,
    operation: Arc<PreparedOperation>,
    nexus: Arc<OfflineOperation>,
    context: SyntheticContext,
    raw: Vec<u8>,
    pcm: Vec<u8>,
}
fn fixture(case: &str) -> Fixture {
    let (raw, pcm, body_sha) = corpus(case);
    let period =
        prepared_period::validate(&raw, pcm.clone().into_boxed_slice(), owner_limits()).unwrap();
    assert_eq!(period.raw_manifest(), raw);
    assert_eq!(period.pcm(), pcm);
    assert_eq!(period.body_sha256(), body_sha);
    let context = synthetic(period.manifest().mode());
    let observed = context.observe(SLOT).unwrap();
    let mode = if case.starts_with("ft8-") {
        ModeKind::Ft8
    } else {
        ModeKind::Ft4
    };
    let suffix = case.split_once('-').unwrap().1;
    let resume = match suffix {
        "cq" | "grid" => None,
        "report" => Some(Msg::parse("K1ABC W9XYZ EN37")),
        "rreport" => Some(Msg::parse("K1ABC W9XYZ -10")),
        "rrr" | "rr73" => Some(Msg::parse("K1ABC W9XYZ R-10")),
        "73" => Some(Msg::parse("K1ABC W9XYZ RR73")),
        _ => panic!("Unknown fixture"),
    };
    let mut adapter = Adapter::new();
    adapter
        .begin(
            Context {
                owner_session: "explicit-fake-owner/session".into(),
                receive_origin: "synthetic-RX-origin-not-TX-authority".into(),
                transmit_context: observed.owner().context.into(),
                clock_epoch: context.clock_epoch.clone(),
                mode,
            },
            OperatorSettings {
                mycall: "K1ABC".into(),
                mygrid: "FN42".into(),
                cq_call_cap: Some(2),
                call_cap: Some(2),
            },
            if suffix == "cq" {
                OperatorIntent::Cq
            } else {
                OperatorIntent::Call {
                    dxcall: "W9XYZ".into(),
                    context: resume.map(|m| (m, -10)),
                    prefer_rrr: suffix == "rrr",
                    skip_tx1: false,
                }
            },
        )
        .unwrap();
    let intent = adapter
        .prepare_slot(
            SLOT,
            SLOT + 100000,
            Format {
                sample_rate: 12000,
                channels: 1,
                encoding: Encoding::F32Le,
            },
            1000.0,
            Limits {
                max_message_bytes: 37,
                max_frames: 180000,
                max_bytes: 720000,
                min_offset_hz: 0.0,
                max_offset_hz: 5000.0,
                max_abs_sample: 1.0,
            },
        )
        .unwrap();
    assert_eq!(intent.period().pcm(), pcm);
    let parsed: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(
        parsed["message"].as_str().unwrap(),
        intent.period().message()
    );
    assert_eq!(
        parsed["content_sha256"].as_str().unwrap(),
        hex(intent.period().content_sha256())
    );
    assert_eq!(
        parsed["pcm_sha256"].as_str().unwrap(),
        hex(intent.period().pcm_sha256())
    );
    let operation =
        Arc::new(PreparedOperation::new(ID.into(), period, &context, SLOT, SLOT + 100000).unwrap());
    let body = operation.body();
    let c = intent.correlation();
    assert_eq!(body.context, c.context.transmit_context);
    assert_eq!(body.clock_epoch, c.context.clock_epoch);
    assert_eq!(body.slot_start_us, c.slot_start_us);
    assert_eq!(body.slot_duration_us, c.slot_duration_us);
    assert_eq!(body.latest_start_us, c.latest_start_us);
    assert_eq!(body.manifest, format!("prepared-period-body-v1:{body_sha}"));
    let raw_body = serde_json::to_vec(body).unwrap(); // ACTUAL Body, no mirror.
    let nexus = Arc::new(
        OfflineOperation::from_offline_fixture(
            &intent,
            ID.into(),
            raw_body,
            operation.period().raw_manifest().to_vec(),
            operation.period().pcm().to_vec(),
        )
        .unwrap(),
    );
    adapter.bind_offline(nexus.clone()).unwrap();
    Fixture {
        adapter,
        intent,
        operation,
        nexus,
        context,
        raw,
        pcm,
    }
}
fn owner(
    operation: Arc<PreparedOperation>,
    context: &SyntheticContext,
) -> (Owner<MemoryJournal>, Arc<FakeSink>) {
    let sink = Arc::new(FakeSink {
        operation,
        observation: context.observe(SLOT).unwrap(),
        started: Mutex::new(Vec::new()),
        released: Mutex::new(Vec::new()),
    });
    let owner = Owner::open(
        Arc::new(Authority::default()),
        MemoryJournal::default(),
        sink.clone(),
    )
    .unwrap();
    (owner, sink)
}
fn observe(f: &Fixture) -> SyntheticObservation {
    f.context.observe(SLOT).unwrap()
}

pub fn replay(case: &str) {
    let mut f = fixture(case);
    let (mut owner, sink) = owner(f.operation.clone(), &f.context);
    assert_eq!(
        owner
            .prepare(f.operation.body().clone(), observe(&f).owner())
            .unwrap(),
        Phase::Prepared
    );
    assert!(sink.started.lock().unwrap().is_empty());
    assert_eq!(
        owner
            .commit(f.operation.body(), observe(&f).owner())
            .unwrap(),
        Phase::Starting
    );
    f.adapter.non_start(&f.nexus, NonStart::Ack).unwrap();
    assert_eq!(f.adapter.view().tx_count, 0);
    assert!(!f.adapter.view().keyed);
    // Duplicate commit cannot produce a second fake effect record.
    assert_eq!(
        owner
            .commit(f.operation.body(), observe(&f).owner())
            .unwrap(),
        Phase::Starting
    );
    let started = sink.started.lock().unwrap();
    assert_eq!(started.len(), 1);
    let record = &started[0];
    assert!(Arc::ptr_eq(&record.operation, &f.operation));
    assert_eq!(
        serde_json::to_vec(record.operation.body()).unwrap(),
        f.nexus.owner_body()
    );
    assert_eq!(record.operation.period().raw_manifest(), f.nexus.manifest());
    assert_eq!(record.operation.period().pcm(), f.nexus.pcm());
    assert_eq!(record.operation.period().raw_manifest(), f.raw);
    assert_eq!(record.operation.period().pcm(), f.pcm);
    // A whitespace-only Body serialization change is not the retained identity.
    let mut changed = f.nexus.owner_body().to_vec();
    changed.push(b' ');
    let wrong = Arc::new(
        OfflineOperation::from_offline_fixture(
            &f.intent,
            ID.into(),
            changed,
            f.raw.clone(),
            f.pcm.clone(),
        )
        .unwrap(),
    );
    assert!(f
        .adapter
        .fake_actual_start(FakeActualStart {
            operation: wrong,
            started_us: SLOT,
            observed_us: SLOT
        })
        .is_err());
    assert_eq!(f.adapter.view().tx_count, 0);
    assert_eq!(
        owner.observe(&record.reservation, false).unwrap(),
        Phase::Active
    );
    // Only this separately consumed EXPLICIT FAKE sink record drives Station.
    f.adapter
        .fake_actual_start(FakeActualStart {
            operation: f.nexus.clone(),
            started_us: record.fake_start_us,
            observed_us: SLOT,
        })
        .unwrap();
    assert_eq!(f.adapter.view().tx_count, 1);
    assert!(f.adapter.view().keyed);
    assert!(f
        .adapter
        .fake_actual_start(FakeActualStart {
            operation: f.nexus.clone(),
            started_us: SLOT,
            observed_us: SLOT
        })
        .is_err());
    assert_eq!(f.adapter.view().tx_count, 1);
    assert_eq!(f.adapter.halt(Halt::Stop).unwrap(), f.nexus);
    owner.release_handle().unwrap().stop().unwrap();
    assert_eq!(
        owner.observe(&record.reservation, true).unwrap(),
        Phase::Completed
    );
    f.adapter.acknowledge_fake_release(&f.nexus).unwrap();
    assert!(f.adapter.release_identity().is_none());
    assert!(!sink.released.lock().unwrap().is_empty());
    assert!(sink
        .released
        .lock()
        .unwrap()
        .iter()
        .all(|r| r == &record.reservation));
    let mut conflict = f.operation.body().clone();
    conflict.manifest.push(' ');
    assert!(owner.prepare(conflict, observe(&f).owner()).is_err());
    let mut replay = f.operation.body().clone();
    replay.operation_id = SECOND_ID.into();
    assert!(owner.prepare(replay, observe(&f).owner()).is_err());
}

pub fn corrupt_pcm_rejected() {
    let (raw, mut pcm, _) = corpus("ft8-cq");
    pcm[0] ^= 1;
    assert!(prepared_period::validate(&raw, pcm.into_boxed_slice(), owner_limits()).is_err());
}
pub fn corrupt_manifest_rejected() {
    let (raw, pcm, _) = corpus("ft4-cq");
    let mut json: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    json["frames"] = 180000.into();
    assert!(prepared_period::validate(
        &serde_json::to_vec(&json).unwrap(),
        pcm.into_boxed_slice(),
        owner_limits()
    )
    .is_err());
}
pub fn whitespace_manifest_changes_body_and_substitution_is_refused() {
    let mut f = fixture("ft8-cq");
    let mut raw = f.raw.clone();
    raw.push(b' ');
    let period =
        prepared_period::validate(&raw, f.pcm.clone().into_boxed_slice(), owner_limits()).unwrap();
    assert_ne!(period.body_sha256(), f.operation.period().body_sha256());
    assert_eq!(period.pcm(), f.operation.period().pcm());
    let changed = Arc::new(
        PreparedOperation::new(ID.into(), period, &f.context, SLOT, SLOT + 100000).unwrap(),
    );
    let (mut owner, sink) = owner(changed, &f.context); // Wrong immutable sink object.
    assert_eq!(
        owner
            .prepare(f.operation.body().clone(), observe(&f).owner())
            .unwrap(),
        Phase::Prepared
    );
    assert!(owner
        .commit(f.operation.body(), observe(&f).owner())
        .is_err());
    assert!(sink.started.lock().unwrap().is_empty());
    assert!(!f.adapter.view().keyed);
    assert_eq!(f.adapter.view().tx_count, 0);
    assert_eq!(
        f.adapter.refuse_or_unknown(&f.nexus, true).unwrap(),
        f.nexus
    );
}
pub fn changed_context_at_fake_effect_refuses_without_advancing() {
    let mut f = fixture("ft4-cq");
    let mut changed = f.context.clone();
    changed.antenna = "ANT2".into();
    let (mut owner, sink) = owner(f.operation.clone(), &changed);
    owner
        .prepare(f.operation.body().clone(), observe(&f).owner())
        .unwrap();
    assert!(owner
        .commit(f.operation.body(), observe(&f).owner())
        .is_err());
    assert!(sink.started.lock().unwrap().is_empty());
    assert_eq!(
        f.adapter.refuse_or_unknown(&f.nexus, false).unwrap(),
        f.nexus
    );
    assert_eq!(f.adapter.view().tx_count, 0);
    assert!(!f.adapter.view().keyed);
}
