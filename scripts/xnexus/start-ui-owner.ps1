#Requires -Version 7.4
# SPDX-License-Identifier: GPL-3.0-only
# Launches only the synthetic HTTP owner, never either application or a device.
param([Parameter(Mandatory)][string]$Executable,
      [Parameter(Mandatory)][ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$ExpectedSha256,
      [switch]$SyntheticAudio)
$ErrorActionPreference='Stop'
if (!$IsWindows) { throw 'Windows fixture launcher required.' }
$exe=(Resolve-Path -LiteralPath $Executable).Path
if ([IO.Path]::GetFileName($exe) -cne 'xnexus-ui-owner.exe' -or
    (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash -ne $ExpectedSha256) {
    throw 'Choose the verified synthetic owner executable.'
}
$id=[guid]::NewGuid()
$root=Join-Path ([IO.Path]::GetTempPath()) "xnexus-ui-$id"
$acl=[Security.AccessControl.DirectorySecurity]::new()
$acl.SetAccessRuleProtection($true,$false)
$user=[Security.Principal.WindowsIdentity]::GetCurrent().User
$acl.SetOwner($user)
foreach ($sid in @($user,[Security.Principal.SecurityIdentifier]::new('S-1-5-18'),[Security.Principal.SecurityIdentifier]::new('S-1-5-32-544'))) {
    $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid,'FullControl','ContainerInherit, ObjectInherit','None','Allow'))
}
[IO.FileSystemAclExtensions]::Create([IO.DirectoryInfo]::new($root),$acl) | Out-Null
@{schema=1;synthetic=$true;id=$id.ToString();synthetic_audio=$(if($SyntheticAudio){'silence48k'}else{'off'})} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root '.xnexus-ui-fixture.json') -Encoding utf8NoBOM
$argument="--fixture-root=$root"
$process=Start-Process -FilePath $exe -ArgumentList ('"'+$argument+'"') -WorkingDirectory $root -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $root 'stdout.log') -RedirectStandardError (Join-Path $root 'stderr.log')
$deadline=[DateTime]::UtcNow.AddSeconds(15)
while ([DateTime]::UtcNow -lt $deadline) {
    if ($process.HasExited) { throw "Synthetic owner exited; retain $root for evidence." }
    if (Test-Path -LiteralPath (Join-Path $root 'owner.json')) {
        $live=Get-CimInstance Win32_Process -Filter "ProcessId=$($process.Id)"
        $pattern='^\s*(?:"'+[regex]::Escape($exe)+'"|'+[regex]::Escape($exe)+')\s+(?:"'+[regex]::Escape($argument)+'"|'+[regex]::Escape($argument)+')\s*$'
        if (!$live -or $live.ExecutablePath -ne $exe -or $live.CommandLine -notmatch $pattern) { throw "Owner process identity unavailable; stop this sequence. Evidence: $root" }
        $owner=Get-Content -LiteralPath (Join-Path $root 'owner.json') -Raw | ConvertFrom-Json
        if ($owner.pid -ne $process.Id -or !$owner.synthetic -or $owner.root.Replace('\\?\','') -ne $root) { throw 'Owner fixture coordinates differ.' }
        $receipt=[ordered]@{root=$root;pid=$process.Id;executable=$exe;arguments=$argument;sha256=$ExpectedSha256;synthetic=$true}
        $receipt | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root 'launch.json') -Encoding utf8NoBOM
        [pscustomobject]$receipt
        return
    }
    Start-Sleep -Milliseconds 100
}
throw "Owner did not become ready; preserve $root. Do not bind an application."
