"""Independent offline byte/geometry checker; no encoder, devices or network."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct


def check(condition, message):
    if not condition:
        raise ValueError(message)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        check(key not in result, f"Duplicate JSON key: {key}")
        result[key] = value
    return result


def sha(data):
    return hashlib.sha256(data).hexdigest()


def fields(value, expected):
    check(isinstance(value, dict) and set(value) == set(expected.split()), "Unexpected fields")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--source-baseline-commit", required=True)
    parser.add_argument("--exporter-sha256", required=True)
    parser.add_argument("--producer-sha256", required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    root = args.corpus
    check(root.is_absolute() and root.is_dir() and not root.is_symlink(), "Invalid corpus root")
    raw_index = (root / "inventory.json").read_bytes()
    index = json.loads(raw_index, object_pairs_hook=unique_object)
    fields(index, "schema_version purpose source_baseline_commit exporter_sha256 producer_sha256 case_count body_digest_domain fixture_limits cases")
    check(index["schema_version"] == 1 and index["purpose"] == "offline_prepared_period_corpus", "Wrong corpus")
    for key in ("source_baseline_commit", "exporter_sha256", "producer_sha256"):
        check(index[key] == getattr(args, key), f"Unbound source provenance: {key}")
    check(index["body_digest_domain"] == "xnexus-prepared-corpus-body-v1", "Wrong body domain")
    check(index["fixture_limits"] == dict(sample_rate_hz=12000, channels=1, offset_hz=1000,
          max_message_bytes=37, max_frames=180000, max_bytes=720000,
          min_offset_hz=0, max_offset_hz=5000, max_abs_sample=1), "Wrong caller limits")
    check(index["case_count"] == 14 and len(index["cases"]) == 14, "Incomplete corpus")
    messages = {"cq": "CQ K1ABC FN42", "grid": "W9XYZ K1ABC FN42",
                "report": "W9XYZ K1ABC -10", "rreport": "W9XYZ K1ABC R-10",
                "rrr": "W9XYZ K1ABC RRR", "rr73": "W9XYZ K1ABC RR73", "73": "W9XYZ K1ABC 73"}
    expected = {f"{mode}-{name}" for mode in ("ft8", "ft4") for name in messages}
    seen, filenames, results = set(), {"inventory.json"}, []
    for item in index["cases"]:
        fields(item, "case manifest pcm manifest_bytes manifest_sha256 pcm_bytes pcm_sha256 content_sha256 body_sha256")
        case = item["case"]
        check(case in expected and case not in seen, "Unknown or duplicate case")
        seen.add(case)
        mode, name = case.split("-", 1)
        check(item["manifest"] == case + ".json" and item["pcm"] == case + ".f32le", "Unsafe filename")
        filenames.update((item["manifest"], item["pcm"]))
        for filename in (item["manifest"], item["pcm"]):
            check(not (root / filename).is_symlink(), "Symlink corpus member")
        raw = (root / item["manifest"]).read_bytes()
        pcm = (root / item["pcm"]).read_bytes()
        check(len(raw) == item["manifest_bytes"] and len(raw) <= 4096 and sha(raw) == item["manifest_sha256"], "Manifest byte identity")
        check(len(pcm) == item["pcm_bytes"] and len(pcm) <= 720000 and sha(pcm) == item["pcm_sha256"], "PCM byte identity")
        m = json.loads(raw, object_pairs_hook=unique_object)
        fields(m, "schema_version mode message_form message format offset_hz_f32_bits frames waveform_frames pcm_byte_count period_duration_us pcm_sha256 content_sha256")
        check(m["schema_version"] == 1 and m["mode"] == mode.upper() and m["message_form"] == "standard_type1" and m["message"] == messages[name], "Prepared intent changed")
        check(m["format"] == {"sample_rate_hz": 12000, "channels": 1, "encoding": "f32le"}, "Unexpected PCM format")
        frames, lead, signal, duration = (180000, 6000, 79 * 1920, 15000000) if mode == "ft8" else (90000, 3600, 105 * 576, 7500000)
        offset_bits = struct.unpack("<I", struct.pack("<f", 1000.0))[0]
        check(m["offset_hz_f32_bits"] == offset_bits, "Wrong offset")
        check(m["frames"] == frames and m["period_duration_us"] == duration and m["pcm_byte_count"] == len(pcm) == frames * 4, "Period geometry")
        check(type(m["waveform_frames"]) is int and lead + signal <= m["waveform_frames"] <= frames, "Truncated waveform")
        samples = [sample[0] for sample in struct.iter_unpack("<f", pcm)]
        check(all(math.isfinite(x) and abs(x) <= 1.0 for x in samples), "Nonfinite/out-of-limit PCM")
        check(all(x == 0.0 for x in samples[:lead]) and any(x != 0.0 for x in samples[lead:m["waveform_frames"]]), "Lead-in or empty waveform")
        check(pcm[m["waveform_frames"] * 4:] == bytes((frames - m["waveform_frames"]) * 4), "Period pad changed")
        message = m["message"].encode("ascii")
        content = (b"xnexus-local-period-v1\0" + bytes([8 if mode == "ft8" else 4])
                   + struct.pack("<IH", 12000, 1) + b"f32le\0" + struct.pack("<I", offset_bits)
                   + struct.pack("<I", len(message)) + message + struct.pack("<Q", len(pcm)) + pcm)
        body = b"xnexus-prepared-corpus-body-v1\0" + struct.pack("<Q", len(raw)) + raw + struct.pack("<Q", len(pcm)) + pcm
        check(m["pcm_sha256"] == sha(pcm) and m["content_sha256"] == item["content_sha256"] == sha(content), "Producer content identity")
        check(item["body_sha256"] == sha(body), "Full manifest/body identity")
        results.append({"case": case, "manifest_sha256": sha(raw), "pcm_sha256": sha(pcm), "content_sha256": sha(content), "body_sha256": sha(body), "frames": frames})
    check(seen == expected and {p.name for p in root.iterdir()} == filenames, "Incomplete or unexpected corpus files")
    receipt = {"status": "passed", "scope": "Independent offline exact bytes, manifest, content/body identities, intent and PCM geometry; no second encoder or owner/hardware acceptance",
               "source_baseline_commit": args.source_baseline_commit, "exporter_sha256": args.exporter_sha256, "producer_sha256": args.producer_sha256,
               "validator_sha256": sha(Path(__file__).read_bytes()), "inventory_sha256": sha(raw_index), "cases": results}
    with args.receipt.open("x", encoding="utf-8") as output:
        json.dump(receipt, output, indent=2)
        output.write("\n")
    print("PASS: 14 actual producer periods independently validated")


if __name__ == "__main__":
    main()
