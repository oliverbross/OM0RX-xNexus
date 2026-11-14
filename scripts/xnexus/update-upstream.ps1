param([switch]$PrepareMerge)
$ErrorActionPreference='Stop'
$repo=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
function GitChecked {
    $result=& git -C $repo @args
    if($LASTEXITCODE -ne 0){throw ('Git failed: '+($args -join ' '))}
    return $result
}
$origin=GitChecked remote get-url origin
if($origin -ne 'https://github.com/oliverbross/OM0RX-xNexus.git'){throw 'Unexpected fork origin'}
if((GitChecked remote get-url upstream) -ne 'https://github.com/kd9taw/Nexus.git'){throw 'Unexpected upstream'}
$pin=Get-Content (Join-Path $repo 'docs/xlog/UPSTREAM.json') -Raw | ConvertFrom-Json
GitChecked fetch upstream main
$upstream=(GitChecked rev-parse upstream/main).Trim()
GitChecked merge-base --is-ancestor $pin.commit upstream/main
Write-Output ('Pinned upstream: '+$pin.commit)
Write-Output ('Available upstream: '+$upstream)
GitChecked log --oneline ($pin.commit+'..'+$upstream)
if(-not $PrepareMerge){Write-Output 'Report only. Use -PrepareMerge from clean current main to prepare a review branch.';exit 0}
if(GitChecked status --porcelain){throw 'Preserve and commit your own work before merging; no stash/reset performed'}
if((GitChecked branch --show-current) -ne 'main'){throw 'Prepare the update from main'}
GitChecked fetch origin main
if((GitChecked rev-parse HEAD) -ne (GitChecked rev-parse origin/main)){throw 'Local main differs from origin/main; reconcile explicitly'}
$head=(GitChecked rev-parse HEAD).Trim()
if ($upstream -eq $pin.commit) { Write-Output 'The recorded upstream pin is current; no review checkout needed.'; exit 0 }
$branch='codex/upstream-'+$upstream.Substring(0,12)
$review=Join-Path $repo ('target/upstream-review/'+$upstream.Substring(0,12))
if (Test-Path -LiteralPath $review) { throw ('Review checkout already exists; inspect its unique work before reusing: '+$review) }
# Keep the canonical checkout on main. Builds and the running installation must
# never start using a half-resolved upstream merge merely because it was prepared.
GitChecked worktree add -b $branch $review $head
Write-Output ('Pre-merge fork SHA: '+$head)
Write-Output ('Review checkout: '+$review)
& git -C $review merge --no-ff --no-commit $upstream
$mergeExit=$LASTEXITCODE
if ((GitChecked rev-parse HEAD).Trim() -ne $head -or (GitChecked branch --show-current) -ne 'main') { throw 'Canonical checkout changed unexpectedly; inspect before proceeding.' }
if ($mergeExit -ne 0) { throw ('Upstream merge needs review in '+$review+'. Conflicts and all evidence are preserved; no abort/reset performed.') }
Write-Output 'Review the isolated merge and run fork/native gates there. Update UPSTREAM.json only after tests. No commit, push, release or installation was performed.'
