// SPDX-License-Identifier: GPL-3.0-only
//! WKWebView ignores data_directory. Its persistent custom stores need macOS 14
//! and a stable identifier unique to the canonical companion root. Independent
//! Nexus mode keeps the default store, which this path must never open.
use std::path::Path;

fn require_supported_version(major: isize) -> Result<(), String> {
    if major < 14 {
        return Err(
            "The xLog companion requires macOS 14 or later for isolated draft storage".into(),
        );
    }
    Ok(())
}

pub(crate) fn identifier(root: &Path) -> Result<[u8; 16], String> {
    let root = root
        .canonicalize()
        .map_err(|_| "Cannot resolve the companion profile for isolated draft storage")?;
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    hash.update(b"sk.om0rx.xnexus.companion.webview-store.v1\0");
    hash.update(root.as_os_str().as_encoded_bytes());
    let mut id = [0; 16];
    id.copy_from_slice(&hash.finish().as_ref()[..16]);
    // RFC 9562 custom UUID: version 8, RFC variant. No random or process-local
    // salt: closing/reopening the same root must recover its saved draft.
    id[6] = (id[6] & 0x0f) | 0x80;
    id[8] = (id[8] & 0x3f) | 0x80;
    Ok(id)
}

pub(crate) fn for_current_system(root: &Path) -> Result<[u8; 16], String> {
    let version = objc2_foundation::NSProcessInfo::processInfo().operatingSystemVersion();
    require_supported_version(version.majorVersion)?;
    identifier(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_macos_never_falls_back_to_shared_storage() {
        assert!(require_supported_version(13).is_err());
        assert!(require_supported_version(14).is_ok());
        assert!(require_supported_version(27).is_ok());
    }

    #[test]
    fn canonical_roots_have_stable_distinct_store_identifiers() {
        use std::{fs, os::unix::fs::symlink};
        let root = std::env::temp_dir().join(format!("xnexus-webview-test-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        let alias = root.join("alias");
        symlink(&first, &alias).unwrap();
        assert_eq!(identifier(&first).unwrap(), identifier(&first).unwrap());
        assert_eq!(identifier(&first).unwrap(), identifier(&alias).unwrap());
        assert_ne!(identifier(&first).unwrap(), identifier(&second).unwrap());
        assert!(identifier(&root.join("missing")).is_err());
        assert_eq!(
            for_current_system(&first).unwrap(),
            identifier(&first).unwrap()
        );
        fs::remove_file(alias).unwrap();
        fs::remove_dir(first).unwrap();
        fs::remove_dir(second).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
