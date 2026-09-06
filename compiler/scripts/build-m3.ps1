param([string]$SdkRoot = $env:ANDROID_SDK_ROOT, [string]$Keystore)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
foreach ($fixture in @('counter', 'calculator')) {
    foreach ($level in @(0, 1)) {
        $output = Join-Path $workspace "testdata/generated/m3/$fixture-o$level"
        & (Join-Path $PSScriptRoot 'build-m1.ps1') -SdkRoot $SdkRoot -Keystore $Keystore -OutputDir $output -InputFile "testdata/$fixture.aic" -OptLevel $level
        if ($LASTEXITCODE -ne 0) { throw "$fixture O$level build failed." }
    }
}
