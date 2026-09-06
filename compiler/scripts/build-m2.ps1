param([string]$SdkRoot = $env:ANDROID_SDK_ROOT, [string]$Keystore)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
foreach ($level in @(0, 1)) {
    $output = Join-Path $workspace "testdata/generated/m2/o$level"
    & (Join-Path $PSScriptRoot 'build-m1.ps1') -SdkRoot $SdkRoot -Keystore $Keystore -OutputDir $output -InputFile 'testdata/compute.aic' -OptLevel $level
}
