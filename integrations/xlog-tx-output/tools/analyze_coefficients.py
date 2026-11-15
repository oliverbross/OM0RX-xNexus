"""Independent direct-DTFT reference. Does not import the generator or Rust.
Uses centered symmetric cosine response plus a second-derivative interpolation
bound over every measured interval, rather than treating a sampled plot as proof.
Result remains numerical offline qualification, not RF/channel commissioning.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct


def analyze(raw):
    if len(raw) != 513 * 8:
        raise ValueError("Coefficient length")
    c = list(struct.unpack("<513d", raw))
    if not all(math.isfinite(v) for v in c):
        raise ValueError("Non-finite coefficient")
    if raw != b"".join(raw[i:i+8] for i in range(len(raw)-8, -1, -8)):
        raise ValueError("Bit symmetry")
    if abs(math.fsum(c) - 4) > 1e-12:
        raise ValueError("Gain")

    # Centered, gain-normalized response is REAL because of exact symmetry.
    # Independent fsum evaluation, unlike implementation's ascending input sum.
    def response(hz):
        return (c[256] + 2 * math.fsum(c[256+k] * math.cos(2*math.pi*hz*k/48000)
                                       for k in range(1, 257))) / 4

    # Bound |A''(f)| globally in units Hz^-2. Linear interpolation error <= M*h²/8.
    second = 2 * math.fsum(abs(c[256+k]) * (2*math.pi*k/48000)**2
                           for k in range(1, 257)) / 4
    step = 1.0
    interpolation = second * step * step / 8
    # Deliberately conservative numerical margin; cross-platform rigorous
    # transcendental-error certification is a separate future requirement.
    numerical = 1e-11
    pass_values = [response(f) for f in range(0, 5201)]
    stop_values = [response(f) for f in range(6800, 24001)]
    lo = min(pass_values) - interpolation - numerical
    hi = max(pass_values) + interpolation + numerical
    stop = max(abs(v) for v in stop_values) + interpolation + numerical
    ripple_db = max(abs(20*math.log10(lo)), abs(20*math.log10(hi))) if lo > 0 else math.inf
    attenuation_db = -20*math.log10(stop) if stop > 0 else math.inf
    return {"pass": ripple_db <= 0.01 and attenuation_db >= 80,
            "passband_ripple_db_bound": ripple_db,
            "stopband_attenuation_db_bound": attenuation_db,
            "sampled_pass_min": min(pass_values), "sampled_pass_max": max(pass_values),
            "sampled_stop_max": max(abs(v) for v in stop_values),
            "second_derivative_bound": second, "interpolation_bound": interpolation,
            "numerical_allowance": numerical, "grid_hz": step,
            "passband_points": len(pass_values), "stopband_points": len(stop_values)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--coefficients", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    if not args.receipt.is_absolute() or args.receipt.exists():
        raise ValueError("Require new receipt")
    raw = args.coefficients.read_bytes()
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    if hashlib.sha256(raw).hexdigest() != manifest["coefficient_sha256"]:
        raise ValueError("Coefficient digest")
    result = analyze(raw)
    # Positive structural control: unity impulse satisfies gain/symmetry but
    # must fail image rejection. A successful validator that never rejects fails.
    impulse = [0.0] * 513
    impulse[256] = 4.0
    control = analyze(struct.pack("<513d", *impulse))
    if control["pass"]:
        raise AssertionError("Image-rejection negative control did not fail")
    damaged = bytearray(raw)
    damaged[0] ^= 1
    try:
        analyze(bytes(damaged))
    except ValueError:
        asymmetric_rejected = True
    else:
        raise AssertionError("Asymmetry negative control did not fail")
    result.update(coefficient_sha256=hashlib.sha256(raw).hexdigest(),
                  unity_impulse_rejected=not control["pass"],
                  asymmetric_rejected=asymmetric_rejected)
    args.receipt.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result))
    if not result["pass"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
