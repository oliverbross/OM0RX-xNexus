param(
    [Parameter(Mandatory = $true)][string]$ExpectedCommit,
    [string]$MsysRoot = 'C:\msys64'
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$source = (git -C $repo rev-parse HEAD).Trim()
if ($source -ne $ExpectedCommit -or (git -C $repo status --porcelain)) {
    throw 'Native tests require the exact clean ExpectedCommit.'
}
$toolchain = '1.93.1-x86_64-pc-windows-gnu'
$env:PATH = "$MsysRoot\ucrt64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:RUSTUP_TOOLCHAIN = $toolchain
$env:CARGO_TARGET_DIR = Join-Path $repo 'target\xnexus-windows'
$env:VPX_MINGW_PREFIX = Join-Path $repo 'target\xnexus-vpx-1.17.0'
$env:CMAKE_GENERATOR = 'Ninja'
$env:CARGO_BUILD_JOBS = '2'
$env:CMAKE_BUILD_PARALLEL_LEVEL = '2'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
$mt = Get-ChildItem -LiteralPath $sdkRoot -Directory |
    Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
    Sort-Object { [version]$_.Name } -Descending |
    ForEach-Object { Join-Path $_.FullName 'x64\mt.exe' } |
    Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (!$mt) { throw 'Windows SDK x64 mt.exe is required for the native test harness.' }
$evidence = Join-Path $repo "target\xnexus-native-tests\$source"
New-Item -ItemType Directory -Force -Path $evidence | Out-Null
$manifest = Join-Path $PSScriptRoot 'native-test.manifest'
$filters = @('fork_self_update_is_disabled_for_every_package',
    'profile_dir_name_default_matches_the_shared_root', 'sstv_scope_test::')
$receipt = [ordered]@{
    source = $source; timestampUtc = [DateTime]::UtcNow.ToString('o')
    toolchain = $toolchain; testDebugSymbols = 0; filters = $filters
    manifest = $manifest; manifestSha256 = (Get-FileHash $manifest -Algorithm SHA256).Hash
    mt = $mt; mtSha256 = (Get-FileHash $mt -Algorithm SHA256).Hash
    status = 'building'; applicationLaunched = $false
}
$receiptPath = Join-Path $evidence 'receipt.json'
function SaveReceipt { $receipt | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 $receiptPath }
SaveReceipt
try {
    $buildOutput = Join-Path $evidence 'cargo.jsonl'
    $buildErrors = Join-Path $evidence 'cargo-stderr.log'
    & cargo "+$toolchain" test --locked --manifest-path "$repo\src-tauri\Cargo.toml" --lib --features radio --no-run --message-format=json 1> $buildOutput 2> $buildErrors
    if ($LASTEXITCODE -ne 0) { throw "Native test compilation failed; see $buildErrors" }
    $artifacts = @(Get-Content -LiteralPath $buildOutput | ForEach-Object {
        $item = $_ | ConvertFrom-Json
        if ($item.reason -eq 'compiler-artifact' -and $item.profile.test -and
            $item.target.name -eq 'tempo_lib' -and $item.executable) { $item.executable }
    })
    if ($artifacts.Count -ne 1) { throw 'Expected exactly one generated tempo_lib test executable.' }
    $original = (Resolve-Path -LiteralPath $artifacts[0]).Path
    $targetRoot = [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR) + [IO.Path]::DirectorySeparatorChar
    if (!$original.StartsWith($targetRoot, [StringComparison]::OrdinalIgnoreCase) -or
        [IO.Path]::GetFileName($original) -notlike 'tempo_lib-*.exe') {
        throw 'Refusing to prepare anything except the generated native lib-test executable.'
    }
    if ((git -C $repo rev-parse HEAD).Trim() -ne $source -or (git -C $repo status --porcelain)) {
        throw 'Source changed during native test compilation.'
    }
    # Tauri emits its Windows resources with rustc-link-arg-bins. Cargo lib-test
    # harnesses therefore lack Common-Controls v6 and cannot import TaskDialogIndirect.
    # Prepare a COPY of that harness only; never modify Cargo output or an app binary.
    $harness = Join-Path $evidence 'xnexus-native-tests.exe'
    $receipt.originalExecutable = $original
    $receipt.originalSha256 = (Get-FileHash $original -Algorithm SHA256).Hash
    Copy-Item -LiteralPath $original -Destination $harness
    & $mt -nologo -manifest $manifest "-outputresource:$harness;#1"
    if ($LASTEXITCODE -ne 0) { throw 'Failed to embed Common-Controls v6 into the test harness copy.' }
    $loader = Join-Path (Split-Path (Split-Path $original -Parent) -Parent) 'WebView2Loader.dll'
    if (!(Test-Path -LiteralPath $loader)) { throw 'Generated test output has no WebView2Loader.dll.' }
    Copy-Item -LiteralPath $loader -Destination (Join-Path $evidence 'WebView2Loader.dll')
    $receipt.harness = $harness
    $receipt.harnessSha256 = (Get-FileHash $harness -Algorithm SHA256).Hash
    $receipt.webViewLoaderSha256 = (Get-FileHash $loader -Algorithm SHA256).Hash
    $listingPath = Join-Path $evidence 'selected-tests.log'
    & $harness @filters --list *> $listingPath
    if ($LASTEXITCODE -ne 0) { throw 'The prepared native test harness could not enumerate tests.' }
    $selected = @(Get-Content -LiteralPath $listingPath |
        Where-Object { $_ -match ': test$' } | ForEach-Object { $_ -replace ': test$', '' })
    if ($selected.Count -eq 0) { throw 'No native tests selected; refuse a vacuous success.' }
    foreach ($filter in $filters) {
        if (!($selected | Where-Object { $_.IndexOf($filter, [StringComparison]::Ordinal) -ge 0 })) {
            throw "Native test filter matched nothing: $filter"
        }
    }
    $receipt.selectedTests = $selected
    $receipt.selectedTestCount = $selected.Count
    $receipt.status = 'running'
    SaveReceipt
    Push-Location "$repo\src-tauri"
    try {
        & $harness @filters --test-threads=1 *> (Join-Path $evidence 'tests.log')
        $result = $LASTEXITCODE
    } finally { Pop-Location }
    $receipt.exitCode = $result
    if ($result -ne 0) { throw "Native tests failed with exit code $result; see $evidence\tests.log" }
    $summary = Get-Content -LiteralPath (Join-Path $evidence 'tests.log') |
        Select-String '^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;'
    if (@($summary).Count -ne 1 -or
        [int]$summary.Matches[0].Groups[1].Value -ne $selected.Count -or
        [int]$summary.Matches[0].Groups[2].Value -ne 0 -or
        [int]$summary.Matches[0].Groups[3].Value -ne 0) {
        throw 'Native test summary did not confirm every selected test passed.'
    }
    if ((git -C $repo rev-parse HEAD).Trim() -ne $source -or (git -C $repo status --porcelain)) {
        throw 'Source changed during native test execution.'
    }
    $receipt.status = 'passed'
    SaveReceipt
    Get-Content -LiteralPath (Join-Path $evidence 'tests.log')
    Write-Output "Native test receipt: $receiptPath"
} catch {
    $receipt.status = 'failed'
    $receipt.error = $_.Exception.Message
    SaveReceipt
    throw
}
