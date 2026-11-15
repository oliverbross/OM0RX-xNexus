# SPDX-License-Identifier: GPL-3.0-only
<#
Prepares evidence for WINDOWS_INSTALLER_ACCEPTANCE_PLAN.md. Never installs or
launches a process. Execution has a separate entry point bound to the exact
reviewed preparation SHA. A prepared report is not installation acceptance.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$BuildReceipt,
    [Parameter(Mandatory)][string]$AuditReceipt,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-fA-F]{64}$')][string]$ExpectedAuditSha256,
    [Parameter(Mandatory)][string]$Version,
    [Parameter(Mandatory)][string]$ExpectedCommit,
    [string]$EvidenceParent='C:\Projects\OM0RX-xLog\.build\xnexus-installer-acceptance',
    [switch]$Execute,
    [switch]$PassThru
)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'installer-acceptance-lib.ps1')
if ($Execute) { throw 'Use execute-installer-acceptance.ps1 with the exact reviewed preparation SHA and explicit -Execute. This preparation command never starts a process.' }
$repo='C:\Projects\OM0RX-xNexus'
$null=Assert-XnLocalPath $EvidenceParent -NoSpaces
if (!(Test-Path -LiteralPath $EvidenceParent -PathType Container)) { throw 'Create/review the private evidence parent separately; preparation never creates arbitrary ancestors.' }
Assert-XnPrivateEvidence $EvidenceParent -RequireProtected
$nonce=[guid]::NewGuid().ToString('N');$evidence=Join-Path $EvidenceParent "acceptance-$nonce";$target=Join-Path $evidence 'app'
Assert-XnFreshTarget $evidence; Assert-XnFreshTarget $target
$report=[ordered]@{schema=1;kind='xnexus-installer-preparation';id=$nonce;source=$ExpectedCommit;version=$Version;preparedUtc=[DateTime]::UtcNow.ToString('o');status='preparing';installed=$false;applicationLaunched=$false;target=$target;executionReady=$false}
# Create only private evidence. Never create/pre-seed the application target.
[IO.Directory]::CreateDirectory($evidence) | Out-Null
$receipt=Join-Path $evidence 'preparation.json'
function Save-Report { $report | ConvertTo-Json -Depth 15 | Set-Content -LiteralPath $receipt -Encoding utf8 }
Save-Report
try {
    Assert-XnHash $AuditReceipt $ExpectedAuditSha256
    $buildHash=Get-XnHash $BuildReceipt
    $build=Get-Content -LiteralPath $BuildReceipt -Raw | ConvertFrom-Json
    $audit=Get-Content -LiteralPath $AuditReceipt -Raw | ConvertFrom-Json
    Assert-XnReceiptIdentity $build $audit $Version $ExpectedCommit $buildHash
    $source=git -C $repo rev-parse HEAD
    if ($LASTEXITCODE -ne 0 -or $source.Trim() -cne $ExpectedCommit) { throw 'Canonical source must match the audited candidate.' }
    $dirty=git -C $repo status --porcelain
    if ($LASTEXITCODE -ne 0 -or $dirty) { throw 'Canonical source is dirty/unreadable.' }
    foreach ($artifact in @($audit.installer,$audit.executable,$audit.payloadExecutable)) { Assert-XnHash $artifact.path $artifact.sha256 }
    if ($build.distribution.policy -cne 'owned-root-nsis-retain-all-profiles') { throw 'Require the hardened distribution receipt.' }
    Assert-XnHash $build.distribution.configPath $build.distribution.configSha256
    foreach ($entry in $build.distribution.sourceHashes.PSObject.Properties) { Assert-XnHash (Join-Path $repo $entry.Name) $entry.Value }
    $config=Get-Content -LiteralPath $build.distribution.configPath -Raw | ConvertFrom-Json
    if ($config.identifier -cne 'sk.om0rx.xnexus' -or $config.bundle.windows.nsis.installMode -cne 'currentUser' -or
        $config.bundle.windows.webviewInstallMode.type -cne 'offlineInstaller') { throw 'Unreviewed identity/install/runtime policy.' }
    Assert-XnNoAssociations $config
    $generated=Join-Path $repo 'target\xnexus-windows\x86_64-pc-windows-gnu\release\nsis\x64\installer.nsi'
    Assert-XnHash $generated $build.generatedNsisSha256
    if ($audit.generatedNsisSha256 -ine $build.generatedNsisSha256) { throw 'Generated NSIS audit mismatch.' }
    $rendered=Get-Content -LiteralPath $generated -Raw
    foreach ($required in @('Call XNValidateCompletePayload','Call XNInvalidateOwnershipMarker','!define MUI_FINISHPAGE_RUN_NOTCHECKED','AllowSkipFiles off')) {
        if (!$rendered.Contains($required)) { throw "Missing hardened instruction: $required" }
    }
    $payloadRoot=Split-Path -Parent $audit.payloadExecutable.path
    $payload=@(foreach ($mapping in Get-XnPayloadMap $rendered) {
        $path=Join-Path $payloadRoot $mapping.destination;$hash=Get-XnHash $path
        if ($mapping.source) { Assert-XnHash (Resolve-XnPayloadSource $mapping.source $repo) $hash }
        elseif ($hash -ine $audit.payloadExecutableSha256) { throw 'Main payload hash mismatch.' }
        @{destination=$mapping.destination;sha256=$hash;extractedPath=$path}
    })
    $report.artifacts=@{buildReceipt=$BuildReceipt;buildReceiptSha256=$buildHash;auditReceipt=$AuditReceipt;auditSha256=$ExpectedAuditSha256;installer=$audit.installer;executable=$audit.executable;generatedNsisSha256=$build.generatedNsisSha256;payload=$payload}
    $registry=@(Get-XnRegistrySnapshot);$report.registry=$registry;Save-Report
    Assert-XnNamespaceEmpty $registry
    $processes=@(Get-CimInstance Win32_Process -Filter "Name='OM0RX-xNexus.exe'" -ErrorAction Stop | Select-Object ProcessId,ParentProcessId,ExecutablePath,CommandLine,CreationDate)
    $report.productProcesses=$processes
    if ($processes.Count) { throw 'A live product process prevents installation acceptance; do not close it.' }
    $default=Join-Path $env:LOCALAPPDATA 'OM0RX-xNexus';$null=Assert-XnLocalPath $default -AllowMissing
    if (Test-Path -LiteralPath $default) { throw 'Default installation namespace exists; preserve it.' }
    $report.defaultInstallAbsent=$default
    $shortcutRoots=@([Environment]::GetFolderPath('DesktopDirectory'),[Environment]::GetFolderPath('CommonDesktopDirectory'),[Environment]::GetFolderPath('StartMenu'),[Environment]::GetFolderPath('CommonStartMenu')) | Select-Object -Unique
    $shortcuts=@(foreach ($root in $shortcutRoots) { Get-XnShortcutManifest $root })
    $report.shortcuts=$shortcuts
    # Inspect existing link targets without executing them; filenames alone are insufficient.
    Assert-XnShortcutTargets $shortcuts
    $profiles=@(foreach ($base in @($env:APPDATA,$env:LOCALAPPDATA) | Select-Object -Unique) {
        $null=Assert-XnLocalPath $base
        foreach ($item in Get-ChildItem -LiteralPath $base -Force -ErrorAction Stop | Where-Object { $_.Name -like 'om0rx-xnexus*' -or $_.Name -eq 'sk.om0rx.xnexus' }) { Get-XnTreeManifest $item.FullName }
    })
    if ($env:XNEXUS_DATA_DIR) { $profiles+=Get-XnTreeManifest $env:XNEXUS_DATA_DIR }
    $report.profiles=$profiles
    # Require the x64 installer's first-choice machine32 registration; do not guess a fallback view.
    $base=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine,[Microsoft.Win32.RegistryView]::Registry32)
    try {
        $key=$base.OpenSubKey('SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',$false)
        try { $runtime=if ($key) { [string]$key.GetValue('pv') } else { '' } } finally { if ($key) { $key.Dispose() } }
    } finally { $base.Dispose() }
    $minimum=[regex]::Matches($rendered,'(?m)^!define MINIMUMWEBVIEW2VERSION "([^"]*)"\r?$')
    if ($minimum.Count -ne 1) { throw 'Cannot establish generated minimum WebView2 policy.' }
    Assert-XnRuntimeVersion $runtime $minimum[0].Groups[1].Value
    Assert-XnHash $audit.webview2.cachePath $audit.webview2.sha256
    $signature=Get-AuthenticodeSignature -LiteralPath $audit.webview2.cachePath
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '(^|, )O=Microsoft Corporation(,|$)') { throw 'Runtime cache signature is not valid Microsoft provenance.' }
    $report.webview2=@{registered=$runtime;minimum=$minimum[0].Groups[1].Value;cacheSha256=$audit.webview2.sha256;provisioningAllowed=$false}
    Assert-XnFreshTarget $target
    Assert-XnHash $AuditReceipt $ExpectedAuditSha256; Assert-XnHash $BuildReceipt $buildHash
    $report.transactionPlan=@{install=@{executable=$audit.installer.path;arguments="/S /NS /D=$target"};uninstall=@{executable=(Join-Path $target 'uninstall.exe');arguments='/S'};marker=@{file='.om0rx-xnexus-owner.ini';Product='sk.om0rx.xnexus';Format='1';Root=$target};sentinels=@('app\qualification-unknown.txt','app\qualification-nested\retained.txt','profiles\synthetic-a\draft.json','profiles\synthetic-b\outbox.json');sentinelCreation='Only after verified owned installation; never seed the fresh target';retention='Preserve all evidence, unknown files and external profiles; never recursive-delete or force-kill'}
    $report.status='prepared-not-executed'
    $report.executionBlockers=@('Package-owner release and exact reviewed preparation/audit SHA are required by the separate explicit executor.', 'Real candidate NSIS temporary handoff remains unqualified until actual execution; unknown process identities stop without forced cleanup.', 'Preparation does not reserve shared registration; executor repeats admission and validates payload, marker, registration and retained bytes around ordinary uninstall.')
    Save-Report
    if ($PassThru) { Write-Output $receipt } else { Write-Output "Preparation only: $receipt" }
} catch {
    $report.status='refused';$report.error=$_.Exception.Message;Save-Report;throw
}
