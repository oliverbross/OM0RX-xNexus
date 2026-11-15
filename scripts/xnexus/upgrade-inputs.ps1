# SPDX-License-Identifier: GPL-3.0-only
. (Join-Path $PSScriptRoot 'upgrade-transaction.ps1')

function Read-XnUpgradeJson([string]$Path) {
    $null=Assert-XnLocalPath $Path
    if((Get-Item -LiteralPath $Path).Length -gt 4MB){throw 'Upgrade JSON input exceeds 4 MiB.'}
    Get-Content -LiteralPath $Path -Raw|ConvertFrom-Json -AsHashtable
}

function Get-XnUpgradeToolNames {
    @('execute-owned-upgrade.ps1','upgrade-inputs.ps1','upgrade-transaction.ps1','installer-transaction.ps1','installer-acceptance-lib.ps1','installer-observer.ps1','installer-observer.cs')
}

function Assert-XnUpgradeQualification($Evidence,[string]$Source) {
    if($Evidence.schema -cne 'xnexus-4-to-5-qualification' -or $Evidence.source -cne $Source -or
        $Evidence.version -cne '1.24.1-xlog.5' -or $Evidence.status -cne 'passed' -or
        $Evidence.privatePairedRetryDecoderPassed -cne $true -or $Evidence.normalLifecyclePassed -cne $true -or
        $Evidence.profileBaselineReceiptSha256 -ine 'D5182423C6DB8275DF4D5AFA4A91326D2B8ED2D8D894E0C8F29B1AB2C64C9FB7') {throw 'Require reviewed candidate-specific paired retry/decoder and lifecycle evidence.'}
    if(@($Evidence.receipts).Count -lt 2 -or @($Evidence.receipts).Count -gt 16){throw 'Qualification receipt inventory incomplete or oversized.'}
    $seen=@{};foreach($receipt in $Evidence.receipts){
        if($seen.ContainsKey($receipt.path)){throw 'Duplicate qualification evidence'};$seen[$receipt.path]=$true
        Assert-XnHash $receipt.path $receipt.sha256
    }
}

function Get-XnUpgradeContext($Review) {
    $repo='C:\Projects\OM0RX-xNexus'
    if($Review.schema -ne 1 -or $Review.newSource -notmatch '^[0-9a-f]{40}$' -or $Review.newVersion -cne '1.24.1-xlog.5'){throw 'Require an exact source-pinned .5 review plan.'}
    $required=@('oldInstallation','oldLaunch','oldAudit','newBuild','newAudit','newConfig','newNsis','newInstaller','qualification')
    if($Review.inputs.Count -ne $required.Count){throw 'Unexpected or missing upgrade input.'}
    foreach($name in $required){if(!$Review.inputs.ContainsKey($name)){throw "Missing upgrade input $name"};Assert-XnHash $Review.inputs[$name].path $Review.inputs[$name].sha256}
    $toolNames=@(Get-XnUpgradeToolNames)
    if($Review.tools.Count -ne $toolNames.Count){throw 'Incomplete reviewed helper inventory.'}
    foreach($name in $toolNames){if(!$Review.tools.ContainsKey($name)){throw "Missing tool pin $name"};Assert-XnHash (Join-Path $PSScriptRoot $name) $Review.tools[$name]}
    # Only the actual installed .4 receipt and verified normal-profile launch are supported.
    if($Review.inputs.oldInstallation.sha256 -ine 'B0FDE05A0AC24A2F6F9427F021914515E855F2AFC97EE09935C1A825B1D813DC' -or
        $Review.inputs.oldLaunch.sha256 -ine 'C069F4EB804E572976B8BD256C771CFA51E750403939789D1FF08F7B5A960D1A' -or
        $Review.inputs.oldAudit.sha256 -ine '0786AFF5B3F7AB70E302280F6535B5B7B2FA9F028075D04DF0D05EEA0AE7A9AB'){throw 'Old installed .4 evidence pin differs.'}
    $old=Read-XnUpgradeJson $Review.inputs.oldInstallation.path
    $launch=Read-XnUpgradeJson $Review.inputs.oldLaunch.path
    $oldAudit=Read-XnUpgradeJson $Review.inputs.oldAudit.path
    $target=Join-Path $env:LOCALAPPDATA 'OM0RX-xNexus'
    $profile=Join-Path $env:APPDATA 'om0rx-xnexus\companion'
    if($env:XNEXUS_PROFILE -or $env:XNEXUS_DATA_DIR){throw 'Custom profile/environment requires separate upgrade review.'}
    if($old.status -cne 'passed-current-user-install-no-app-launch' -or !$old.installedVerified -or $old.forcedCleanup -or $old.target -cne $target -or
        $launch.status -cne 'verified-installed-normal-process-coordinates' -or $launch.source -cne $old.source -or $launch.version -cne $old.version -or
        $launch.arguments -cne '' -or $launch.profile -cne $profile -or $launch.executable -cne (Join-Path $target 'OM0RX-xNexus.exe') -or
        $launch.executableSha256 -ine $oldAudit.payloadExecutableSha256){throw 'Old receipt/launch/profile coordinates differ.'}
    $viewPath=Join-Path $profile 'webview'
    $matches=[regex]::Matches($launch.browser.commandLine,'(?:^|\s)--user-data-dir=(?:"([^"]+)"|(\S+))')
    if($matches.Count -ne 1){throw 'Prior browser data-directory coordinate missing/ambiguous.'}
    $actual=if($matches[0].Groups[1].Success){$matches[0].Groups[1].Value}else{$matches[0].Groups[2].Value}
    if($actual -cne $viewPath){throw 'Prior browser profile is not the supported companion store.'}
    $build=Read-XnUpgradeJson $Review.inputs.newBuild.path;$audit=Read-XnUpgradeJson $Review.inputs.newAudit.path
    Assert-XnReceiptIdentity $build $audit $Review.newVersion $Review.newSource $Review.inputs.newBuild.sha256
    if($Review.inputs.newInstaller.sha256 -ine $audit.installer.sha256 -or $Review.inputs.newConfig.sha256 -ine $audit.distribution.configSha256 -or
        $Review.inputs.newNsis.sha256 -ine $audit.generatedNsisSha256){throw 'New retained inputs differ from passing audit.'}
    if($audit.commonControlsV6 -cne $true){throw 'Current artifact Common Controls audit missing.'}
    Assert-XnHash $oldAudit.distribution.configPath $oldAudit.distribution.configSha256
    $oldConfig=Read-XnUpgradeJson $oldAudit.distribution.configPath;$config=Read-XnUpgradeJson $Review.inputs.newConfig.path
    if($config.version -cne $Review.newVersion){throw 'Distribution version differs.'}
    $oldConfig.Remove('version');$config.Remove('version')
    Assert-XnSame $oldConfig $config 'distribution configuration except candidate version';Assert-XnNoAssociations $config
    Assert-XnSame @($oldAudit.distribution.sourceHashes.Keys|Sort-Object) @($audit.distribution.sourceHashes.Keys|Sort-Object) 'installer policy input inventory'
    foreach($name in $oldAudit.distribution.sourceHashes.Keys){
        if($name -eq 'src-tauri/tauri.conf.json'){continue}
        if($audit.distribution.sourceHashes[$name] -cne $oldAudit.distribution.sourceHashes[$name]){throw "Installer policy source changed: $name"}
    }
    $head=(& git -C $repo rev-parse HEAD).Trim();if($LASTEXITCODE -or $head -cne $Review.newSource){throw 'Canonical candidate source changed.'}
    $dirty=& git -C $repo status --porcelain;if($LASTEXITCODE -or $dirty){throw 'Canonical candidate is not clean.'}
    # Candidate-specific receive UI evidence is still required. Reuse of the
    # historical A/B storage proof binds the unchanged startup and draft paths.
    $boundaries=@('src-tauri/src/lib.rs','src-tauri/src/xlog_mode.rs','src-tauri/src/xlog_profile_store.rs','src-tauri/capabilities','ui/src/xlog/main.tsx','ui/src/xlog/App.tsx','ui/src/xlog/api.ts','ui/src/xlog/draftStore.ts','ui/src/xlog/QsoPanel.tsx','ui/src/xlog/HistoryPanel.tsx')
    $boundaryProof=@{}
    foreach($name in $boundaries){
        $a=(& git -C $repo rev-parse ($old.source+':'+$name)).Trim();if($LASTEXITCODE){throw 'Cannot read old source boundary'}
        $b=(& git -C $repo rev-parse ($Review.newSource+':'+$name)).Trim();if($LASTEXITCODE -or $a -cne $b){throw "Qualification boundary changed: $name"};$boundaryProof[$name]=$a
    }
    $qualification=Read-XnUpgradeJson $Review.inputs.qualification.path;Assert-XnUpgradeQualification $qualification $Review.newSource
    $rendered=Get-Content -LiteralPath $Review.inputs.newNsis.path -Raw
    foreach($line in @('!define INSTALLMODE "currentUser"','!define PRODUCTNAME "OM0RX-xNexus"','!define STARTMENUFOLDER ""','!define BUNDLEID "sk.om0rx.xnexus"','!define ARCH "x64"')){if(!$rendered.Contains($line)){throw 'Generated installer identity changed.'}}
    $minimum=[regex]::Matches($rendered,'(?m)^!define MINIMUMWEBVIEW2VERSION "([^"]*)"\r?$')
    if($minimum.Count -ne 1){throw 'Unknown WebView runtime minimum.'}
    $payloadRoot=Split-Path -Parent $audit.payloadExecutable.path
    $payload=@(foreach($entry in Get-XnPayloadMap $rendered){
        $path=Join-Path $payloadRoot $entry.destination;$hash=Get-XnHash $path
        if($entry.source){Assert-XnHash (Resolve-XnPayloadSource $entry.source $repo) $hash}else{Assert-XnHash $path $audit.payloadExecutableSha256}
        @{destination=$entry.destination;sha256=$hash}
    })
    $context=@{target=$target;profile=$profile;webview=$viewPath;minimumWebview=$minimum[0].Groups[1].Value;sourceBoundaries=$boundaryProof;
        old=@{source=$old.source;version=$old.version;payload=$old.payload;installation=$old.ownedInstallation;registry=$old.after.registry;shortcuts=$old.shortcuts};
        new=@{source=$Review.newSource;version=$Review.newVersion;payload=$payload;installer=$Review.inputs.newInstaller}}
    Assert-XnUpgradeIdentity $context $target
    return $context
}
