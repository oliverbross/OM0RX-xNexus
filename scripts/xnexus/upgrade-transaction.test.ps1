# SPDX-License-Identifier: GPL-3.0-only
# Inert adapter tests plus fresh disposable file backups. No installed paths,
# registry changes, installers, native observers, applications or device access.
param([Parameter(Mandatory)][string]$EvidenceRoot)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'upgrade-inputs.ps1')
$null=Assert-XnLocalPath $EvidenceRoot -AllowMissing -NoSpaces
if(Test-Path -LiteralPath $EvidenceRoot){throw 'Fresh test evidence directory required.'}
[void][IO.Directory]::CreateDirectory($EvidenceRoot)
$acl=[Security.AccessControl.DirectorySecurity]::new();$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User
$acl.SetOwner($sid);$acl.SetAccessRuleProtection($true,$false)
foreach($allowed in @($sid.Value,'S-1-5-18','S-1-5-32-544')){
    $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new([Security.Principal.SecurityIdentifier]::new($allowed),'FullControl','ContainerInherit,ObjectInherit','None','Allow'))
}
Set-Acl -LiteralPath $EvidenceRoot -AclObject $acl
$results=[Collections.Generic.List[object]]::new()
function Pass([string]$Case){$results.Add(@{case=$Case;passed=$true})}
function Refuse([string]$Case,[scriptblock]$Action,[string]$Pattern){
    $message=$null;try{& $Action|Out-Null}catch{$message=$_.Exception.Message}
    if(!$message -or $message -notlike $Pattern){throw "Expected specific refusal for $Case; received: $message"};Pass $Case
}
function Clone($Value){ConvertTo-XnCanonicalJson $Value|ConvertFrom-Json -AsHashtable -NoEnumerate}
function Fixture {
    $target=Join-Path $EvidenceRoot 'inert-app';$installer=Join-Path $EvidenceRoot 'inert-candidate.exe'
    [void][IO.Directory]::CreateDirectory($target)
    $oldPayload=@(1..37|ForEach-Object{@{destination=('file'+$_+'.dat');sha256=('a'*64)}})
    $newPayload=@(1..37|ForEach-Object{@{destination=('file'+$_+'.dat');sha256=('b'*64)}})
    $rows=@(foreach($h in 'CurrentUser','LocalMachine'){foreach($v in 'Registry32','Registry64'){foreach($k in 'product','uninstall','run'){
        $present=$h -eq 'CurrentUser' -and $k -ne 'run';$values=@{};$kinds=@{}
        if($present){
            if($k -eq 'product'){$values['']=$target;$kinds['']='String'}else{
                $values=@{DisplayVersion='1.24.1-xlog.4';InstallLocation=('"'+$target+'"');UninstallString=('"'+$target+'\uninstall.exe"');EstimatedSize=100}
                $kinds=@{DisplayVersion='String';InstallLocation='String';UninstallString='String';EstimatedSize='DWord'}
            }
        }
        @{hive=$h;view=$v;kind=$k;present=$present;values=$values;valueKinds=$kinds}
    }}})
    $files=@($oldPayload|ForEach-Object{@{relative=$_.destination;sha256=$_.sha256}})+@(@{relative='.om0rx-xnexus-owner.ini';sha256=('c'*64)},@{relative='uninstall.exe';sha256=('d'*64)})
    $owned=@{markerText="[Ownership]`r`nProduct=sk.om0rx.xnexus`r`nFormat=1`r`nRoot=$target`r`n";markerSha256=('c'*64);uninstallerSha256=('d'*64);uninstallerPe=$true;directories=@('empty');files=$files}
    $shortcuts=@(@{path='C:\inert-upgrade\desktop\OM0RX-xNexus.lnk';sha256=('e'*64);target=($target+'\OM0RX-xNexus.exe');arguments='';appUserModelId='sk.om0rx.xnexus'})
    $plan=@{target=$target;old=@{source='ae35fc6435f861e38029049299833afe57de4885';version='1.24.1-xlog.4';payload=$oldPayload;installation=(Clone $owned);registry=(Clone $rows);shortcuts=(Clone $shortcuts)};new=@{source=('5'*40);version='1.24.1-xlog.5';payload=$newPayload;installer=@{path=$installer;sha256=('f'*64)}}}
    $before=@{productProcesses=@();installation=$owned;registry=$rows;shortcuts=$shortcuts;profiles=@(@{path='C:\inert-profile';exists=$true;files=@(@{path='C:\inert-profile\outbox';bytes=10;sha256=('a'*64)});directories=@('webview\empty')});otherShortcuts=@();runtime='157.0.4341.46';protectedRegistry=@()}
    $before.installation.files+=@{relative='operator-note.txt';sha256=('e'*64)}
    $after=Clone $before
    foreach($entry in $after.installation.files){if($entry.relative -like 'file*.dat'){$entry.sha256='b'*64}}
    foreach($row in $after.registry){if($row.present -and $row.kind -eq 'uninstall'){$row.values.DisplayVersion='1.24.1-xlog.5';$row.values.EstimatedSize=101}}
    $after.installation.uninstallerSha256='9'*64
    ($after.installation.files|Where-Object relative -EQ 'uninstall.exe').sha256='9'*64
    $after.shortcuts[0].sha256='9'*64
    $observation=@{status='retired-identities-observed';atomicMembership=$true;limitFlags=0;memoryLimit=0;rootExited=$true;rootExitCode=0;activeProcesses=0;terminatedByObserver=$false;totalProcesses=1;rootPid=1;identities=@(@{pid=1;sha256=('f'*64);executable=$installer;arguments=('"'+$installer+'" /S /D='+$target);exited=$true;exitCode=0})}
    return @{plan=$plan;before=$before;after=$after;observation=$observation}
}
function Scenario([string]$Fault,[int]$ExpectedDispatch) {
    $f=Fixture;$state=@{snapshots=0;dispatch=0;verify=0;backupChecks=0;saved=$null}
    $adapter=@{
        VerifyInputs={
            $state.verify++;Assert-XnUpgradeIdentity $f.plan (Join-Path $EvidenceRoot 'inert-app')
            if($Fault -eq 'artifact-race' -and $state.verify -eq 2){throw 'Injected artifact change'}
        }
        Snapshot={
            $state.snapshots++;$snapshot=Clone $(if($state.dispatch){$f.after}else{$f.before})
            switch($Fault){
                'live-app' {$snapshot.productProcesses=@(@{pid=7})}
                'live-webview-before-dispatch' {if($state.snapshots -eq 2){$snapshot.productProcesses=@(@{pid=8})}}
                'profile-race' {if($state.snapshots -eq 2){$snapshot.profiles[0].files[0].sha256='0'*64}}
                'foreign-root' {$snapshot.registry[0].values['']='C:\foreign'}
                'old-marker' {if(!$state.dispatch){$snapshot.installation.markerText=$snapshot.installation.markerText.Replace('sk.om0rx.xnexus','foreign')}}
                'old-uninstaller' {if(!$state.dispatch){$snapshot.installation.uninstallerSha256='0'*64;($snapshot.installation.files|Where-Object relative -EQ 'uninstall.exe').sha256='0'*64}}
                'old-payload' {if(!$state.dispatch){$snapshot.installation.files[0].sha256='0'*64}}
                'foreign-executable' {if(!$state.dispatch){$snapshot.installation.files+=@{relative='foreign.exe';sha256=('0'*64)}}}
                'lost-profile' {if($state.dispatch){$snapshot.profiles[0].files=@()}}
                'lost-sentinel' {if($state.dispatch){$snapshot.installation.files=@($snapshot.installation.files|Where-Object relative -NE 'operator-note.txt')}}
                'lost-empty-directory' {if($state.dispatch){$snapshot.installation.directories=@()}}
                'new-payload' {if($state.dispatch){$snapshot.installation.files[0].sha256='0'*64}}
                'registry-addition' {if($state.dispatch){$snapshot.registry[1].values.Unexpected='value';$snapshot.registry[1].valueKinds.Unexpected='String'}}
                'registry-kind' {if($state.dispatch){$snapshot.registry[1].valueKinds.DisplayVersion='ExpandString'}}
                'boolean-size' {if($state.dispatch){$snapshot.registry[1].values.EstimatedSize=$true}}
                'shortcut-target' {if($state.dispatch){$snapshot.shortcuts[0].target='C:\foreign.exe'}}
                'runtime-changed' {if($state.dispatch){$snapshot.runtime='158.0.0.0'}}
            }
            $snapshot
        }
        Backup={param($trees) if($Fault -eq 'backup-failure'){throw 'Injected backup failure'};@{synthetic=$true}}
        VerifyBackup={param($backup,$trees) $state.backupChecks++;if($Fault -eq 'backup-corrupted' -and $state.backupChecks -eq 2){throw 'Injected backup corruption'}}
        Install={
            $state.dispatch++;if($Fault -eq 'partial-install'){throw 'Injected failure after dispatch'}
            $o=Clone $f.observation
            switch($Fault){
                'timeout' {$o.status='timeout-unresolved';$o.activeProcesses=1}
                'nonzero-exit' {$o.rootExitCode=1}
                'child' {$o.totalProcesses=2;$o.identities+=Clone $o.identities[0]}
                'wrong-arguments' {$o.identities[0].arguments+=' /UPDATE'}
                'observer-force' {$o.terminatedByObserver=$true}
            };$o
        }
        Save={param($report) $state.saved=Clone $report}
    }
    $errorText=$null;try{$null=Invoke-XnUpgradeTransaction $f.plan $adapter}catch{$errorText=$_.Exception.Message}
    if($state.dispatch -ne $ExpectedDispatch){throw "Unexpected dispatch count $Fault : $($state.dispatch); error=$errorText"}
    if($Fault -eq 'pass'){
        if($errorText -or !$state.saved.installedVerified -or $state.saved.applicationLaunched -cne $false -or $state.saved.status -cne 'passed-owned-upgrade-no-app-launch'){throw "Positive upgrade failed: $errorText"}
    }else{
        if(!$errorText -or $state.saved.installedVerified -or $state.saved.status -cne 'stopped-preserve-state'){throw "Expected preserved failure: $Fault"}
        $expected=if($ExpectedDispatch){'unknown-after-upgrade-dispatch'}else{'old-installation-not-modified'}
        if($state.saved.installationState -cne $expected){throw "Misleading partial state: $Fault"}
    }
    if($state.saved.applicationLaunchRequested -or $state.saved.forcedCleanup -or $state.saved.uninstallerInvoked){throw 'Forbidden side effect recorded.'}
    Pass $Fault
}
Scenario 'pass' 1
foreach($case in 'artifact-race','live-app','live-webview-before-dispatch','profile-race','foreign-root','old-marker','old-uninstaller','old-payload','foreign-executable','backup-failure','backup-corrupted'){Scenario $case 0}
foreach($case in 'lost-profile','lost-sentinel','lost-empty-directory','new-payload','registry-addition','registry-kind','boolean-size','shortcut-target','runtime-changed','partial-install','timeout','nonzero-exit','child','wrong-arguments','observer-force'){Scenario $case 1}
foreach($fault in 'old-version','new-version','same-source','target','payload-addition'){
    $f=Fixture
    switch($fault){
        'old-version' {$f.plan.old.version='1.24.1-xlog.3'}
        'new-version' {$f.plan.new.version='1.24.1-xlog.4'}
        'same-source' {$f.plan.new.source=$f.plan.old.source}
        'target' {$f.plan.target='C:\foreign'}
        'payload-addition' {$f.plan.new.payload+=@{destination='foreign.dll';sha256=('a'*64)}}
    }
    Refuse $fault {Assert-XnUpgradeIdentity $f.plan (Join-Path $EvidenceRoot 'inert-app')} '*'
}
# Actual backup bytes, including draft, outbox and WebView-shaped disposable data.
$source=Join-Path $EvidenceRoot 'synthetic-profile';[void][IO.Directory]::CreateDirectory((Join-Path $source 'webview\empty'))
foreach($name in 'draft.json','outbox.json','webview\state.dat'){[IO.File]::WriteAllText((Join-Path $source $name),('synthetic-only-'+$name))}
$tree=Get-XnUpgradeTree $source
$backup=Copy-XnUpgradeProfileBackup @($tree) (Join-Path $EvidenceRoot 'backup')
Assert-XnUpgradeBackup $backup @($tree);Assert-XnSame $tree (Get-XnUpgradeTree $source) 'original synthetic profile';Pass 'actual-backup-bytes-and-empty-directories'
Refuse 'backup-never-overwrites' {Copy-XnUpgradeProfileBackup @($tree) $backup.root} '*exists*'
[IO.File]::WriteAllText($backup.files[0].backup,'corrupted')
Refuse 'actual-backup-corruption' {Assert-XnUpgradeBackup $backup @($tree)} '*hash*'
Assert-XnSame $tree (Get-XnUpgradeTree $source) 'original survives backup corruption';Pass 'original-not-modified-by-corrupt-backup'
$report=@{status='passed-inert-upgrade-controls';cases=$results.Count;results=$results.ToArray();productDispatched=$false;registryMutated=$false;sourceFiles=@(Get-XnUpgradeToolNames|ForEach-Object{@{name=$_;sha256=Get-XnHash (Join-Path $PSScriptRoot $_)}})}
$report|ConvertTo-Json -Depth 8|Set-Content -LiteralPath (Join-Path $EvidenceRoot 'tests.json') -Encoding utf8NoBOM
Write-Output "$($results.Count) inert upgrade checks passed; evidence $EvidenceRoot"
