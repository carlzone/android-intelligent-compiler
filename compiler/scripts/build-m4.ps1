param([string]$SdkRoot = $env:ANDROID_SDK_ROOT, [string]$Keystore)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$buildTools = Join-Path $SdkRoot 'build-tools/35.0.0'
$dexdump = Join-Path $buildTools 'dexdump.exe'
$apksigner = Join-Path $buildTools 'apksigner.bat'
foreach ($level in @(0, 1)) {
    $output = Join-Path $workspace "testdata/generated/m4/notes-o$level"
    & (Join-Path $PSScriptRoot 'build-m1.ps1') -SdkRoot $SdkRoot -Keystore $Keystore -OutputDir $output -InputFile 'testdata/notes.aic' -OptLevel $level
    if ($LASTEXITCODE -ne 0) { throw "notes O$level build failed." }
    $manifest = Get-Content -Raw (Join-Path $output 'AndroidManifest.xml')
    if ($manifest -match '<uses-permission') { throw "notes O$level requested an unexpected permission." }
    & $dexdump -f (Join-Path $output 'classes.dex') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "notes O$level DEX inspection failed." }
    & $apksigner verify --verbose (Join-Path $output 'hello-signed.apk') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "notes O$level signature verification failed." }
}
