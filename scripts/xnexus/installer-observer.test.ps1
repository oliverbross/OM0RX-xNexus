# SPDX-License-Identifier: GPL-3.0-only
# Small finite-lived inert processes only; no candidate install or GUI launch.
param([string]$OnlyCase='')
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'installer-observer.ps1')
. (Join-Path $PSScriptRoot 'installer-transaction.ps1')
$repo=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$evidence=Join-Path $repo ('target\installer-observer-tests\'+[guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($evidence) | Out-Null
$exe=Join-Path $evidence 'inert-observer-fixture.exe'
$compiler=Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
# Match NSIS's windowed PE subsystem without creating any window. A console PE
# adds separately owned conhost children and is a different process-tree fixture.
& $compiler /nologo /target:winexe "/out:$exe" (Join-Path $PSScriptRoot 'installer-observer-fixture.cs') *> (Join-Path $evidence 'compiler.log')
if ($LASTEXITCODE -ne 0) { throw 'Inert fixture compile failed; see compiler.log' }
$hash=Get-XnHash $exe
if (!('Xnexus.InstallerObservation.Observer' -as [type])) { Add-Type -Path (Join-Path $PSScriptRoot 'installer-observer.cs') }
$literalTemp=Join-Path $evidence '~nsu-test.tmp';[IO.Directory]::CreateDirectory($literalTemp) | Out-Null
$temporaryImage=Join-Path $literalTemp 'Au_.exe';Copy-Item -LiteralPath $exe -Destination $temporaryImage
if ((Get-XnHash $temporaryImage -ObservedImage) -cne $hash) { throw 'Literal NSIS-like temporary name was not canonicalized correctly' }
$alias=Join-Path $evidence 'temporary-alias';New-Item -ItemType Junction -Path $alias -Target $literalTemp | Out-Null
$rejected=$false
try { Get-XnHash (Join-Path $alias 'Au_.exe') -ObservedImage | Out-Null } catch { $rejected=$true }
if (!$rejected) { throw 'Observed executable accepted a reparse alias' }
$cases=@()
$script:identityMode='';$script:rootReads=0
# Test-scoped CIM fault only; kernel membership and real finite processes remain
# unchanged. The production observer exposes no injected process/identity API.
function Get-CimInstance {
    [CmdletBinding()]param([Parameter(Position=0)][string]$ClassName,[string]$Filter,[uint32]$OperationTimeoutSec)
    $value=CimCmdlets\Get-CimInstance @PSBoundParameters
    if ($script:identityMode -and $value -and $value.ExecutablePath -eq $exe) {
        if ($value.CommandLine -match '\sparent\s') {
            $script:rootReads++
            if (($script:identityMode -eq 'known-root-race' -and $script:rootReads -gt 1) -or $script:identityMode -eq 'immediate-root') {
                return [pscustomobject]@{ExecutablePath=$null;CommandLine=$null;CreationDate=$null}
            }
        } elseif ($script:identityMode -eq 'missing-child' -and $value.CommandLine -match '\schild\s') { return $null }
    }
    return $value
}
foreach ($case in @(@{name='ordinary';timeout=20;childExit=0},@{name='timeout';timeout=2;childExit=0},@{name='child-error';timeout=20;childExit=1},@{name='known-root-race';timeout=20;childExit=0},@{name='immediate-root';timeout=20;childExit=0},@{name='missing-child';timeout=20;childExit=0}) | Where-Object { !$OnlyCase -or $_.name -eq $OnlyCase }) {
    $directory=Join-Path $evidence $case.name;[IO.Directory]::CreateDirectory($directory) | Out-Null
    [IO.File]::WriteAllText((Join-Path $directory 'inert-observer-fixture.marker'),'Finite inert qualification process fixture')
    $script:identityMode=$case.name;$script:rootReads=0
    $observationError=$null
    $rootDelay=if($case.name -eq 'immediate-root'){0}else{1200}
    try { $result=Invoke-XnProcessObservation -Executable $exe -ExpectedSha256 $hash -Arguments "parent `"$directory`" $($case.childExit) $rootDelay" -WorkingDirectory $directory -ReceiptPath (Join-Path $directory 'observation.json') -TimeoutSeconds $case.timeout }
    catch { $observationError=$_ }
    finally { $script:identityMode='' }
    if ($observationError) {
        # Preserve the failing receipt, and let only these finite fixtures exit
        # on their own. This is test cleanup observation, never termination.
        $deadline=[DateTime]::UtcNow.AddSeconds(15)
        do { $remaining=@(CimCmdlets\Get-CimInstance Win32_Process -Filter "Name='inert-observer-fixture.exe'"|Where-Object ExecutablePath -eq $exe);if (!$remaining.Count) { break };Start-Sleep -Milliseconds 100 } while([DateTime]::UtcNow -lt $deadline)
        @{status='failed';error=$observationError.Exception.Message;rootAndChildrenExitedNaturally=($remaining.Count -eq 0);activeProcesses=$remaining.Count}|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $directory 'failure.json')
        throw $observationError
    }
    if (!$result.atomicMembership -or $result.limitFlags -ne 0 -or $result.memoryLimit -ne 0 -or $result.terminatedByObserver) { throw 'Observer changed process policy' }
    $expectedIdentities=if($case.name -eq 'missing-child'){1}else{2}
    if ($result.identities.Count -ne $expectedIdentities -or @($result.identities | Where-Object { $_.executable -ine $exe -or $_.sha256 -ine $hash }).Count) { throw 'Did not observe expected exact fixture identities' }
    if ($script:rootReads -ne 0) { throw 'Owned root identity depended on CIM' }
    $rootProof=@($result.identities|Where-Object pid -eq $result.rootPid)
    if ($rootProof.Count -ne 1 -or $rootProof[0].identitySource -ne 'owned-CreateProcessW-handle' -or $rootProof[0].hashSource -ne 'verified-before-dispatch') { throw 'Root native handle/hash provenance missing' }
    if ($case.name -eq 'missing-child') {
        if ($result.status -ne 'retired-identity-gap' -or $result.totalProcesses -ne 2 -or $result.activeProcesses -ne 0) { throw 'Missed child identity was not retained as accounting gap' }
        $refused=$false;try { Assert-XnObservedRetirement $result $hash } catch { $refused=$true }
        if (!$refused) { throw 'Transaction accepted missing child identity' }
    } elseif ($case.name -ne 'timeout') {
        if ($result.status -ne 'retired-identities-observed' -or !$result.rootExited -or $result.activeProcesses -ne 0 -or $result.rootExitCode -ne 0) { throw 'Normal retirement not verified' }
        if ($case.name -in 'ordinary','known-root-race','immediate-root') { Assert-XnObservedRetirement $result $hash }
        else {
            $nonzero=@($result.identities | Where-Object { $_.exited -and $_.exitCode -eq 1 -and $_.pid -ne $result.rootPid })
            if ($nonzero.Count -ne 1) { throw 'Did not retain exact child exit1 after root exit0' }
            $refused=$false;try { Assert-XnObservedRetirement $result $hash } catch { $refused=$true }
            if (!$refused) { throw 'Transaction accepted nonzero child exit' }
        }
    } else {
        if ($result.status -ne 'timeout-unresolved' -or !$result.rootExited -or $result.activeProcesses -ne 1) { throw 'Root-before-child timeout control did not fire' }
        $childId=[int](Get-Content -LiteralPath (Join-Path $directory 'child.pid'))
        $proof=@($result.identities | Where-Object pid -eq $childId)
        $live=Get-CimInstance Win32_Process -Filter "ProcessId=$childId"
        if ($proof.Count -ne 1 -or !$live -or $live.ExecutablePath -ine $exe -or $live.CreationDate.ToUniversalTime().ToString('o') -cne $proof[0].createdUtc) { throw 'Timed-out child failed survival/identity proof' }
        # Obtain this positively identified child's handle and wait for its ordinary finite completion.
        $child=[Diagnostics.Process]::GetProcessById($childId)
        # Hold a native process handle while it is alive, so ExitCode remains
        # available after Windows retires its PID (Get-Process alone is lazy).
        $null=$child.Handle
        $child.EnableRaisingEvents=$true
        if (!$child.WaitForExit(15000)) { throw 'Inert child exceeded its own finite lifetime; preserve, never force kill' }
        if ($child.ExitCode -ne 0) { throw 'Child did not finish normally' }
        $child.Dispose()
    }
    if (!(Test-Path -LiteralPath (Join-Path $directory 'child-exited.txt')) -or !(Test-Path -LiteralPath (Join-Path $directory 'parent-exited.txt'))) { throw 'Fixture completion markers missing' }
    $live=@(Get-CimInstance Win32_Process -Filter "Name='inert-observer-fixture.exe'" | Where-Object ExecutablePath -eq $exe)
    if ($live.Count) { throw 'Owned fixture remains live' }
    $cases+=@{name=$case.name;status='passed';rootExited=$true;activeProcesses=0;observationStatus=$result.status}
}
@{status='passed';cases=$cases;observedImagePathControls=2;fixtureExecutable=$exe;fixtureSha256=$hash;candidateLaunched=$false;installed=$false;forceTerminated=$false;completedUtc=[DateTime]::UtcNow.ToString('o')} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $evidence 'receipt.json') -Encoding utf8
Write-Output "$($cases.Count) inert process lifecycle cases passed; all roots/children exited naturally: $evidence"
