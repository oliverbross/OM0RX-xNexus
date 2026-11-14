param(
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$ExpectedCommit,
    [string]$MsysRoot = 'C:\msys64',
    [Parameter(Mandatory)][string]$EvidenceRoot
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if ($repo -ne 'C:\Projects\OM0RX-xNexus') { throw 'Packaging guard must run from canonical C:\Projects\OM0RX-xNexus.' }
$source = (& git -C $repo rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $source -ne $ExpectedCommit) { throw 'ExpectedCommit must match canonical HEAD.' }
if (& git -C $repo status --porcelain) { throw 'Packaging requires clean canonical source.' }
if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect canonical source state.' }
$build = Join-Path $repo 'scripts\xnexus\build-windows.ps1'
if (!(Get-Command $build).Parameters.ContainsKey('Jobs')) { throw 'Reviewed build-windows.ps1 Jobs interface is required.' }
. (Join-Path $PSScriptRoot 'windows-guard\Guard.ps1')
$evidence = Join-Path ([IO.Path]::GetFullPath($EvidenceRoot)) ([DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffffffZ') + '-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $evidence | Out-Null
$version = (Get-Content -Raw -LiteralPath (Join-Path $repo 'src-tauri\tauri.conf.json') | ConvertFrom-Json).version
Copy-XnexusBuildReceipts $repo $version (Join-Path $evidence 'before')
$driver = Join-Path $evidence 'build-driver.ps1'
$log = Join-Path $evidence 'build.log'
$body = '$ErrorActionPreference = ''Stop''' + "`ntry {`n& " + (ConvertTo-XnexusPsLiteral $build) +
    ' -ExpectedCommit ' + (ConvertTo-XnexusPsLiteral $ExpectedCommit) + ' -MsysRoot ' + (ConvertTo-XnexusPsLiteral $MsysRoot) +
    ' -Jobs 1 *> ' + (ConvertTo-XnexusPsLiteral $log) + "`nexit 0`n} catch {`n" +
    '$_ | Out-String | Add-Content -LiteralPath ' + (ConvertTo-XnexusPsLiteral $log) + "`nexit 1`n}"
[IO.File]::WriteAllText($driver,$body)
$pwsh = Join-Path $PSHOME 'pwsh.exe'
if (!(Test-Path $pwsh)) { throw 'PowerShell 7 pwsh.exe is required.' }
$encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes('& ' + (ConvertTo-XnexusPsLiteral $driver)))
$provenance = @{source=$source; version=$version; canonicalRepo=$repo; jobs=1; buildScript=$build; createdUtc=[DateTime]::UtcNow.ToString('o'); boundedAttempt=$true; retry=$false; files=@()}
foreach ($file in @($PSCommandPath,$build,$driver,(Join-Path $PSScriptRoot 'windows-guard\Guard.ps1'),(Join-Path $PSScriptRoot 'windows-guard\NativeJob.cs'))) {
    $provenance.files += @{path=$file;sha256=(Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant()}
}
$provenance | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $evidence 'provenance.json') -Encoding utf8
try {
    $result = Invoke-XnexusGuardedChild -Executable $pwsh -Arguments "-NoLogo -NoProfile -NonInteractive -EncodedCommand $encoded" -WorkingDirectory $repo -EvidenceDirectory $evidence
} finally {
    Copy-XnexusBuildReceipts $repo $version (Join-Path $evidence 'after')
}
Write-Output "Guard evidence: $evidence"
if ($result.effectiveExitCode -ne 0) { throw "Bounded package attempt $($result.status): $($result.reason) $($result.error). Evidence and partial artifacts preserved; no retry." }
