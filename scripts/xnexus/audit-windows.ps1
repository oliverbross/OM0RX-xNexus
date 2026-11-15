# SPDX-License-Identifier: GPL-3.0-only
# Reads/extracts the produced artifacts. Never launches or modifies an application.
param(
    [Parameter(Mandatory=$true)][string]$Version,
    [Parameter(Mandatory=$true)][string]$ExpectedCommit,
    [string]$RepositoryRoot='C:\Projects\OM0RX-xNexus',
    [string]$Mt='C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\mt.exe',
    [string]$SevenZip='C:\Program Files\7-Zip\7z.exe',
    [string]$Objdump='C:\msys64\ucrt64\bin\objdump.exe',
    [string]$CliProcessReceipt=''
)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path -LiteralPath $RepositoryRoot).Path
if($repo -ne 'C:\Projects\OM0RX-xNexus') { throw 'Audit the canonical Windows build only.' }
if($Version -notmatch '^\d+\.\d+\.\d+-xlog\.\d+$' -or $ExpectedCommit -notmatch '^[0-9a-f]{40}$') {
    throw 'Require the exact fork candidate version and full source commit.'
}
function Hash([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash }
function CheckSource {
    if((git -C $repo rev-parse HEAD).Trim() -ne $ExpectedCommit -or (git -C $repo status --porcelain)) {
        throw 'Canonical source no longer matches the clean packaged candidate.'
    }
}
function Run([string]$Program,[string[]]$Arguments,[string]$Log) {
    & $Program @Arguments *> $Log
    if($LASTEXITCODE -ne 0) { throw "Audit tool failed: $Program; see $Log" }
}
CheckSource
$receiptPath=Join-Path $repo "target\xnexus-provenance\$Version.json"
$build=Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
if($build.source -ne $ExpectedCommit -or $build.version -ne $Version -or
   $build.status -ne 'packaged-unsigned-not-installed' -or $build.signed -or $build.installed) {
    throw 'Require the completed unsigned, uninstalled build receipt for this exact candidate.'
}
if(@($build.artifacts).Count -ne 1) { throw 'Expected exactly one NSIS installer.' }
$release=Join-Path $repo 'target\xnexus-windows\x86_64-pc-windows-gnu\release'
$executable=Join-Path $release 'OM0RX-xNexus.exe'
$installer=(Resolve-Path -LiteralPath $build.artifacts[0].path).Path
$bundleRoot=(Resolve-Path -LiteralPath (Join-Path $release 'bundle\nsis')).Path+'\'
if(!$installer.StartsWith($bundleRoot,[StringComparison]::OrdinalIgnoreCase) -or
   [IO.Path]::GetExtension($installer) -ne '.exe') { throw 'Installer is outside the canonical NSIS output.' }
if((Hash $executable) -ne $build.executableSha256 -or
   (Hash $installer) -ne $build.artifacts[0].sha256) { throw 'Built artifact hash differs from the receipt.' }
foreach($tool in @($Mt,$SevenZip,$Objdump)) { if(!(Test-Path -LiteralPath $tool -PathType Leaf)) { throw "Missing audit tool: $tool" } }
$evidence=Join-Path $repo "target\xnexus-audit\$Version"
if(Test-Path -LiteralPath $evidence) { throw 'Preserve the existing audit directory; choose a new candidate for changed artifacts.' }
$audit=[ordered]@{
    version=$Version;source=$ExpectedCommit;startedUtc=[DateTime]::UtcNow.ToString('o');status='auditing'
    buildReceiptSha256=(Hash $receiptPath);applicationLaunched=$false;installed=$false
    executable=@{path=$executable;sha256=(Hash $executable)}
    installer=@{path=$installer;sha256=(Hash $installer);bytes=(Get-Item -LiteralPath $installer).Length}
    tools=@($Mt,$SevenZip,$Objdump | ForEach-Object { @{path=$_;sha256=(Hash $_)} })
    auditScriptSha256=(Hash $PSCommandPath)
}
$cliRoot=Join-Path $repo 'target\xnexus-tools\node_modules\@tauri-apps'
foreach($package in @('cli','cli-win32-x64-msvc')) {
    $cliVersion=(Get-Content -LiteralPath (Join-Path $cliRoot "$package\package.json") -Raw | ConvertFrom-Json).version
    if($cliVersion -ne '2.11.5') { throw 'Review bundle-marker verification for the changed Tauri CLI.' }
}
$cliFiles=@('cli\package.json','cli\tauri.js','cli\main.js','cli\index.js',
    'cli-win32-x64-msvc\package.json','cli-win32-x64-msvc\cli.win32-x64-msvc.node')
$audit.tauriCliFiles=@($cliFiles | ForEach-Object {
    $path=Join-Path $cliRoot $_
    @{path=$path;sha256=(Hash $path)}
})
if($CliProcessReceipt) {
    $processEvidence=Get-Content -LiteralPath $CliProcessReceipt -Raw | ConvertFrom-Json
    $nativeCli=Join-Path $cliRoot 'cli-win32-x64-msvc\cli.win32-x64-msvc.node'
    $actual=@($processEvidence.modules | Where-Object path -eq $nativeCli)
    if($actual.Count -ne 1 -or $actual[0].sha256 -ne (Hash $nativeCli) -or
       !$processEvidence.command.Contains((Join-Path $repo 'target\xnexus-tools'))) {
        throw 'Live package CLI evidence does not match the audited native CLI.'
    }
    $audit.tauriCliProcess=@{evidencePath=$CliProcessReceipt;sha256=(Hash $CliProcessReceipt);process=$processEvidence}
}
New-Item -ItemType Directory -Path $evidence -Force | Out-Null
$auditPath=Join-Path $evidence 'receipt.json'
function Save { $audit | ConvertTo-Json -Depth 7 | Set-Content -LiteralPath $auditPath -Encoding utf8 }
Save
try {
    $manifest=Join-Path $evidence 'application.manifest'
    Run $Mt @('-nologo',"-inputresource:$executable;#1","-out:$manifest") (Join-Path $evidence 'manifest-tool.log')
    [xml]$xml=Get-Content -LiteralPath $manifest -Raw
    $controls=$xml.SelectSingleNode("//*[local-name()='assemblyIdentity' and @name='Microsoft.Windows.Common-Controls' and @version='6.0.0.0']")
    if(!$controls) { throw 'Application lacks the Common-Controls v6 activation manifest.' }
    $audit.commonControlsV6=$true
    $audit.manifestSha256=(Hash $manifest)
    Run $Objdump @('-p',$executable) (Join-Path $evidence 'pe-imports.log')
    $imports=@(Get-Content -LiteralPath (Join-Path $evidence 'pe-imports.log') | Select-String 'DLL Name:' | ForEach-Object {$_.Line.Trim()})
    if(!$imports -or $imports -match 'DLL Name:.*lib(gfortran|stdc\+\+|quadmath|winpthread|gcc|fftw|gomp)') {
        throw 'Missing import evidence or dependency on a builder-only GNU runtime DLL.'
    }
    $audit.executable.imports=$imports
    $info=(Get-Item -LiteralPath $executable).VersionInfo
    if($info.ProductName -ne 'OM0RX-xNexus') { throw 'Executable product identity is not the fork.' }
    if($info.ProductVersion -ne $Version -or $info.FileVersion -ne $Version) {
        throw 'Executable string version resources do not match the exact candidate.'
    }
    $audit.executable.productName=$info.ProductName
    $audit.executable.productVersion=$info.ProductVersion
    $audit.executable.fileVersion=$info.FileVersion
    $audit.executable.signature=(Get-AuthenticodeSignature -LiteralPath $executable).Status.ToString()
    $audit.installer.signature=(Get-AuthenticodeSignature -LiteralPath $installer).Status.ToString()
    Run $SevenZip @('l','-slt',$installer) (Join-Path $evidence 'installer-list.log')
    $payload=Join-Path $evidence 'payload'
    Run $SevenZip @('x','-y',"-o$payload",$installer) (Join-Path $evidence 'installer-extract.log')
    $files=@(Get-ChildItem -LiteralPath $payload -Recurse -File)
    $config=Get-Content -LiteralPath (Join-Path $repo 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
    if($config.bundle.windows.webviewInstallMode.type -ne 'offlineInstaller') {
        throw 'Review the audit for this changed WebView2 installation mode.'
    }
    $nsisScript=Join-Path $release 'nsis\x64\installer.nsi'
    if($build.distribution) {
        if((Hash $nsisScript) -ne $build.generatedNsisSha256) { throw 'Generated hardened NSIS source changed after packaging.' }
        if((Hash $build.distribution.configPath).ToLowerInvariant() -ne $build.distribution.configSha256) { throw 'Distribution config differs from the packaged receipt.' }
        foreach($entry in $build.distribution.sourceHashes.PSObject.Properties) {
            if((Hash (Join-Path $repo $entry.Name)).ToLowerInvariant() -ne $entry.Value) { throw "Distribution source mismatch: $($entry.Name)" }
        }
        $rendered=Get-Content -LiteralPath $nsisScript -Raw
        foreach($required in @('Function XNValidateCompletePayload','Call XNInvalidateOwnershipMarker','AllowSkipFiles off','!define MUI_FINISHPAGE_RUN_NOTCHECKED')) {
            if(!$rendered.Contains($required)) { throw "Generated installer lacks hardened policy: $required" }
        }
        $audit.distribution=$build.distribution
        $audit.generatedNsisSha256=(Hash $nsisScript)
    }
    $runtimeDefinition=[regex]::Match((Get-Content -LiteralPath $nsisScript -Raw),
        '(?m)^!define WEBVIEW2INSTALLERPATH "([^"]+)"')
    if(!$runtimeDefinition.Success) { throw 'Cannot identify the exact offline WebView2 input.' }
    $runtimeCache=$runtimeDefinition.Groups[1].Value
    $runtimeHash=Hash $runtimeCache
    $runtimePayload=@($files | Where-Object Name -eq 'MicrosoftEdgeWebView2RuntimeInstaller.exe')
    if($runtimePayload.Count -ne 1 -or (Hash $runtimePayload[0].FullName) -ne $runtimeHash) {
        throw 'Embedded WebView2 runtime does not match the retained download.'
    }
    $runtimeSignature=Get-AuthenticodeSignature -LiteralPath $runtimePayload[0].FullName
    if($runtimeSignature.Status -ne 'Valid' -or
       $runtimeSignature.SignerCertificate.Subject -notmatch '(^|, )O=Microsoft Corporation(,|$)') {
        throw 'Embedded WebView2 runtime lacks a valid Microsoft signature.'
    }
    $audit.webview2=@{mode='offlineInstaller';cachePath=$runtimeCache;sha256=$runtimeHash
        payloadPath=$runtimePayload[0].FullName;bytes=$runtimePayload[0].Length
        signature=$runtimeSignature.Status.ToString();signer=$runtimeSignature.SignerCertificate.Subject
        certificateThumbprint=$runtimeSignature.SignerCertificate.Thumbprint
        productVersion=$runtimePayload[0].VersionInfo.ProductVersion;nsisScriptSha256=(Hash $nsisScript)}
    $apps=@($files | Where-Object Name -eq 'OM0RX-xNexus.exe')
    if($apps.Count -ne 1) { throw 'Expected exactly one executable payload.' }
    $markerVerifier=Join-Path $PSScriptRoot 'windows-bundle-marker.cs'
    Add-Type -Path $markerVerifier
    $markerOffset=[XNexusWindowsBundleMarker]::VerifyFiles($executable,$apps[0].FullName)
    $audit.bundleMarker=@{offset=$markerOffset;changedBytes=3;standalone='__TAURI_BUNDLE_TYPE_VAR_UNK'
        payload='__TAURI_BUNDLE_TYPE_VAR_NSS';allOtherBytesIdentical=$true;verifierSha256=(Hash $markerVerifier)
        cliSource='https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.5/crates/tauri-bundler/src/bundle.rs'}
    $expected=@()
    foreach($property in $build.modelHashes.PSObject.Properties) { $expected+=@{name=$property.Name;sha256=$property.Value} }
    $expected+=@($build.hamlibHashes)
    $expected+=@{name='WebView2Loader.dll';sha256=(Hash (Join-Path $release 'WebView2Loader.dll'))}
    foreach($name in @('COPYING','NOTICE')) { $expected+=@{name=$name;sha256=(Hash (Join-Path $repo $name))} }
    if($build.distribution) { $expected+=@{name='LICENSE-MIT.tauri';sha256=(Hash (Join-Path $repo 'scripts\xnexus\nsis\LICENSE-MIT.tauri'))} }
    $verified=@()
    foreach($item in $expected) {
        $payloadMatches=@($files | Where-Object Name -eq $item.name | Where-Object {(Hash $_.FullName) -eq $item.sha256})
        if(!$payloadMatches) { throw "Installer lacks the expected resource/license hash: $($item.name)" }
        $verified+=@{name=$item.name;sha256=$item.sha256;payloadPaths=@($payloadMatches.FullName)}
    }
    $audit.verifiedResources=$verified
    # Verify all mapped notice/license/provenance text at its exact destination.
    # Source hashes were captured before the build; basename-only matches cannot
    # satisfy this check (several resources are named THIRD-PARTY.txt).
    if (!$build.distribution.noticeResources) { throw 'Build receipt lacks mapped notice coverage; do not retrofit an old candidate receipt.' }
    $noticeReport=Join-Path $evidence 'notice-resources.json'
    Run 'node' @((Join-Path $PSScriptRoot 'notice-resources.mjs'),$repo,$build.distribution.configPath,$payload,$receiptPath,$noticeReport) (Join-Path $evidence 'notice-resources.log')
    $audit.noticeResources=@{reportPath=$noticeReport;sha256=(Hash $noticeReport);helperSha256=(Hash (Join-Path $PSScriptRoot 'notice-resources.mjs'))}
    $audit.payloadExecutableSha256=(Hash $apps[0].FullName)
    $payloadInfo=$apps[0].VersionInfo
    $audit.payloadExecutable=@{path=$apps[0].FullName;bytes=$apps[0].Length;sha256=$audit.payloadExecutableSha256
        productName=$payloadInfo.ProductName;productVersion=$payloadInfo.ProductVersion;fileVersion=$payloadInfo.FileVersion
        signature=(Get-AuthenticodeSignature -LiteralPath $apps[0].FullName).Status.ToString()}
    CheckSource
    if((Hash $executable) -ne $build.executableSha256 -or
       (Hash $installer) -ne $build.artifacts[0].sha256) { throw 'Artifacts changed during read-only audit.' }
    $audit.status='passed-artifact-audit-not-launched-not-installed'
    $audit.completedUtc=[DateTime]::UtcNow.ToString('o'); Save
    Write-Output "Artifact audit receipt: $auditPath"
} catch {
    $audit.status='failed';$audit.error=$_.Exception.Message;Save;throw
}
