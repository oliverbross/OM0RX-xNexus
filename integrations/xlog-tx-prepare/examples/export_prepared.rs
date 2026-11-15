// SPDX-License-Identifier: GPL-3.0-only
//! Offline corpus only: actual prepare() bytes, no Engine/device/scheduler/HTTP.
use modes::ModeKind;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use xnexus_tx_prepare::{prepare, Encoding, Format, Limits, PreparedPeriod};

const CASES: [(&str, &str); 7] = [
    ("cq", "CQ K1ABC FN42"),
    ("grid", "W9XYZ K1ABC FN42"),
    ("report", "W9XYZ K1ABC -10"),
    ("rreport", "W9XYZ K1ABC R-10"),
    ("rrr", "W9XYZ K1ABC RRR"),
    ("rr73", "W9XYZ K1ABC RR73"),
    ("73", "W9XYZ K1ABC 73"),
];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// These are fixture-only strings, not an arbitrary JSON serializer. Refuse
/// anything requiring escaping rather than produce ambiguous manifest bytes.
fn literal(value: &str) -> io::Result<String> {
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b'_' | b'-' | b'+' | b'.'))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unsafe fixture JSON literal",
        ));
    }
    Ok(format!("\"{value}\""))
}

fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Independent OFFLINE corpus identity, not a proposed endpoint/journal digest.
/// Unlike PreparedPeriod.content_sha256, it binds every exact JSON byte too.
fn body_digest(manifest: &[u8], pcm: &[u8]) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"xnexus-prepared-corpus-body-v1\0");
    digest.update((manifest.len() as u64).to_le_bytes());
    digest.update(manifest);
    digest.update((pcm.len() as u64).to_le_bytes());
    digest.update(pcm);
    digest.finalize().into()
}

fn manifest(period: &PreparedPeriod) -> io::Result<Vec<u8>> {
    let mode = match period.mode() {
        ModeKind::Ft8 => "FT8",
        ModeKind::Ft4 => "FT4",
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported prepared mode",
            ))
        }
    };
    let format = period.format();
    if format.sample_rate != 12_000 || format.channels != 1 || format.encoding != Encoding::F32Le {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unsupported prepared format",
        ));
    }
    let duration = (period.frames() as u64)
        .checked_mul(1_000_000)
        .filter(|n| n % u64::from(format.sample_rate) == 0)
        .map(|n| n / u64::from(format.sample_rate))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid period geometry"))?;
    Ok(format!(
        concat!(
        "{{\n  \"schema_version\": 1,\n  \"mode\": {},\n",
        "  \"message_form\": \"standard_type1\",\n  \"message\": {},\n",
        "  \"format\": {{\"sample_rate_hz\": {}, \"channels\": {}, \"encoding\": \"f32le\"}},\n",
        "  \"offset_hz_f32_bits\": {},\n  \"frames\": {},\n  \"waveform_frames\": {},\n",
        "  \"pcm_byte_count\": {},\n  \"period_duration_us\": {},\n",
        "  \"pcm_sha256\": {},\n  \"content_sha256\": {}\n}}\n"),
        literal(mode)?,
        literal(period.message())?,
        format.sample_rate,
        format.channels,
        period.offset_hz().to_bits(),
        period.frames(),
        period.waveform_frames(),
        period.pcm().len(),
        duration,
        literal(&hex(period.pcm_sha256()))?,
        literal(&hex(period.content_sha256()))?
    )
    .into_bytes())
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 8
        || args[0] != "--output"
        || args[2] != "--source-baseline-commit"
        || args[4] != "--exporter-sha256"
        || args[6] != "--producer-sha256"
    {
        return Err("Usage: export_prepared --output <new absolute directory> --source-baseline-commit <40 lowercase hex SHA> --exporter-sha256 <64 lowercase hex SHA> --producer-sha256 <64 lowercase hex SHA>".into());
    }
    let output = PathBuf::from(&args[1]);
    let source = args[3].to_str().ok_or("Source commit must be ASCII")?;
    let exporter = args[5].to_str().ok_or("Exporter hash must be ASCII")?;
    let producer = args[7].to_str().ok_or("Producer hash must be ASCII")?;
    if !output.is_absolute()
        || [(source, 40), (exporter, 64), (producer, 64)]
            .iter()
            .any(|(value, size)| {
                value.len() != *size
                    || !value
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
    {
        return Err(
            "Absolute output and exact source baseline/exporter/producer hashes required".into(),
        );
    }
    // The reviewed runner pins this caller-supplied SHA to actual source/tools.
    // Fail if the output exists; never erase/overwrite old or partial evidence.
    fs::create_dir(&output)?;
    let mut inventory = Vec::new();
    for (mode, tag) in [(ModeKind::Ft8, "ft8"), (ModeKind::Ft4, "ft4")] {
        for (name, message) in CASES {
            // Deliberate corpus limits, not owner-approved route/drive defaults.
            let period = prepare(
                mode,
                message,
                Format {
                    sample_rate: 12_000,
                    channels: 1,
                    encoding: Encoding::F32Le,
                },
                1000.0,
                Limits {
                    max_message_bytes: 37,
                    max_frames: 180_000,
                    max_bytes: 720_000,
                    min_offset_hz: 0.0,
                    max_offset_hz: 5000.0,
                    max_abs_sample: 1.0,
                },
            )
            .map_err(|error| format!("Preparation refused {tag}/{name}: {error:?}"))?;
            let json = manifest(&period)?;
            let stem = format!("{tag}-{name}");
            let manifest_name = format!("{stem}.json");
            let pcm_name = format!("{stem}.f32le");
            write_new(&output.join(&manifest_name), &json)?;
            write_new(&output.join(&pcm_name), period.pcm())?;
            inventory.push(format!(
                concat!(
                    "    {{\"case\": {}, \"manifest\": {}, \"pcm\": {}, ",
                    "\"manifest_bytes\": {}, \"manifest_sha256\": {}, \"pcm_bytes\": {}, ",
                    "\"pcm_sha256\": {}, \"content_sha256\": {}, \"body_sha256\": {}}}"
                ),
                literal(&stem)?,
                literal(&manifest_name)?,
                literal(&pcm_name)?,
                json.len(),
                literal(&hex(&Sha256::digest(&json)))?,
                period.pcm().len(),
                literal(&hex(period.pcm_sha256()))?,
                literal(&hex(period.content_sha256()))?,
                literal(&hex(&body_digest(&json, period.pcm())))?
            ));
        }
    }
    let index = format!(concat!("{{\n  \"schema_version\": 1,\n  \"purpose\": \"offline_prepared_period_corpus\",\n",
        "  \"source_baseline_commit\": {},\n  \"exporter_sha256\": {},\n  \"producer_sha256\": {},\n  \"case_count\": {},\n",
        "  \"body_digest_domain\": \"xnexus-prepared-corpus-body-v1\",\n",
        "  \"fixture_limits\": {{\"sample_rate_hz\":12000,\"channels\":1,\"offset_hz\":1000,\"max_message_bytes\":37,\"max_frames\":180000,\"max_bytes\":720000,\"min_offset_hz\":0,\"max_offset_hz\":5000,\"max_abs_sample\":1}},\n",
        "  \"cases\": [\n{}\n  ]\n}}\n"), literal(source)?, literal(exporter)?, literal(producer)?, inventory.len(), inventory.join(",\n"));
    // Publication marker last. On failure retain the incomplete fresh directory;
    // a corpus with no inventory must never be reported as completed.
    write_new(&output.join("inventory.json"), index.as_bytes())?;
    println!(
        "Exported {} actual prepared periods to {}",
        inventory.len(),
        output.display()
    );
    println!(
        "inventory_sha256={}",
        hex(&Sha256::digest(index.as_bytes()))
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_body_identity_binds_all_manifest_bytes_and_pcm() {
        assert_ne!(
            body_digest(b"{\"schema_version\":1}", b"pcm"),
            body_digest(b"{\"schema_version\":2}", b"pcm")
        );
        assert_ne!(body_digest(b"ab", b"c"), body_digest(b"a", b"bc"));
        assert_ne!(body_digest(b"{}", b"pcm"), body_digest(b"{}", b"pcn"));
    }

    #[test]
    fn fixture_literals_cannot_smuggle_json() {
        for text in ["bad\"quote", "bad\\slash", "bad\nline", "é"] {
            assert!(literal(text).is_err());
        }
        assert_eq!(literal("W9XYZ K1ABC R-10").unwrap(), "\"W9XYZ K1ABC R-10\"");
    }
}
