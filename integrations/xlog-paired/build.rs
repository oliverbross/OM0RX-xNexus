// SPDX-License-Identifier: GPL-3.0-only
// Compile the separately checked-out owner's transport for acceptance only.
// It is never copied into the xNexus application or shipped in its installer.
fn main() {
    println!("cargo:rerun-if-env-changed=XNEXUS_XLOG_SOURCE");
    let source = std::path::PathBuf::from(
        std::env::var_os("XNEXUS_XLOG_SOURCE")
            .expect("Set XNEXUS_XLOG_SOURCE to xLog's companion_bridge.rs"),
    );
    let source = source
        .canonicalize()
        .expect("xLog bridge source must exist");
    assert_eq!(source.file_name().unwrap(), "companion_bridge.rs");
    println!("cargo:rerun-if-changed={}", source.display());
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(
        output.join("owner_transport.rs"),
        format!(
            "#[path = {:?}] mod companion_bridge;\n",
            source.to_str().unwrap()
        ),
    )
    .unwrap();
}
