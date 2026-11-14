# SPDX-License-Identifier: GPL-3.0-only
# Read-only admission primitives. No installer/application/registry writes.
Set-StrictMode -Version Latest

function Assert-XnLocalPath([string]$Path, [switch]$AllowMissing, [switch]$NoSpaces) {
    if ($Path -notmatch '^[A-Za-z]:\\[^\\]' -or $Path.Contains('/') -or
        $Path.Substring(2) -match '[:*?"<>|~]' -or $Path.EndsWith('\') -or
        ($NoSpaces -and $Path -match '\s')) { throw 'Require a canonical local path (no alias, UNC, ADS, wildcard or trailing slash).' }
    foreach ($part in $Path.Substring(3).Split('\')) {
        if (!$part -or $part -in '.', '..' -or $part -match '[. ]$') { throw 'Noncanonical path component.' }
    }
    if ([IO.Path]::GetFullPath($Path) -cne $Path) { throw 'Path is not canonical.' }
    $drive = [IO.DriveInfo]::new($Path.Substring(0,3))
    if ($drive.DriveType -ne [IO.DriveType]::Fixed) { throw 'Only a fixed local volume is allowed.' }
    $cursor = $Path
    $missing = $false
    while ($cursor) {
        if (Test-Path -LiteralPath $cursor) {
            $item = Get-Item -LiteralPath $cursor -Force -ErrorAction Stop
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Reparse point refused: $cursor" }
            if ($cursor -ne $Path -and !$item.PSIsContainer) { throw 'Ancestor is not a directory.' }
            if ($item.FullName.TrimEnd('\') -ine $cursor.TrimEnd('\')) { throw 'Filesystem path alias refused.' }
        } else { $missing = $true }
        $parent = [IO.Directory]::GetParent($cursor)
        $cursor = if ($parent) { $parent.FullName } else { $null }
    }
    if ($missing -and !$AllowMissing) { throw "Path is missing: $Path" }
    $Path
}

function Get-XnHash([string]$Path) {
    $null = Assert-XnLocalPath $Path
    if (!(Test-Path -LiteralPath $Path -PathType Leaf)) { throw 'Expected a regular file.' }
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
}

function Assert-XnHash([string]$Path, [string]$Expected) {
    if ($Expected -notmatch '^[0-9a-fA-F]{64}$' -or (Get-XnHash $Path) -ine $Expected) { throw "Hash mismatch: $Path" }
}

function Assert-XnFreshTarget([string]$Path) {
    $null = Assert-XnLocalPath $Path -AllowMissing -NoSpaces
    if (Test-Path -LiteralPath $Path) { throw 'Require an absent install target; preserve all existing contents.' }
}

function Assert-XnReceiptIdentity($Build, $Audit, [string]$Version, [string]$Source, [string]$BuildHash) {
    if ($Version -notmatch '^\d+\.\d+\.\d+-xlog\.[1-9]\d*$' -or $Version -in '1.24.1-xlog.1','1.24.1-xlog.2' -or
        $Source -notmatch '^[0-9a-f]{40}$') { throw 'Require a unique unrejected candidate version and exact source.' }
    if ($Build.version -cne $Version -or $Audit.version -cne $Version -or
        $Build.source -cne $Source -or $Audit.source -cne $Source -or
        $Build.status -cne 'packaged-unsigned-not-installed' -or
        $Audit.status -cne 'passed-artifact-audit-not-launched-not-installed' -or
        $Build.signed -cne $false -or $Build.installed -cne $false -or
        $Audit.installed -cne $false -or $Audit.applicationLaunched -cne $false -or
        $Audit.buildReceiptSha256 -ine $BuildHash -or @($Build.artifacts).Count -ne 1) {
        throw 'Build and passing static-audit receipts do not identify one uninstalled candidate.'
    }
    if ($Audit.installer.path -ine $Build.artifacts[0].path -or
        $Audit.installer.sha256 -ine $Build.artifacts[0].sha256 -or
        $Audit.executable.sha256 -ine $Build.executableSha256) { throw 'Artifact identities disagree across receipts.' }
}

function Assert-XnNamespaceEmpty($Rows) {
    if (@($Rows).Count -ne 12) { throw 'Both registry views and hives must be inspected for all three namespaces.' }
    $seen = @{}
    foreach ($row in $Rows) {
        $id = "$($row.hive)/$($row.view)/$($row.kind)"
        if ($row.hive -notin 'CurrentUser','LocalMachine' -or $row.view -notin 'Registry32','Registry64' -or
            $row.kind -notin 'product','uninstall','run' -or $seen.ContainsKey($id)) { throw 'Ambiguous registry inventory.' }
        $seen[$id] = $true
        if ($row.present -cne $false) { throw "Existing or unreadable fork namespace: $id" }
    }
}

function Get-XnRegistrySnapshot {
    $paths = @{ product='Software\OM0RX\OM0RX-xNexus'; uninstall='Software\Microsoft\Windows\CurrentVersion\Uninstall\OM0RX-xNexus'; run='Software\Microsoft\Windows\CurrentVersion\Run' }
    foreach ($hive in 'CurrentUser','LocalMachine') {
        foreach ($view in 'Registry32','Registry64') {
            $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::$hive,[Microsoft.Win32.RegistryView]::$view)
            try {
                foreach ($kind in 'product','uninstall','run') {
                    $key = $base.OpenSubKey($paths[$kind],$false)
                    try {
                        $present = if ($kind -eq 'run') { $null -ne $key -and @($key.GetValueNames()) -contains 'OM0RX-xNexus' } else { $null -ne $key }
                        # Only bounded relevant values; never dump unrelated Run entries.
                        $values = @{}
                        if ($present) {
                            $names = if ($kind -eq 'run') { @('OM0RX-xNexus') } else { @('','DisplayVersion','InstallLocation','UninstallString') }
                            foreach ($name in $names) {
                                $value = $key.GetValue($name,$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
                                if ($null -ne $value) { $values[$name] = ([string]$value).Substring(0,[Math]::Min(([string]$value).Length,2048)) }
                            }
                        }
                        [pscustomobject]@{hive=$hive;view=$view;kind=$kind;present=[bool]$present;values=$values}
                    } finally { if ($key) { $key.Dispose() } }
                }
            } finally { $base.Dispose() }
        }
    }
}

function Get-XnTreeManifest([string]$Path, [int]$MaximumFiles=20000) {
    $null = Assert-XnLocalPath $Path -AllowMissing
    if (!(Test-Path -LiteralPath $Path)) { return [pscustomobject]@{path=$Path;exists=$false;files=@()} }
    $pending = [Collections.Generic.Stack[string]]::new(); $pending.Push($Path)
    $files = [Collections.Generic.List[object]]::new(); $directories=0
    while ($pending.Count) {
        $directory=$pending.Pop(); $directories++
        if ($directories -gt 4096) { throw 'Directory inventory bound exceeded; review rather than truncate.' }
        foreach ($item in Get-ChildItem -LiteralPath $directory -Force -ErrorAction Stop) {
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Profile/shortcut inventory contains a reparse point.' }
            if ($item.PSIsContainer) { $pending.Push($item.FullName) }
            else {
                if ($files.Count -ge $MaximumFiles) { throw 'File inventory bound exceeded; review rather than truncate.' }
                $files.Add([pscustomobject]@{path=$item.FullName;bytes=$item.Length;sha256=(Get-XnHash $item.FullName)})
            }
        }
    }
    [pscustomobject]@{path=$Path;exists=$true;files=@($files.ToArray() | Sort-Object path)}
}

function Assert-XnRuntimeVersion([string]$Installed,[string]$Minimum) {
    if ($Installed -notmatch '^\d+\.\d+\.\d+\.\d+$') { throw 'Existing WebView2 registration is absent or ambiguous.' }
    if ($Minimum -and ($Minimum -notmatch '^\d+\.\d+\.\d+\.\d+$' -or [version]$Installed -lt [version]$Minimum)) {
        throw 'WebView2 provisioning/update would affect shared state; stop.'
    }
}

function Get-XnPayloadMap([string]$Rendered) {
    $sections=[regex]::Matches($Rendered,'(?ms)^Section Install\r?\n(.*?)^SectionEnd\s*$')
    if ($sections.Count -ne 1) { throw 'Expected exactly one generated Install section.' }
    $result=[Collections.Generic.List[object]]::new(); $seen=@{}; $rootSelected=$false
    foreach ($line in $sections[0].Groups[1].Value.Split("`n")) {
        $line=$line.Trim()
        if ($line -match '^SetOutPath(?:\s|$)') {
            if ($line -cne 'SetOutPath $INSTDIR' -or $rootSelected) { throw 'Unreviewed output-directory change in Install section.' }
            $rootSelected=$true; continue
        }
        if ($line -match '^(StrCpy|StrCpyS|Pop|ReadRegStr|GetFullPathName)\s+\$(INSTDIR|OUTDIR)(?:\s|$)' -or
            $line -match '^(Goto|Return|SetShellVarContext)(?:\s|$)') { throw 'Unreviewed output-directory/control mutation in Install section.' }
        if ($line -notmatch '^File(?:\s|$)') { continue }
        if (!$rootSelected) { throw 'File extraction precedes verified $INSTDIR output selection.' }
        if ($line -ceq 'File "${MAINBINARYSRCPATH}"') { $destination='OM0RX-xNexus.exe'; $source=$null }
        elseif ($line -cmatch '^File /a "/oname=([^"$]+)" "([^"$]+)"$') { $destination=$Matches[1].Replace('/','\'); $source=$Matches[2] }
        else { throw "Unreviewed generated File instruction: $line" }
        if ($destination -notmatch '^[A-Za-z0-9_.-]+(?:\\[A-Za-z0-9_.-]+)*$' -or
            @($destination.Split('\') | Where-Object { $_ -in '.','..' -or $_ -match '[. ]$' }).Count -or
            $seen.ContainsKey($destination)) { throw 'Unsafe or duplicate installed payload destination.' }
        $seen[$destination]=$true; $result.Add([pscustomobject]@{destination=$destination;source=$source})
    }
    if (!$seen.ContainsKey('OM0RX-xNexus.exe') -or !$seen.ContainsKey('WebView2Loader.dll') -or $result.Count -lt 4) { throw 'Incomplete generated payload map.' }
    $result.ToArray()
}

function Assert-XnOwnershipMarker([string]$Text,[string]$Root) {
    $null=Assert-XnLocalPath $Root
    $lines=@($Text.TrimStart([char]0xfeff).Split("`n") | ForEach-Object { $_.TrimEnd("`r") } | Where-Object { $_ -ne '' })
    if ($lines.Count -ne 4 -or $lines[0] -cne '[Ownership]') { throw 'Unexpected ownership marker sections/fields.' }
    $expected=@('Product=sk.om0rx.xnexus','Format=1',"Root=$Root")
    foreach ($entry in $expected) { if (@($lines | Where-Object { $_ -ceq $entry }).Count -ne 1) { throw 'Ownership marker product, format or exact root mismatch.' } }
}

function Assert-XnRetainedFiles($Before) {
    foreach ($tree in @($Before)) {
        if (!$tree.exists) {
            if (Test-Path -LiteralPath $tree.path) { throw 'Previously absent retained profile now exists.' }
            continue
        }
        $after=Get-XnTreeManifest $tree.path
        if (!$after.exists -or @($after.files).Count -ne @($tree.files).Count) { throw 'Retained file inventory changed.' }
        foreach ($file in $tree.files) {
            $match=@($after.files | Where-Object { $_.path -ceq $file.path -and $_.bytes -eq $file.bytes -and $_.sha256 -ceq $file.sha256 })
            if ($match.Count -ne 1) { throw 'Retained bytes changed.' }
        }
    }
}
