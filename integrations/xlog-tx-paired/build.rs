// SPDX-License-Identifier: GPL-3.0-only
// Import separately qualified owner source for acceptance, never application code.
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};

const SOURCES: [(&str, &str); 4] = [
    (
        "digital_tx_owner",
        "a2ea1f912d9ab39971df566845249b6822acd080619c8ee13b0298f961f67a9a",
    ),
    (
        "output_authority",
        "f50218604e690e04f9e2a628d8a5611eff786e28e72b00be4cf0d6a6ff413452",
    ),
    (
        "prepared_period",
        "0627cef6c6366dda4d9fb1e03d2b1606dbe54d14b5eef1cca3fcb34d5509378b",
    ),
    (
        "prepared_period_adapter",
        "e942f7f5e111009f44b239217560ed900a485b4d73d7f69f1e726f7d1cd8f98d",
    ),
];

fn main() {
    println!("cargo:rerun-if-env-changed=XNEXUS_XLOG_SOURCE_ROOT");
    let root = PathBuf::from(
        env::var_os("XNEXUS_XLOG_SOURCE_ROOT")
            .expect("Set XNEXUS_XLOG_SOURCE_ROOT to the separately qualified xLog checkout root"),
    );
    assert!(root.is_absolute(), "Owner root must be absolute");
    let root = root.canonicalize().expect("Owner root must exist");
    assert!(root.is_dir(), "Owner root must be a directory");
    let mut glue = String::from("// Generated exact owner imports; no source copy.\n");
    for (module, expected) in SOURCES {
        let source = root
            .join("prototype/src-tauri/src")
            .join(format!("{module}.rs"));
        let source = source
            .canonicalize()
            .expect("All four actual owner modules are required");
        assert!(source.starts_with(&root), "Owner module escaped checkout");
        assert!(source.is_file(), "Owner module must be a regular file");
        let bytes = fs::read(&source).expect("Read owner module");
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            expected,
            "Owner source changed: {module}; independently qualify and review new pins"
        );
        println!("cargo:rerun-if-changed={}", source.display());
        glue.push_str(&format!(
            "#[path = {:?}] mod {module};\n",
            source.to_str().expect("UTF-8 source path")
        ));
    }
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR required"));
    assert!(
        output.is_absolute() && output.is_dir(),
        "Existing absolute OUT_DIR required"
    );
    fs::write(output.join("owner_modules.rs"), glue).expect("Write generated owner glue");
}
