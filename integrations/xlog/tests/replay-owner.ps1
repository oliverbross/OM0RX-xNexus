# Historical synthetic Swift DTO replay. Invoke within the approved resource Job.
param(
    [Parameter(Mandatory)][string]$EvidenceRoot,
    [Parameter(Mandatory)][ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ExpectedManifestSha256,
    [Parameter(Mandatory)][ValidateRange(1,256)][int]$ExpectedCases,
    [string]$CargoToolchain = '1.93.1-x86_64-pc-windows-gnu'
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $EvidenceRoot).Path
$manifestPath = Join-Path $root 'replies-manifest.json'
if ((Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash -ne $ExpectedManifestSha256) { throw 'Manifest identity mismatch.' }
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if (!$manifest.nativeWireBytesUnmodified -or @($manifest.files).Count -ne 2 * $ExpectedCases) { throw 'Missing or unexpected export cases.' }
$provenance = Join-Path $root 'swift-provenance.json'
$objects = Join-Path $root 'linked-object-hashes.json'
if ((Get-Content -LiteralPath $provenance -Raw | ConvertFrom-Json).sourceHead -ne $manifest.sourceHead) { throw 'Swift source provenance mismatch.' }
$proof = @($manifestPath, $provenance, $objects) | ForEach-Object {
    [pscustomobject]@{ path = $_; sha256 = (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash }
}
$names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($file in $manifest.files) {
    if ($file.file -notmatch '^[a-fA-F0-9-]{36}\.(request|response)\.json$' -or !$names.Add($file.file)) { throw 'Invalid or duplicate export filename.' }
    $path = Join-Path (Join-Path $root 'replies') $file.file
    if ((Get-Item -LiteralPath $path).Length -ne $file.bytes -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $file.sha256) { throw 'Export file content differs from manifest.' }
    $proof += [pscustomobject]@{ path = $path; sha256 = $file.sha256 }
}
$actual = @(Get-ChildItem -LiteralPath (Join-Path $root 'replies') -File)
if ($actual.Count -ne $names.Count -or @($actual | Where-Object { !$names.Contains($_.Name) }).Count) { throw 'Unlisted or missing export files.' }
$oldRoot = $env:XN_OWNER_REPLAY_ROOT
$oldCases = $env:XN_OWNER_REPLAY_CASES
try {
    $env:XN_OWNER_REPLAY_ROOT = $root
    $env:XN_OWNER_REPLAY_CASES = [string]$ExpectedCases
    $sdkManifest = Join-Path (Split-Path $PSScriptRoot -Parent) 'Cargo.toml'
    $cargoArgs = @("+$CargoToolchain", 'test', '--locked', '--offline', '--manifest-path', $sdkManifest, '--lib', '-j1', 'needs::tests::owner_replay::actual_swift_export_contract_replay', '--', '--ignored', '--exact', '--nocapture', '--test-threads=1')
    & cargo @cargoArgs 2>&1 | Tee-Object -Variable replayOutput
    if ($LASTEXITCODE -ne 0) { throw "SDK replay failed with exit $LASTEXITCODE." }
    if (($replayOutput -join "`n") -notmatch 'test result: ok\. 1 passed; 0 failed; 0 ignored;') { throw 'Exact replay test did not execute once.' }
} finally {
    $env:XN_OWNER_REPLAY_ROOT = $oldRoot
    $env:XN_OWNER_REPLAY_CASES = $oldCases
    foreach ($file in $proof) {
        if ((Get-FileHash -LiteralPath $file.path -Algorithm SHA256).Hash -ne $file.sha256) { throw 'Export/provenance changed during replay.' }
    }
}
[pscustomobject]@{ kind = 'historical-synthetic-swift-contract-replay'; sourceHead = $manifest.sourceHead; cases = $ExpectedCases; manifestSha256 = $ExpectedManifestSha256; sourceProof = $proof[0..2]; immutableSourceFiles = $proof.Count; liveBridgeQualified = $false; installedUiQualified = $false } | ConvertTo-Json -Depth 5
