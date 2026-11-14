# No Start-Process or GUI operation is executed by these checks.
# Parse and import only the launcher's function definitions, never its body.
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$scriptFile=Join-Path $PSScriptRoot 'start-candidate.ps1'
$tokens=$null;$parseErrors=$null
$ast=[System.Management.Automation.Language.Parser]::ParseFile($scriptFile,[ref]$tokens,[ref]$parseErrors)
if ($parseErrors.Count) { throw ($parseErrors | Out-String) }
foreach ($definition in $ast.FindAll({param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst]},$false)) {
    Invoke-Expression $definition.Extent.Text
}
$results=[Collections.Generic.List[object]]::new()
function Check([string]$Name,[scriptblock]$Body) {
    & $Body
    $results.Add(@{name=$Name;passed=$true})
}
function Reject([scriptblock]$Body) {
    $refused=$false
    try { & $Body | Out-Null } catch { $refused=$true }
    if (!$refused) { throw 'Expected refusal was not produced.' }
}
function Require([bool]$Condition) { if (!$Condition) { throw 'Guard assertion failed.' } }
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
$testRoot=Join-Path $repoRoot "target/xnexus-launcher-guards/$([guid]::NewGuid())"
New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
$webRoot=Join-Path $testRoot 'profile with spaces\webview'
New-Item -ItemType Directory -Path (Join-Path $webRoot 'EBWebView') -Force | Out-Null
$exe='C:\Programs\Example App\OM0RX-xNexus.exe'
$argument='--xlog-data-dir=C:\Private\Profile With Spaces'
Check 'syntax and single launch command' {
    $starts=@($ast.FindAll({param($n) $n -is [System.Management.Automation.Language.CommandAst] -and $n.GetCommandName() -eq 'Start-Process'},$true))
    Require ($starts.Count -eq 1)
    Require ($starts[0].Extent.Text -match '-WindowStyle Hidden')
    Require ($ast.Extent.Text.IndexOf('if ($ValidateOnly)') -lt $starts[0].Extent.StartOffset)
    Require ($ast.Extent.Text -notmatch '\bStop-Process\b|\.Kill\(|Get-App|Get-UIA|AutomationElement')
}
Check 'exact full command accepts only one private argument' {
    Require (Exact-Command ('"'+$exe+'" "'+$argument+'"') $exe $argument)
    Require (!(Exact-Command ('"'+$exe+'" "'+$argument+'" --independent-mode') $exe $argument))
    Require (!(Exact-Command ('"'+$exe+'"') $exe $argument))
    Require (!(Exact-Command ('"C:\Other.exe" "'+$argument+'"') $exe $argument))
}
Check 'unsafe paths refuse relative, UNC, traversal, ADS and trailing ambiguity' {
    foreach($unsafe in @('relative','\\server\share\file','C:\safe\..\other','C:\file:stream','C:\folder.\file','C:\file ')) {
        Reject { Safe-Path $unsafe }
    }
}
Check 'missing leaf permitted only for a fresh child' {
    Require ((Safe-Path (Join-Path $testRoot 'fresh') -AllowMissingLeaf) -eq (Join-Path $testRoot 'fresh'))
    Reject { Safe-Path (Join-Path $testRoot 'missing-parent\fresh') -AllowMissingLeaf }
}
Check 'prefix lookalike is not a descendant' {
    Require (Is-Within (Join-Path $webRoot 'EBWebView') $webRoot)
    Require (!(Is-Within ($webRoot+'-foreign\data') $webRoot))
}
Check 'WebView private coordinate accepts normal and quoted argument forms' {
    Require ((WebView-Root ('msedgewebview2.exe --user-data-dir="'+$webRoot+'" --type=renderer')) -eq $webRoot)
    Require ((WebView-Root ('msedgewebview2.exe "--user-data-dir='+$webRoot+'" --type=gpu-process')) -eq $webRoot)
}
Check 'WebView missing or duplicate profile coordinate refuses' {
    Reject { WebView-Root 'msedgewebview2.exe --type=renderer' }
    Reject { WebView-Root ('msedgewebview2.exe --user-data-dir="'+$webRoot+'" --user-data-dir="'+$webRoot+'"') }
}
Check 'reparse directory refuses' {
    $junction=Join-Path $testRoot 'junction'
    New-Item -ItemType Junction -Path $junction -Target $webRoot | Out-Null
    Reject { Safe-Path $junction }
    Reject { Safe-Path (Join-Path $junction 'EBWebView') }
}
Check 'hash mismatch refuses' {
    $file=Join-Path $testRoot 'plain.txt';Set-Content -LiteralPath $file -Value 'fixture'
    Reject { Hash-Equals $file ('0'*64) }
    Hash-Equals $file (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash
}
Check 'unmarked non-TEMP owner fixture refuses' { Reject { Private-Fixture $testRoot } }

$candidate=[pscustomobject]@{ProcessId=100;ParentProcessId=1;Name='OM0RX-xNexus.exe';ExecutablePath=$exe;CommandLine=('"'+$exe+'" "'+$argument+'"');CreationDate=[DateTime]::UtcNow}
$candidateProof=Candidate-Record $candidate $exe $argument
Check 'initial absent or partial candidate metadata may wait' {
    Require ($null -eq (Candidate-Record $null $exe $argument))
    $partial=$candidate.PSObject.Copy();$partial.CommandLine=$null
    Require ($null -eq (Candidate-Record $partial $exe $argument))
    Require ((Candidate-Record $candidate $exe $argument).pid -eq 100)
}
Check 'initial known mismatch never waits for other missing metadata' {
    $partial=$candidate.PSObject.Copy();$partial.CommandLine=$null;$partial.ExecutablePath='C:\foreign.exe'
    Reject { Candidate-Record $partial $exe $argument }
    $partial=$candidate.PSObject.Copy();$partial.ExecutablePath=$null;$partial.CommandLine+=' --independent-mode'
    Reject { Candidate-Record $partial $exe $argument }
}
Check 'missing candidate metadata after positive identity is a hard stop' {
    Reject { Candidate-Record $null $exe $argument $candidateProof.creationUtc }
    $partial=$candidate.PSObject.Copy();$partial.CommandLine=$null
    Reject { Candidate-Record $partial $exe $argument $candidateProof.creationUtc }
}
$viewExe=Join-Path $testRoot 'msedgewebview2.exe';Set-Content -LiteralPath $viewExe -Value 'not executable'
$browser=[pscustomobject]@{ProcessId=101;ParentProcessId=100;Name='msedgewebview2.exe';ExecutablePath=$viewExe;CommandLine=('"'+$viewExe+'" --embedded-browser-webview=1 --user-data-dir="'+$webRoot+'"');CreationDate=$candidate.CreationDate.AddSeconds(1)}
$utility=[pscustomobject]@{ProcessId=102;ParentProcessId=101;Name='msedgewebview2.exe';ExecutablePath=$viewExe;CommandLine=('"'+$viewExe+'" --type=utility --user-data-dir="'+$webRoot+'"');CreationDate=$candidate.CreationDate.AddSeconds(2)}
$script:fakeProcesses=@{101=$browser;102=$utility}
function Get-Live([int]$ProcessId) { return $script:fakeProcesses[$ProcessId] }
$profile=[IO.Path]::GetDirectoryName($webRoot)
$viewProof=Verify-WebViews $candidate $profile @($browser,$utility)
Check 'browser is positively distinguished from live utility' {
    Require ($viewProof.browser.pid -eq 101)
    Require ($viewProof.webviews.Count -eq 2)
}
Check 'vanished utility is skipped while persistent browser remains verified' {
    $script:fakeProcesses.Remove(102)
    $latest=Verify-WebViews $candidate $profile @($browser,$utility) $viewProof.browser
    Require ($latest.browser.pid -eq 101)
    Require ($latest.webviews.Count -eq 1)
}
Check 'vanished persistent browser remains a hard stop' {
    $script:fakeProcesses.Remove(101)
    Reject { Verify-WebViews $candidate $profile @($browser,$utility) $viewProof.browser }
    $script:fakeProcesses[101]=$browser
}
Check 'new foreign live descendant refuses final verification' {
    $foreign=$utility.PSObject.Copy();$foreign.CommandLine=('"'+$viewExe+'" --type=utility --user-data-dir="'+$testRoot+'"')
    $script:fakeProcesses[102]=$foreign
    Reject { Verify-WebViews $candidate $profile @($browser,$foreign) $viewProof.browser }
}
Check 'live descendant with unknown coordinates refuses' {
    $unknown=$utility.PSObject.Copy();$unknown.CommandLine=$null
    $script:fakeProcesses[102]=$unknown
    Reject { Verify-WebViews $candidate $profile @($browser,$unknown) $viewProof.browser }
}
Check 'utility alone cannot stand in for persistent browser' {
    $direct=$utility.PSObject.Copy();$direct.ParentProcessId=100
    $script:fakeProcesses=@{102=$direct}
    $latest=Verify-WebViews $candidate $profile @($direct)
    Require ($null -eq $latest.browser)
}

# Owner coordinate checks use an in-memory process record; no process is launched.
$ownerExe=Join-Path $testRoot 'xnexus-ui-owner.exe';Set-Content -LiteralPath $ownerExe -Value 'not executable'
$owner=[pscustomobject]@{pid=4321;root=$testRoot;executable=$ownerExe;arguments="--fixture-root=$testRoot";synthetic=$true}
@{pid=4321;root=$testRoot;synthetic=$true} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $testRoot 'owner.json')
$script:fakeLive=[pscustomobject]@{ProcessId=4321;ParentProcessId=1;Name='xnexus-ui-owner.exe';ExecutablePath=$ownerExe;CommandLine=('"'+$ownerExe+'" "--fixture-root='+$testRoot+'"');CreationDate=[DateTime]::UtcNow}
function Get-Live([int]$ProcessId) { return $script:fakeLive }
Check 'matching owner process is positively verified' { Require ((Verify-Owner $owner $testRoot).pid -eq 4321) }
Check 'owner extra arguments refuse' {
    $saved=$script:fakeLive.CommandLine
    $script:fakeLive.CommandLine+=' --production'
    Reject { Verify-Owner $owner $testRoot }
    $script:fakeLive.CommandLine=$saved
}
Check 'recycled owner creation time refuses' { Reject { Verify-Owner $owner $testRoot '2000-01-01T00:00:00.0000000Z' } }
Check 'exited owner refuses' {
    $saved=$script:fakeLive;$script:fakeLive=$null
    Reject { Verify-Owner $owner $testRoot }
    $script:fakeLive=$saved
}
Check 'STOP sentinel refuses a still-live owner' {
    New-Item -ItemType File -Path (Join-Path $testRoot 'STOP') | Out-Null
    Reject { Verify-Owner $owner $testRoot }
}
@{status='passed';candidateLaunched=$false;testCount=$results.Count;tests=$results;launcherSha256=(Get-FileHash -LiteralPath $scriptFile -Algorithm SHA256).Hash} |
    ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $testRoot 'results.json') -Encoding utf8NoBOM
[pscustomobject]@{passed=$results.Count;candidateLaunched=$false;evidence=$testRoot}
