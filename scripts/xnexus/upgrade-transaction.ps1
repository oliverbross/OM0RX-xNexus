# SPDX-License-Identifier: GPL-3.0-only
# Narrow .4 -> .5 upgrade orchestration. Real dispatch exists only in the separate entry point.
. (Join-Path $PSScriptRoot 'installer-transaction.ps1')

function Assert-XnUpgradeIdentity($Plan,[string]$ExpectedTarget) {
    if ($Plan.old.version -cne '1.24.1-xlog.4' -or $Plan.old.source -cne 'ae35fc6435f861e38029049299833afe57de4885' -or
        $Plan.new.version -cne '1.24.1-xlog.5' -or $Plan.new.source -notmatch '^[0-9a-f]{40}$' -or
        $Plan.new.source -ceq $Plan.old.source -or $Plan.target -cne $ExpectedTarget) { throw 'Require the exact owned .4 to reviewed .5 transition.' }
    $old=@{};foreach($file in $Plan.old.payload){
        if($old.ContainsKey($file.destination)){throw 'Duplicate old payload destination'};$old[$file.destination]=$true
    }
    $seen=@{};foreach($file in $Plan.new.payload){
        if($seen.ContainsKey($file.destination) -or !$old.ContainsKey($file.destination) -or $file.sha256 -notmatch '^[a-fA-F0-9]{64}$'){throw 'Payload file set changed; separate upgrade review required.'};$seen[$file.destination]=$true
    }
    if($old.Count -ne 37 -or $seen.Count -ne $old.Count){throw 'Require the reviewed 37-file payload set.'}
}

function Assert-XnUpgradeClosed($State) {
    if(@($State.productProcesses).Count){throw 'xNexus or its profile WebView is live; ordinary close is required.'}
}

function Get-XnUpgradeExtras($State,$Plan) {
    $known=@{};foreach($file in $Plan.old.installation.files){$known[$file.relative]=$true}
    foreach($file in $State.installation.files){
        if(!$known.ContainsKey($file.relative)){
            if($file.relative -match '(^|[\\/])\.\.?([\\/]|$)|[:/]' -or $file.relative.StartsWith('\') -or
                [IO.Path]::GetExtension($file.relative) -in '.exe','.dll','.com','.msi','.bat','.cmd','.ps1','.vbs') {throw 'Foreign or noncanonical file in owned installation.'}
            if(@($Plan.new.payload|Where-Object destination -IEQ $file.relative).Count){throw 'New payload collides with an unrelated file.'}
            @{relativeToApp=$file.relative;sha256=$file.sha256}
        }
    }
}

function Assert-XnUpgradeOldState($State,$Plan) {
    Assert-XnUpgradeClosed $State
    Assert-XnOwnedRegistry $State.registry $Plan.target $Plan.old.version
    Assert-XnSame $Plan.old.registry $State.registry 'exact old registration'
    Assert-XnSame $Plan.old.shortcuts $State.shortcuts 'exact old shortcuts'
    $extras=@(Get-XnUpgradeExtras $State $Plan)
    Assert-XnInstallation $State.installation @{target=$Plan.target;artifacts=@{payload=$Plan.old.payload}} $extras
    Assert-XnSame $Plan.old.installation.markerSha256 $State.installation.markerSha256 'old ownership marker'
    Assert-XnSame $Plan.old.installation.uninstallerSha256 $State.installation.uninstallerSha256 'old uninstaller'
    return ,$extras
}

function Assert-XnUpgradeNewState($Before,$After,$Plan,$Extras) {
    Assert-XnUpgradeClosed $After
    Assert-XnInstallation $After.installation @{target=$Plan.target;artifacts=@{payload=$Plan.new.payload}} $Extras
    Assert-XnSame $Before.installation.markerSha256 $After.installation.markerSha256 'ownership marker bytes'
    Assert-XnSame $Before.installation.directories $After.installation.directories 'retained installation directories'
    Assert-XnOwnedRegistry $After.registry $Plan.target $Plan.new.version
    # Only version and actual installer-estimated size may change in the owned keys.
    $expected=ConvertTo-XnCanonicalJson $Before.registry|ConvertFrom-Json -AsHashtable -NoEnumerate
    foreach($row in $expected){
        if($row.present -and $row.kind -eq 'uninstall'){
            $matches=@($After.registry|Where-Object{$_.hive -ceq $row.hive -and $_.view -ceq $row.view -and $_.kind -ceq $row.kind})
            $size=if($matches.Count -eq 1){$matches[0].values.EstimatedSize}else{$null}
            if(($size -isnot [int] -and $size -isnot [long] -and $size -isnot [uint32]) -or $size -lt 1 -or $size -gt [uint32]::MaxValue){throw 'Invalid new installer size registration'}
            $row.values.DisplayVersion=$Plan.new.version;$row.values.EstimatedSize=$matches[0].values.EstimatedSize
        }
    }
    Assert-XnSame $expected $After.registry 'new registry field set/kinds'
    foreach($key in 'profiles','otherShortcuts','runtime','protectedRegistry'){Assert-XnSame $Before[$key] $After[$key] $key}
    if(@($After.shortcuts).Count -ne @($Before.shortcuts).Count){throw 'Shortcut count changed'}
    foreach($old in $Before.shortcuts){
        $next=@($After.shortcuts|Where-Object path -CEQ $old.path)
        if($next.Count -ne 1){throw 'Shortcut location changed'}
        foreach($key in 'target','arguments','appUserModelId'){Assert-XnSame $old[$key] $next[0][$key] "shortcut $key"}
    }
}

function Get-XnUpgradeTree([string]$Path) {
    $tree=Get-XnTreeManifest $Path
    foreach($file in $tree.files){if((Get-Item -LiteralPath $file.path -Force).LinkType){throw 'Hard-linked profile/install file refused.'}}
    $directories=[Collections.Generic.List[string]]::new()
    if($tree.exists){
        $pending=[Collections.Generic.Stack[string]]::new();$pending.Push($Path)
        while($pending.Count){
            foreach($item in Get-ChildItem -LiteralPath $pending.Pop() -Directory -Force -ErrorAction Stop){
                $null=Assert-XnLocalPath $item.FullName
                if($directories.Count -ge 4096){throw 'Directory backup bound exceeded.'}
                $directories.Add($item.FullName.Substring($Path.Length+1));$pending.Push($item.FullName)
            }
        }
    }
    return @{path=$tree.path;exists=$tree.exists;files=@($tree.files);directories=@($directories.ToArray()|Sort-Object)}
}

function Copy-XnUpgradeProfileBackup($Trees,[string]$Destination) {
    $null=Assert-XnLocalPath $Destination -AllowMissing
    if(Test-Path -LiteralPath $Destination){throw 'Backup destination exists; never overwrite.'}
    [void][IO.Directory]::CreateDirectory($Destination)
    Assert-XnPrivateEvidence $Destination
    $copies=[Collections.Generic.List[object]]::new();$roots=[Collections.Generic.List[object]]::new();[ulong]$bytes=0;$index=0
    foreach($tree in $Trees){
        $index++;$backupRoot=Join-Path $Destination $index.ToString()
        if(!$tree.exists){continue}
        [void][IO.Directory]::CreateDirectory($backupRoot)
        foreach($relative in $tree.directories){[void][IO.Directory]::CreateDirectory((Join-Path $backupRoot $relative))}
        $roots.Add(@{source=$tree.path;backup=$backupRoot;directories=@($tree.directories)})
        foreach($file in $tree.files){
            $bytes+=[ulong]$file.bytes;if($bytes -gt 8GB){throw 'Profile backup exceeds reviewed 8 GiB bound.'}
            $null=Assert-XnLocalPath $file.path
            if((Get-Item -LiteralPath $file.path -Force).LinkType){throw 'Hard-linked backup input refused.'}
            if(!$file.path.StartsWith($tree.path+'\',[StringComparison]::Ordinal)){throw 'Backup input escaped its profile root.'}
            $relative=$file.path.Substring($tree.path.Length+1);$copy=Join-Path $backupRoot $relative
            [void][IO.Directory]::CreateDirectory((Split-Path -Parent $copy))
            # Deny concurrent writers/deleters while copying; never replace an existing backup.
            $input=[IO.File]::Open($file.path,'Open','Read','Read')
            try{$output=[IO.File]::Open($copy,'CreateNew','Write','None');try{$input.CopyTo($output);$output.Flush($true)}finally{$output.Dispose()}}finally{$input.Dispose()}
            Assert-XnHash $file.path $file.sha256;Assert-XnHash $copy $file.sha256
            $copies.Add(@{source=$file.path;backup=$copy;bytes=$file.bytes;sha256=$file.sha256})
        }
    }
    foreach($tree in $Trees){Assert-XnSame $tree (Get-XnUpgradeTree $tree.path) 'profile during backup'}
    return @{root=$Destination;roots=@($roots.ToArray());files=@($copies.ToArray());bytes=$bytes}
}

function Assert-XnUpgradeBackup($Backup,$Trees) {
    $existing=@($Trees|Where-Object exists)
    if(@($Backup.roots).Count -ne $existing.Count){throw 'Incomplete profile backup roots.'}
    foreach($tree in $existing){
        $root=@($Backup.roots|Where-Object source -CEQ $tree.path)
        if($root.Count -ne 1 -or !$root[0].backup.StartsWith($Backup.root+'\',[StringComparison]::Ordinal)){throw 'Backup root mismatch.'}
        $actual=Get-XnUpgradeTree $root[0].backup
        Assert-XnSame @($tree.directories) @($actual.directories) 'backup directories'
        if(@($actual.files).Count -ne @($tree.files).Count){throw 'Unexpected or missing backup file.'}
    }
    $expected=@($Trees|ForEach-Object{$_.files})
    if(@($Backup.files).Count -ne $expected.Count){throw 'Incomplete profile backup.'}
    foreach($file in $expected){
        $copy=@($Backup.files|Where-Object source -CEQ $file.path)
        if($copy.Count -ne 1 -or $copy[0].sha256 -ine $file.sha256 -or $copy[0].bytes -ne $file.bytes){throw 'Backup manifest mismatch.'}
        if(!$copy[0].backup.StartsWith($Backup.root+'\',[StringComparison]::Ordinal)){throw 'Backup copy escaped private backup root.'}
        Assert-XnHash $copy[0].backup $file.sha256
    }
}

function Invoke-XnUpgradeTransaction($Plan,[hashtable]$Adapter) {
    foreach($name in 'VerifyInputs','Snapshot','Backup','VerifyBackup','Install','Save'){
        if($Adapter[$name] -isnot [scriptblock]){throw "Missing fixed upgrade adapter: $name"}
    }
    $report=[ordered]@{schema=1;status='preflight';stage='preflight';oldSource=$Plan.old.source;oldVersion=$Plan.old.version;source=$Plan.new.source;version=$Plan.new.version;target=$Plan.target;installDispatched=$false;installedVerified=$false;installationState='old-installation-not-modified';applicationLaunchRequested=$false;applicationLaunched=$null;forcedCleanup=$false;uninstallerInvoked=$false}
    function Save([string]$Stage){$report.stage=$Stage;& $Adapter.Save $report|Out-Null}
    try{
        Save 'preflight'; & $Adapter.VerifyInputs|Out-Null
        $before=& $Adapter.Snapshot;$extras=Assert-XnUpgradeOldState $before $Plan
        $report.before=$before;$report.retainedExtraFiles=$extras;Save 'old-installation-verified'
        $backup=& $Adapter.Backup $before.profiles;$report.backup=$backup
        & $Adapter.VerifyBackup $backup $before.profiles|Out-Null;Save 'backup-verified'
        & $Adapter.VerifyInputs|Out-Null
        $fresh=& $Adapter.Snapshot;$null=Assert-XnUpgradeOldState $fresh $Plan
        Assert-XnSame $before $fresh 'pre-dispatch installation/profile state'
        & $Adapter.VerifyBackup $backup $fresh.profiles|Out-Null
        $report.installDispatched=$true;$report.installationState='unknown-after-upgrade-dispatch';Save 'upgrade-dispatch'
        $observed=& $Adapter.Install;$report.observation=$observed;Save 'installer-returned'
        Assert-XnObservedRetirement $observed $Plan.new.installer.sha256
        if($observed.totalProcesses -ne 1){throw 'Unexpected installer child; preserve partial upgrade.'}
        $root=@($observed.identities|Where-Object pid -EQ $observed.rootPid)
        $args='/S /D='+$Plan.target
        if($root.Count -ne 1 -or $root[0].executable -cne $Plan.new.installer.path -or $root[0].arguments -cne ('"'+$Plan.new.installer.path+'" '+$args)){throw 'Installer process coordinates differ.'}
        $after=& $Adapter.Snapshot;Assert-XnUpgradeNewState $before $after $Plan $extras
        & $Adapter.VerifyBackup $backup $before.profiles|Out-Null;& $Adapter.VerifyInputs|Out-Null
        $report.after=$after;$report.installedVerified=$true;$report.applicationLaunched=$false;$report.installationState='new-owned-installation-data-preserved';$report.status='passed-owned-upgrade-no-app-launch';Save 'verified'
    }catch{$report.status='stopped-preserve-state';$report.error=$_.Exception.Message;Save 'stopped';throw}
    return [pscustomobject]$report
}
