param(
    [Parameter(Mandatory=$true)][string]$XLogPath,
    [string]$CargoTargetDirectory
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$owner = (Resolve-Path -LiteralPath $XLogPath).Path
$source = Join-Path $owner 'prototype/src-tauri/src/companion_bridge.rs'
if (!(Test-Path -LiteralPath $source -PathType Leaf)) { throw 'xLog companion bridge source is missing.' }
foreach ($checkout in @($repo, $owner)) {
    $commit = & git -C $checkout rev-parse HEAD
    if ($LASTEXITCODE -ne 0) { throw "Cannot establish source commit: $checkout" }
    $dirty = & git -C $checkout status --porcelain
    if ($LASTEXITCODE -ne 0) { throw "Cannot establish source status: $checkout" }
    Write-Output ("Source: {0} at {1}; dirty={2}" -f $checkout, $commit, [bool]$dirty)
}
Write-Output ("Owner transport SHA-256: {0}" -f (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash)
$priorSource = $env:XNEXUS_XLOG_SOURCE
$priorTarget = $env:CARGO_TARGET_DIR
try {
    $env:XNEXUS_XLOG_SOURCE = $source
    if ($CargoTargetDirectory) { $env:CARGO_TARGET_DIR = $CargoTargetDirectory }
    & cargo test --locked --manifest-path (Join-Path $repo 'integrations/xlog-paired/Cargo.toml') -j 1
    if ($LASTEXITCODE -ne 0) { throw "Paired implementation checks failed ($LASTEXITCODE)." }
} finally {
    $env:XNEXUS_XLOG_SOURCE = $priorSource
    $env:CARGO_TARGET_DIR = $priorTarget
}
