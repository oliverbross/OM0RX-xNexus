$ErrorActionPreference = 'Stop'
if (-not ('Xnexus.Guard.NativeJob' -as [type])) {
    Add-Type -Path (Join-Path $PSScriptRoot 'NativeJob.cs')
}

function Assert-XnexusHeadroom([ulong]$Physical, [ulong]$Commit) {
    if ($Physical -lt 8GB -or $Commit -lt 6GB) {
        throw "Admission held: require 8 GiB physical and 6 GiB available Windows commit; observed $Physical / $Commit bytes."
    }
}

function Get-XnexusGuardReason([ulong]$Physical, [ulong]$Commit, [bool]$MemoryLimitNotification) {
    if ($Physical -lt 2GB) { return 'host-physical-below-2GiB' }
    if ($Commit -lt 2GB) { return 'host-commit-below-2GiB' }
    if ($MemoryLimitNotification) { return 'owned-job-memory-limit' }
    return $null
}

function Get-XnexusNotificationReason($Job) {
    if ($Job.MemoryLimitNotification) { return 'owned-job-memory-limit' }
    if ($Job.AbnormalExitNotification) { return 'owned-job-abnormal-exit' }
    return $null
}

function Update-XnexusNotificationReceipt($Receipt, $Job) {
    $Receipt.memoryLimitNotificationObserved = $Job.MemoryLimitNotification
    $Receipt.processMemoryLimitNotificationObserved = $Job.ProcessMemoryLimitNotification
    $Receipt.jobMemoryLimitNotificationObserved = $Job.JobMemoryLimitNotification
    $Receipt.abnormalExitNotificationObserved = $Job.AbnormalExitNotification
}

function ConvertTo-XnexusPsLiteral([string]$Value) { return "'" + $Value.Replace("'", "''") + "'" }

function Copy-XnexusBuildReceipts([string]$Repo, [string]$Version, [string]$Destination) {
    if ($Version -notmatch '^[a-zA-Z0-9][a-zA-Z0-9.+-]{0,99}$') { throw 'Invalid candidate version for receipt preservation.' }
    if (Test-Path $Destination) { throw 'Receipt snapshot already exists; never overwrite prior evidence.' }
    New-Item -ItemType Directory -Path $Destination | Out-Null
    $inventory = @()
    foreach ($suffix in @('.json','.distribution.json','.distribution.config.json')) {
        $source = Join-Path $Repo ('target\xnexus-provenance\' + $Version + $suffix)
        $entry = @{source=$source; exists=(Test-Path -LiteralPath $source -PathType Leaf)}
        if ($entry.exists) {
            $copy = Join-Path $Destination ($Version + $suffix)
            [IO.File]::Copy($source,$copy,$false)
            $entry.sha256 = (Get-FileHash -LiteralPath $copy -Algorithm SHA256).Hash.ToLowerInvariant()
            $entry.bytes = (Get-Item -LiteralPath $copy).Length
        }
        $inventory += $entry
    }
    $inventory | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $Destination 'inventory.json') -Encoding utf8
}

function Invoke-XnexusGuardedChild {
    param(
        [Parameter(Mandatory)][string]$Executable,
        [Parameter(Mandatory)][string]$Arguments,
        [Parameter(Mandatory)][string]$WorkingDirectory,
        [Parameter(Mandatory)][string]$EvidenceDirectory,
        [ulong]$JobLimitBytes = 4GB,
        [scriptblock]$ReadHeadroom = { [Xnexus.Guard.NativeJob]::Headroom() },
        [int]$MaximumSeconds = 43200,
        [int]$PollMilliseconds = 1000
    )
    $receipt = [ordered]@{
        startedUtc = [DateTime]::UtcNow.ToString('o'); status = 'starting'
        executable = $Executable; arguments = $Arguments; workingDirectory = $WorkingDirectory
        jobLimitBytes = $JobLimitBytes; priority = 'BelowNormal'; inheritedHandles = $false
        breakawayAllowed = $false; killOnJobClose = $true; assignedBeforeResume = $false
        minimumPhysicalBytes = [ulong]::MaxValue; minimumCommitBytes = [ulong]::MaxValue
        peakJobBytes = [ulong]0; processExitCode = $null; effectiveExitCode = 125
        memoryLimitNotificationObserved = $false
        processMemoryLimitNotificationObserved = $false; jobMemoryLimitNotificationObserved = $false
        abnormalExitNotificationObserved = $false
        cleanupConfirmed = $false; finalActiveProcesses = $null; rootTerminationConfirmed = $false
    }
    $job = $null
    $launchAttempted = $false
    $receiptPath = Join-Path $EvidenceDirectory 'guard.json'
    $telemetryPath = Join-Path $EvidenceDirectory 'telemetry.jsonl'
    if (Test-Path $receiptPath) { throw 'Evidence receipt already exists; choose a fresh directory.' }
    New-Item -ItemType Directory -Path $EvidenceDirectory -Force | Out-Null
    try {
        $memory = & $ReadHeadroom
        Assert-XnexusHeadroom $memory.AvailablePhysical $memory.AvailableCommit
        $receipt.admission = @{physicalBytes=$memory.AvailablePhysical; commitBytes=$memory.AvailableCommit}
        $receipt | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $receiptPath -Encoding utf8
        $launchAttempted = $true
        $job = [Xnexus.Guard.NativeJob]::new($Executable, $Arguments, $WorkingDirectory, $JobLimitBytes)
        $receipt.pid = $job.Pid; $receipt.assignedBeforeResume = $job.AssignedBeforeResume
        $receipt.verifiedJobLimitBytes = $job.Limit
        $receipt.status = 'running'
        $receipt | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $receiptPath -Encoding utf8
        $clock = [Diagnostics.Stopwatch]::StartNew()
        while ($true) {
            $memory = & $ReadHeadroom
            $job.DrainNotifications()
            Update-XnexusNotificationReceipt $receipt $job
            $receipt.minimumPhysicalBytes = [Math]::Min($receipt.minimumPhysicalBytes, $memory.AvailablePhysical)
            $receipt.minimumCommitBytes = [Math]::Min($receipt.minimumCommitBytes, $memory.AvailableCommit)
            $receipt.peakJobBytes = [Math]::Max($receipt.peakJobBytes, $job.PeakJobBytes)
            $active = $job.ActiveProcesses
            @{utc=[DateTime]::UtcNow.ToString('o');physicalBytes=$memory.AvailablePhysical;commitBytes=$memory.AvailableCommit;peakJobBytes=$job.PeakJobBytes;activeProcesses=$active} |
                ConvertTo-Json -Compress | Add-Content -LiteralPath $telemetryPath -Encoding utf8
            $reason = Get-XnexusGuardReason $memory.AvailablePhysical $memory.AvailableCommit $job.MemoryLimitNotification
            if (!$reason) { $reason = Get-XnexusNotificationReason $job }
            if ($clock.Elapsed.TotalSeconds -ge $MaximumSeconds) { $reason = 'owned-job-timeout' }
            if ($reason) { $receipt.status='interrupted'; $receipt.reason=$reason; $job.Stop(); break }
            if ($job.RootExited) {
                $receipt.processExitCode = $job.ExitCode
                # A successful root may not leave an unobserved child build running.
                if ($active -gt 0) {
                    Start-Sleep -Milliseconds 100
                    if ($job.ActiveProcesses -gt 0) { $receipt.status='failed'; $receipt.reason='root-left-active-descendants'; $job.Stop(); break }
                }
                $job.DrainNotifications()
                Update-XnexusNotificationReceipt $receipt $job
                $notificationReason = Get-XnexusNotificationReason $job
                if ($notificationReason) { $receipt.status='failed'; $receipt.reason=$notificationReason; break }
                $receipt.effectiveExitCode = $receipt.processExitCode
                $receipt.status = if ($receipt.processExitCode -eq 0) { 'passed' } else { 'failed' }
                break
            }
            Start-Sleep -Milliseconds $PollMilliseconds
        }
    } catch {
        $receipt.status='guard-failed'; $receipt.error=$_.Exception.Message
        if ($job) { $job.Stop() }
    } finally {
        if ($job) {
            try {
                # TerminateJobObject is asynchronous. Retain the controlling handle
                # until both the Job accounting and root process signal prove exit.
                $job.Stop()
                $receipt.cleanupConfirmed = $job.WaitForTermination(5000)
                $receipt.finalActiveProcesses = $job.ActiveProcesses
                $receipt.rootTerminationConfirmed = $job.RootExited
                if ($receipt.rootTerminationConfirmed -and $null -eq $receipt.processExitCode) { $receipt.processExitCode = $job.ExitCode }
            } catch {
                $receipt.cleanupConfirmed = $false
                $receipt.cleanupError = $_.Exception.Message
            } finally { $job.Dispose() }
        } elseif (!$launchAttempted) {
            $receipt.cleanupConfirmed = $true
            $receipt.finalActiveProcesses = 0
        }
        if (!$receipt.cleanupConfirmed) {
            $receipt.status = 'cleanup-unconfirmed'
            $receipt.effectiveExitCode = 125
        }
        $receipt.finishedUtc=[DateTime]::UtcNow.ToString('o')
        $receipt | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $receiptPath -Encoding utf8
    }
    return [pscustomobject]$receipt
}
