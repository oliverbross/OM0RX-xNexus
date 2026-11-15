#Requires -Version 7.4
# Process-only acceptance launcher. No UI binding, fallback, termination or cleanup.
# Source contract: xlog_mode::data_root_from accepts --xlog-data-dir=<absolute>;
# xlog_mode::run places the WebView profile beneath <root>\webview.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Executable,
    [Parameter(Mandatory)][ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$ExpectedSha256,
    [Parameter(Mandatory)][ValidatePattern('^[a-f0-9]{40}$')][string]$ExpectedSource,
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+-xlog\.\d+$')][string]$ExpectedVersion,
    [Parameter(Mandatory)][string]$PackageReceipt,
    [Parameter(Mandatory)][string]$OwnerLaunchReceipt,
    [string]$PriorLaunchReceipt,
    [ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$ReviewedPriorLauncherSha256,
    [switch]$ValidateOnly
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Safe-Path([string]$Value, [switch]$AllowMissingLeaf) {
    if ($Value -notmatch '^[A-Za-z]:[\\/]' -or $Value.Substring(2).Contains(':') -or $Value -match '["\x00-\x1f]' -or
        $Value -match '(?:^|[\\/])\.{1,2}(?:[\\/]|$)' -or $Value -match '[. ](?:[\\/]|$)') {
        throw "Refusing ambiguous or non-local absolute path: $Value"
    }
    $full = [IO.Path]::GetFullPath($Value).TrimEnd('\')
    $cursor = $full
    $leaf = $true
    while ($cursor) {
        if (Test-Path -LiteralPath $cursor) {
            $item = Get-Item -LiteralPath $cursor -Force
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Reparse path refused: $cursor" }
        } elseif (!$leaf -or !$AllowMissingLeaf) { throw "Required path missing: $cursor" }
        $leaf = $false
        $parent = [IO.Directory]::GetParent($cursor)
        $cursor = if ($parent) { $parent.FullName } else { $null }
    }
    return $full
}
function Is-Within([string]$Child, [string]$Parent) {
    return $Child.StartsWith($Parent.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)
}
function Read-Json([string]$File) {
    $safe = Safe-Path $File
    if ((Get-Item -LiteralPath $safe).Length -gt 1MB) { throw 'Receipt unexpectedly large.' }
    return Get-Content -LiteralPath $safe -Raw | ConvertFrom-Json
}
function Hash-Equals([string]$File, [string]$Expected) {
    if ($Expected -notmatch '^[A-Fa-f0-9]{64}$' -or
        (Get-FileHash -LiteralPath (Safe-Path $File) -Algorithm SHA256).Hash -ne $Expected) {
        throw "SHA256 mismatch: $File"
    }
}
function Same-Creation($Left, $Right) {
    return ([datetime]$Left).ToUniversalTime().Ticks -eq ([datetime]$Right).ToUniversalTime().Ticks
}
function Prior-Launcher-Matches([string]$Actual,[string]$Current,[string]$Reviewed) {
    $expected=if ($Reviewed) { $Reviewed } else { $Current }
    return $Actual -match '^[A-Fa-f0-9]{64}$' -and $Actual -ieq $expected
}
function Exact-Command([string]$Line, [string]$Exe, [string]$Argument) {
    $pattern = '^\s*(?:"' + [regex]::Escape($Exe) + '"|' + [regex]::Escape($Exe) +
        ')\s+(?:"' + [regex]::Escape($Argument) + '"|' + [regex]::Escape($Argument) + ')\s*$'
    return $Line -cmatch $pattern
}
function Process-Record($Process) {
    return [ordered]@{ pid=[int]$Process.ProcessId; parentPid=[int]$Process.ParentProcessId;
        executable=$Process.ExecutablePath; commandLine=$Process.CommandLine;
        creationUtc=$Process.CreationDate.ToUniversalTime().ToString('o') }
}
function Get-Live([int]$ProcessId) {
    return Get-CimInstance Win32_Process -Filter "ProcessId=$ProcessId" -ErrorAction Stop
}
function Candidate-Record($Live, [string]$Exe, [string]$Argument, [string]$KnownCreation = '') {
    # Only the very first CIM observation may be incomplete. The caller separately
    # verifies that the exact Start-Process handle is still alive while waiting.
    if ($Live -and (($Live.ExecutablePath -and $Live.ExecutablePath -ne $Exe) -or
        ($Live.CommandLine -and !(Exact-Command $Live.CommandLine $Exe $Argument)))) {
        throw 'Live candidate executable/full arguments differ; do not bind.'
    }
    if (!$Live -or !$Live.ExecutablePath -or !$Live.CommandLine -or !$Live.CreationDate) {
        if ($KnownCreation) { throw 'Previously verified candidate metadata disappeared; do not bind.' }
        return $null
    }
    $record = Process-Record $Live
    if ($KnownCreation -and $record.creationUtc -ne $KnownCreation) { throw 'Candidate PID changed identity.' }
    return $record
}
function Verify-Owner($Launch, [string]$Root, [string]$Creation = '') {
    if (Test-Path -LiteralPath (Join-Path $Root 'STOP')) { throw 'Synthetic owner STOP exists; stop this sequence.' }
    $owner = Read-Json (Join-Path $Root 'owner.json')
    $ownerRoot = [string]$owner.root
    if ($ownerRoot.StartsWith('\\?\')) { $ownerRoot = $ownerRoot.Substring(4) }
    if ($owner.synthetic -ne $true -or $owner.pid -ne $Launch.pid -or (Safe-Path $ownerRoot) -ne $Root) {
        throw 'Synthetic owner published coordinates differ.'
    }
    $live = Get-Live ([int]$Launch.pid)
    if (!$live -or !$live.ExecutablePath -or !$live.CommandLine -or !$live.CreationDate -or
        $live.ExecutablePath -ne $Launch.executable -or
        !(Exact-Command $live.CommandLine $Launch.executable "--fixture-root=$Root")) {
        throw 'Synthetic owner exited or its PID/executable/full arguments cannot be verified.'
    }
    $record = Process-Record $live
    if ($Creation -and $record.creationUtc -ne $Creation) { throw 'Synthetic owner PID was replaced.' }
    return $record
}
function WebView-Root([string]$Line) {
    # Chromium accepts --user-data-dir=value; quoted values may contain spaces.
    $coordinates = [regex]::Matches($Line, '(?:^|\s)(?:"--user-data-dir=([^"]+)"|--user-data-dir="([^"]+)"|--user-data-dir=([^\s"]+))(?=\s|$)')
    if ($coordinates.Count -ne 1) { throw 'WebView must expose exactly one user-data-dir coordinate.' }
    $value = @($coordinates[0].Groups | Select-Object -Skip 1 | Where-Object Success)[0].Value
    return Safe-Path $value
}
function Verify-WebViews($Candidate, [string]$Profile, [object[]]$Snapshot, $KnownBrowser = $null) {
    $tree = @{}; $tree[[int]$Candidate.ProcessId] = $Candidate
    do {
        $added = $false
        foreach ($child in $Snapshot) {
            if (!$tree.ContainsKey([int]$child.ProcessId) -and $tree.ContainsKey([int]$child.ParentProcessId)) {
                if (!$child.CreationDate) { throw 'Descendant creation time unavailable.' }
                if ($child.CreationDate -ge $tree[[int]$child.ParentProcessId].CreationDate) {
                    $tree[[int]$child.ProcessId] = $child; $added = $true
                }
            }
        }
    } while ($added)
    $verified = @(); $browsers = @()
    foreach ($observed in @($tree.Values | Where-Object Name -EQ 'msedgewebview2.exe')) {
        # Utilities can terminate normally after the tree snapshot. A live but
        # unidentifiable process remains a hard stop; a vanished utility is skipped.
        $view = Get-Live ([int]$observed.ProcessId)
        if (!$view) { continue }
        if (!$view.ExecutablePath -or !$view.CommandLine -or !$view.CreationDate -or $view.Name -ne 'msedgewebview2.exe' -or
            [IO.Path]::GetFileName($view.ExecutablePath) -ne 'msedgewebview2.exe' -or
            $view.CreationDate -ne $observed.CreationDate -or $view.ParentProcessId -ne $observed.ParentProcessId) {
            throw 'Live WebView coordinates are unknown or replaced.'
        }
        $null = Safe-Path $view.ExecutablePath
        $viewRoot = WebView-Root $view.CommandLine
        $expectedWebRoot = Join-Path $Profile 'webview'
        if ($viewRoot -ne $expectedWebRoot -and !(Is-Within $viewRoot $expectedWebRoot)) { throw 'WebView uses a foreign profile; stop this sequence.' }
        # WebView2 may append EBWebView or other descendants beneath data_directory.
        $entry = Process-Record $view; $entry.userDataDir = $viewRoot
        $entry.role = 'child'
        # Chromium's persistent browser has no --type switch and is a direct
        # child of this candidate. Renderers/GPU/utilities carry --type=... .
        if ($view.ParentProcessId -eq $Candidate.ProcessId -and
            $view.CommandLine -notmatch '(?:^|\s)"?--type(?:=|\s|")') {
            $entry.role = 'browser'; $browsers += $entry
        }
        $verified += $entry
    }
    if ($browsers.Count -gt 1) { throw 'Persistent WebView browser is ambiguous.' }
    $browser = if ($browsers.Count -eq 1) { $browsers[0] } else { $null }
    if ($KnownBrowser -and (!$browser -or $browser.pid -ne $KnownBrowser.pid -or
        $browser.creationUtc -ne $KnownBrowser.creationUtc -or $browser.executable -ne $KnownBrowser.executable -or
        $browser.commandLine -cne $KnownBrowser.commandLine)) { throw 'Persistent WebView browser exited or changed identity.' }
    return [pscustomobject]@{webviews=$verified;browser=$browser}
}
function Private-Fixture([string]$Root) {
    $tempRoot = Safe-Path ([IO.Path]::GetTempPath())
    if ([IO.Path]::GetDirectoryName($Root) -ne $tempRoot -or
        [IO.Path]::GetFileName($Root) -notmatch '^xnexus-ui-([a-f0-9-]{36})$') { throw 'Owner root is not a direct marked TEMP fixture.' }
    $marker = Read-Json (Join-Path $Root '.xnexus-ui-fixture.json')
    if ($marker.schema -ne 1 -or $marker.synthetic -ne $true -or
        [IO.Path]::GetFileName($Root) -ne "xnexus-ui-$($marker.id)") { throw 'Synthetic fixture marker mismatch.' }
    $acl = Get-Acl -LiteralPath $Root
    $user = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $ownerSid = $acl.GetOwner([Security.Principal.SecurityIdentifier]).Value
    if (!$acl.AreAccessRulesProtected -or $ownerSid -ne $user) { throw 'Fixture is not owned and privately protected by the current user.' }
    foreach ($rule in $acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])) {
        if ($rule.AccessControlType -eq 'Allow' -and $rule.IdentityReference.Value -notin @($user,'S-1-5-18','S-1-5-32-544')) {
            throw 'Fixture ACL grants another principal access.'
        }
    }
    return $marker
}

$evidence = $null
$launchRecord = $null
$launcherSha256 = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash
try {
    if (!$IsWindows) { throw 'This launcher is Windows-only.' }
    $exe = Safe-Path $Executable
    $canonical = 'C:\Projects\OM0RX-xNexus'
    $expectedExe = Join-Path $canonical 'target\xnexus-windows\x86_64-pc-windows-gnu\release\OM0RX-xNexus.exe'
    if ($exe -ne $expectedExe) { throw 'Only the exact canonical packaged xNexus executable is accepted.' }
    $packageFile = Safe-Path $PackageReceipt
    if ($packageFile -ne (Join-Path $canonical "target\xnexus-provenance\$ExpectedVersion.json")) { throw 'Package receipt path differs from canonical version receipt.' }
    $package = Read-Json $packageFile
    if ($package.status -ne 'packaged-unsigned-not-installed' -or $package.source -cne $ExpectedSource -or
        $package.version -cne $ExpectedVersion -or $package.executableSha256 -ne $ExpectedSha256 -or
        $package.target -ne 'x86_64-pc-windows-gnu') { throw 'Completed package receipt does not match candidate source/version/hash/target.' }
    Hash-Equals $exe $ExpectedSha256
    if (@($package.artifacts).Count -lt 1) { throw 'Completed package has no recorded installer.' }
    foreach ($artifact in $package.artifacts) {
        $artifactPath = Safe-Path $artifact.path
        if (!(Is-Within $artifactPath (Join-Path ([IO.Path]::GetDirectoryName($exe)) 'bundle\nsis'))) { throw 'Package artifact is outside the canonical installer directory.' }
        Hash-Equals $artifactPath $artifact.sha256
    }
    $ownerFile = Safe-Path $OwnerLaunchReceipt
    $ownerLaunch = Read-Json $ownerFile
    $fixture = Safe-Path $ownerLaunch.root
    if ($ownerFile -ne (Join-Path $fixture 'launch.json') -or $ownerLaunch.synthetic -ne $true -or
        $ownerLaunch.arguments -cne "--fixture-root=$fixture" -or [int]$ownerLaunch.pid -le 0) { throw 'Owner launch receipt is not the verified synthetic coordinate.' }
    $marker = Private-Fixture $fixture
    $ownerExe = Safe-Path $ownerLaunch.executable
    if ([IO.Path]::GetFileName($ownerExe) -cne 'xnexus-ui-owner.exe') { throw 'Only the synthetic owner is accepted.' }
    Hash-Equals $ownerExe $ownerLaunch.sha256
    $ownerBefore = Verify-Owner $ownerLaunch $fixture
    foreach ($name in @('WEBVIEW2_USER_DATA_FOLDER','WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS','WEBVIEW2_BROWSER_EXECUTABLE_FOLDER')) {
        if ([Environment]::GetEnvironmentVariable($name)) { throw "Inherited WebView override refused: $name" }
    }
    $prior = $null
    if ($ReviewedPriorLauncherSha256 -and !$PriorLaunchReceipt) { throw 'Reviewed prior launcher hash requires an exact prior receipt.' }
    if ($PriorLaunchReceipt) {
        $priorFile = Safe-Path $PriorLaunchReceipt
        $priorDirectory = [IO.Path]::GetDirectoryName($priorFile)
        if ([IO.Path]::GetFileName($priorFile) -ne 'receipt.json' -or
            [IO.Path]::GetDirectoryName($priorDirectory) -ne $fixture -or
            [IO.Path]::GetFileName($priorDirectory) -notmatch '^candidate-launch-[a-f0-9-]{36}$') { throw 'Prior receipt is outside this launcher fixture.' }
        $prior = Read-Json $priorFile
        if ($prior.schema -ne 1 -or $prior.status -ne 'verified-process-isolation' -or !(Prior-Launcher-Matches $prior.launcherSha256 $launcherSha256 $ReviewedPriorLauncherSha256) -or
            $prior.source -cne $ExpectedSource -or $prior.version -cne $ExpectedVersion -or
            $prior.executable -ne $exe -or $prior.executableSha256 -ne $ExpectedSha256 -or
            $prior.fixture -ne $fixture -or $prior.fixtureId -ne $marker.id -or
            $prior.owner.pid -ne $ownerBefore.pid -or !(Same-Creation $prior.owner.creationUtc $ownerBefore.creationUtc) -or
            $prior.owner.executable -ne $ownerBefore.executable -or $prior.owner.commandLine -cne $ownerBefore.commandLine) {
            throw 'Prior launch is not the same verified disposable candidate/owner.'
        }
        $profile = Safe-Path $prior.profile
        if ([IO.Path]::GetDirectoryName($profile) -ne $fixture -or [IO.Path]::GetFileName($profile) -notmatch '^app-profile-[a-f0-9-]{36}$') { throw 'Prior profile is not an exact launcher child.' }
        $profileMarker = Read-Json (Join-Path $profile '.xnexus-acceptance-profile.json')
        if ($profileMarker.schema -ne 1 -or $profileMarker.fixtureId -ne $marker.id -or $profileMarker.source -cne $ExpectedSource -or
            $profileMarker.version -cne $ExpectedVersion -or $profileMarker.executableSha256 -ne $ExpectedSha256) { throw 'Prior profile marker differs.' }
        foreach ($processId in @($prior.candidate.pid) + @($prior.webviews | ForEach-Object pid)) {
            if (Get-Live ([int]$processId)) { throw 'Prior candidate/WebView PID is still present; use ordinary Quit and verify exit first.' }
        }
        foreach ($live in Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe' OR Name='OM0RX-xNexus.exe'") {
            if ($live.CommandLine -and $live.CommandLine.Replace('/','\').IndexOf($profile,[StringComparison]::OrdinalIgnoreCase) -ge 0) {
                throw 'A live process still advertises the prior profile; no relaunch.'
            }
        }
    } else {
        $profile = Safe-Path (Join-Path $fixture "app-profile-$([guid]::NewGuid())") -AllowMissingLeaf
        if (Test-Path -LiteralPath $profile) { throw 'Fresh profile unexpectedly exists.' }
    }
    if ($ValidateOnly) {
        [pscustomobject]@{status='validated-no-launch';source=$ExpectedSource;version=$ExpectedVersion;profile=$profile;owner=$ownerBefore}
        return
    }
    # All preflight checks above are read-only. Create only a fresh child/receipt.
    if (!$prior) {
        New-Item -ItemType Directory -Path $profile | Out-Null
        @{schema=1;fixtureId=$marker.id;source=$ExpectedSource;version=$ExpectedVersion;executableSha256=$ExpectedSha256} |
            ConvertTo-Json | Set-Content -LiteralPath (Join-Path $profile '.xnexus-acceptance-profile.json') -Encoding utf8NoBOM
    }
    $evidence = Join-Path $fixture "candidate-launch-$([guid]::NewGuid())"
    New-Item -ItemType Directory -Path $evidence | Out-Null
    $argument = "--xlog-data-dir=$profile"
    $launchRecord = [ordered]@{schema=1;status='launching';launcherSha256=$launcherSha256;source=$ExpectedSource;version=$ExpectedVersion;
        executable=$exe;executableSha256=$ExpectedSha256;packageReceipt=$packageFile;
        packageReceiptSha256=(Get-FileHash -LiteralPath $packageFile -Algorithm SHA256).Hash;
        fixture=$fixture;fixtureId=$marker.id;profile=$profile;arguments=$argument;owner=$ownerBefore;
        priorLaunchReceipt=$PriorLaunchReceipt;priorLauncherSha256=$(if ($prior) { $prior.launcherSha256 } else { $null });reviewedPriorLauncherSha256=$ReviewedPriorLauncherSha256;startedUtc=[DateTime]::UtcNow.ToString('o');webviews=@()}
    $launchRecord | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $evidence 'receipt.json') -Encoding utf8NoBOM
    $null = Verify-Owner $ownerLaunch $fixture $ownerBefore.creationUtc
    Hash-Equals $exe $ExpectedSha256
    $null = Safe-Path $profile
    $process = Start-Process -FilePath $exe -ArgumentList ('"'+$argument+'"') -WorkingDirectory $profile -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput (Join-Path $evidence 'stdout.log') -RedirectStandardError (Join-Path $evidence 'stderr.log')
    $launchRecord.launchedPid = $process.Id
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    $creation = $null
    $verifiedViews = @()
    $browserProof = $null
    while ([DateTime]::UtcNow -lt $deadline) {
        $null = Verify-Owner $ownerLaunch $fixture $ownerBefore.creationUtc
        $process.Refresh()
        if ($process.HasExited) { throw 'Candidate exited; do not bind or retry.' }
        $live = Get-Live $process.Id
        $record = Candidate-Record $live $exe $argument $creation
        if (!$record) {
            $process.Refresh()
            if ($process.HasExited) { throw 'Candidate exited before initial metadata appeared; do not bind or retry.' }
            Start-Sleep -Milliseconds 200
            continue
        }
        $creation = $record.creationUtc
        $launchRecord.candidate = $record
        $hadBrowser = $null -ne $browserProof
        $viewState = Verify-WebViews $live $profile @(Get-CimInstance Win32_Process) $browserProof
        $verifiedViews = @($viewState.webviews)
        $browserProof = $viewState.browser
        $launchRecord.webviews = $verifiedViews
        $launchRecord.browser = $browserProof
        $launchRecord | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $evidence 'receipt.json') -Encoding utf8NoBOM
        if ($hadBrowser -and $browserProof) { break }
        Start-Sleep -Milliseconds 200
    }
    if (!$browserProof) { throw 'No persistent isolated WebView browser appeared before the deadline; do not bind or retry.' }
    $null = Verify-Owner $ownerLaunch $fixture $ownerBefore.creationUtc
    $process.Refresh()
    if ($process.HasExited) { throw 'Candidate exited before final verification.' }
    $last = Get-Live $process.Id
    $launchRecord.candidate = Candidate-Record $last $exe $argument $creation
    # Fresh tree, not the old list: exited utilities are harmless; newly live
    # descendants still require exact private-root proof. Browser identity persists.
    $finalViews = Verify-WebViews $last $profile @(Get-CimInstance Win32_Process) $browserProof
    $launchRecord.webviews = @($finalViews.webviews)
    $launchRecord.browser = $finalViews.browser
    $launchRecord.status = 'verified-process-isolation'
    $launchRecord.verifiedUtc = [DateTime]::UtcNow.ToString('o')
    $launchRecord.receiptPath = Join-Path $evidence 'receipt.json'
    $launchRecord | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $launchRecord.receiptPath -Encoding utf8NoBOM
    [pscustomobject]$launchRecord
} catch {
    if ($evidence -and $launchRecord) {
        $launchRecord.status = 'failed-hard-stop'
        $launchRecord.error = $_.Exception.Message
        $launchRecord | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $evidence 'receipt.json') -Encoding utf8NoBOM
    } else {
        $evidence = Join-Path $PSScriptRoot "preflight-failure-$([guid]::NewGuid()).json"
        @{schema=1;status='failed-before-launch';error=$_.Exception.Message;executable=$Executable;
            source=$ExpectedSource;version=$ExpectedVersion;ownerLaunchReceipt=$OwnerLaunchReceipt;
            launcherSha256=$launcherSha256;timestampUtc=[DateTime]::UtcNow.ToString('o')} |
            ConvertTo-Json | Set-Content -LiteralPath $evidence -Encoding utf8NoBOM
    }
    throw "Acceptance launch stopped: $($_.Exception.Message). Preserve evidence at '$evidence'; no GUI binding, fallback, relaunch or force kill."
}
