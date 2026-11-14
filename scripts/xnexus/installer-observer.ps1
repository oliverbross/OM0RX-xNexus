# SPDX-License-Identifier: GPL-3.0-only
# Library only: no entry point and no installation transaction.
. (Join-Path $PSScriptRoot 'installer-acceptance-lib.ps1')
function Invoke-XnProcessObservation {
    param(
        [Parameter(Mandatory)][string]$Executable,
        [Parameter(Mandatory)][string]$ExpectedSha256,
        [Parameter(Mandatory)][string]$Arguments,
        [Parameter(Mandatory)][string]$WorkingDirectory,
        [Parameter(Mandatory)][string]$ReceiptPath,
        [ValidateRange(1,300)][int]$TimeoutSeconds=120
    )
    Assert-XnHash $Executable $ExpectedSha256
    $null=Assert-XnLocalPath $WorkingDirectory
    $null=Assert-XnLocalPath $ReceiptPath -AllowMissing
    if (Test-Path -LiteralPath $ReceiptPath) { throw 'Preserve existing observer receipt.' }
    if (!('Xnexus.InstallerObservation.Observer' -as [type])) { Add-Type -Path (Join-Path $PSScriptRoot 'installer-observer.cs') }
    $receipt=[ordered]@{schema=1;scope='ordinary CreateProcess tree membership only';status='starting';executable=$Executable;sha256=$ExpectedSha256;arguments=$Arguments;cwd=$WorkingDirectory;timeoutSeconds=$TimeoutSeconds;startedUtc=[DateTime]::UtcNow.ToString('o');identities=@();terminatedByObserver=$false}
    function Save-Observation { $receipt | ConvertTo-Json -Depth 9 | Set-Content -LiteralPath $ReceiptPath -Encoding utf8 }
    Save-Observation
    $observer=$null;$known=@{};$handles=@{};$timer=[Diagnostics.Stopwatch]::StartNew()
    function Update-MemberExitCodes {
        foreach ($key in $handles.Keys) {
            $known[$key].exited=$handles[$key].Exited
            $known[$key].exitCode=if ($handles[$key].Exited) { $handles[$key].ExitCode } else { $null }
        }
    }
    try {
        $observer=[Xnexus.InstallerObservation.Observer]::new($Executable,$Arguments,$WorkingDirectory)
        $receipt.rootPid=$observer.Pid;$receipt.actualRootExecutable=$observer.Executable
        $receipt.atomicMembership=$observer.AtomicMembershipVerified
        $receipt.limitFlags=$observer.LimitFlags;$receipt.memoryLimit=$observer.MemoryLimit
        if (!$observer.AtomicMembershipVerified -or $observer.LimitFlags -ne 0 -or $observer.MemoryLimit -ne 0) { throw 'Unexpected observer Job policy; no process will be terminated.' }
        do {
            $members=@($observer.ActivePids())
            foreach ($member in $members) {
                $process=Get-CimInstance Win32_Process -Filter "ProcessId=$member" -OperationTimeoutSec 3 -ErrorAction Stop
                if (!$process) { continue } # A short-lived exit is counted, never invented as verified identity.
                if (!$process.ExecutablePath -or !$process.CommandLine -or !$process.CreationDate) { throw "Live process identity unavailable: $member" }
                $key="$member/$($process.CreationDate.ToUniversalTime().ToString('o'))"
                if (!$known.ContainsKey($key)) {
                    # Retain a query/synchronize handle: PID reuse cannot import
                    # an unrelated process and child exit codes survive PID retirement.
                    $handle=$observer.Capture([uint32]$member)
                    try {
                        if ($handle.Executable -ine $process.ExecutablePath -or [Math]::Abs(($handle.CreatedUtc-$process.CreationDate.ToUniversalTime()).TotalMilliseconds) -ge 1) { throw 'Observed process creation identity changed.' }
                        $known[$key]=[ordered]@{pid=[uint32]$member;parentPid=$process.ParentProcessId;createdUtc=$process.CreationDate.ToUniversalTime().ToString('o');handleCreatedUtc=$handle.CreatedUtc.ToString('o');executable=$process.ExecutablePath;arguments=$process.CommandLine;sha256=(Get-XnHash $process.ExecutablePath -ObservedImage);exited=$false;exitCode=$null}
                        $handles[$key]=$handle;$handle=$null
                    } finally { if ($handle) { $handle.Dispose() } }
                }
            }
            $counts=$observer.Counts
            $receipt.rootExited=$observer.RootExited;$receipt.activeProcesses=$counts.ActiveProcesses;$receipt.totalProcesses=$counts.TotalProcesses
            Update-MemberExitCodes
            $receipt.identities=@($known.Values);$receipt.activePids=@($observer.ActivePids())
            if ($receipt.rootExited) { $receipt.rootExitCode=$observer.RootExitCode }
            Save-Observation
            if ($receipt.rootExited -and $counts.ActiveProcesses -eq 0) {
                # A membership count is authoritative retirement evidence, not complete historical identity.
                $receipt.status=if ($known.Count -eq $counts.TotalProcesses) { 'retired-identities-observed' } else { 'retired-identity-gap' }
                break
            }
            if ($timer.Elapsed.TotalSeconds -ge $TimeoutSeconds) { $receipt.status='timeout-unresolved';break }
            Start-Sleep -Milliseconds 100
        } while ($true)
    } catch {
        $receipt.status='observation-failed';$receipt.error=$_.Exception.ToString()
        throw
    } finally {
        if ($observer) {
            try {
                $receipt.rootExited=$observer.RootExited;$receipt.activeProcesses=$observer.Counts.ActiveProcesses
                $receipt.activePids=@($observer.ActivePids());Update-MemberExitCodes;$receipt.identities=@($known.Values)
            } catch { $receipt.finalSnapshotError=$_.Exception.Message }
            $observer.Dispose() # Close handles only; live descendants are deliberately left to retire normally.
        }
        foreach ($handle in $handles.Values) { $handle.Dispose() }
        $receipt.elapsedSeconds=$timer.Elapsed.TotalSeconds;$receipt.finishedUtc=[DateTime]::UtcNow.ToString('o');Save-Observation
    }
    [pscustomobject]$receipt
}
