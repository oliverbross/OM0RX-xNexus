// SPDX-License-Identifier: GPL-3.0-only
// Tauri CLI 2.11.5 patches the first marker for NSIS, then restores the standalone file:
// https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.5/crates/tauri-bundler/src/bundle.rs
using System;
using System.IO;
using System.Text;

public static class XNexusWindowsBundleMarker
{
    public static long VerifyFiles(string standalone, string payload)
    {
        return Verify(File.ReadAllBytes(standalone), File.ReadAllBytes(payload));
    }

    public static long Verify(byte[] standalone, byte[] payload)
    {
        byte[] original = Encoding.ASCII.GetBytes("__TAURI_BUNDLE_TYPE_VAR_UNK");
        byte[] bundled = Encoding.ASCII.GetBytes("__TAURI_BUNDLE_TYPE_VAR_NSS");
        if (standalone.Length != payload.Length)
            throw new InvalidDataException("Standalone and payload lengths differ.");
        int offset = -1;
        for (int i = 0; i <= standalone.Length - original.Length; ++i)
        {
            if (standalone[i] != original[0]) continue;
            bool matches = true;
            for (int j = 1; j < original.Length; ++j)
                if (standalone[i + j] != original[j]) { matches = false; break; }
            if (!matches) continue;
            if (offset >= 0) throw new InvalidDataException("Multiple original bundle markers.");
            offset = i;
        }
        if (offset < 0) throw new InvalidDataException("Original bundle marker is absent.");
        for (int i = 0; i < standalone.Length; ++i)
        {
            byte expected = i >= offset && i < offset + bundled.Length
                ? bundled[i - offset] : standalone[i];
            if (payload[i] != expected)
                throw new InvalidDataException("Unexpected payload difference at byte " + i + ".");
        }
        return offset;
    }
}
