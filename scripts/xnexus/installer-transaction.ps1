# SPDX-License-Identifier: GPL-3.0-only
# Orchestration only. The executable entry point supplies fixed real adapters;
# tests supply inert filesystem/process/registry interfaces, never production data.
. (Join-Path $PSScriptRoot 'installer-acceptance-lib.ps1')

function ConvertTo-XnCanonicalJson($Value) {
    function Normalize($Item) {
        if ($null -eq $Item) { return $null }
        if ($Item -is [Collections.IDictionary]) {
            $ordered=[ordered]@{}; foreach ($key in @($Item.Keys) | Sort-Object) { $ordered[$key]=Normalize $Item[$key] };return $ordered
        }
        if ($Item -is [pscustomobject]) {
            $ordered=[ordered]@{}; foreach ($key in @($Item.PSObject.Properties.Name) | Sort-Object) { $ordered[$key]=Normalize $Item.$key };return $ordered
        }
        if ($Item -is [Collections.IEnumerable] -and $Item -isnot [string]) { return ,@($Item | ForEach-Object { Normalize $_ }) }
        return $Item
    }
    Normalize $Value | ConvertTo-Json -Depth 30 -Compress
}
function Assert-XnSame($Before,$After,[string]$Label) {
    if ((ConvertTo-XnCanonicalJson $Before) -cne (ConvertTo-XnCanonicalJson $After)) { throw "Changed $Label; preserve state and stop." }
}
function Assert-XnObservedRetirement($Observation,[string]$ExpectedHash) {
    if ($Observation.status -cne 'retired-identities-observed' -or !$Observation.atomicMembership -or
        $Observation.limitFlags -ne 0 -or $Observation.memoryLimit -ne 0 -or !$Observation.rootExited -or
        $Observation.rootExitCode -ne 0 -or $Observation.activeProcesses -ne 0 -or
        $Observation.terminatedByObserver -or @($Observation.identities).Count -ne $Observation.totalProcesses -or
        $Observation.totalProcesses -lt 1) { throw 'Process tree did not retire successfully with complete identities; no cleanup will be attempted.' }
    foreach ($identity in $Observation.identities) {
        if ($identity.sha256 -ine $ExpectedHash -or !$identity.executable -or !$identity.arguments -or
            !$identity.exited -or $null -eq $identity.exitCode -or $identity.exitCode -ne 0) {
            throw 'Unexpected helper or missing process identity; preserve partial installation.'
        }
    }
}
function Assert-XnOwnedRegistry($Rows,[string]$Root,[string]$Version) {
    if (@($Rows).Count -ne 12) { throw 'Incomplete installed registry inventory.' }
    $seen=@{};$required=@{}
    foreach ($row in $Rows) {
        $id="$($row.hive)/$($row.view)/$($row.kind)"
        if ($seen.ContainsKey($id) -or $row.hive -notin 'CurrentUser','LocalMachine' -or $row.view -notin 'Registry32','Registry64' -or $row.kind -notin 'product','uninstall','run') { throw 'Ambiguous installed registration.' }
        $seen[$id]=$true
        if ($row.hive -eq 'LocalMachine' -or $row.kind -eq 'run') {
            if ($row.present) { throw 'Unexpected machine/autorun registration.' };continue
        }
        # HKCU keys can be shared between registry views. Any present copy must
        # identify the exact owned root, and the x64 install view must be present.
        if ($row.view -eq 'Registry64' -and !$row.present) { throw 'Missing installed x64 registration.' }
        if (!$row.present) { continue }
        if ($row.kind -eq 'product') {
            if ($row.values.'' -cne $Root) { throw 'Foreign product registry root.' }
        } else {
            if ($row.values.DisplayVersion -cne $Version -or $row.values.InstallLocation -cne ('"'+$Root+'"') -or
                $row.values.UninstallString -cne ('"'+$Root+'\uninstall.exe"')) { throw 'Foreign uninstall registration.' }
        }
    }
}
function Assert-XnInstallation($Installation,$Plan,$ExtraFiles=@()) {
    Assert-XnOwnershipMarker $Installation.markerText $Plan.target
    $expected=@{}
    foreach ($entry in $Plan.artifacts.payload) { $expected[$entry.destination]=$entry.sha256 }
    $expected['.om0rx-xnexus-owner.ini']=$Installation.markerSha256
    if ($Installation.uninstallerSha256 -notmatch '^[0-9a-fA-F]{64}$' -or !$Installation.uninstallerPe) { throw 'Generated uninstaller identity is missing.' }
    $expected['uninstall.exe']=$Installation.uninstallerSha256
    foreach ($entry in $ExtraFiles) {
        if ($entry.relativeToApp) {
            if ($expected.ContainsKey($entry.relativeToApp)) { throw 'Sentinel collides with payload.' }
            $expected[$entry.relativeToApp]=$entry.sha256
        }
    }
    if (@($Installation.files).Count -ne $expected.Count) { throw 'Installed payload has missing or unexpected files.' }
    $seen=@{}
    foreach ($file in $Installation.files) {
        if ($seen.ContainsKey($file.relative) -or !$expected.ContainsKey($file.relative) -or $expected[$file.relative] -ine $file.sha256) { throw 'Exact installed payload mismatch.' }
        $seen[$file.relative]=$true
    }
}
function Assert-XnProtectedState($Before,$After) {
    foreach ($key in 'profiles','shortcuts','runtime','otherRegistry','forkShellMetadata','defaultInstallAbsent') { Assert-XnSame $Before.$key $After.$key $key }
    if (@($After.productProcesses).Count) { throw 'A product process is live; never close it automatically.' }
}

function Invoke-XnInstallerTransaction($Plan,[hashtable]$Adapter) {
    foreach ($name in 'VerifyInputs','Snapshot','Install','InspectInstallation','CreateSentinels','Uninstall','VerifyRetention','Save') {
        if ($Adapter[$name] -isnot [scriptblock]) { throw "Missing transaction interface: $name" }
    }
    $report=[ordered]@{schema=1;source=$Plan.source;version=$Plan.version;target=$Plan.target;status='preflight';stage='preflight';events=@();installedVerified=$false;uninstalledVerified=$false;installDispatched=$false;uninstallDispatched=$false;installationState='not-dispatched';applicationLaunched=$null;applicationLaunchRequested=$false;forcedCleanup=$false}
    function Record([string]$Stage) { $report.stage=$Stage;$report.events+=@{stage=$Stage;utc=[DateTime]::UtcNow.ToString('o')}; & $Adapter.Save $report | Out-Null }
    try {
        Record 'preflight'
        & $Adapter.VerifyInputs 'before-install' | Out-Null
        $before=& $Adapter.Snapshot
        Assert-XnNamespaceEmpty $before.registry
        if (@($before.productProcesses).Count) { throw 'Live product process at admission.' }
        $report.before=$before
        $report.installDispatched=$true;$report.installationState='unknown-after-install-dispatch'
        Record 'install-dispatch'
        $install=& $Adapter.Install
        $report.installObservation=$install; & $Adapter.Save $report | Out-Null
        Assert-XnObservedRetirement $install $Plan.artifacts.installer.sha256
        $owned=& $Adapter.InspectInstallation
        Assert-XnInstallation $owned $Plan
        $afterInstall=& $Adapter.Snapshot
        Assert-XnProtectedState $before $afterInstall
        Assert-XnOwnedRegistry $afterInstall.registry $Plan.target $Plan.version
        $report.installedVerified=$true;$report.installationState='owned-install-verified';$report.ownedInstallation=$owned;$report.afterInstall=$afterInstall
        Record 'owned-install-verified'
        $sentinels=@(& $Adapter.CreateSentinels)
        if ($sentinels.Count -ne 4) { throw 'All four retained sentinel files must be recorded.' }
        $report.sentinels=$sentinels
        Record 'pre-uninstall'
        & $Adapter.VerifyInputs 'before-uninstall' | Out-Null
        $current=& $Adapter.InspectInstallation
        Assert-XnInstallation $current $Plan $sentinels
        Assert-XnSame $owned.markerSha256 $current.markerSha256 'ownership marker bytes'
        Assert-XnSame $owned.uninstallerSha256 $current.uninstallerSha256 'uninstaller identity'
        $currentState=& $Adapter.Snapshot
        Assert-XnProtectedState $before $currentState
        Assert-XnOwnedRegistry $currentState.registry $Plan.target $Plan.version
        Assert-XnSame $afterInstall.registry $currentState.registry 'retained installation registration'
        $report.uninstallDispatched=$true;$report.installationState='unknown-after-uninstall-dispatch'
        Record 'uninstall-dispatch'
        $uninstall=& $Adapter.Uninstall $owned.uninstallerSha256
        $report.uninstallObservation=$uninstall;& $Adapter.Save $report | Out-Null
        Assert-XnObservedRetirement $uninstall $owned.uninstallerSha256
        $after=& $Adapter.Snapshot
        Assert-XnNamespaceEmpty $after.registry
        Assert-XnProtectedState $before $after
        & $Adapter.VerifyRetention $sentinels | Out-Null
        $report.after=$after;$report.uninstalledVerified=$true;$report.installationState='uninstalled-retention-verified';$report.applicationLaunched=$false;$report.status='passed-silent-install-uninstall'
        Record 'retention-verified'
    } catch {
        $report.status='stopped-preserve-state';$report.error=$_.Exception.Message
        & $Adapter.Save $report | Out-Null
        throw
    }
    [pscustomobject]$report
}
