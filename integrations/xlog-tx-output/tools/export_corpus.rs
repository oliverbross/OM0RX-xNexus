// SPDX-License-Identifier: GPL-3.0-only
// Maintained OFFLINE acceptance driver. No owner imports, backend or device.
use modes::ModeKind;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path};
use xnexus_tx_output::{
    fir::CandidateFilter, prepare_candidate, BoundSource, Channel, OutputPolicy,
};
use xnexus_tx_prepare::{prepare, Encoding, Format, Limits};

const INVENTORY: &str = "b77dae43d212848e65ca7d3116af47a361b35b42825c490b8c6b7b6fca9ba481";
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
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hash(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn read(path: &Path, maximum: usize) -> Vec<u8> {
    let info = fs::symlink_metadata(path).expect("Required regular pinned input");
    assert!(info.file_type().is_file() && !info.file_type().is_symlink());
    assert!(info.len() > 0 && info.len() <= maximum as u64);
    let bytes = fs::read(path).expect("Read pinned input");
    assert_eq!(bytes.len() as u64, info.len());
    bytes
}
fn write(path: &Path, bytes: &[u8]) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .expect("New output only");
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    assert_eq!(args.len(), 3, "Explicit corpus and NEW output required");
    let corpus = Path::new(&args[1]);
    let out = Path::new(&args[2]);
    assert!(corpus.is_absolute() && out.is_absolute() && !out.exists());
    let inventory_raw = read(&corpus.join("inventory.json"), 32768);
    assert_eq!(
        digest(&inventory_raw),
        INVENTORY,
        "Exact pinned corpus inventory required"
    );
    let inventory: Value = serde_json::from_slice(&inventory_raw).unwrap();
    let cases = inventory["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 14);
    let filter = CandidateFilter::fixed_candidate().unwrap();
    assert_eq!(
        hash(&filter.sha256()),
        "b3b266e2346da0272cdb1910943a6ba8020d17a56e55a7b0eb0f12cd2c73c764"
    );
    fs::create_dir(out).expect("New output directory with existing parent");
    let mut outputs = Vec::new();
    for (name, record) in CASES.iter().zip(cases) {
        assert_eq!(record["case"].as_str().unwrap(), *name);
        assert_eq!(record["manifest"], format!("{name}.json"));
        assert_eq!(record["pcm"], format!("{name}.f32le"));
        let raw = read(&corpus.join(format!("{name}.json")), 4096);
        let original_pcm = read(&corpus.join(format!("{name}.f32le")), 720000);
        assert_eq!(digest(&raw), record["manifest_sha256"].as_str().unwrap());
        assert_eq!(
            digest(&original_pcm),
            record["pcm_sha256"].as_str().unwrap()
        );
        let descriptor: Value = serde_json::from_slice(&raw).unwrap();
        let mode = match descriptor["mode"].as_str().unwrap() {
            "FT8" => ModeKind::Ft8,
            "FT4" => ModeKind::Ft4,
            _ => panic!("Mode"),
        };
        let offset = f32::from_bits(
            u32::try_from(descriptor["offset_hz_f32_bits"].as_u64().unwrap()).unwrap(),
        );
        let source = prepare(
            mode,
            descriptor["message"].as_str().unwrap(),
            Format {
                sample_rate: 12000,
                channels: 1,
                encoding: Encoding::F32Le,
            },
            offset,
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
        assert_eq!(
            source.pcm(),
            original_pcm,
            "Actual producer changed source bytes: {name}"
        );
        let bound = BoundSource::bind(&source, &raw).unwrap();
        assert_eq!(
            hash(bound.whole_body_sha256()),
            record["body_sha256"].as_str().unwrap()
        );
        let result = prepare_candidate(
            &bound,
            &filter,
            OutputPolicy {
                channels: [
                    Channel::Source {
                        gain_f32_bits: 0.5f32.to_bits(),
                    },
                    Channel::Silence,
                ],
                source_to_both_explicit: false,
                route_policy_identity: "OFFLINE-FIXTURE-CHANNEL0-HALF-CHANNEL1-SILENT".into(),
                max_abs_sample: 1.0,
                max_frames: 720000,
                max_bytes: 5760000,
            },
        )
        .unwrap();
        let frames = if mode == ModeKind::Ft8 {
            720000
        } else {
            360000
        };
        assert_eq!(result.frames(), frames);
        assert_eq!(result.pcm().len(), frames * 8);
        assert_eq!(result.source_pcm(), original_pcm);
        assert_eq!(result.source_raw_manifest(), raw);
        assert!(result
            .pcm()
            .chunks_exact(8)
            .all(|frame| frame[4..] == [0; 4]));
        let (start, end) = result.support();
        assert!(end <= frames);
        assert!(result.pcm()[..start * 8].iter().all(|b| *b == 0));
        assert!(result.pcm()[end * 8..].iter().all(|b| *b == 0));
        let prefix = out.join(name);
        fs::create_dir(&prefix).unwrap();
        write(&prefix.join("source.json"), result.source_raw_manifest());
        write(&prefix.join("source.f32le"), result.source_pcm());
        write(&prefix.join("final.json"), result.raw_manifest());
        write(&prefix.join("final.f32le"), result.pcm());
        outputs.push(json!({"case":name,"source_pcm_sha256":digest(result.source_pcm()),
            "source_raw_sha256":digest(result.source_raw_manifest()),"source_body_sha256":hash(result.source_body_sha256()),
            "final_manifest_sha256":digest(result.raw_manifest()),"final_pcm_sha256":hash(result.pcm_sha256()),
            "final_body_sha256":hash(result.whole_body_sha256()),"frames":frames,"bytes":frames*8,
            "support_start":start,"support_end_exclusive":end,"delay_frames":result.group_delay_frames()}));
        println!(
            "{name}: actual source unchanged; candidate frames={frames}; support=[{start},{end})"
        );
    }
    assert_eq!(outputs.len(), 14);
    write(
        &out.join("export.json"),
        &serde_json::to_vec_pretty(&json!({"status":"passed-source-export-only",
        "cases":outputs,"channel_policy":"explicit source0 gain0.5 / silence1; not commissioned",
        "corpus_inventory_sha256":INVENTORY}))
        .unwrap(),
    );
}
