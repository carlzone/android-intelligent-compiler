param(
    [string]$SdkRoot = $env:ANDROID_SDK_ROOT,
    [string]$Keystore,
    [switch]$SkipBuild
)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$adb = Join-Path $SdkRoot 'platform-tools/adb.exe'
$fixtures = @(
    @{ Name = 'compute'; Package = 'dev.aic.generated.compute'; Expected = 'Result: 36' },
    @{ Name = 'dynamic-string'; Package = 'dev.aic.generated.dynamic'; Expected = 'Value: 7' },
    @{ Name = 'multi-function'; Package = 'dev.aic.generated.multifunction'; Expected = 'Answer: 22' },
    @{ Name = 'string-equality'; Package = 'dev.aic.generated.strings'; Expected = 'Equal: true, different: true' },
    @{ Name = 'short-circuit'; Package = 'dev.aic.generated.shortcircuit'; Expected = 'Short circuit: false,true' }
)
$serials = @(& $adb devices | Select-Object -Skip 1 | ForEach-Object {
    if ($_ -match '^([^\s]+)\s+device$') { $Matches[1] }
})
$arm64 = @($serials | Where-Object {
    (& $adb -s $_ shell getprop ro.product.cpu.abi).Trim() -match '^arm64'
})
if ($arm64.Count -ne 1) {
    throw "Expected exactly one authorized ARM64 device; found $($arm64.Count)."
}
$serial = $arm64[0]

foreach ($fixture in $fixtures) {
    foreach ($level in @(0, 1)) {
        $output = Join-Path $workspace "testdata/generated/m2/$($fixture.Name)-o$level"
        if (-not $SkipBuild) {
            & (Join-Path $PSScriptRoot 'build-m1.ps1') `
                -SdkRoot $SdkRoot `
                -Keystore $Keystore `
                -OutputDir $output `
                -InputFile "testdata/$($fixture.Name).aic" `
                -OptLevel $level
            if ($LASTEXITCODE -ne 0) { throw "$($fixture.Name) O$level build failed." }
        }
        $apk = Join-Path $output 'hello-signed.apk'
        & $adb -s $serial install -r $apk
        if ($LASTEXITCODE -ne 0) { throw "$($fixture.Name) O$level installation failed." }
        & $adb -s $serial logcat -c
        & $adb -s $serial shell am force-stop $fixture.Package
        & $adb -s $serial shell am start -W -n "$($fixture.Package)/.MainActivity"
        if ($LASTEXITCODE -ne 0) { throw "$($fixture.Name) O$level launch failed." }

        $visible = $false
        foreach ($attempt in 1..3) {
            Start-Sleep -Seconds 2
            & $adb -s $serial shell uiautomator dump /sdcard/aic-m2-acceptance.xml | Out-Null
            if ($LASTEXITCODE -eq 0) {
                $ui = & $adb -s $serial shell cat /sdcard/aic-m2-acceptance.xml
                if ($ui -match [regex]::Escape($fixture.Expected)) {
                    $visible = $true
                    break
                }
            }
        }
        if (-not $visible) {
            throw "$($fixture.Name) O$level expected text was not visible: $($fixture.Expected)"
        }
        $crashes = & $adb -s $serial logcat -d -b crash
        if (($crashes | Out-String) -match 'FATAL EXCEPTION') {
            throw "$($fixture.Name) O$level crash output detected: $crashes"
        }
        Set-Content -LiteralPath (Join-Path $output 'device-evidence.txt') -Value @(
            "serial=$serial",
            "fixture=$($fixture.Name)",
            "optLevel=$level",
            "visibleText=$($fixture.Expected)",
            'crashBuffer=clean',
            'result=PASS'
        )
        Write-Output "$($fixture.Name) O$level PASS: $($fixture.Expected)"
    }
}
