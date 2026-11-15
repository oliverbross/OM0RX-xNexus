#Requires -Version 7.4
# Acceptance-only focused compiler/driver. Invoke INSIDE the reviewed private Job guard.
param([Parameter(Mandatory)][string]$Evidence,
      [Parameter(Mandatory)][string]$OwnerRoot,
      [Parameter(Mandatory)][string]$DependencyReceipt)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$source='ae35fc6435f861e38029049299833afe57de4885'
$ownerSource='8bd486abd2e62aa079e8b4248126e1e186561452'
if(Test-Path (Join-Path $Evidence 'compile-receipt.json')){throw 'Fresh evidence directory required'}
New-Item -ItemType Directory -Force -Path $Evidence | Out-Null
& git -C $repo diff --exit-code $source -- src-tauri integrations/xlog integrations/xlog-rx/src
if($LASTEXITCODE -ne 0){throw 'Product source differs from frozen .4'}
$ownerFiles=@('companion_bridge.rs','companion_rx_buffer.rs','companion_needs.rs')
foreach($file in $ownerFiles){& git -C $OwnerRoot diff --exit-code $ownerSource -- "prototype/src-tauri/src/$file";if($LASTEXITCODE -ne 0){throw "Owner source changed: $file"}}
if((Get-FileHash -LiteralPath $DependencyReceipt).Hash -ne '22D0FF86D35F8B7B50AEBEBF9FB300A9B7CF59732A49B8E905A6E85EB4E851E8'){throw 'Preserved rlib receipt changed'}
$prior=Get-Content -LiteralPath $DependencyReceipt -Raw|ConvertFrom-Json
$libs=@{};foreach($lib in $prior.rlibs){if((Get-FileHash -LiteralPath $lib.path).Hash -ne $lib.sha256){throw "Rlib changed $($lib.name)"};$libs[$lib.name]=$lib.path}
$deps=Split-Path $libs.tempo_core -Parent
$env:PATH="C:\msys64\ucrt64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
$toolchain='1.93.1-x86_64-pc-windows-gnu'
$common=@("+$toolchain",'--edition=2021','-C','opt-level=1','-L',"dependency=$deps",'-L','native=C:\msys64\ucrt64\lib')
$sources=@(Get-ChildItem (Join-Path $repo 'integrations/xlog-paired') -Recurse -File | Where-Object Extension -In @('.rs','.toml') | ForEach-Object FullName)+
 @(Get-ChildItem (Join-Path $repo 'integrations/xlog/src') -Recurse -Filter '*.rs' | ForEach-Object FullName)+
 @('src-tauri/src/xlog_rx_worker.rs','src-tauri/src/xlog_decoded_needs.rs','src-tauri/src/xlog_decode.rs','integrations/xlog-rx/src/lib.rs','scripts/xnexus/start-ui-owner.ps1' | ForEach-Object {Join-Path $repo $_})+
 @($ownerFiles | ForEach-Object {Join-Path $OwnerRoot "prototype/src-tauri/src/$_"})+@($PSCommandPath)
$hashes=@($sources|Sort-Object -Unique|ForEach-Object{@{path=$_;sha256=(Get-FileHash -LiteralPath $_).Hash}})
$receipt=[ordered]@{status='building';productSource=$source;fixtureBase=(git -C $repo rev-parse HEAD).Trim();ownerSource=$ownerSource;sourceHashes=$hashes;rlibs=$prior.rlibs;dependencyReceiptHash=(Get-FileHash -LiteralPath $DependencyReceipt).Hash;toolchain=$toolchain;hardware=$false;gui=$false;actualSwift=$false;syntheticClassification=$true;startedUtc=[DateTime]::UtcNow.ToString('o')}
function Save {$receipt|ConvertTo-Json -Depth 8|Set-Content -LiteralPath (Join-Path $Evidence 'compile-receipt.json')}
function Compile([string]$Name,[string[]]$Tail){& rustc @common @Tail *> (Join-Path $Evidence "$Name.log");if($LASTEXITCODE -ne 0){throw "Compilation failed: $Name"}}
Save
try{
 $modules=@($ownerFiles|ForEach-Object{$name=[IO.Path]::GetFileNameWithoutExtension($_);$path=(Join-Path $OwnerRoot "prototype/src-tauri/src/$_")|ConvertTo-Json -Compress;"#[path = $path] mod $name;"})
 $modules|Set-Content -LiteralPath (Join-Path $Evidence 'owner_transport.rs') -Encoding utf8
 $env:OUT_DIR=$Evidence
 $external=@('--extern',"serde=$($libs.serde)",'--extern',"serde_json=$($libs.serde_json)",'--extern',"base64=$($libs.base64)",'--extern',"uuid=$($libs.uuid)")
 $owner=Join-Path $Evidence 'xnexus-ui-owner.exe';$ownerSourcePath=Join-Path $repo 'integrations/xlog-paired/src/bin/ui-owner-fixture.rs'
 Compile 'owner' (@('--cfg','feature="owner-audio"',$ownerSourcePath,'-o',$owner)+$external)
 $callback=Join-Path $Evidence 'callback-tests.exe'
 Compile 'callback' (@('--cfg','feature="owner-audio"','--test',$ownerSourcePath,'-o',$callback)+$external)
 & $callback 'ui_fixture_audio::tests::period_wrap_and_origin_are_sample_aligned' --exact --nocapture *> (Join-Path $Evidence 'callback-tests.log')
 if($LASTEXITCODE -ne 0 -or !(Select-String -LiteralPath (Join-Path $Evidence 'callback-tests.log') -Pattern '1 passed; 0 failed')){throw 'Callback regression failed'}
 $generator=Join-Path $Evidence 'generate-ui-ft4.exe'
 Compile 'generator' @((Join-Path $repo 'integrations/xlog-paired/src/generate_ui_ft4.rs'),'--extern',"tempo_core=$($libs.tempo_core)",'-o',$generator)
 $wave=Join-Path $Evidence 'generated-ft4.s16le'; & $generator $wave *> (Join-Path $Evidence 'generator-run.log');if($LASTEXITCODE -ne 0){throw 'Generator failed'}
 $sdk=Join-Path $Evidence 'libxnexus_xlog_client.rlib';$rx=Join-Path $Evidence 'libxnexus_xlog_rx.rlib'
 Compile 'sdk' (@('--crate-name','xnexus_xlog_client','--crate-type','rlib',(Join-Path $repo 'integrations/xlog/src/lib.rs'),'--extern',"reqwest=$($libs.reqwest)",'-o',$sdk)+$external)
 Compile 'assembler' @('--crate-name','xnexus_xlog_rx','--crate-type','rlib',(Join-Path $repo 'integrations/xlog-rx/src/lib.rs'),'--extern',"tempo_codec=$($libs.tempo_codec)",'-o',$rx)
 $workerModules=Join-Path $Evidence 'worker-modules.rs'
 @('xlog_decode','xlog_rx_worker')|ForEach-Object{$path=(Join-Path $repo "src-tauri/src/$_.rs")|ConvertTo-Json -Compress;"#[path = $path] mod $_;"}|Set-Content -LiteralPath $workerModules -Encoding utf8
 $env:XNEXUS_UI_WORKER_MODULES=$workerModules;$headless=Join-Path $Evidence 'ui-fixture-headless.exe'
 Compile 'headless' (@((Join-Path $repo 'integrations/xlog-paired/src/ui_fixture_headless.rs'),'--extern',"xnexus_xlog_client=$sdk",'--extern',"xnexus_xlog_rx=$rx",'--extern',"tempo_core=$($libs.tempo_core)",'-o',$headless)+$external)
 $env:XNEXUS_UI_OWNER=$owner;$env:XNEXUS_UI_WAVE=$wave;$env:XNEXUS_UI_EVIDENCE=$Evidence
 $receipt.status='running';Save
 & $headless *> (Join-Path $Evidence 'headless.log');if($LASTEXITCODE -ne 0){throw 'Headless lifecycle failed'}
 foreach($h in $hashes){if((Get-FileHash -LiteralPath $h.path).Hash -ne $h.sha256){throw "Source changed: $($h.path)"}}
 foreach($lib in $prior.rlibs){if((Get-FileHash -LiteralPath $lib.path).Hash -ne $lib.sha256){throw "Retained rlib changed: $($lib.name)"}}
 $receipt.outputs=@($owner,$callback,$generator,$wave,$headless|ForEach-Object{@{path=$_;sha256=(Get-FileHash -LiteralPath $_).Hash}})
 $receipt.status='passed-headless-no-gui';Save
 Get-Content -LiteralPath (Join-Path $Evidence 'headless.log')
}catch{$receipt.status='failed';$receipt.error=$_.Exception.Message;Save;throw}
