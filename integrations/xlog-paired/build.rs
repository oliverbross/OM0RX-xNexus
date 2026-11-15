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
    let mut modules = format!(
        "#[path = {:?}] mod companion_bridge;\n",
        source.to_str().unwrap()
    );
    // Current owner bridge delegates input/reply validation to this sibling.
    let needs = source.parent().unwrap().join("companion_needs.rs");
    if needs.exists() {
        let needs = needs.canonicalize().unwrap();
        println!("cargo:rerun-if-changed={}", needs.display());
        modules.push_str(&format!("#[path = {:?}] mod companion_needs;\n", needs.to_str().unwrap()));
    }
    if std::env::var_os("CARGO_FEATURE_OWNER_AUDIO").is_some() {
        let buffer = source
            .parent()
            .unwrap()
            .join("companion_rx_buffer.rs")
            .canonicalize()
            .expect("The owner-audio feature requires the actual xLog receive buffer source");
        println!("cargo:rerun-if-changed={}", buffer.display());
        modules.push_str(&format!(
            "#[path = {:?}] mod companion_rx_buffer;\n",
            buffer.to_str().unwrap()
        ));
    }
    std::fs::write(output.join("owner_transport.rs"), modules).unwrap();
}
