// SPDX-License-Identifier: GPL-3.0-only
//! Acceptance-only generator, compiled with the recorded tempo_core rlib.
//! Writes one mono48k s16le 7.5-second period; no device or application access.
use std::io::Write;
use tempo_core::modes::{self, ModeKind};
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 1, "one fresh absolute waveform output required");
    let path = std::path::Path::new(&args[0]);
    assert!(path.is_absolute());
    let modem = modes::make_mode(ModeKind::Ft4);
    let tones = modem.encode("CQ K1ABC FN42");
    assert!(!tones.is_empty());
    let wave = modem.gen_wave(&tones, 12_000.0, 1500.0);
    assert!(!wave.is_empty() && wave.len() < 90_000);
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();
    for index in 0..360_000_u64 {
        // Same independent interpolation used by the passed owner_chain fixture.
        let numerator = index * 12_000;
        let whole = (numerator / 48_000) as usize;
        let fraction = (numerator % 48_000) as f32 / 48_000.0;
        let sample = if whole + 1 < wave.len() {
            wave[whole] * (1.0 - fraction) + wave[whole + 1] * fraction
        } else { 0.0 };
        let sample = (sample * 1000.0).round().clamp(-32768.0, 32767.0) as i16;
        file.write_all(&sample.to_le_bytes()).unwrap();
    }
    file.sync_all().unwrap();
    println!("Synthetic CQ K1ABC FN42 FT4: 48000 Hz, 360000 mono frames, 720000 bytes");
}
