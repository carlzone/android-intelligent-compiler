param(
    [string]$SdkRoot = $env:ANDROID_SDK_ROOT,
    [string]$Keystore,
    [switch]$SkipBuild,
    [ValidateSet(0, 1)][int[]]$Levels = @(0, 1)
)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$adb = Join-Path $SdkRoot 'platform-tools/adb.exe'
if (-not $SkipBuild) { & (Join-Path $PSScriptRoot 'build-m4.ps1') -SdkRoot $SdkRoot -Keystore $Keystore }
$serials = @(& $adb devices | Select-Object -Skip 1 | ForEach-Object { if ($_ -match '^([^\s]+)\s+device$') { $Matches[1] } })
$arm64 = @($serials | Where-Object { (& $adb -s $_ shell getprop ro.product.cpu.abi).Trim() -match '^arm64' })
if ($arm64.Count -ne 1) { throw "Expected exactly one authorized ARM64 device; found $($arm64.Count)." }
$serial = $arm64[0]
function Get-Ui { & $adb -s $serial shell uiautomator dump /sdcard/aic-m4.xml | Out-Null; (& $adb -s $serial shell cat /sdcard/aic-m4.xml) | Out-String }
function Tap-Text([string]$text) { $ui = Get-Ui; $pattern = 'text="' + [regex]::Escape($text) + '"[^>]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"'; if ($ui -notmatch $pattern) { throw "Control not found: $text" }; & $adb -s $serial shell input tap (([int]$Matches[1]+[int]$Matches[3])/2) (([int]$Matches[2]+[int]$Matches[4])/2) | Out-Null }
function Enter-Text([string]$control, [string]$value) { Tap-Text $control; & $adb -s $serial shell input text $value | Out-Null }
function Assert-Visible([string]$value) { foreach ($attempt in 1..6) { Start-Sleep -Milliseconds 400; if ((Get-Ui) -match [regex]::Escape($value)) { return } }; throw "Expected UI text not found: $value" }
foreach ($level in $Levels) {
    $output = Join-Path $workspace "testdata/generated/m4/notes-o$level"; $apk = Join-Path $output 'hello-signed.apk'; $package = 'dev.aic.generated.notes'
    & $adb -s $serial uninstall $package | Out-Null
    & $adb -s $serial install $apk | Out-Null; if ($LASTEXITCODE -ne 0) { throw "notes O$level installation failed." }
    & $adb -s $serial logcat -c; & $adb -s $serial shell am start -W -n "$package/.MainActivity" | Out-Null
    Enter-Text 'Title' 'First'; Enter-Text 'Body' 'Original'; Tap-Text 'Create'; Assert-Visible 'Created'
    & $adb -s $serial shell am force-stop $package; & $adb -s $serial shell am start -W -n "$package/.MainActivity" | Out-Null; Assert-Visible '1'; Tap-Text 'Read'; Assert-Visible 'Found'; Assert-Visible 'First'; Assert-Visible 'Original'
    Tap-Text 'First'; & $adb -s $serial shell input keyevent 123 | Out-Null; 1..20 | ForEach-Object { & $adb -s $serial shell input keyevent 67 | Out-Null }; & $adb -s $serial shell input text Updated | Out-Null; Tap-Text 'Update'; Assert-Visible 'Updated'
    & $adb -s $serial shell am force-stop $package; & $adb -s $serial shell am start -W -n "$package/.MainActivity" | Out-Null; Tap-Text 'Read'; Assert-Visible 'Updated'; Tap-Text 'Delete'; Assert-Visible 'Deleted'
    & $adb -s $serial shell am force-stop $package; & $adb -s $serial shell am start -W -n "$package/.MainActivity" | Out-Null; Tap-Text 'Read'; Assert-Visible 'Not found'
    Tap-Text '1'; & $adb -s $serial shell input keyevent 123 | Out-Null; 1..20 | ForEach-Object { & $adb -s $serial shell input keyevent 67 | Out-Null }; & $adb -s $serial shell input text 99999999999 | Out-Null; Tap-Text 'Read'; Assert-Visible 'Invalid ID'
    $crashes = (& $adb -s $serial logcat -d -b crash) | Out-String; if ($crashes -match 'FATAL EXCEPTION') { throw "notes O$level crashed: $crashes" }
    Set-Content -LiteralPath (Join-Path $output 'device-evidence.txt') -Value @("serial=$serial", "fixture=notes", "optLevel=$level", 'crud=create,read,update,delete', 'relaunch=persistent', 'permissions=none', 'crashBuffer=clean', 'result=PASS')
    Write-Output "notes O$level PASS"
}
