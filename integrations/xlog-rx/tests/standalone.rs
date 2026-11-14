// Fast std-only harness for the actual assembler and Nexus's actual FIR source.
// This avoids building unused Opus/Tauri/device dependencies for pure assembly tests.
extern crate self as tempo_codec;
#[path = "../../../crates/tempo-codec/src/capture_resample.rs"]
pub mod capture_resample;
#[path = "../src/lib.rs"]
mod receiver;
