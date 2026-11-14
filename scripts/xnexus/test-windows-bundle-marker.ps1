# SPDX-License-Identifier: GPL-3.0-only
$ErrorActionPreference='Stop'
Add-Type -Path (Join-Path $PSScriptRoot 'windows-bundle-marker.cs')
function Bytes([string]$text) { [Text.Encoding]::ASCII.GetBytes($text) }
$original=Bytes 'prefix__TAURI_BUNDLE_TYPE_VAR_UNKsuffix'
$payload=Bytes 'prefix__TAURI_BUNDLE_TYPE_VAR_NSSsuffix'
if([XNexusWindowsBundleMarker]::Verify($original,$payload) -ne 6) { throw 'Wrong marker offset.' }
function Reject([byte[]]$a,[byte[]]$b) {
    $rejected=$false
    try { [void][XNexusWindowsBundleMarker]::Verify($a,$b) } catch { $rejected=$true }
    if(!$rejected) { throw 'Invalid payload was accepted.' }
}
Reject $original (Bytes 'Prefix__TAURI_BUNDLE_TYPE_VAR_NSSsuffix')
Reject $original (Bytes 'prefix__TAURI_BUNDLE_TYPE_VAR_MSIsuffix')
Reject (Bytes 'no-marker') (Bytes 'no-marker')
Reject (Bytes '__TAURI_BUNDLE_TYPE_VAR_UNK__TAURI_BUNDLE_TYPE_VAR_UNK') (Bytes '__TAURI_BUNDLE_TYPE_VAR_NSS__TAURI_BUNDLE_TYPE_VAR_UNK')
Reject $original (Bytes 'prefix__TAURI_BUNDLE_TYPE_VAR_NSSsuffix-extra')
Write-Output 'Bundle marker verification: 6 tests passed.'
