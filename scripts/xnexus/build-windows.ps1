param(
    [string]$ExpectedCommit,
    [string]$MsysRoot = 'C:\msys64',
    [switch]$CheckOnly,
    [ValidateRange(1,2)][int]$Jobs = 1
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
function Run([scriptblock]$Command) {
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "Command failed with exit code $LASTEXITCODE" }
}
$toolchain = '1.93.1-x86_64-pc-windows-gnu'
$env:PATH = "$MsysRoot\ucrt64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:CARGO_TARGET_DIR = Join-Path $repo 'target\xnexus-windows'
$env:VPX_MINGW_PREFIX = Join-Path $repo 'target\xnexus-vpx-1.17.0'
$env:CMAKE_GENERATOR = 'Ninja'
$env:CARGO_BUILD_JOBS = $Jobs.ToString()
$env:CMAKE_BUILD_PARALLEL_LEVEL = $Jobs.ToString()
$env:RUSTUP_TOOLCHAIN = $toolchain
# The complete UI typecheck exceeds smaller V8 heaps. Pin the measured frontend
# allowance while the outer Job still bounds the entire package process tree.
$env:NODE_OPTIONS = '--max-old-space-size=2560'
$source = (git -C $repo rev-parse HEAD).Trim()
$config = Get-Content -Raw (Join-Path $repo 'src-tauri\tauri.conf.json') | ConvertFrom-Json
Run { node (Join-Path $repo 'scripts\xnexus\fork-policy.test.mjs') }
if ($CheckOnly) {
    Run { cargo "+$toolchain" check --locked --manifest-path "$repo\src-tauri\Cargo.toml" --features radio --all-targets }
    return
}
if ($repo -ne 'C:\Projects\OM0RX-xNexus') { throw 'Package only from the canonical C:\Projects\OM0RX-xNexus checkout.' }
if (!$ExpectedCommit -or $ExpectedCommit -ne $source) { throw 'ExpectedCommit must be the exact reviewed clean HEAD.' }
if (git -C $repo status --porcelain) { throw 'Packaging requires a clean tracked/untracked source tree.' }
if ($env:NEXUS_ALLOW_MISSING_AICW) { throw 'Do not bypass the release model gate.' }
if ($env:TAURI_CONFIG) { throw 'Remove unreviewed inherited TAURI_CONFIG before packaging.' }
$receiptDir = Join-Path $repo 'target\xnexus-provenance'
New-Item -ItemType Directory -Force -Path $receiptDir | Out-Null
$receiptPath = Join-Path $receiptDir "$($config.version).json"
if (Test-Path $receiptPath) {
    $prior = Get-Content -Raw $receiptPath | ConvertFrom-Json
    if ($prior.source -ne $source) { throw 'This candidate version already belongs to another source. Advance the version first.' }
}
Run { node (Join-Path $repo 'scripts\xnexus\build-nsis-template.mjs') --check }
Run { node (Join-Path $repo 'scripts\xnexus\nsis-policy.test.mjs') }
Run { node (Join-Path $repo 'scripts\xnexus\distribution-preflight.mjs') $receiptDir }
$distributionPath = Join-Path $receiptDir "$($config.version).distribution.json"
$distribution = Get-Content -LiteralPath $distributionPath -Raw | ConvertFrom-Json
if ($distribution.version -ne $config.version) { throw 'Distribution preflight version mismatch.' }
function CheckDistribution {
    if ((Get-FileHash -LiteralPath $distribution.configPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $distribution.configSha256) {
        throw 'The exact materialized distribution config changed.'
    }
    foreach ($entry in $distribution.sourceHashes.PSObject.Properties) {
        if ((Get-FileHash -LiteralPath (Join-Path $repo $entry.Name) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry.Value) {
            throw "Distribution source changed: $($entry.Name)"
        }
    }
}
CheckDistribution
$modelDir = Join-Path $repo 'src-tauri\resources\deepcw'
$models = @{
    'model.onnx' = 'a7b3b4aa2610d3e88b8c7d4420eceee31be2d0ae595b3413264334f3ffb1eab5'
    'model.onnx.json' = 'b4342157b90229ee7380e165f3d8036179b80d1e6ae02f2557388b7fcd558c01'
}
foreach ($name in $models.Keys) {
    $file = Join-Path $modelDir $name
    if (!(Test-Path $file)) {
        Invoke-WebRequest -Uri "https://github.com/kd9taw/Nexus/releases/download/deepcw-model-v1/$name" -OutFile $file
    }
    if ((Get-FileHash $file -Algorithm SHA256).Hash.ToLowerInvariant() -ne $models[$name]) { throw "DeepCW hash mismatch: $name" }
}
$upstream = Get-Content -Raw "$repo\docs\xlog\UPSTREAM.json" | ConvertFrom-Json
Run { git -C $repo merge-base --is-ancestor $upstream.commit HEAD }
$receipt = [ordered]@{
    version = $config.version; source = $source; timestampUtc = [DateTime]::UtcNow.ToString('o')
    upstreamBase = $upstream.commit; upstreamVersion = $upstream.upstream_version
    availableUpstream = (git -C $repo rev-parse upstream/main).Trim(); toolchain = $toolchain
    profile = 'release'; target = 'x86_64-pc-windows-gnu'; features = 'radio,custom-protocol'
    signed = $false; installed = $false; modelHashes = $models; status = 'started'
    cargoBuildJobs = $Jobs; cmakeBuildParallelLevel = $Jobs
    distribution = $distribution; distributionReceiptSha256 = (Get-FileHash -LiteralPath $distributionPath -Algorithm SHA256).Hash
    rustc = (& rustc "+$toolchain" -Vv) -join "`n"
    gfortran = (& gfortran --version | Select-Object -First 1)
    node = (& node --version); npm = (& npm --version); nodeOptions = $env:NODE_OPTIONS
    cargoLockSha256 = (Get-FileHash "$repo\src-tauri\Cargo.lock" -Algorithm SHA256).Hash
    libvpxArchiveSha256 = (Get-FileHash "$env:VPX_MINGW_PREFIX\lib\libvpx.a" -Algorithm SHA256).Hash
    libvpxSourceSha256 = (Get-Content -Raw "$env:VPX_MINGW_PREFIX\.xnexus-sha256").Trim()
    hamlibHashes = @(Get-ChildItem "$repo\src-tauri\resources\hamlib" -File |
        Where-Object { $_.Extension -in '.exe', '.dll' } |
        ForEach-Object { @{name=$_.Name; sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash} })
}
$receipt | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 $receiptPath
Push-Location "$repo\src-tauri"
try {
    Run { npm --prefix "$repo\ui" ci }
    $cliRoot = Join-Path $repo 'target\xnexus-tools'
    Run { npm install --prefix $cliRoot --no-save --package-lock=false '@tauri-apps/cli@2.11.5' }
    CheckDistribution
    Run { & "$cliRoot\node_modules\.bin\tauri.cmd" build --config $distribution.configPath --target x86_64-pc-windows-gnu --features radio,custom-protocol --bundles nsis --no-sign }
    $executable = "$env:CARGO_TARGET_DIR\x86_64-pc-windows-gnu\release\OM0RX-xNexus.exe"
    $imports = & "$MsysRoot\ucrt64\bin\objdump.exe" -p $executable
    if ($LASTEXITCODE -ne 0) { throw 'Unable to audit executable DLL imports.' }
    if ($imports -match 'DLL Name:.*(libgfortran|libstdc\+\+|libquadmath|libwinpthread|libgcc|libfftw)') {
        throw 'Executable still depends on builder-only GNU runtime DLLs; reject installer.'
    }
    if ((git -C $repo rev-parse HEAD).Trim() -ne $source -or (git -C $repo status --porcelain)) { throw 'Source changed during packaging; reject the artifacts.' }
    CheckDistribution
    $artifacts = Get-ChildItem "$env:CARGO_TARGET_DIR\x86_64-pc-windows-gnu\release\bundle\nsis\*_$($config.version)_*-setup.exe"
    if (!$artifacts) { throw 'No NSIS installer produced.' }
    $receipt.status = 'packaged-unsigned-not-installed'
    $receipt.executableSha256 = (Get-FileHash $executable -Algorithm SHA256).Hash
    $receipt.executableImports = @($imports | Select-String 'DLL Name:' | ForEach-Object { $_.Line.Trim() })
    $receipt.generatedNsisSha256 = (Get-FileHash "$env:CARGO_TARGET_DIR\x86_64-pc-windows-gnu\release\nsis\x64\installer.nsi" -Algorithm SHA256).Hash
    $receipt.artifacts = @($artifacts | ForEach-Object { @{path=$_.FullName; sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash; bytes=$_.Length} })
    $receipt | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 $receiptPath
    Write-Output "Build receipt: $receiptPath"
} finally { Pop-Location }
