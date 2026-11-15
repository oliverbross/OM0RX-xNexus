# SPDX-License-Identifier: GPL-3.0-only
# Explicit candidate transaction. Default mode only validates the review inputs.
# No fake adapter or registry override is accepted by this production entry point.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$PreparedReceipt,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-fA-F]{64}$')][string]$ExpectedPreparationSha256,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-fA-F]{64}$')][string]$ExpectedAuditSha256,
    [switch]$Execute
)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'installer-transaction.ps1')
. (Join-Path $PSScriptRoot 'installer-observer.ps1')
Assert-XnHash $PreparedReceipt $ExpectedPreparationSha256
$plan=Get-Content -LiteralPath $PreparedReceipt -Raw | ConvertFrom-Json
if ($plan.schema -ne 1 -or $plan.kind -cne 'xnexus-installer-preparation' -or $plan.status -cne 'prepared-not-executed' -or
    $plan.installed -cne $false -or $plan.applicationLaunched -cne $false -or $plan.artifacts.auditSha256 -ine $ExpectedAuditSha256 -or
    $plan.id -notmatch '^[0-9a-f]{32}$') { throw 'Require the exact reviewed preparation and audit identities.' }
$evidence=Split-Path -Parent $PreparedReceipt
if ((Split-Path -Leaf $evidence) -cne "acceptance-$($plan.id)" -or $PreparedReceipt -cne (Join-Path $evidence 'preparation.json') -or
    $plan.target -cne (Join-Path $evidence 'app')) { throw 'Preparation target escaped its exact marked evidence directory.' }
$null=Assert-XnLocalPath $evidence -NoSpaces;Assert-XnFreshTarget $plan.target
Assert-XnHash $plan.artifacts.auditReceipt $ExpectedAuditSha256
Assert-XnHash $plan.artifacts.buildReceipt $plan.artifacts.buildReceiptSha256
if (!$Execute) { Write-Output 'Review inputs verified; no process, installation or registry mutation. Pass -Execute only after package-owner release.'; return }
Assert-XnPrivateEvidence (Split-Path -Parent $evidence) -RequireProtected
Assert-XnPrivateEvidence $evidence
$receiptPath=Join-Path $evidence 'transaction.json'
if (Test-Path -LiteralPath $receiptPath) { throw 'A transaction already exists; preserve it, never resume or retry automatically.' }
# An exclusive CreateNew journal prevents accidental duplicate invocation. The
# marker is retained on every outcome, including preflight failure.
$claim=[IO.File]::Open((Join-Path $evidence 'transaction.claim'),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
try {
    $claim.WriteByte(1);$claim.Flush()
    $freshPath=& (Join-Path $PSScriptRoot 'prepare-installer-acceptance.ps1') -BuildReceipt $plan.artifacts.buildReceipt -AuditReceipt $plan.artifacts.auditReceipt -ExpectedAuditSha256 $ExpectedAuditSha256 -Version $plan.version -ExpectedCommit $plan.source -EvidenceParent (Split-Path -Parent $evidence) -PassThru
    $fresh=Get-Content -LiteralPath $freshPath -Raw | ConvertFrom-Json
    foreach ($key in 'source','version','artifacts','registry','profiles','shortcuts','webview2','defaultInstallAbsent') { Assert-XnSame $plan.$key $fresh.$key "reviewed preparation $key" }
    Assert-XnHash $PreparedReceipt $ExpectedPreparationSha256
    $audit=Get-Content -LiteralPath $plan.artifacts.auditReceipt -Raw | ConvertFrom-Json
    $repo='C:\Projects\OM0RX-xNexus'
    $generated=Join-Path $repo 'target\xnexus-windows\x86_64-pc-windows-gnu\release\nsis\x64\installer.nsi'
    $rendered=Get-Content -LiteralPath $generated -Raw
    if ($rendered -notmatch '(?m)^!define BUNDLEID "sk\.om0rx\.xnexus"\r?$' -or
        $rendered -notmatch '(?m)^!define ARCH "x64"\r?$') { throw 'Fork AppUserModelID/x64 context is not proven in generated source.' }
    $utils=Join-Path (Split-Path -Parent $generated) 'utils.nsh'
    $utilsText=Get-Content -LiteralPath $utils -Raw
    $deleteList=[regex]::Match($utilsText,'(?ms)^!macro DeleteAppUserModelId\r?\n(.*?)^!macroend')
    if (!$deleteList.Success -or !$deleteList.Value.Contains('${BUNDLEID}') -or $deleteList.Value -match 'com\.kd9taw|tempo') { throw 'Unreviewed shared destination-list identity.' }
    $utilsHash=Get-XnHash $utils

    function Get-RegistryFingerprint([string]$Hive,[string]$View,[string]$Path,[string]$Exclude='') {
        $base=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::$Hive,[Microsoft.Win32.RegistryView]::$View)
        try {
            $key=$base.OpenSubKey($Path,$false)
            try {
                if (!$key) { return @{hive=$Hive;view=$View;path=$Path;exists=$false;values=@();subkeys=@()} }
                $names=@($key.GetValueNames() | Where-Object { $_ -cne $Exclude } | Sort-Object)
                if ($names.Count -gt 256 -or $key.SubKeyCount -gt 256) { throw 'Relevant registry bound exceeded.' }
                $values=@(foreach ($name in $names) {
                    $json=ConvertTo-XnCanonicalJson ($key.GetValue($name,$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames))
                    $hash=[Security.Cryptography.SHA256]::Create()
                    try { @{name=$name;kind=$key.GetValueKind($name).ToString();valueSha256=([BitConverter]::ToString($hash.ComputeHash([Text.Encoding]::UTF8.GetBytes($json))).Replace('-','').ToLowerInvariant())} } finally { $hash.Dispose() }
                })
                @{hive=$Hive;view=$View;path=$Path;exists=$true;values=$values;subkeys=@($key.GetSubKeyNames() | Sort-Object)}
            } finally { if ($key) { $key.Dispose() } }
        } finally { $base.Dispose() }
    }
    function Get-CurrentRuntime {
        $base=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine,[Microsoft.Win32.RegistryView]::Registry32)
        try {
            $key=$base.OpenSubKey('SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',$false)
            try { if (!$key) { throw 'Registered WebView2 disappeared.' };$value=[string]$key.GetValue('pv') } finally { if ($key) { $key.Dispose() } }
        } finally { $base.Dispose() }
        Assert-XnRuntimeVersion $value $plan.webview2.minimum
        $value
    }
    function Read-Snapshot {
        $productProcesses=@(Get-CimInstance Win32_Process -Filter "Name='OM0RX-xNexus.exe'" -OperationTimeoutSec 3 -ErrorAction Stop | Select-Object ProcessId,ParentProcessId,ExecutablePath,CommandLine,CreationDate)
        $profiles=@(foreach ($base in @($env:APPDATA,$env:LOCALAPPDATA) | Select-Object -Unique) {
            foreach ($item in Get-ChildItem -LiteralPath $base -Force -ErrorAction Stop | Where-Object { $_.Name -like 'om0rx-xnexus*' -or $_.Name -eq 'sk.om0rx.xnexus' }) { Get-XnTreeManifest $item.FullName }
        })
        if ($env:XNEXUS_DATA_DIR) { $profiles+=Get-XnTreeManifest $env:XNEXUS_DATA_DIR }
        $shortcuts=@(foreach ($tree in $plan.shortcuts) { Get-XnShortcutManifest $tree.path })
        if (Test-Path -LiteralPath $plan.defaultInstallAbsent) { throw 'Default installation namespace appeared.' }
        $other=@(foreach ($hive in 'CurrentUser','LocalMachine') { foreach ($view in 'Registry32','Registry64') {
            Get-RegistryFingerprint $hive $view 'Software\Microsoft\Windows\CurrentVersion\Run' 'OM0RX-xNexus'
        } })
        $shell=@(foreach ($hive in 'CurrentUser','LocalMachine') { foreach ($view in 'Registry32','Registry64') {
            Get-RegistryFingerprint $hive $view 'Software\Classes\AppUserModelId\sk.om0rx.xnexus'
            Get-RegistryFingerprint $hive $view 'Software\Classes\sk.om0rx.xnexus'
        } })
        foreach ($entry in $shell) { if ($entry.exists) { throw 'Existing or unexpected fork shell/AppID namespace; preserve it.' } }
        # Never collect or parse global Recent histories. The reviewed NSIS helper
        # uses only the proven fork AppID; actual destination-list runtime effects
        # remain a recorded candidate-specific boundary, not fabricated evidence.
        [pscustomobject]@{registry=@(Get-XnRegistrySnapshot);productProcesses=$productProcesses;profiles=$profiles;shortcuts=$shortcuts;runtime=(Get-CurrentRuntime);otherRegistry=$other;forkShellMetadata=$shell;defaultInstallAbsent=$plan.defaultInstallAbsent}
    }
    function Verify-Inputs([string]$Stage) {
        Assert-XnPrivateEvidence $evidence
        Assert-XnHash $PreparedReceipt $ExpectedPreparationSha256
        Assert-XnHash $plan.artifacts.auditReceipt $ExpectedAuditSha256
        Assert-XnHash $plan.artifacts.buildReceipt $plan.artifacts.buildReceiptSha256
        Assert-XnHash $generated $plan.artifacts.generatedNsisSha256;Assert-XnHash $utils $utilsHash
        Assert-XnHash $plan.artifacts.installer.path $plan.artifacts.installer.sha256
        Assert-XnHash $audit.webview2.cachePath $audit.webview2.sha256
        if ((git -C $repo rev-parse HEAD).Trim() -cne $plan.source -or $LASTEXITCODE -ne 0) { throw 'Canonical source changed.' }
        $dirty=git -C $repo status --porcelain;if ($LASTEXITCODE -ne 0 -or $dirty) { throw 'Canonical source is dirty/unreadable.' }
        if ($Stage -eq 'before-install') { Assert-XnFreshTarget $plan.target }
        else { $null=Assert-XnLocalPath $plan.target -NoSpaces }
    }
    function Inspect-Installation {
        $tree=Get-XnTreeManifest $plan.target
        if (!$tree.exists) { throw 'Installation root is absent.' }
        $marker=Join-Path $plan.target '.om0rx-xnexus-owner.ini';$uninstaller=Join-Path $plan.target 'uninstall.exe'
        if ((Get-Item -LiteralPath $marker).Length -gt 4096) { throw 'Ownership marker is oversized.' }
        $pe=$false;$stream=[IO.File]::OpenRead($uninstaller)
        try {
            $reader=[IO.BinaryReader]::new($stream)
            if ($reader.ReadUInt16() -eq 0x5a4d -and $stream.Length -gt 64) {
                $stream.Position=60;$offset=$reader.ReadUInt32()
                if ($offset -ge 64 -and $offset -lt ($stream.Length-4)) { $stream.Position=$offset;$pe=$reader.ReadUInt32() -eq 0x00004550 }
            }
        } finally { $stream.Dispose() }
        @{markerText=(Get-Content -LiteralPath $marker -Raw);markerSha256=(Get-XnHash $marker);uninstallerSha256=(Get-XnHash $uninstaller);uninstallerPe=$pe;files=@($tree.files | ForEach-Object { @{relative=$_.path.Substring($plan.target.Length+1);sha256=$_.sha256} })}
    }
    function Observe-Installer([string]$Exe,[string]$Hash,[string]$Arguments,[string]$Name) {
        $observation=Invoke-XnProcessObservation -Executable $Exe -ExpectedSha256 $Hash -Arguments $Arguments -WorkingDirectory $evidence -ReceiptPath (Join-Path $evidence "$Name-processes.json") -TimeoutSeconds 180
        Assert-XnObservedRetirement $observation $Hash
        $root=@($observation.identities | Where-Object pid -eq $observation.rootPid)
        if ($root.Count -ne 1 -or $root[0].executable -ine $Exe -or $root[0].arguments -cne ('"'+$Exe+'" '+$Arguments)) { throw 'Root launch coordinates differ.' }
        foreach ($child in $observation.identities | Where-Object pid -ne $observation.rootPid) {
            # Normal NSIS uninstall copies itself to TEMP. Same audited generated
            # bytes and the retained install-root argument are mandatory; unknown
            # helper forms stop rather than silently extending the allowlist.
            $temp=[IO.Path]::GetFullPath($env:TEMP).TrimEnd('\')+'\'
            if ($Name -ne 'uninstall' -or !$child.executable.StartsWith($temp,[StringComparison]::OrdinalIgnoreCase) -or
                $child.arguments -cne ('"'+$child.executable+'" /S _?='+$plan.target)) { throw 'Unreviewed NSIS temporary handoff identity/arguments.' }
        }
        $observation
    }
    $adapter=@{
        VerifyInputs={param($stage) Verify-Inputs $stage}
        Snapshot={ Read-Snapshot }
        Install={
            # Recheck namespace and retained trees immediately before dispatch,
            # not merely before computing the earlier preparation manifest.
            Verify-Inputs 'before-install'
            $admission=Read-Snapshot;Assert-XnNamespaceEmpty $admission.registry
            if (@($admission.productProcesses).Count) { throw 'A product process appeared before dispatch.' }
            Assert-XnSame $plan.profiles $admission.profiles 'reviewed profiles before dispatch'
            Assert-XnSame $plan.shortcuts $admission.shortcuts 'reviewed shortcuts before dispatch'
            Assert-XnSame $plan.webview2.registered $admission.runtime 'reviewed WebView2 before dispatch'
            Observe-Installer $plan.artifacts.installer.path $plan.artifacts.installer.sha256 "/S /NS /D=$($plan.target)" 'install'
        }
        InspectInstallation={ Inspect-Installation }
        CreateSentinels={
            foreach ($relative in $plan.transactionPlan.sentinels) {
                $path=Join-Path $evidence $relative;$null=Assert-XnLocalPath $path -AllowMissing -NoSpaces
                if (!$path.StartsWith($evidence+'\',[StringComparison]::Ordinal)) { throw 'Sentinel escaped evidence.' }
                [IO.Directory]::CreateDirectory((Split-Path -Parent $path)) | Out-Null
                $bytes=[Text.Encoding]::UTF8.GetBytes("INERT synthetic installer qualification data; retain $($plan.id) $relative`n")
                $file=[IO.File]::Open($path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
                try { $file.Write($bytes,0,$bytes.Length) } finally { $file.Dispose() }
                @{path=$path;sha256=(Get-XnHash $path);relativeToApp=$(if ($relative.StartsWith('app\')) { $relative.Substring(4) } else { $null })}
            }
        }
        Uninstall={param($hash) Observe-Installer (Join-Path $plan.target 'uninstall.exe') $hash '/S' 'uninstall'}
        VerifyRetention={param($sentinels)
            foreach ($sentinel in $sentinels) { Assert-XnHash $sentinel.path $sentinel.sha256 }
            foreach ($relative in @($plan.artifacts.payload.destination)+@('.om0rx-xnexus-owner.ini','uninstall.exe')) {
                if (Test-Path -LiteralPath (Join-Path $plan.target $relative)) { throw 'Enumerated owned payload was not removed.' }
            }
            $remaining=Get-XnTreeManifest $plan.target
            $expected=@($sentinels | Where-Object relativeToApp)
            if (@($remaining.files).Count -ne $expected.Count) { throw 'Unexpected residual installation file.' }
            foreach ($file in $remaining.files) { if (@($expected | Where-Object { $_.path -ceq $file.path -and $_.sha256 -ceq $file.sha256 }).Count -ne 1) { throw 'Residual unknown file differs.' } }
        }
        Save={param($report)
            $report['preparationReceiptSha256']=$ExpectedPreparationSha256;$report['freshPreflightReceipt']=$freshPath
            $report['utilsSha256']=$utilsHash;$report['shellHistoryScope']='Fork AppID source/metadata checked; no unrelated Recent history read or claimed unchanged'
            $report | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $receiptPath -Encoding utf8
        }
    }
    Invoke-XnInstallerTransaction $plan $adapter | Out-Null
    Write-Output "Transaction receipt: $receiptPath"
} catch {
    # Failures before the orchestration begins also leave a diagnostic receipt.
    if (!(Test-Path -LiteralPath $receiptPath)) { @{status='stopped-before-transaction';error=$_.Exception.ToString();installed=$false;applicationLaunchRequested=$false;forcedCleanup=$false} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $receiptPath -Encoding utf8 }
    throw
} finally { $claim.Dispose() }
