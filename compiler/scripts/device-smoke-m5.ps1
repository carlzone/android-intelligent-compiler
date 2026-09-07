param([string]$SdkRoot = $env:ANDROID_SDK_ROOT, [string]$Keystore, [switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
if (-not $SkipBuild) { & (Join-Path $PSScriptRoot 'build-m5.ps1') -SdkRoot $SdkRoot -Keystore $Keystore }
$adb = Join-Path $SdkRoot 'platform-tools/adb.exe'
$serials = @(& $adb devices | Select-Object -Skip 1 | ForEach-Object { if ($_ -match '^([^\s]+)\s+device$') { $Matches[1] } })
$arm64 = @($serials | Where-Object { (& $adb -s $_ shell getprop ro.product.cpu.abi).Trim() -match '^arm64' })
if ($arm64.Count -ne 1) { throw "Expected exactly one authorized ARM64 device; found $($arm64.Count)." }
$serial = $arm64[0]; $package = 'dev.aic.generated.optimizer'
function Get-Ui { & $adb -s $serial shell uiautomator dump /sdcard/aic-m5.xml | Out-Null; (& $adb -s $serial shell cat /sdcard/aic-m5.xml) | Out-String }
function Tap-Text([string]$text) { $ui=Get-Ui; $pattern='text="'+[regex]::Escape($text)+'"[^>]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"'; if($ui -notmatch $pattern){throw "Control not found: $text"}; & $adb -s $serial shell input tap (([int]$Matches[1]+[int]$Matches[3])/2) (([int]$Matches[2]+[int]$Matches[4])/2) | Out-Null }
foreach ($level in @(0,1)) {
    $output=Join-Path $workspace "testdata/generated/m5/optimizer-o$level"; $apk=Join-Path $output 'hello-signed.apk'
    & $adb -s $serial uninstall $package | Out-Null; & $adb -s $serial install $apk | Out-Null
    & $adb -s $serial logcat -c; & $adb -s $serial shell am start -W -n "$package/.MainActivity" | Out-Null
    Tap-Text 'Increment'; Start-Sleep -Milliseconds 400
    if ((Get-Ui) -notmatch 'Count: 1') { throw "M5 O$level behavior mismatch." }
    $crashes=(& $adb -s $serial logcat -d -b crash)|Out-String; if($crashes -match 'FATAL EXCEPTION'){throw "M5 O$level crashed: $crashes"}
    Set-Content -LiteralPath (Join-Path $output 'device-evidence.txt') -Value @("serial=$serial","optLevel=$level",'visible=Count: 1','crashBuffer=clean','result=PASS')
}
Write-Output 'M5 device differential PASS'
