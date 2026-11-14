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
$branch='codex/upstream-'+$upstream.Substring(0,12)
GitChecked switch -c $branch
Write-Output ('Pre-merge fork SHA: '+(GitChecked rev-parse HEAD))
GitChecked merge --no-ff --no-commit $upstream
Write-Output 'Review all changes and conflicts. Update UPSTREAM.json only after tests. No commit, push, release or installation was performed.'
