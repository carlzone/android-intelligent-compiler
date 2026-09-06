param([string]$SdkRoot = $env:ANDROID_SDK_ROOT, [string]$Keystore, [switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$adb = Join-Path $SdkRoot 'platform-tools/adb.exe'
if (-not $SkipBuild) { & (Join-Path $PSScriptRoot 'build-m3.ps1') -SdkRoot $SdkRoot -Keystore $Keystore }
$serials = @(& $adb devices | Select-Object -Skip 1 | ForEach-Object { if ($_ -match '^([^\s]+)\s+device$') { $Matches[1] } })
$arm64 = @($serials | Where-Object { (& $adb -s $_ shell getprop ro.product.cpu.abi).Trim() -match '^arm64' })
if ($arm64.Count -ne 1) { throw "Expected exactly one authorized ARM64 device; found $($arm64.Count)." }
$serial = $arm64[0]
function Get-Ui { & $adb -s $serial shell uiautomator dump /sdcard/aic-m3.xml | Out-Null; (& $adb -s $serial shell cat /sdcard/aic-m3.xml) | Out-String }
function Tap-Text([string]$text) {
    $ui = Get-Ui
    $escaped = [regex]::Escape($text)
    $pattern = 'text="' + $escaped + '"[^>]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"'
    if ($ui -notmatch $pattern) { throw "Control not found: $text" }
    $x = ([int]$Matches[1] + [int]$Matches[3]) / 2; $y = ([int]$Matches[2] + [int]$Matches[4]) / 2
    & $adb -s $serial shell input tap $x $y | Out-Null
}
function Assert-Visible([string]$expected, [string]$failure) {
    foreach ($attempt in 1..5) {
        Start-Sleep -Milliseconds 500
        if ((Get-Ui) -match [regex]::Escape($expected)) { return }
    }
    throw $failure
}
foreach ($level in @(0, 1)) {
    $output = Join-Path $workspace "testdata/generated/m3/counter-o$level"
    & $adb -s $serial install -r (Join-Path $output 'hello-signed.apk') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Counter O$level installation failed." }
    & $adb -s $serial logcat -c; & $adb -s $serial shell am force-stop dev.aic.generated.counter
    & $adb -s $serial shell am start -W -n dev.aic.generated.counter/.MainActivity | Out-Null
    Start-Sleep -Milliseconds 700
    Tap-Text 'Increment'
    Assert-Visible 'Count: 1' "Counter O$level did not increment."
    Tap-Text 'Decrement'; Assert-Visible 'Count: 0' "Counter O$level did not decrement."
    Tap-Text 'Increment'; Assert-Visible 'Count: 1' "Counter O$level second increment failed."
    & $adb -s $serial shell am force-stop dev.aic.generated.counter
    & $adb -s $serial shell am start -W -n dev.aic.generated.counter/.MainActivity | Out-Null
    Assert-Visible 'Count: 0' "Counter O$level did not reset after recreation."
    Tap-Text 'Increment'; Assert-Visible 'Count: 1' "Counter O$level post-recreation increment failed."
    Tap-Text 'Reset'; Assert-Visible 'Count: 0' "Counter O$level did not reset."
    $crashes = (& $adb -s $serial logcat -d -b crash) | Out-String
    if ($crashes -match 'FATAL EXCEPTION') { throw "Counter O$level crashed: $crashes" }
    Set-Content -LiteralPath (Join-Path $output 'device-evidence.txt') -Value @("serial=$serial", "fixture=counter", "optLevel=$level", 'interactions=increment,decrement,reset', 'recreation=initial-state-reset', 'crashBuffer=clean', 'result=PASS')
    Write-Output "counter O$level PASS"
}
function Run-Calculator([int]$level, [string]$left, [string]$right, [string]$operation, [string]$expected) {
    & $adb -s $serial shell am force-stop dev.aic.generated.calculator
    & $adb -s $serial shell am start -W -n dev.aic.generated.calculator/.MainActivity | Out-Null
    Start-Sleep -Milliseconds 500
    Tap-Text 'First integer'; & $adb -s $serial shell input text $left | Out-Null
    Tap-Text 'Second integer'; & $adb -s $serial shell input text $right | Out-Null
    Tap-Text $operation
    Assert-Visible $expected "Calculator O$level $operation did not show $expected."
}
foreach ($level in @(0, 1)) {
    $output = Join-Path $workspace "testdata/generated/m3/calculator-o$level"
    & $adb -s $serial install -r (Join-Path $output 'hello-signed.apk') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Calculator O$level installation failed." }
    & $adb -s $serial logcat -c
    Run-Calculator $level '7' '3' 'Add' '10'
    Run-Calculator $level '7' '3' 'Subtract' '4'
    Run-Calculator $level '7' '3' 'Multiply' '21'
    Run-Calculator $level '7' '3' 'Divide' '2'
    Run-Calculator $level '7' '0' 'Divide' 'Cannot divide by zero'
    Run-Calculator $level '3000000000' '1' 'Add' 'Invalid input'
    $crashes = (& $adb -s $serial logcat -d -b crash) | Out-String
    if ($crashes -match 'FATAL EXCEPTION') { throw "Calculator O$level crashed: $crashes" }
    Set-Content -LiteralPath (Join-Path $output 'device-evidence.txt') -Value @("serial=$serial", "fixture=calculator", "optLevel=$level", 'operations=add,subtract,multiply,divide,zero-divisor,invalid-input', 'recreation=initial-state-reset', 'crashBuffer=clean', 'result=PASS')
    Write-Output "calculator O$level PASS"
}
