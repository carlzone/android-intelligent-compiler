param([string]$SdkRoot = $env:ANDROID_SDK_ROOT)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$adb = Join-Path $SdkRoot 'platform-tools/adb.exe'
$serials = @(& $adb devices | Select-Object -Skip 1 | ForEach-Object { if ($_ -match '^([^\s]+)\s+device$') { $Matches[1] } })
$arm64 = @($serials | Where-Object { (& $adb -s $_ shell getprop ro.product.cpu.abi).Trim() -match '^arm64' })
if ($arm64.Count -ne 1) { throw "Expected exactly one authorized ARM64 device; found $($arm64.Count)." }
$serial = $arm64[0]
foreach ($level in @(0, 1)) {
    $output = Join-Path $workspace "testdata/generated/m2/o$level"
    $apk = Join-Path $output 'hello-signed.apk'
    & $adb -s $serial install -r $apk; if ($LASTEXITCODE -ne 0) { throw "O$level installation failed." }
    & $adb -s $serial logcat -c
    & $adb -s $serial shell am force-stop dev.aic.generated.compute
    & $adb -s $serial shell am start -W -n dev.aic.generated.compute/.MainActivity; if ($LASTEXITCODE -ne 0) { throw "O$level launch failed." }
    Start-Sleep -Seconds 2
    & $adb -s $serial shell uiautomator dump /sdcard/aic-m2-ui.xml | Out-Null
    $ui = & $adb -s $serial shell cat /sdcard/aic-m2-ui.xml
    if ($ui -notmatch 'Result: 36') { throw "O$level expected computation result was not visible." }
    $crashes = & $adb -s $serial logcat -d -b crash
    if (($crashes | Out-String) -match 'dev\.aic\.generated\.compute|FATAL EXCEPTION') { throw "O$level crash output detected: $crashes" }
    Set-Content -LiteralPath (Join-Path $output 'device-evidence.txt') -Value @("serial=$serial", "optLevel=$level", 'visibleText=Result: 36', 'result=PASS')
}
