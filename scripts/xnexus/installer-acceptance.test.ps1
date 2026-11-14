# SPDX-License-Identifier: GPL-3.0-only
# Inert filesystem/DTO controls only. Never invokes an installer or registry write.
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'installer-acceptance-lib.ps1')
$repo=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$fixture=Join-Path $repo ('target\installer-acceptance-tests\'+[guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($fixture) | Out-Null
$passed=[Collections.Generic.List[string]]::new()
function Pass([string]$Name,[scriptblock]$Body) { & $Body; $passed.Add($Name) }
function Reject([string]$Name,[scriptblock]$Body,[string]$Reason='') {
    $caught=$null
    try { & $Body | Out-Null } catch { $caught=$_.Exception.Message }
    if (!$caught -or ($Reason -and $caught -notlike "*$Reason*")) { throw "Expected refusal ($Name / $Reason), got: $caught" }
    $passed.Add($Name)
}
Pass 'canonical local existing path' { $null=Assert-XnLocalPath $fixture -NoSpaces }
Pass 'absent child admitted without creation' { Assert-XnFreshTarget (Join-Path $fixture 'app'); if (Test-Path (Join-Path $fixture 'app')) { throw 'Created target' } }
foreach ($path in @('C:\a\..\b','C:\a\x.','C:\a\x ','C:\a\\b','C:\a\x:stream','\\server\share\x','C:\a\PROGRA~1','C:/a/b')) {
    Reject "noncanonical $path" { Assert-XnLocalPath $path -AllowMissing }
}
$retained=Join-Path $fixture 'retained';[IO.Directory]::CreateDirectory($retained) | Out-Null
[IO.File]::WriteAllText((Join-Path $retained 'sentinel.txt'),'INERT qualification sentinel; retain')
$sentinelHash=Get-XnHash (Join-Path $retained 'sentinel.txt')
Reject 'existing directory refused, even empty/owned' { Assert-XnFreshTarget $retained } 'absent install target'
Pass 'tree manifest hashes retained bytes' { $m=Get-XnTreeManifest $retained; if (@($m.files).Count -ne 1 -or $m.files[0].sha256 -ne $sentinelHash) { throw 'Wrong manifest' } }
Reject 'bounded inventory never truncates' { Get-XnTreeManifest $retained -MaximumFiles 0 } 'bound exceeded'
$junction=Join-Path $fixture 'junction'
New-Item -ItemType Junction -Path $junction -Target $retained | Out-Null
Reject 'actual junction rejected' { Assert-XnLocalPath $junction } 'Reparse'
Reject 'junction ancestor of missing path rejected' { Assert-XnLocalPath (Join-Path $junction 'missing') -AllowMissing } 'Reparse'
Reject 'artifact mismatch' { Assert-XnHash (Join-Path $retained 'sentinel.txt') ('0'*64) } 'Hash mismatch'
Pass 'artifact matching positive control' { Assert-XnHash (Join-Path $retained 'sentinel.txt') $sentinelHash }
$rows=@(foreach ($h in 'CurrentUser','LocalMachine') { foreach ($v in 'Registry32','Registry64') { foreach ($k in 'product','uninstall','run') { [pscustomobject]@{hive=$h;view=$v;kind=$k;present=$false} } } })
Pass 'all twelve registry absences admitted' { Assert-XnNamespaceEmpty $rows }
$rows[7].present=$true
Reject 'existing registration rejected' { Assert-XnNamespaceEmpty $rows } 'namespace'
$rows[7].present=$false
Reject 'missing registry view rejected' { Assert-XnNamespaceEmpty $rows[0..10] } 'Both registry'
$rows[7].kind=$rows[6].kind
Reject 'duplicate registry coordinate rejected' { Assert-XnNamespaceEmpty $rows } 'Ambiguous'
Pass 'existing runtime meets minimum' { Assert-XnRuntimeVersion '157.0.4341.46' '100.0.0.0' }
Reject 'runtime upgrade forbidden' { Assert-XnRuntimeVersion '100.0.0.0' '157.0.4341.46' } 'shared state'
Reject 'runtime registration missing' { Assert-XnRuntimeVersion '' '' } 'absent'
$source='a'*40;$hash='b'*64
$build=[pscustomobject]@{version='1.24.1-xlog.4';source=$source;status='packaged-unsigned-not-installed';signed=$false;installed=$false;artifacts=@([pscustomobject]@{path='C:\inert\setup.exe';sha256=$hash});executableSha256=$hash}
$audit=[pscustomobject]@{version=$build.version;source=$source;status='passed-artifact-audit-not-launched-not-installed';installed=$false;applicationLaunched=$false;buildReceiptSha256=$hash;installer=$build.artifacts[0];executable=[pscustomobject]@{sha256=$hash}}
Pass 'exact receipt chain admitted' { Assert-XnReceiptIdentity $build $audit $build.version $source $hash }
Reject 'foreign source rejected' { Assert-XnReceiptIdentity $build $audit $build.version ('c'*40) $hash } 'receipts'
Reject 'audit build hash mismatch' { Assert-XnReceiptIdentity $build $audit $build.version $source ('c'*64) } 'receipts'
Reject 'historical rejected candidate forbidden' { Assert-XnReceiptIdentity $build $audit '1.24.1-xlog.2' $source $hash } 'unrejected'
$audit.installed=$true
Reject 'already installed receipt forbidden' { Assert-XnReceiptIdentity $build $audit $build.version $source $hash } 'receipts'
$audit.installed=$false
$rendered=@'
Section Install
  SetOutPath $INSTDIR
  File "${MAINBINARYSRCPATH}"
  File /a "/oname=WebView2Loader.dll" "C:\inert\WebView2Loader.dll"
  File /a "/oname=resources\deepcw\model.onnx" "C:\inert\model.onnx"
  File /a "/oname=COPYING" "C:\inert\COPYING"
SectionEnd
'@
Pass 'exact generated destination mapping' { $m=@(Get-XnPayloadMap $rendered); if ($m.Count -ne 4 -or $m[2].destination -cne 'resources\deepcw\model.onnx') { throw 'Mapping incorrect' } }
Reject 'changed output directory rejected' { Get-XnPayloadMap ($rendered.Replace('SetOutPath $INSTDIR','SetOutPath $TEMP')) } 'output-directory'
Reject 'output directory change before later file rejected' { Get-XnPayloadMap ($rendered.Replace('  File /a "/oname=COPYING"',"  SetOutPath `$INSTDIR\other`n  File /a `"/oname=COPYING`"")) } 'output-directory'
Reject 'missing output-directory selection rejected' { Get-XnPayloadMap ($rendered.Replace('SetOutPath $INSTDIR','; no output directory')) } 'precedes'
Reject 'install root reassignment rejected' { Get-XnPayloadMap ($rendered.Replace('  File /a "/oname=COPYING"',"  StrCpy `$INSTDIR `$TEMP`n  File /a `"/oname=COPYING`"")) } 'mutation'
Reject 'destination traversal rejected' { Get-XnPayloadMap ($rendered.Replace('resources\deepcw\model.onnx','..\escape')) } 'Unsafe'
Reject 'unknown NSIS extraction switch rejected' { Get-XnPayloadMap ($rendered.Replace('File /a','File /r')) } 'Unreviewed'
Reject 'case-insensitive destination collision rejected' { Get-XnPayloadMap ($rendered.Replace('/oname=COPYING','/oname=webview2loader.dll')) } 'duplicate'
Reject 'execute cannot launch anything' {
    & (Join-Path $PSScriptRoot 'prepare-installer-acceptance.ps1') -BuildReceipt 'C:\inert\build.json' -AuditReceipt 'C:\inert\audit.json' -ExpectedAuditSha256 $hash -Version $build.version -ExpectedCommit $source -Execute
} 'Execution is not implemented'
Pass 'refusals retain sentinel bytes' { Assert-XnHash (Join-Path $retained 'sentinel.txt') $sentinelHash }
$marker="[Ownership]`r`nProduct=sk.om0rx.xnexus`r`nFormat=1`r`nRoot=$retained`r`n"
Pass 'exact ordinary marker positive control' { Assert-XnOwnershipMarker $marker $retained }
Reject 'foreign marker root rejected' { Assert-XnOwnershipMarker ($marker.Replace("Root=$retained","Root=$fixture")) $retained } 'mismatch'
Reject 'duplicate marker root rejected' { Assert-XnOwnershipMarker ($marker+"Root=$retained`r`n") $retained } 'fields'
Reject 'foreign product marker rejected' { Assert-XnOwnershipMarker ($marker.Replace('sk.om0rx.xnexus','com.other.app')) $retained } 'mismatch'
$before=Get-XnTreeManifest $retained
Pass 'retained external profile positive control' { Assert-XnRetainedFiles @($before) }
[IO.File]::AppendAllText((Join-Path $retained 'sentinel.txt'),' changed by inert negative control')
Reject 'changed retained profile bytes detected' { Assert-XnRetainedFiles @($before) } 'Retained bytes'
# This deliberately altered inert fixture remains intact; no test cleanup hides it.
$receipt=[ordered]@{status='passed';tests=$passed.Count;names=$passed.ToArray();fixture=$fixture;inert=$true;installerInvoked=$false;registryChanged=$false;filesRetained=$true;finishedUtc=[DateTime]::UtcNow.ToString('o')}
$receipt | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $fixture 'receipt.json') -Encoding utf8
Write-Output "$($passed.Count) inert tests passed; evidence: $fixture"
