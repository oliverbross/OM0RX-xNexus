param([Parameter(Mandatory)][string]$EvidenceRoot)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'windows-guard\Guard.ps1')
$root = Join-Path ([IO.Path]::GetFullPath($EvidenceRoot)) ('fixture-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$pwsh = Join-Path $PSHOME 'pwsh.exe'
$script:checks = 0
function Assert([bool]$Condition,[string]$Message) {
    if (!$Condition) { throw $Message }; $script:checks++
}
function Args([string]$Body) {
    return '-NoLogo -NoProfile -NonInteractive -EncodedCommand ' + [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($Body))
}
function Wait-File([string]$Path) {
    $clock = [Diagnostics.Stopwatch]::StartNew()
    while (!(Test-Path $Path)) {
        if ($clock.Elapsed.TotalSeconds -gt 15) { throw "Fixture did not signal: $Path" }
        Start-Sleep -Milliseconds 25
    }
}
function Wait-Exited([uint]$ChildId) {
    $clock = [Diagnostics.Stopwatch]::StartNew()
    while (Get-Process -Id $ChildId -ErrorAction SilentlyContinue) {
        if ($clock.Elapsed.TotalSeconds -gt 10) { throw "Owned child survived: $ChildId" }
        Start-Sleep -Milliseconds 25
    }
}
$good = { [pscustomobject]@{AvailablePhysical=[ulong]12GB;AvailableCommit=[ulong]10GB} }
try {
    $fakeRepo = Join-Path $root 'receipt-fixture'
    $original = Join-Path $fakeRepo 'target\xnexus-provenance'
    New-Item -ItemType Directory -Path $original -Force | Out-Null
    [IO.File]::WriteAllText((Join-Path $original '1.2.3-xlog.4.json'),'old receipt')
    Copy-XnexusBuildReceipts $fakeRepo '1.2.3-xlog.4' (Join-Path $root 'before')
    [IO.File]::WriteAllText((Join-Path $original '1.2.3-xlog.4.json'),'new receipt')
    [IO.File]::WriteAllText((Join-Path $original '1.2.3-xlog.4.distribution.json'),'new distribution')
    Copy-XnexusBuildReceipts $fakeRepo '1.2.3-xlog.4' (Join-Path $root 'after')
    Assert ((Get-Content -LiteralPath (Join-Path $root 'before\1.2.3-xlog.4.json') -Raw) -eq 'old receipt') 'Previous build receipt was overwritten.'
    Assert ((Get-Content -LiteralPath (Join-Path $root 'after\1.2.3-xlog.4.json') -Raw) -eq 'new receipt') 'Final build receipt missing.'
    $duplicateRejected = $false
    try { Copy-XnexusBuildReceipts $fakeRepo '1.2.3-xlog.4' (Join-Path $root 'before') } catch { $duplicateRejected=$true }
    Assert $duplicateRejected 'Existing evidence snapshot was replaceable.'
    Assert-XnexusHeadroom 8GB 6GB
    $rejects = 0
    foreach ($values in @(@((8GB-1),6GB),@(8GB,(6GB-1)))) {
        try { Assert-XnexusHeadroom $values[0] $values[1] } catch { $rejects++ }
    }
    Assert ($rejects -eq 2) 'Admission must enforce both exact boundaries.'
    Assert ((Get-XnexusGuardReason 2GB 2GB $false) -eq $null) 'Exact running floor is permitted.'
    Assert ((Get-XnexusGuardReason (2GB-1) 12GB $false) -eq 'host-physical-below-2GiB') 'Physical guard missing.'
    Assert ((Get-XnexusGuardReason 12GB (2GB-1) $false) -eq 'host-commit-below-2GiB') 'Commit guard missing.'
    Assert ((Get-XnexusGuardReason 12GB 12GB $true) -eq 'owned-job-memory-limit') 'Memory notification must fail.'

    $marker = Join-Path $root 'normal.json'
    $body = '@{pid=$PID; priority=[Diagnostics.Process]::GetCurrentProcess().PriorityClass.ToString()} | ConvertTo-Json | Set-Content -LiteralPath ' + (ConvertTo-XnexusPsLiteral $marker)
    $normal = Invoke-XnexusGuardedChild -Executable $pwsh -Arguments (Args $body) -WorkingDirectory $root -EvidenceDirectory (Join-Path $root 'normal') -JobLimitBytes 512MB -ReadHeadroom $good -PollMilliseconds 50 -MaximumSeconds 20
    Assert ($normal.status -eq 'passed') "Normal child failed: $($normal | ConvertTo-Json -Compress)"
    Assert $normal.assignedBeforeResume 'Child was not proven assigned before execution.'
    $observed = Get-Content -Raw -LiteralPath $marker | ConvertFrom-Json
    Assert ($observed.priority -eq 'BelowNormal') 'Job priority did not reach child.'
    Assert ($observed.pid -eq $normal.pid) 'Receipt does not identify executing child.'

    $absent = Join-Path $root 'must-not-run'
    $held = Invoke-XnexusGuardedChild -Executable $pwsh -Arguments (Args ('Set-Content -LiteralPath ' + (ConvertTo-XnexusPsLiteral $absent) + ' -Value bad')) -WorkingDirectory $root -EvidenceDirectory (Join-Path $root 'admission') -ReadHeadroom { [pscustomobject]@{AvailablePhysical=[ulong]12GB;AvailableCommit=[ulong]5GB} }
    Assert ($held.status -eq 'guard-failed' -and !(Test-Path $absent) -and !$held.pid) 'Rejected admission launched child.'

    foreach ($kind in @('physical','commit')) {
        $counter = @{ calls=0 }
        $memory = { $counter.calls++; if ($counter.calls -eq 1) { & $good } elseif ($kind -eq 'physical') { [pscustomobject]@{AvailablePhysical=[ulong]1GB;AvailableCommit=[ulong]12GB} } else { [pscustomobject]@{AvailablePhysical=[ulong]12GB;AvailableCommit=[ulong]1GB} } }.GetNewClosure()
        $stopped = Invoke-XnexusGuardedChild -Executable $pwsh -Arguments (Args 'Start-Sleep -Seconds 30') -WorkingDirectory $root -EvidenceDirectory (Join-Path $root $kind) -JobLimitBytes 512MB -ReadHeadroom $memory -PollMilliseconds 50
        Assert ($stopped.status -eq 'interrupted' -and $stopped.effectiveExitCode -ne 0) "$kind pressure was reported as success."
        Wait-Exited $stopped.pid
    }

    $other = $null; $family = $null
    try {
        $other = [Xnexus.Guard.NativeJob]::new($pwsh,(Args 'Start-Sleep -Seconds 30'),$root,[ulong]512MB)
        $childFile = Join-Path $root 'descendant.pid'
        $nestedArgs = Args ('Set-Content -LiteralPath ' + (ConvertTo-XnexusPsLiteral $childFile) + ' -Value $PID; Start-Sleep -Seconds 30')
        $familyBody = 'Start-Process -FilePath ' + (ConvertTo-XnexusPsLiteral $pwsh) + ' -ArgumentList ' + (ConvertTo-XnexusPsLiteral $nestedArgs) + ' -WindowStyle Hidden; Start-Sleep -Seconds 30'
        $family = [Xnexus.Guard.NativeJob]::new($pwsh,(Args $familyBody),$root,[ulong]512MB)
        Wait-File $childFile
        $descendant = [uint](Get-Content -LiteralPath $childFile)
        Assert ($family.Contains($descendant)) 'Descendant escaped owned Job.'
        Assert (!$family.Contains($other.Pid)) 'Unrelated positive control entered Job.'
        $family.Dispose(); $family=$null
        Wait-Exited $descendant
        Assert (!$other.RootExited) 'Owned cleanup affected unrelated positive control.'
    } finally { if ($family) { $family.Dispose() }; if ($other) { $other.Dispose() } }

    $closePidFile = Join-Path $root 'kill-on-close.pid'
    $nativeSource = Join-Path $PSScriptRoot 'windows-guard\NativeJob.cs'
    $closeBody = 'Add-Type -Path ' + (ConvertTo-XnexusPsLiteral $nativeSource) +
        '; $owned=[Xnexus.Guard.NativeJob]::new(' + (ConvertTo-XnexusPsLiteral $pwsh) + ',' +
        (ConvertTo-XnexusPsLiteral (Args 'Start-Sleep -Seconds 30')) + ',' + (ConvertTo-XnexusPsLiteral $root) +
        ',[ulong]512MB); Set-Content -LiteralPath ' + (ConvertTo-XnexusPsLiteral $closePidFile) + ' -Value $owned.Pid; exit 0'
    # Supervisor exits without Dispose: only the OS handle-close policy can clean its child.
    $closed = Invoke-XnexusGuardedChild -Executable $pwsh -Arguments (Args $closeBody) -WorkingDirectory $root -EvidenceDirectory (Join-Path $root 'kill-on-close') -JobLimitBytes 512MB -ReadHeadroom $good -PollMilliseconds 50 -MaximumSeconds 20
    Assert ($closed.status -eq 'passed') 'Supervisor handle-close cleanup failed.'
    Wait-Exited ([uint](Get-Content -LiteralPath $closePidFile))
    Assert $true 'Kill-on-close descendant exit verified.'

    $allocationMarker = Join-Path $root 'allocation.txt'
    $allocate = 'try { $bytes = [byte[]]::new(512MB); Set-Content -LiteralPath ' + (ConvertTo-XnexusPsLiteral $allocationMarker) + " -Value unexpected; exit 0 } catch { Set-Content -LiteralPath " + (ConvertTo-XnexusPsLiteral $allocationMarker) + ' -Value denied; exit 17 }'
    $limited = Invoke-XnexusGuardedChild -Executable $pwsh -Arguments (Args $allocate) -WorkingDirectory $root -EvidenceDirectory (Join-Path $root 'memory-limit') -JobLimitBytes 256MB -ReadHeadroom $good -PollMilliseconds 25 -MaximumSeconds 20
    Assert ($limited.effectiveExitCode -ne 0 -and $limited.status -ne 'passed') 'Memory-limited allocation was reported successful.'
    if (Test-Path $allocationMarker) { Assert ((Get-Content -LiteralPath $allocationMarker) -ne 'unexpected') 'Allocation exceeded hard Job limit.' }

    # Each allocation fits alone; the parent plus its descendant must share one cap.
    $aloneBody = '$bytes=[byte[]]::new(256MB); [GC]::KeepAlive($bytes); exit 0'
    $alone = Invoke-XnexusGuardedChild -Executable $pwsh -Arguments (Args $aloneBody) -WorkingDirectory $root -EvidenceDirectory (Join-Path $root 'allocation-alone') -JobLimitBytes 512MB -ReadHeadroom $good -PollMilliseconds 25 -MaximumSeconds 20
    Assert ($alone.status -eq 'passed') 'Aggregate fixture allocation does not fit by itself.'
    $aggregateChild = Args 'try { $bytes=[byte[]]::new(256MB); [GC]::KeepAlive($bytes); exit 0 } catch { exit 17 }'
    $aggregateBody = '$bytes=[byte[]]::new(256MB); $child=Start-Process -FilePath ' + (ConvertTo-XnexusPsLiteral $pwsh) +
        ' -ArgumentList ' + (ConvertTo-XnexusPsLiteral $aggregateChild) + ' -WindowStyle Hidden -Wait -PassThru; [GC]::KeepAlive($bytes); exit $child.ExitCode'
    $aggregate = Invoke-XnexusGuardedChild -Executable $pwsh -Arguments (Args $aggregateBody) -WorkingDirectory $root -EvidenceDirectory (Join-Path $root 'aggregate-limit') -JobLimitBytes 512MB -ReadHeadroom $good -PollMilliseconds 25 -MaximumSeconds 20
    Assert ($aggregate.status -ne 'passed' -and $aggregate.effectiveExitCode -ne 0) 'Descendants did not share aggregate cap.'

    $timeout = Invoke-XnexusGuardedChild -Executable $pwsh -Arguments (Args 'Start-Sleep -Seconds 30') -WorkingDirectory $root -EvidenceDirectory (Join-Path $root 'timeout') -JobLimitBytes 512MB -ReadHeadroom $good -MaximumSeconds 1 -PollMilliseconds 25
    Assert ($timeout.status -eq 'interrupted' -and $timeout.reason -eq 'owned-job-timeout') 'Timeout did not preserve failure.'
    Wait-Exited $timeout.pid
    $files = @($PSCommandPath,(Join-Path $PSScriptRoot 'run-windows-guarded.ps1'),(Join-Path $PSScriptRoot 'windows-guard\Guard.ps1'),$nativeSource) |
        ForEach-Object { @{path=$_;sha256=(Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant()} }
    @{passed=$script:checks;root=$root;scope='disposable guard fixtures, no package';files=$files} | ConvertTo-Json -Depth 5 | Tee-Object -FilePath (Join-Path $root 'tests.json')
} catch {
    @{passed=$script:checks;root=$root;error=$_.Exception.Message} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root 'tests.json')
    throw
}
