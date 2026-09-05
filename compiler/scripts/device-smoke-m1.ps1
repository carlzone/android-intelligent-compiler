param([string]$SdkRoot = $env:ANDROID_SDK_ROOT, [string]$OutputDir)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($OutputDir)) { $OutputDir = Join-Path $workspace 'testdata/generated/m1' }
$adb = Join-Path $SdkRoot 'platform-tools/adb.exe'; $apk = Join-Path $OutputDir 'hello-signed.apk'; $evidence = Join-Path $OutputDir 'device-evidence.txt'
$serials = @(& $adb devices | Select-Object -Skip 1 | ForEach-Object { if ($_ -match '^([^\s]+)\s+device$') { $Matches[1] } })
$arm64 = @($serials | Where-Object { (& $adb -s $_ shell getprop ro.product.cpu.abi).Trim() -match '^arm64' })
if ($arm64.Count -ne 1) { throw "Expected exactly one authorized ARM64 device; found $($arm64.Count)." }
$serial = $arm64[0]; $model = (& $adb -s $serial shell getprop ro.product.model).Trim(); $abi = (& $adb -s $serial shell getprop ro.product.cpu.abi).Trim(); $release = (& $adb -s $serial shell getprop ro.build.version.release).Trim(); $fingerprint = (& $adb -s $serial shell getprop ro.build.fingerprint).Trim()
& $adb -s $serial install -r $apk; if ($LASTEXITCODE -ne 0) { throw 'APK installation failed.' }
& $adb -s $serial logcat -c
& $adb -s $serial shell am force-stop dev.aic.generated.hello
& $adb -s $serial shell am start -W -n dev.aic.generated.hello/.MainActivity; if ($LASTEXITCODE -ne 0) { throw 'Activity launch failed.' }
Start-Sleep -Seconds 2
& $adb -s $serial shell uiautomator dump /sdcard/aic-m1-ui.xml | Out-Null
$ui = & $adb -s $serial shell cat /sdcard/aic-m1-ui.xml
if ($ui -notmatch 'Hello from AndroidIntelligentCompiler') { throw 'Expected Hello World text was not visible.' }
$crashes = & $adb -s $serial logcat -d -b crash
if (($crashes | Out-String) -match 'dev\.aic\.generated\.hello|FATAL EXCEPTION') { throw "Crash output detected: $crashes" }
Set-Content -LiteralPath $evidence -Value @("serial=$serial","model=$model","abi=$abi","android=$release","fingerprint=$fingerprint","visibleText=Hello from AndroidIntelligentCompiler","result=PASS")
Write-Output $evidence
