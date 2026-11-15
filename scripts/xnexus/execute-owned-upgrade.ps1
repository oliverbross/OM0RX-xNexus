# SPDX-License-Identifier: GPL-3.0-only
# Deliberately supports only the reviewed default-profile .4 -> .5 transition.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ReviewPlan,
    [Parameter(Mandatory)][ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ExpectedReviewSha256,
    [switch]$Execute
)
$ErrorActionPreference='Stop'
function Assert-RawPins {
    if((Get-FileHash -LiteralPath $ReviewPlan -Algorithm SHA256).Hash -ine $ExpectedReviewSha256){throw 'Review plan changed.'}
    if((Get-Item -LiteralPath $ReviewPlan).Length -gt 4MB){throw 'Oversized review plan.'}
    $review=Get-Content -LiteralPath $ReviewPlan -Raw|ConvertFrom-Json -AsHashtable
    $names=@('execute-owned-upgrade.ps1','upgrade-inputs.ps1','upgrade-transaction.ps1','installer-transaction.ps1','installer-acceptance-lib.ps1','installer-observer.ps1','installer-observer.cs')
    if($review.tools.Count -ne $names.Count){throw 'Incomplete reviewed helper inventory.'}
    foreach($name in $names){
        if(!$review.tools.ContainsKey($name) -or (Get-FileHash -LiteralPath (Join-Path $PSScriptRoot $name) -Algorithm SHA256).Hash -ine $review.tools[$name]){throw "Reviewed helper changed: $name"}
    }
    return $review
}
# Verify the complete import closure before executing any helper source.
$review=Assert-RawPins
. (Join-Path $PSScriptRoot 'upgrade-inputs.ps1')
. (Join-Path $PSScriptRoot 'installer-observer.ps1')
$context=Get-XnUpgradeContext $review
$evidence=Split-Path -Parent $ReviewPlan
$null=Assert-XnLocalPath $ReviewPlan -NoSpaces
if(!$evidence.StartsWith('C:\Projects\OM0RX-xLog\.build\xnexus-upgrade-acceptance\',[StringComparison]::Ordinal) -or
    (Split-Path -Leaf $evidence) -notmatch '^upgrade-[0-9a-f]{32}$' -or (Split-Path -Leaf $ReviewPlan) -cne 'review.json'){
    throw 'Use a fresh nonce directory under the reviewed private upgrade evidence root.'
}
Assert-XnPrivateEvidence $evidence -RequireProtected
if(!$Execute){Write-Output 'Source, artifacts and helper pins verified; no profile read, backup, process or installation started.';return}
if('Xnexus.InstallerObservation.Observer' -as [type]){throw 'Use a fresh PowerShell process; preloaded observer refused.'}
$claimPath=Join-Path $evidence 'upgrade.claim';$receiptPath=Join-Path $evidence 'upgrade.json'
if(Test-Path -LiteralPath $receiptPath){throw 'Preserve prior attempt; no automatic retry.'}
$claim=[IO.File]::Open($claimPath,'CreateNew','Write','None')
try {
    $claim.WriteByte(1);$claim.Flush($true)
    $links=@($context.old.shortcuts.path)
    $expectedLinks=@((Join-Path ([Environment]::GetFolderPath('Programs')) 'OM0RX-xNexus.lnk'),(Join-Path ([Environment]::GetFolderPath('DesktopDirectory')) 'OM0RX-xNexus.lnk'))
    Assert-XnSame @($expectedLinks|Sort-Object) @($links|Sort-Object) 'exact two current-user shortcut locations'
    function Assert-Closed {
        $processes=@(Get-CimInstance Win32_Process -Filter "Name='OM0RX-xNexus.exe' OR Name='msedgewebview2.exe'" -OperationTimeoutSec 3 -ErrorAction Stop)
        foreach($process in $processes){
            if($process.Name -ieq 'OM0RX-xNexus.exe'){throw 'Ordinary close of xNexus required; no termination attempted.'}
            if(!$process.CommandLine -or !$process.ExecutablePath){throw 'Cannot exclude an unidentified WebView owner.'}
            if($process.CommandLine.IndexOf($context.profile,[StringComparison]::OrdinalIgnoreCase) -ge 0){throw 'Companion WebView still live; wait for ordinary retirement.'}
        }
    }
    function Runtime {
        $base=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine,[Microsoft.Win32.RegistryView]::Registry32)
        try{$key=$base.OpenSubKey('SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',$false)
            try{if(!$key){throw 'Existing WebView2 is required; no provisioning authorized.'};$value=[string]$key.GetValue('pv')}finally{if($key){$key.Dispose()}}
        }finally{$base.Dispose()}
        Assert-XnRuntimeVersion $value $context.minimumWebview;return $value
    }
    function ProtectedRegistry {
        foreach($hive in 'CurrentUser','LocalMachine'){foreach($view in 'Registry32','Registry64'){
            $base=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::$hive,[Microsoft.Win32.RegistryView]::$view)
            try{foreach($path in 'Software\Classes\AppUserModelId\sk.om0rx.xnexus','Software\Classes\sk.om0rx.xnexus'){
                $key=$base.OpenSubKey($path,$false)
                try{if($key){throw 'Unexpected fork shell namespace.'};@{hive=$hive;view=$view;path=$path;exists=$false}}finally{if($key){$key.Dispose()}}
            }}finally{$base.Dispose()}
        }}
        # Exact fork Run values are already captured by Get-XnRegistrySnapshot.
    }
    function ProductShortcuts {
        $shell=New-Object -ComObject WScript.Shell;$shellApp=New-Object -ComObject Shell.Application
        try{foreach($path in $links){
            $null=Assert-XnLocalPath $path;$link=$shell.CreateShortcut($path);$folder=$null;$item=$null
            try{$folder=$shellApp.NameSpace((Split-Path -Parent $path));$item=$folder.ParseName((Split-Path -Leaf $path));$appId=$item.ExtendedProperty('System.AppUserModel.ID')
                if($link.TargetPath -ine (Join-Path $context.target 'OM0RX-xNexus.exe') -or $link.Arguments -cne '' -or $appId -cne 'sk.om0rx.xnexus'){throw 'Shortcut target/arguments/AppID mismatch.'}
                @{path=$path;sha256=(Get-XnHash $path);target=$link.TargetPath;arguments=$link.Arguments;appUserModelId=$appId}
            }finally{foreach($com in @($item,$folder,$link)){if($com){[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($com)}}}
        }}finally{[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shell);[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shellApp)}
    }
    function OtherShortcuts {
        # Only shortcut namespaces; no unrelated profile discovery or documents.
        foreach($root in @([Environment]::GetFolderPath('DesktopDirectory'),[Environment]::GetFolderPath('CommonDesktopDirectory'),[Environment]::GetFolderPath('StartMenu'),[Environment]::GetFolderPath('CommonStartMenu'))|Select-Object -Unique){
            $null=Assert-XnLocalPath $root -AllowMissing
            if(!(Test-Path -LiteralPath $root)){@{path=$root;exists=$false;files=@()};continue}
            $pending=[Collections.Generic.Stack[string]]::new();$pending.Push($root);$files=[Collections.Generic.List[object]]::new();$count=0
            while($pending.Count){
                $directory=$pending.Pop();if(++$count -gt 4096){throw 'Shortcut directory bound exceeded.'}
                foreach($item in Get-ChildItem -LiteralPath $directory -Force -ErrorAction Stop){
                    if($links -icontains $item.FullName){continue}
                    if($item.Name -match '(?i)xnexus'){throw 'Unexpected fork shortcut namespace.'}
                    if(!$item.PSIsContainer -and $item.Extension -notin '.lnk','.url'){continue}
                    $null=Assert-XnLocalPath $item.FullName
                    if($item.PSIsContainer){$pending.Push($item.FullName)}else{
                        if($files.Count -ge 20000){throw 'Shortcut file bound exceeded.'}
                        $files.Add(@{path=$item.FullName;bytes=$item.Length;sha256=(Get-XnHash $item.FullName)})
                    }
                }
            }
            @{path=$root;exists=$true;files=@($files.ToArray()|Sort-Object path)}
        }
    }
    function Installation {
        $target=$context.target;$tree=Get-XnUpgradeTree $target
        $marker=Join-Path $target '.om0rx-xnexus-owner.ini';$uninstaller=Join-Path $target 'uninstall.exe'
        if((Get-Item -LiteralPath $marker).Length -gt 4096){throw 'Ownership marker oversized.'}
        $stream=[IO.File]::OpenRead($uninstaller);$pe=$false
        try{$reader=[IO.BinaryReader]::new($stream)
            if($reader.ReadUInt16() -eq 0x5a4d -and $stream.Length -gt 64){$stream.Position=60;$offset=$reader.ReadUInt32();if($offset -ge 64 -and $offset -lt ($stream.Length-4)){$stream.Position=$offset;$pe=$reader.ReadUInt32() -eq 0x4550}}
        }finally{$stream.Dispose()}
        @{markerText=(Get-Content -LiteralPath $marker -Raw);markerSha256=(Get-XnHash $marker);uninstallerSha256=(Get-XnHash $uninstaller);uninstallerPe=$pe;directories=@($tree.directories);files=@($tree.files|ForEach-Object{@{relative=$_.path.Substring($target.Length+1);sha256=$_.sha256}})}
    }
    function Snapshot {
        Assert-Closed
        $profile=Get-XnUpgradeTree $context.profile
        if(!$profile.exists -or !(Test-Path -LiteralPath $context.webview -PathType Container)){throw 'Previously verified current companion profile/WebView root is absent.'}
        $state=@{productProcesses=@();installation=(Installation);profiles=@($profile);shortcuts=@(ProductShortcuts);otherShortcuts=@(OtherShortcuts);runtime=(Runtime);protectedRegistry=@(ProtectedRegistry);registry=@(Get-XnRegistrySnapshot)}
        Assert-Closed;return $state
    }
    $adapter=@{
        VerifyInputs={
            $freshReview=Assert-RawPins;Assert-XnPrivateEvidence $evidence -RequireProtected
            Assert-XnSame $context (Get-XnUpgradeContext $freshReview) 'reviewed upgrade context'
            Assert-Closed
        }
        Snapshot={Snapshot}
        Backup={param($trees) Copy-XnUpgradeProfileBackup $trees (Join-Path $evidence 'profile-backup')}
        VerifyBackup={param($backup,$trees) Assert-XnUpgradeBackup $backup $trees}
        Install={
            # Final check directly before dispatch; never launch the product.
            Assert-Closed;Assert-XnHash $context.new.installer.path $context.new.installer.sha256
            Invoke-XnProcessObservation -Executable $context.new.installer.path -ExpectedSha256 $context.new.installer.sha256 -Arguments ('/S /D='+$context.target) -WorkingDirectory $evidence -ReceiptPath (Join-Path $evidence 'installer-processes.json') -TimeoutSeconds 180
        }
        Save={param($report) $report.reviewSha256=$ExpectedReviewSha256;$report.updatedUtc=[DateTime]::UtcNow.ToString('o');$report|ConvertTo-Json -Depth 30|Set-Content -LiteralPath $receiptPath -Encoding utf8NoBOM}
    }
    $null=Invoke-XnUpgradeTransaction $context $adapter
    Write-Output "Verified owned upgrade; application remains closed. Receipt: $receiptPath"
}finally{$claim.Dispose()}
