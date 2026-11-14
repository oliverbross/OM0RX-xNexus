# SPDX-License-Identifier: GPL-3.0-only
# Adapter controls are intentionally inert. No installer/registry/GUI invocation.
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'installer-transaction.ps1')
$repo=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$evidence=Join-Path $repo ('target\installer-transaction-tests\'+[guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($evidence) | Out-Null
$results=[Collections.Generic.List[object]]::new()
function Scenario([string]$Fault,[string]$ExpectedStage) {
    $root=Join-Path $evidence $Fault;[IO.Directory]::CreateDirectory($root) | Out-Null
    $target=Join-Path $root 'app';[IO.Directory]::CreateDirectory($target) | Out-Null
    $plan=[pscustomobject]@{source=('a'*40);version='1.24.1-xlog.4';target=$target;artifacts=@{installer=@{sha256=('a'*64)};payload=@(@{destination='OM0RX-xNexus.exe';sha256=('b'*64)},@{destination='COPYING';sha256=('c'*64)})}}
    $marker="[Ownership]`r`nProduct=sk.om0rx.xnexus`r`nFormat=1`r`nRoot=$target`r`n"
    $state=@{installed=$false;uninstalled=$false;seeded=$false;installCalls=0;uninstallCalls=0;snapshotCalls=0;inspectCalls=0;saved=$null}
    function RegistryRows {
        @(foreach ($h in 'CurrentUser','LocalMachine') { foreach ($v in 'Registry32','Registry64') { foreach ($k in 'product','uninstall','run') {
            $present=$state.installed -and !$state.uninstalled -and $h -eq 'CurrentUser' -and $k -ne 'run'
            $values=@{}
            if ($present) {
                if ($k -eq 'product') { $values['']=$target }
                else { $values=@{DisplayVersion=$plan.version;InstallLocation=('"'+$target+'"');UninstallString=('"'+$target+'\uninstall.exe"')} }
            }
            [pscustomobject]@{hive=$h;view=$v;kind=$k;present=$present;values=$values}
        } } })
    }
    function Observation([string]$Hash) {
        [pscustomobject]@{status='retired-identities-observed';atomicMembership=$true;limitFlags=0;memoryLimit=0;rootExited=$true;rootExitCode=0;activeProcesses=0;terminatedByObserver=$false;totalProcesses=1;identities=@(@{sha256=$Hash;executable='C:\inert.exe';arguments='inert';exited=$true;exitCode=0})}
    }
    $adapter=@{
        Save={param($value) $state.saved=$value; $value | ConvertTo-Json -Depth 25 | Set-Content -LiteralPath (Join-Path $root 'receipt.json') -Encoding utf8}
        VerifyInputs={param($stage)
            if (($Fault -eq 'artifact-before-install' -and $stage -eq 'before-install') -or ($Fault -eq 'artifact-before-uninstall' -and $stage -eq 'before-uninstall')) { throw 'Injected artifact identity mismatch' }
        }
        Snapshot={
            $state.snapshotCalls++;$rows=RegistryRows
            if ($Fault -eq 'initial-registration' -and $state.snapshotCalls -eq 1) { $rows[0].present=$true }
            if ($Fault -eq 'foreign-installed-root' -and $state.installed) { $rows[0].values['']='C:\Foreign' }
            if ($Fault -eq 'registration-replaced' -and $state.snapshotCalls -eq 3) { $rows[0].values['extra']='new unrelated state' }
            if ($Fault -eq 'registration-not-removed' -and $state.uninstalled) { $rows[0].present=$true }
            $profiles=@(@{path='inert';exists=$true;files=@(@{path='inert-draft';sha256='unchanged'})})
            if ($Fault -eq 'profile-changed-after-install' -and $state.snapshotCalls -eq 2) { $profiles[0].files[0].sha256='changed' }
            if ($Fault -eq 'profile-changed-after-uninstall' -and $state.uninstalled) { $profiles[0].files[0].sha256='changed' }
            $live=if ($Fault -eq 'live-product' -and $state.snapshotCalls -eq 1) { @(@{pid=123}) } else { @() }
            [pscustomobject]@{registry=$rows;profiles=$profiles;shortcuts=@();runtime='157.0.4341.46';otherRegistry=@();forkShellMetadata=@();defaultInstallAbsent='C:\inert-default';productProcesses=$live}
        }
        Install={
            $state.installCalls++;$state.installed=$true;$o=Observation ('a'*64)
            if ($Fault -eq 'install-throws-after-side-effect') { throw 'Injected failure after filesystem/registration side effects' }
            if ($Fault -eq 'install-timeout') { $o.status='timeout-unresolved';$o.activeProcesses=1 }
            if ($Fault -eq 'install-exit-error') { $o.rootExitCode=1 }
            if ($Fault -eq 'install-identity-gap') { $o.status='retired-identity-gap' }
            if ($Fault -eq 'unexpected-helper') { $o.identities[0].sha256='e'*64 }
            if ($Fault -eq 'install-member-exit-error') { $o.identities[0].exitCode=1 }
            $o
        }
        InspectInstallation={
            $state.inspectCalls++
            $files=@(@{relative='OM0RX-xNexus.exe';sha256=('b'*64)},@{relative='COPYING';sha256=('c'*64)},@{relative='.om0rx-xnexus-owner.ini';sha256=('d'*64)},@{relative='uninstall.exe';sha256=('e'*64)})
            if ($state.seeded) { $files+=@(@{relative='unknown.txt';sha256=('f'*64)},@{relative='nested\unknown.txt';sha256=('f'*64)}) }
            if ($Fault -eq 'payload-corrupt') { $files[0].sha256='0'*64 }
            if ($Fault -eq 'unexpected-payload') { $files+=@(@{relative='foreign.dll';sha256=('0'*64)}) }
            $uninstaller='e'*64
            if ($Fault -eq 'uninstaller-replaced' -and $state.inspectCalls -eq 2) { $uninstaller='0'*64;$files[3].sha256=$uninstaller }
            $text=if ($Fault -eq 'foreign-marker') { $marker.Replace('sk.om0rx.xnexus','com.foreign') } else { $marker }
            @{markerText=$text;markerSha256=('d'*64);uninstallerSha256=$uninstaller;uninstallerPe=$true;files=$files}
        }
        CreateSentinels={
            $state.seeded=$true
            if ($Fault -eq 'sentinel-write-failure') { throw 'Injected CreateNew failure; preserve partial sentinels' }
            @(@{path='inert-app-unknown';relativeToApp='unknown.txt';sha256=('f'*64)},@{path='inert-app-nested';relativeToApp='nested\unknown.txt';sha256=('f'*64)},@{path='inert-draft';relativeToApp=$null;sha256=('f'*64)},@{path='inert-outbox';relativeToApp=$null;sha256=('f'*64)})
        }
        Uninstall={param($hash)
            $state.uninstallCalls++;$state.uninstalled=$true;$o=Observation $hash
            if ($Fault -eq 'uninstall-throws-after-side-effect') { throw 'Injected failure after uninstall side effects' }
            if ($Fault -eq 'uninstall-timeout') { $o.status='timeout-unresolved';$o.activeProcesses=1 }
            $o
        }
        VerifyRetention={param($sentinels) if ($Fault -eq 'sentinel-lost') { throw 'Injected retained bytes missing' } }
    }
    $failure=$null
    try { Invoke-XnInstallerTransaction $plan $adapter | Out-Null } catch { $failure=$_.Exception.Message }
    if ($Fault -eq 'success') {
        if ($failure -or $state.saved.status -cne 'passed-silent-install-uninstall' -or !$state.seeded -or $state.installCalls -ne 1 -or $state.uninstallCalls -ne 1) { throw "Success control failed: $failure" }
    } else {
        if (!$failure -or $state.saved.status -cne 'stopped-preserve-state' -or $state.saved.stage -cne $ExpectedStage) { throw "Wrong refusal for $Fault at $($state.saved.stage): $failure" }
        if ($ExpectedStage -eq 'preflight' -and $state.installCalls) { throw 'Install occurred after admission failure' }
        if ($ExpectedStage -in 'preflight','install-dispatch','owned-install-verified','pre-uninstall' -and $state.uninstallCalls) { throw 'Uninstall cleanup occurred after an earlier failure' }
    }
    if ($state.saved.forcedCleanup) { throw 'Unexpected cleanup action' }
    if ($Fault -eq 'install-throws-after-side-effect' -and (!$state.saved.installDispatched -or $state.saved.installedVerified -or $state.saved.installationState -cne 'unknown-after-install-dispatch')) { throw 'Partial installation incorrectly claimed absent/verified' }
    if ($Fault -eq 'uninstall-throws-after-side-effect' -and (!$state.saved.uninstallDispatched -or $state.saved.uninstalledVerified -or $state.saved.installationState -cne 'unknown-after-uninstall-dispatch')) { throw 'Partial uninstall incorrectly claimed completed' }
    $results.Add(@{case=$Fault;status='passed';installCalls=$state.installCalls;uninstallCalls=$state.uninstallCalls;stage=$state.saved.stage;failure=$failure})
}
Scenario 'success' ''
foreach ($case in 'artifact-before-install','initial-registration','live-product') { Scenario $case 'preflight' }
foreach ($case in 'install-timeout','install-exit-error','install-identity-gap','unexpected-helper','payload-corrupt','unexpected-payload','foreign-marker','foreign-installed-root','profile-changed-after-install','install-throws-after-side-effect','install-member-exit-error') { Scenario $case 'install-dispatch' }
Scenario 'sentinel-write-failure' 'owned-install-verified'
foreach ($case in 'artifact-before-uninstall','uninstaller-replaced','registration-replaced') { Scenario $case 'pre-uninstall' }
foreach ($case in 'uninstall-timeout','registration-not-removed','profile-changed-after-uninstall','sentinel-lost','uninstall-throws-after-side-effect') { Scenario $case 'uninstall-dispatch' }
# Exercise the actual production entry point's default: exact inert review files
# are readable, but no claim/journal, target, process or registry action is taken.
$id=[guid]::NewGuid().ToString('N');$reviewRoot=Join-Path $evidence "acceptance-$id"
[IO.Directory]::CreateDirectory($reviewRoot) | Out-Null
$fakeBuild=Join-Path $reviewRoot 'inert-build.json';[IO.File]::WriteAllText($fakeBuild,'{"inert":true}')
$fakeAudit=Join-Path $reviewRoot 'inert-audit.json';[IO.File]::WriteAllText($fakeAudit,'{"inert":true}')
$fakePlan=Join-Path $reviewRoot 'preparation.json'
@{schema=1;kind='xnexus-installer-preparation';status='prepared-not-executed';id=$id;installed=$false;applicationLaunched=$false;target=(Join-Path $reviewRoot 'app');artifacts=@{buildReceipt=$fakeBuild;buildReceiptSha256=(Get-XnHash $fakeBuild);auditReceipt=$fakeAudit;auditSha256=(Get-XnHash $fakeAudit)}} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $fakePlan -Encoding utf8
$proof=Get-XnTreeManifest $reviewRoot
& (Join-Path $PSScriptRoot 'execute-installer-acceptance.ps1') -PreparedReceipt $fakePlan -ExpectedPreparationSha256 (Get-XnHash $fakePlan) -ExpectedAuditSha256 (Get-XnHash $fakeAudit) | Out-Null
Assert-XnRetainedFiles @($proof)
if ((Test-Path -LiteralPath (Join-Path $reviewRoot 'app')) -or (Test-Path -LiteralPath (Join-Path $reviewRoot 'transaction.claim'))) { throw 'Default entry point mutated transaction state' }
$results.Add(@{case='actual-executor-default';status='passed';installCalls=0;uninstallCalls=0})
$refused=$false
try { & (Join-Path $PSScriptRoot 'execute-installer-acceptance.ps1') -PreparedReceipt $fakePlan -ExpectedPreparationSha256 ('0'*64) -ExpectedAuditSha256 (Get-XnHash $fakeAudit) -Execute | Out-Null } catch { $refused=$_.Exception.Message -like '*Hash mismatch*' }
if (!$refused) { throw 'Explicit Execute did not refuse mismatched review identity' }
Assert-XnRetainedFiles @($proof)
$results.Add(@{case='actual-executor-wrong-review-hash';status='passed';installCalls=0;uninstallCalls=0})
# Actual bounded projection used by Get-XnRegistrySnapshot, with an inert
# RegistryKey-shaped object. No real registry values are read or written.
$key=[pscustomobject]@{SubKeyCount=0;Values=@{''='C:\inert';DisplayVersion='1.24.1-xlog.4'};Kinds=@{''='String';DisplayVersion='String'}}
$key | Add-Member ScriptMethod GetValueNames { return [string[]]@($this.Values.Keys) }
$key | Add-Member ScriptMethod GetValueKind { param($name) return [Microsoft.Win32.RegistryValueKind]::$($this.Kinds[$name]) }
$key | Add-Member ScriptMethod GetValue { param($name,$fallback,$options) return $this.Values[$name] }
$baseline=Get-XnOwnedRegistryValues $key
Assert-XnSame $baseline (Get-XnOwnedRegistryValues $key) 'positive typed projection'
$results.Add(@{case='actual-registry-projection-positive';status='passed'})
$key.Values.extra='new unrelated value';$key.Kinds.extra='String'
$refused=$false;try { Assert-XnSame $baseline (Get-XnOwnedRegistryValues $key) 'extra owned value' } catch { $refused=$true }
if (!$refused) { throw 'Actual projection omitted added owned value' }
$results.Add(@{case='actual-registry-added-value';status='passed'})
$key.Values.Remove('extra');$key.Kinds.Remove('extra');$key.Kinds['']='ExpandString'
$refused=$false;try { Assert-XnSame $baseline (Get-XnOwnedRegistryValues $key) 'native registry type' } catch { $refused=$true }
if (!$refused) { throw 'Actual projection omitted REG_SZ to REG_EXPAND_SZ change' }
$results.Add(@{case='actual-registry-native-type-change';status='passed'})
$key.SubKeyCount=1
$refused=$false;try { Get-XnOwnedRegistryValues $key | Out-Null } catch { $refused=$_.Exception.Message -like '*subkey*' }
if (!$refused) { throw 'Actual projection accepted an owned subkey' }
$results.Add(@{case='actual-registry-added-subkey';status='passed'})
$descriptor=@{ownerIsCurrentUser=$true;currentUserSid='S-1-5-21-123';protected=$true;allowedSids=@('S-1-5-21-123','S-1-5-18','S-1-5-32-544')}
Assert-XnEvidenceDescriptor $descriptor -RequireProtected
$results.Add(@{case='private-evidence-principals-positive';status='passed'})
$descriptor.allowedSids+=@('S-1-1-0')
$refused=$false;try { Assert-XnEvidenceDescriptor $descriptor } catch { $refused=$true }
if (!$refused) { throw 'Private nonce ACL allowed Everyone' }
$results.Add(@{case='private-evidence-foreign-principal';status='passed'})
$descriptor.allowedSids=@('S-1-5-21-123');$descriptor.ownerIsCurrentUser=$false
$refused=$false;try { Assert-XnEvidenceDescriptor $descriptor } catch { $refused=$true }
if (!$refused) { throw 'Private nonce ACL accepted a different owner' }
$results.Add(@{case='private-evidence-foreign-owner';status='passed'})
@{status='passed';cases=$results.ToArray();count=$results.Count;interfaces='inert';realInstallerInvoked=$false;registryChanged=$false} | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $evidence 'receipt.json') -Encoding utf8
Write-Output "$($results.Count) inert transaction cases passed: $evidence"
