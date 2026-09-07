param([string]$SdkRoot = $env:ANDROID_SDK_ROOT, [int]$Samples = 10, [switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$workspace=Split-Path -Parent $PSScriptRoot
if(-not $SkipBuild){& (Join-Path $PSScriptRoot 'build-m5.ps1') -SdkRoot $SdkRoot}
$out=Join-Path $workspace 'testdata/generated/m5'
$adb=Join-Path $SdkRoot 'platform-tools/adb.exe'; $serials=@(& $adb devices|Select-Object -Skip 1|ForEach-Object{if($_ -match '^([^\s]+)\s+device$'){$Matches[1]}});$arm64=@($serials|Where-Object{(& $adb -s $_ shell getprop ro.product.cpu.abi).Trim() -match '^arm64'})
if($arm64.Count -ne 1){throw "Expected exactly one ARM64 device; found $($arm64.Count)."}; $serial=$arm64[0]; $package='dev.aic.generated.optimizer'; $rows=@(); $actions=@()
function Get-Ui { & $adb -s $serial shell uiautomator dump /sdcard/aic-m5-bench.xml | Out-Null; (& $adb -s $serial shell cat /sdcard/aic-m5-bench.xml) | Out-String }
function Get-Median([object[]]$Values) { $sorted=@($Values|Sort-Object);$middle=[math]::Floor($sorted.Count/2);if($sorted.Count%2 -eq 0){return ($sorted[$middle-1]+$sorted[$middle])/2}return $sorted[$middle] }
foreach($level in @(0,1)){
 $apk=Join-Path $workspace "testdata/generated/m5/optimizer-o$level/hello-signed.apk"; & $adb -s $serial uninstall $package|Out-Null; & $adb -s $serial install $apk|Out-Null;if($LASTEXITCODE -ne 0){throw "M5 O$level installation failed; unlock the device and approve USB installation."}
 & $adb -s $serial shell am force-stop $package; & $adb -s $serial shell am start -W -n "$package/.MainActivity"|Out-Null
 foreach($sample in 1..$Samples){& $adb -s $serial shell am force-stop $package;$raw=(& $adb -s $serial shell am start -W -n "$package/.MainActivity")|Out-String;$total=if($raw -match 'TotalTime:\s*(\d+)'){[int]$Matches[1]}else{$null};$rows += [pscustomobject]@{optimizationLevel=$level;sample=$sample;coldStartTotalMs=$total}}
 $ui=Get-Ui; if($ui -notmatch 'text="Increment"[^>]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"'){throw 'Increment control not found.'};$x=([int]$Matches[1]+[int]$Matches[3])/2;$y=([int]$Matches[2]+[int]$Matches[4])/2
 $watch=[Diagnostics.Stopwatch]::StartNew(); & $adb -s $serial shell input tap $x $y|Out-Null; do{Start-Sleep -Milliseconds 25;$ui=Get-Ui}while($ui -notmatch 'Count: 1' -and $watch.ElapsedMilliseconds -lt 5000);$watch.Stop();if($ui -notmatch 'Count: 1'){throw "O$level interaction timed out."};$actions += [pscustomobject]@{optimizationLevel=$level;interactionWallMs=$watch.ElapsedMilliseconds}
 & $adb -s $serial shell dumpsys meminfo $package -d | Set-Content -LiteralPath (Join-Path $out "meminfo-o$level-raw.txt")
}
$summary=@(foreach($level in @(0,1)){$times=@($rows|Where-Object optimizationLevel -eq $level|Select-Object -ExpandProperty coldStartTotalMs);[pscustomobject]@{optimizationLevel=$level;medianColdStartMs=(Get-Median $times);interactionWallMs=($actions|Where-Object optimizationLevel -eq $level).interactionWallMs}})
$rows|Export-Csv -NoTypeInformation -LiteralPath (Join-Path $out 'device-results.csv');@{samples=$rows;summary=$summary;actions=$actions}|ConvertTo-Json -Depth 4|Set-Content -LiteralPath (Join-Path $out 'device-results.json')
Set-Content -LiteralPath (Join-Path $out 'device-profile.txt') -Value @("serial=$serial","model=$((& $adb -s $serial shell getprop ro.product.model).Trim())","abi=$((& $adb -s $serial shell getprop ro.product.cpu.abi).Trim())","android=$((& $adb -s $serial shell getprop ro.build.version.release).Trim())","fingerprint=$((& $adb -s $serial shell getprop ro.build.fingerprint).Trim())","samples=$Samples")
Set-Content -LiteralPath (Join-Path $out 'allocation-method.txt') -Value 'Preferred: Perfetto heapprofd on a supported debuggable reference build. Fallback recorded for this run: dumpsys meminfo -d object and heap counters; allocation total unavailable.'
Write-Output 'M5 raw device benchmark results recorded.'
