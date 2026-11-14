// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Apply storage directly to the window builder before it can create a WebView.
//! Tauri 2.11.2 drops these fields when converting WindowConfig to attributes.

use std::path::PathBuf;

pub(crate) trait Builder: Sized {
    type Output;
    type Error;

    fn data_directory(self, directory: PathBuf) -> Self;
    fn data_store_identifier(self, identifier: [u8; 16]) -> Self;
    fn build(self) -> Result<Self::Output, Self::Error>;
}

pub(crate) fn build<B: Builder>(
    builder: B,
    directory: PathBuf,
    store: Option<[u8; 16]>,
) -> Result<B::Output, B::Error> {
    let builder = builder.data_directory(directory);
    let builder = match store {
        Some(identifier) => builder.data_store_identifier(identifier),
        None => builder,
    };
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct RecordingBuilder {
        directory: Option<PathBuf>,
        store: Option<[u8; 16]>,
        actions: Vec<&'static str>,
        fail: bool,
    }
    impl RecordingBuilder {
        // Reproduce the locked framework's conversion omission. Merely putting
        // private coordinates in configuration must not satisfy these tests.
        fn from_config(_directory: PathBuf, _store: Option<[u8; 16]>) -> Self {
            Self::default()
        }
    }
    impl Builder for RecordingBuilder {
        type Output = Self;
        type Error = &'static str;
        fn data_directory(mut self, directory: PathBuf) -> Self {
            self.directory = Some(directory);
            self.actions.push("directory");
            self
        }
        fn data_store_identifier(mut self, identifier: [u8; 16]) -> Self {
            self.store = Some(identifier);
            self.actions.push("store");
            self
        }
        fn build(mut self) -> Result<Self, Self::Error> {
            self.actions.push("build");
            if self.fail {
                Err("construction failed")
            } else {
                Ok(self)
            }
        }
    }
    #[test]
    fn config_only_positive_control_reproduces_missing_storage() {
        let direct = RecordingBuilder::from_config(PathBuf::from("/private/a"), Some([1; 16]))
            .build()
            .unwrap();
        assert!(direct.directory.is_none());
        assert!(direct.store.is_none());
    }
    #[test]
    fn directory_is_applied_before_native_creation_without_mac_store() {
        let directory = PathBuf::from("/private/a/webview");
        let result = build(
            RecordingBuilder::from_config(directory.clone(), None),
            directory.clone(),
            None,
        )
        .unwrap();
        assert_eq!(result.directory, Some(directory));
        assert_eq!(result.store, None);
        assert_eq!(result.actions, ["directory", "build"]);
    }
    #[test]
    fn mac_profiles_apply_distinct_stores_before_creation() {
        let a = build(
            RecordingBuilder::default(),
            "/private/a/webview".into(),
            Some([1; 16]),
        )
        .unwrap();
        let b = build(
            RecordingBuilder::default(),
            "/private/b/webview".into(),
            Some([2; 16]),
        )
        .unwrap();
        assert_ne!(a.directory, b.directory);
        assert_eq!(a.store, Some([1; 16]));
        assert_eq!(b.store, Some([2; 16]));
        assert_eq!(a.actions, ["directory", "store", "build"]);
        assert_eq!(b.actions, ["directory", "store", "build"]);
    }
    #[test]
    fn failed_creation_is_returned_without_shared_store_fallback() {
        let builder = RecordingBuilder {
            fail: true,
            ..Default::default()
        };
        assert_eq!(
            build(builder, "/private/a/webview".into(), Some([1; 16])).unwrap_err(),
            "construction failed"
        );
    }
}
