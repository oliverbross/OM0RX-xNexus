param(
    [string]$ExpectedCommit,
    [string]$MsysRoot = 'C:\msys64',
    [switch]$CheckOnly
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
$env:CARGO_BUILD_JOBS = '2'
$env:CMAKE_BUILD_PARALLEL_LEVEL = '2'
$env:RUSTUP_TOOLCHAIN = $toolchain
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
$receiptDir = Join-Path $repo 'target\xnexus-provenance'
New-Item -ItemType Directory -Force -Path $receiptDir | Out-Null
$receiptPath = Join-Path $receiptDir "$($config.version).json"
if (Test-Path $receiptPath) {
    $prior = Get-Content -Raw $receiptPath | ConvertFrom-Json
    if ($prior.source -ne $source) { throw 'This candidate version already belongs to another source. Advance the version first.' }
}
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
$receipt = [ordered]@{
    version = $config.version; source = $source; timestampUtc = [DateTime]::UtcNow.ToString('o')
    upstream = (git -C $repo rev-parse upstream/main).Trim(); toolchain = $toolchain
    profile = 'release'; target = 'x86_64-pc-windows-gnu'; features = 'radio,custom-protocol'
    signed = $false; installed = $false; modelHashes = $models; status = 'started'
}
$receipt | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 $receiptPath
Push-Location "$repo\src-tauri"
try {
    Run { npm --prefix "$repo\ui" ci }
    $cliRoot = Join-Path $repo 'target\xnexus-tools'
    Run { npm install --prefix $cliRoot --no-save --package-lock=false '@tauri-apps/cli@2.11.5' }
    Run { & "$cliRoot\node_modules\.bin\tauri.cmd" build --target x86_64-pc-windows-gnu --features radio,custom-protocol --bundles nsis --no-sign }
    if ((git -C $repo rev-parse HEAD).Trim() -ne $source -or (git -C $repo status --porcelain)) { throw 'Source changed during packaging; reject the artifacts.' }
    $artifacts = Get-ChildItem "$env:CARGO_TARGET_DIR\x86_64-pc-windows-gnu\release\bundle\nsis\*-setup.exe"
    if (!$artifacts) { throw 'No NSIS installer produced.' }
    $receipt.status = 'packaged-unsigned-not-installed'
    $receipt.artifacts = @($artifacts | ForEach-Object { @{path=$_.FullName; sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash; bytes=$_.Length} })
    $receipt | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 $receiptPath
    Write-Output "Build receipt: $receiptPath"
} finally { Pop-Location }
