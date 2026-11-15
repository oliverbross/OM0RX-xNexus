# SPDX-License-Identifier: GPL-3.0-only
# Compiles the real guard helper. Run only in the explicitly released small-test slot.
param([Parameter(Mandatory)][string]$EvidenceRoot,[switch]$RunProcessFixtures)
$ErrorActionPreference='Stop'
if ('Xnexus.Guard.NativeJob' -as [type]) { throw 'Use fresh pwsh: an existing helper type is not source provenance.' }
. (Join-Path $PSScriptRoot 'windows-guard\Guard.ps1')
$root=Join-Path ([IO.Path]::GetFullPath($EvidenceRoot)) ('notifications-'+[guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($root)
$checks=[Collections.Generic.List[string]]::new()
function Assert([bool]$Condition,[string]$Name) {
    if (!$Condition) { throw $Name }; $checks.Add($Name)
}
function JobView($Notifications) {
    [pscustomobject]@{
        MemoryLimitNotification=$Notifications.MemoryLimit
        ProcessMemoryLimitNotification=$Notifications.ProcessMemoryLimit
        JobMemoryLimitNotification=$Notifications.JobMemoryLimit
        AbnormalExitNotification=$Notifications.AbnormalExit
    }
}
try {
    # Literal numeric inputs independently exercise the real DrainNotifications classifier.
    foreach ($message in @(8,9,10)) {
        $n=[Xnexus.Guard.JobNotifications]::new()
        $n.Observe([uint]$message)
        Assert ($n.AbnormalExit -eq ($message -eq 8)) "message $message abnormal classification"
        Assert ($n.ProcessMemoryLimit -eq ($message -eq 9)) "message $message process-memory classification"
        Assert ($n.JobMemoryLimit -eq ($message -eq 10)) "message $message job-memory classification"
        Assert ($n.MemoryLimit -eq ($message -ne 8)) "message $message aggregate memory classification"
        $view=JobView $n
        $expected=if($message -eq 8){'owned-job-abnormal-exit'}else{'owned-job-memory-limit'}
        Assert ((Get-XnexusNotificationReason $view) -eq $expected) "message $message fails with correct reason"
        $receipt=[ordered]@{}
        Update-XnexusNotificationReceipt $receipt $view
        Assert ($receipt.abnormalExitNotificationObserved -eq ($message -eq 8) -and $receipt.memoryLimitNotificationObserved -eq ($message -ne 8)) "message $message receipt separates crash from memory"
        $n.Observe(7)
        Assert ((Get-XnexusNotificationReason (JobView $n)) -eq $expected) "message $message remains sticky after normal exit"
    }
    $n=[Xnexus.Guard.JobNotifications]::new()
    foreach($message in @(0,1,2,3,4,6,7,11,4294967295)){ $n.Observe([uint]$message) }
    Assert (!$n.MemoryLimit -and !$n.AbnormalExit) 'unrelated messages are not misclassified'
    Assert ($null -eq (Get-XnexusNotificationReason (JobView $n))) 'no failure notification permits ordinary exit evaluation'
    foreach($message in @(8,9,10,8)){ $n.Observe([uint]$message) }
    Assert ($n.AbnormalExit -and $n.ProcessMemoryLimit -and $n.JobMemoryLimit) 'multiple failures retain every distinct flag'
    $receipt=[ordered]@{};Update-XnexusNotificationReceipt $receipt (JobView $n)
    Assert ($receipt.abnormalExitNotificationObserved -and $receipt.processMemoryLimitNotificationObserved -and $receipt.jobMemoryLimitNotificationObserved) 'combined receipt does not erase earlier failures'
    Assert ((Get-XnexusNotificationReason (JobView $n)) -eq 'owned-job-memory-limit') 'combined failure remains fail-closed with memory priority'

    if($RunProcessFixtures) {
        # Real admission/floors, no synthetic headroom. ExitProcess with the native
        # access-violation status creates a real abnormal-exit notification without
        # invalid memory access, WER UI, device access or a user profile fixture.
        $pwsh=Join-Path $PSHOME 'pwsh.exe'
        foreach($case in @(@{name='normal';code='0'},@{name='abnormal';code='3221225477'})) {
            $body='Add-Type -TypeDefinition ''using System; using System.Runtime.InteropServices; public static class NotificationChild { [DllImport("kernel32.dll")] public static extern void ExitProcess(uint code); }''; [NotificationChild]::ExitProcess([uint]'+$case.code+')'
            $encoded=[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($body))
            $result=Invoke-XnexusGuardedChild -Executable $pwsh -Arguments "-NoLogo -NoProfile -NonInteractive -EncodedCommand $encoded" -WorkingDirectory $root -EvidenceDirectory (Join-Path $root $case.name) -JobLimitBytes 512MB -MaximumSeconds 20 -PollMilliseconds 25
            Assert ($result.cleanupConfirmed -and $result.finalActiveProcesses -eq 0 -and $result.rootTerminationConfirmed) "$($case.name) exact owned process exited"
            Assert (!$result.memoryLimitNotificationObserved) "$($case.name) was not reported as allocation failure"
            if($case.name -eq 'normal') {
                Assert ($result.status -eq 'passed' -and $result.effectiveExitCode -eq 0 -and !$result.abnormalExitNotificationObserved) 'normal actual child passes'
            } else {
                Assert ($result.status -ne 'passed' -and $result.effectiveExitCode -ne 0) 'actual abnormal child fails closed'
                Assert ($result.processExitCode -eq [uint]3221225477) 'actual native access-violation exit status retained'
                Assert ($result.abnormalExitNotificationObserved -and $result.reason -eq 'owned-job-abnormal-exit') 'actual completion-port message8 is distinct from memory'
            }
        }
    }
    $files=@($PSCommandPath,(Join-Path $PSScriptRoot 'windows-guard\NativeJob.cs'),(Join-Path $PSScriptRoot 'windows-guard\Guard.ps1'))|ForEach-Object{@{path=$_;sha256=(Get-FileHash -LiteralPath $_).Hash}}
    @{status='passed';checks=$checks;processFixtures=[bool]$RunProcessFixtures;files=$files;scope='guard classification only; no application/build/profile/hardware'}|ConvertTo-Json -Depth 6|Set-Content -LiteralPath (Join-Path $root 'tests.json')
    Write-Output "$($checks.Count) notification checks passed: $root"
} catch {
    @{status='failed';checks=$checks;error=$_.Exception.Message;processFixtures=[bool]$RunProcessFixtures}|ConvertTo-Json -Depth 5|Set-Content -LiteralPath (Join-Path $root 'tests.json')
    throw
}
