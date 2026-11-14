# SPDX-License-Identifier: GPL-3.0-only
param([string]$NsisRoot="$env:LOCALAPPDATA\tauri\NSIS")
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$root=Join-Path $repo ('target\nsis-guards\'+[guid]::NewGuid().ToString('N'))
if(Test-Path -LiteralPath $root){throw 'Never reuse a guard-test directory.'}
New-Item -ItemType Directory -Path $root -Force | Out-Null
$compiler=Join-Path $NsisRoot 'Bin\makensis.exe'
$plugin=Join-Path $NsisRoot 'Plugins\x86-unicode\additional\nsis_tauri_utils.dll'
if((Get-FileHash -LiteralPath $plugin -Algorithm SHA1).Hash -ne '75197FEE3C6A814FE035788D1C34EAD39349B860') {throw 'Require pinned nsis_tauri_utils0.5.3.'}
$exe=Join-Path $root 'xnexus-guard-probe.exe'
$guard=Join-Path $PSScriptRoot 'nsis\ownership.nsh'
$harness=Join-Path $PSScriptRoot 'nsis\guard-harness.nsi'
& $compiler '/V3' "/DOUT_FILE=$exe" "/DTEST_ROOT=$root" "/DGUARD_INCLUDE=$guard" "/DPLUGIN_ROOT=$(Split-Path $plugin)" $harness *> (Join-Path $root 'compile.log')
if($LASTEXITCODE -ne 0){throw "Guard harness compilation failed: $root\compile.log"}
function NewFixtureDirectory([string]$Name){$p=Join-Path $root $Name;New-Item -ItemType Directory -Path $p -Force | Out-Null;return $p}
function TextFile([string]$Path,[string]$Text){[IO.File]::WriteAllText($Path,$Text,[Text.Encoding]::Unicode)}
function Owned([string]$Name){
    $p=NewFixtureDirectory $Name
    TextFile (Join-Path $p '.om0rx-xnexus-owner.ini') "[Ownership]`r`nProduct=sk.om0rx.xnexus`r`nFormat=1`r`nRoot=$p`r`n"
    TextFile (Join-Path $p 'OM0RX-xNexus.exe') 'inert fixture bytes, never executable'
    TextFile (Join-Path $p 'uninstall.exe') 'inert fixture bytes, never executable'
    return $p
}
function StartHarness([string]$Program,[string[]]$Arguments){
    $info=[Diagnostics.ProcessStartInfo]::new($Program)
    $info.UseShellExecute=$false;$info.CreateNoWindow=$true;$info.WindowStyle='Hidden'
    foreach($arg in $Arguments){$info.ArgumentList.Add($arg)}
    return [Diagnostics.Process]::Start($info)
}
$cases=@();$counter=0
function Case([string]$Name,[string]$Path,[int]$Expected,[string]$Mode='root',[string]$Retained='',[int]$RequireOwned=0,[string]$ProcessName=''){
    $script:counter++;$id=$script:counter.ToString('00')
    TextFile (Join-Path $root "$id.ini") "[Test]`r`nPath=`"$Path`"`r`nRequireOwned=$RequireOwned`r`nMode=$Mode`r`nRetained=$Retained`r`nProcessName=$ProcessName`r`n"
    $p=StartHarness $exe @("/SCENARIO=$id")
    if(!$p.WaitForExit(30000)){throw "Harness did not exit: $Name PID $($p.Id)"}
    $result=Get-Content -LiteralPath (Join-Path $root "$id.ini.result")
    if($p.ExitCode -ne $Expected -or [int]$result[0] -ne $Expected){throw "$Name expected $Expected, exit $($p.ExitCode), result $result"}
    $script:cases+=@{name=$Name;expected=$Expected;exit=$p.ExitCode;reason=($result | Select-Object -Skip 1) -join ' '}
}
$empty=NewFixtureDirectory 'empty';$owned=Owned 'owned';$foreign=NewFixtureDirectory 'foreign-xlog'
TextFile (Join-Path $foreign 'xlog-prototype.exe') 'foreign sentinel'
$queue=NewFixtureDirectory 'companion\xlog-outbox';TextFile (Join-Path $queue 'pending.json') 'pending QSO sentinel'
TextFile (Join-Path $owned 'unrelated.txt') 'must survive'
$bad=Owned 'wrong-marker';TextFile (Join-Path $bad '.om0rx-xnexus-owner.ini') '[Ownership]'
$mixed=Owned 'mixed';TextFile (Join-Path $mixed 'xlog-prototype.exe') 'foreign sentinel'
$junction=Join-Path $root 'junction';New-Item -ItemType Junction -Path $junction -Target $foreign | Out-Null
$nested=Owned 'nested-junction';New-Item -ItemType Junction -Path (Join-Path $nested 'resources') -Target $foreign | Out-Null
$hard=Owned 'hardlink';New-Item -ItemType HardLink -Path (Join-Path $hard 'borrowed.txt') -Target (Join-Path $owned 'unrelated.txt') | Out-Null
$sentinels=@((Join-Path $foreign 'xlog-prototype.exe'),(Join-Path $queue 'pending.json'),(Join-Path $owned 'unrelated.txt'))
$before=@($sentinels | ForEach-Object {@{path=$_;sha256=(Get-FileHash -LiteralPath $_).Hash}})
Case 'new absent folder' (Join-Path $root 'new') 0
Case 'empty folder' $empty 0
# Hard link makes the original owned sentinel unsafe too; remove neither link.
Case 'owned hardlinked file refused' $hard 1
Case 'original hardlinked sentinel refused' $owned 1
$clean=Owned 'clean-owned';TextFile (Join-Path $clean 'unrelated.txt') 'preserved ordinary file'
Case 'owned folder with unrelated ordinary sentinel' $clean 0
Case 'foreign xlog folder' $foreign 1
Case 'wrong marker' $bad 1
Case 'owned but foreign executable' $mixed 1
Case 'junction root' $junction 1
Case 'junction ancestor' (Join-Path $junction 'new') 1
Case 'nested resource junction' $nested 1
Case 'drive root' ([IO.Path]::GetPathRoot($root)) 1
Case 'relative path' 'relative\path' 1
Case 'trailing dot alias' ($empty+'.') 1
Case 'trailing space alias' ($empty+' ') 1
Case 'ancestor dot alias' ($empty+'.\child') 1
Case 'noncanonical parent path' ($empty+'\..\child') 1
Case 'reserved DOS device' ($empty+'\NUL') 1
Case 'retained foreign path before maintenance' $empty 1 'locations' $foreign
Case 'retained foreign same path before maintenance' $foreign 1 'locations' $foreign
Case 'retained owned path before maintenance' $clean 0 'locations' $clean
Case 'retained owned path relocation refused' $empty 1 'locations' $clean
Case 'uninstaller requires ownership' $empty 1 'root' '' 1
Case 'cancel before operation preserves queue' $clean 2 'cancel'
$runningExe=Join-Path $root 'xnexus-owned-test-process.exe';Copy-Item -LiteralPath $exe -Destination $runningExe
$running=StartHarness $runningExe @('/HOLD')
try {
    $ready=Join-Path $root 'hold.ready';$deadline=[DateTime]::UtcNow.AddSeconds(10)
    while(!(Test-Path -LiteralPath $ready) -and [DateTime]::UtcNow -lt $deadline){Start-Sleep -Milliseconds 50}
    if(!(Test-Path -LiteralPath $ready) -or $running.HasExited){throw 'Owned inert test process did not become ready.'}
    Case 'running instance refused without kill' '' 1 'closed' '' 0 'xnexus-owned-test-process.exe'
    if($running.HasExited){throw 'Admission check terminated the owned running process.'}
} finally {TextFile (Join-Path $root 'hold.stop') 'ordinary test shutdown';if(!$running.WaitForExit(15000)){throw 'Owned inert process failed ordinary shutdown.'}}
Case 'ordinary closure allows admission' '' 0 'closed' '' 0 'xnexus-owned-test-process.exe'
foreach($item in $before){if((Get-FileHash -LiteralPath $item.path).Hash -ne $item.sha256){throw 'Sentinel changed during guard-only tests.'}}
if(!(Test-Path -LiteralPath (Join-Path $clean 'unrelated.txt')) -or (Test-Path -LiteralPath (Join-Path $root 'new'))){throw 'Read-only guard modified a fixture.'}
$receipt=@{status='passed-actual-nsis-guard-harness-no-product-install';cases=$cases;count=$cases.Count;source=(git -C $repo rev-parse HEAD).Trim();sourceStatus=@(git -C $repo status --porcelain);guardSha256=(Get-FileHash -LiteralPath $guard).Hash;harnessSha256=(Get-FileHash -LiteralPath $harness).Hash;compilerSha256=(Get-FileHash -LiteralPath $compiler).Hash;pluginSha256=(Get-FileHash -LiteralPath $plugin).Hash;sentinels=$before;root=$root;completedUtc=[DateTime]::UtcNow.ToString('o')}
$receipt | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $root 'receipt.json')
Write-Output "NSIS guard tests: $($cases.Count) passed; $root\receipt.json"
