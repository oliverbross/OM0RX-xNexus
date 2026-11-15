"""Offline candidate generator; does not certify the produced filter.
No dependencies, network, Rust build, runtime use, overwrite or retry.
The resulting coefficient bytes must be independently analyzed and reviewed.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct

ID = "kaiser-513-fc6000-beta10-x4-candidate1"


def i0(x):
    total = term = 1.0
    for k in range(1, 100):
        term *= (x * x / 4) / (k * k)
        total += term
        if term <= total * 1e-17:
            return total
    raise ValueError("Bessel series did not converge")


def coefficients():
    # Evaluate each symmetric pair only once, so bit symmetry is explicit.
    values = [0.0] * 513
    denom = i0(10.0)
    for n in range(257):
        k = n - 256
        ideal = 0.25 if k == 0 else math.sin(math.pi * k / 4) / (math.pi * k)
        window = i0(10 * math.sqrt(max(0.0, 1 - (k / 256) ** 2))) / denom
        values[n] = values[512 - n] = ideal * window
    scale = 4 / math.fsum(values)
    return [v * scale for v in values]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not args.output.is_absolute() or args.output.exists():
        raise ValueError("Require a new absolute output directory")
    args.output.mkdir(parents=False, exist_ok=False)
    values = coefficients()
    raw = b"".join(struct.pack("<d", v) for v in values)
    digest = hashlib.sha256(raw).hexdigest()
    (args.output / "coefficients.f64le").write_bytes(raw)
    bits = [struct.unpack("<Q", struct.pack("<d", v))[0] for v in values]
    source = "// GENERATED OFFLINE CANDIDATE; qualification required.\n"
    source += "pub const COEFFICIENT_BITS: [u64; 513] = [\n"
    source += "".join(f"    0x{bit:016x},\n" for bit in bits) + "];\n"
    source += "pub const COEFFICIENT_SHA256: [u8; 32] = [" + ",".join(str(b) for b in bytes.fromhex(digest)) + "];\n"
    (args.output / "coefficients.rs").write_text(source, encoding="utf-8", newline="\n")
    manifest = {"schema": 1, "candidate_id": ID, "sample_rate": 48000,
                "factor": 4, "taps": 513, "cutoff_hz": 6000, "beta": 10,
                "delay_frames": 256, "coefficient_sha256": digest,
                "coefficient_bytes": len(raw), "qualified": False}
    (args.output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(manifest))


if __name__ == "__main__":
    main()
