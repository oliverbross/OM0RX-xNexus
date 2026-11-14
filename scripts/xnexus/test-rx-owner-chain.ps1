param(
    [Parameter(Mandatory=$true)][string]$SdkRoot,
    [Parameter(Mandatory=$true)][string]$SdkCommit,
    [Parameter(Mandatory=$true)][string]$OwnerRoot,
    [Parameter(Mandatory=$true)][string]$OwnerCommit,
    [Parameter(Mandatory=$true)][string]$CoreEvidence
)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$evidence=Join-Path $repo 'target\xnexus-rx-owner-chain'
New-Item -ItemType Directory -Force -Path $evidence | Out-Null
$core=Get-Content (Join-Path $CoreEvidence 'receipt.json') -Raw | ConvertFrom-Json
if($core.status -ne 'passed' -or $core.toolchain -ne '1.93.1-x86_64-pc-windows-gnu') {
    throw 'Require the passed native GNU core receipt.'
}
& git -C $SdkRoot diff --exit-code $SdkCommit -- integrations/xlog/src integrations/xlog/Cargo.toml
if($LASTEXITCODE -ne 0) { throw 'SDK source differs from the requested commit.' }
$ownerFiles=@('prototype/src-tauri/src/companion_bridge.rs','prototype/src-tauri/src/companion_rx_buffer.rs')
& git -C $OwnerRoot diff --exit-code $OwnerCommit -- @ownerFiles
if($LASTEXITCODE -ne 0) { throw 'Owner buffer/HTTP source differs from the requested commit.' }
$inventory=Join-Path $CoreEvidence 'cargo.jsonl'
$artifacts=@(Get-Content $inventory | ForEach-Object { $_ | ConvertFrom-Json })
function Rlib([string]$Name,[string]$Version='',[string]$Feature='') {
    $found=@($artifacts | Where-Object {
        $_.reason -eq 'compiler-artifact' -and $_.target.name -eq $Name -and
        (!$Version -or $_.package_id.EndsWith("@$Version")) -and
        (!$Feature -or $_.features -contains $Feature)
    } | ForEach-Object { $_.filenames | Where-Object { $_ -like '*.rlib' } })
    if($found.Count -ne 1) { throw "Require one recorded artifact: $Name $Version $Feature" }
    return (Resolve-Path -LiteralPath $found[0]).Path
}
$libs=[ordered]@{
    tempo_core=(Rlib 'tempo_core'); tempo_codec=(Rlib 'tempo_codec')
    serde=(Rlib 'serde' '1.0.229' 'rc')
    serde_json=(Rlib 'serde_json' '1.0.154' 'raw_value')
    reqwest=(Rlib 'reqwest' '0.13.5'); base64=(Rlib 'base64' '0.23.1')
    uuid=(Rlib 'uuid' '1.23.2' 'v4')
}
$deps=Split-Path $libs.tempo_core -Parent
$toolchain='1.93.1-x86_64-pc-windows-gnu'
$env:PATH="C:\msys64\ucrt64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
$ownerModules=Join-Path $evidence 'owner-modules.rs'
$bridgePath=(Join-Path $OwnerRoot $ownerFiles[0]) | ConvertTo-Json -Compress
$bufferPath=(Join-Path $OwnerRoot $ownerFiles[1]) | ConvertTo-Json -Compress
@("#[path = $bridgePath] mod companion_bridge;", "#[path = $bufferPath] mod companion_rx_buffer;") |
    Set-Content -LiteralPath $ownerModules -Encoding utf8
$env:XNEXUS_RX_OWNER_MODULES=$ownerModules
$sources=@((Join-Path $repo 'integrations/xlog-rx/src/lib.rs'),
    (Join-Path $repo 'src-tauri/src/xlog_decode.rs'),
    (Join-Path $repo 'integrations/xlog-rx/tests/owner_chain.rs'),
    (Join-Path $SdkRoot 'integrations/xlog/src/lib.rs')) +
    @(Get-ChildItem (Join-Path $SdkRoot 'integrations/xlog/src') -Recurse -Filter '*.rs' | ForEach-Object { $_.FullName }) +
    @($ownerFiles | ForEach-Object { Join-Path $OwnerRoot $_ })
$hashes=@($sources | Sort-Object -Unique | ForEach-Object { @{path=$_; sha256=(Get-FileHash $_ -Algorithm SHA256).Hash} })
$receipt=[ordered]@{
    sourceBase=(git -C $repo rev-parse HEAD).Trim(); sdkSource=$SdkCommit; ownerSource=$OwnerCommit
    coreDependencySource=$core.source; coreReceiptSha256=(Get-FileHash (Join-Path $CoreEvidence 'receipt.json') -Algorithm SHA256).Hash
    coreInventorySha256=(Get-FileHash $inventory -Algorithm SHA256).Hash
    sourceHashes=$hashes; rlibs=@($libs.GetEnumerator() | ForEach-Object { @{name=$_.Key;path=$_.Value;sha256=(Get-FileHash $_.Value -Algorithm SHA256).Hash} })
    toolchain=$toolchain; actualOwnerBuffer=$true; actualOwnerHttp=$true; actualSdk=$true
    pacedCapture=$true; hardwareUsed=$false; applicationLaunched=$false
    status='building'; expectedTests=2; modes=@('FT8','FT4'); sampleRate=48000
}
$receiptPath=Join-Path $evidence 'receipt.json'
function Save { $receipt | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $receiptPath -Encoding utf8 }
Save
try {
    $common=@("+$toolchain",'--edition=2021','-C','opt-level=1','-L',"dependency=$deps")
    $sdkLib=Join-Path $evidence 'libxnexus_xlog_client.rlib'
    & rustc @common --crate-name xnexus_xlog_client --crate-type rlib (Join-Path $SdkRoot 'integrations/xlog/src/lib.rs') `
        --extern "reqwest=$($libs.reqwest)" --extern "serde=$($libs.serde)" --extern "serde_json=$($libs.serde_json)" `
        --extern "base64=$($libs.base64)" -o $sdkLib *> (Join-Path $evidence 'compile-sdk.log')
    if($LASTEXITCODE -ne 0) { throw 'Focused SDK compilation failed; see compile-sdk.log.' }
    $rxLib=Join-Path $evidence 'libxnexus_xlog_rx.rlib'
    & rustc @common --crate-name xnexus_xlog_rx --crate-type rlib (Join-Path $repo 'integrations/xlog-rx/src/lib.rs') `
        --extern "tempo_codec=$($libs.tempo_codec)" -o $rxLib *> (Join-Path $evidence 'compile-assembler.log')
    if($LASTEXITCODE -ne 0) { throw 'Focused assembler compilation failed.' }
    $exe=Join-Path $evidence 'owner-chain-tests.exe'
    & rustc @common --test --crate-name xnexus_rx_owner_chain (Join-Path $repo 'integrations/xlog-rx/tests/owner_chain.rs') `
        --extern "xnexus_xlog_client=$sdkLib" --extern "xnexus_xlog_rx=$rxLib" `
        --extern "tempo_core=$($libs.tempo_core)" --extern "serde=$($libs.serde)" `
        --extern "serde_json=$($libs.serde_json)" --extern "base64=$($libs.base64)" `
        --extern "uuid=$($libs.uuid)" -L 'native=C:\msys64\ucrt64\lib' -o $exe *> (Join-Path $evidence 'compile-chain.log')
    if($LASTEXITCODE -ne 0) { throw 'Focused owner-chain compilation failed; see compile-chain.log.' }
    $receipt.executableSha256=(Get-FileHash $exe -Algorithm SHA256).Hash
    $filters=@('complete_ft8_owner_http_chain','complete_ft4_owner_http_chain')
    & $exe @filters --list *> (Join-Path $evidence 'selected-tests.log')
    if($LASTEXITCODE -ne 0 -or @(Get-Content (Join-Path $evidence 'selected-tests.log') | Where-Object { $_ -match ': test$' }).Count -ne 2) {
        throw 'Expected exactly the two whole-chain tests.'
    }
    $receipt.status='running'; Save
    & $exe @filters --test-threads=1 --nocapture *> (Join-Path $evidence 'tests.log')
    $receipt.exitCode=$LASTEXITCODE
    if($LASTEXITCODE -ne 0) { throw 'Whole-chain tests failed; see tests.log.' }
    if(!(Select-String -Path (Join-Path $evidence 'tests.log') -Pattern '^test result: ok\. 2 passed; 0 failed; 0 ignored;')) {
        throw 'Whole-chain summary did not prove both tests passed.'
    }
    foreach($source in $hashes) {
        if((Get-FileHash $source.path -Algorithm SHA256).Hash -ne $source.sha256) { throw "Source changed: $($source.path)" }
    }
    $receipt.status='passed-focused-whole-chain-not-installed'; Save
    Get-Content (Join-Path $evidence 'tests.log')
    Write-Output "Whole-chain receipt: $receiptPath"
} catch {
    $receipt.status='failed'; $receipt.error=$_.Exception.Message; Save; throw
}
