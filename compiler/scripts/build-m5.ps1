param([string]$SdkRoot = $env:ANDROID_SDK_ROOT, [string]$Keystore)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$buildTools = Join-Path $SdkRoot 'build-tools/35.0.0'
$dexdump = Join-Path $buildTools 'dexdump.exe'
$apksigner = Join-Path $buildTools 'apksigner.bat'
$rows = @()
foreach ($level in @(0, 1)) {
    $output = Join-Path $workspace "testdata/generated/m5/optimizer-o$level"
    & (Join-Path $PSScriptRoot 'build-m1.ps1') -SdkRoot $SdkRoot -Keystore $Keystore -OutputDir $output -InputFile 'testdata/m5-optimizer.aic' -OptLevel $level
    if ($LASTEXITCODE -ne 0) { throw "M5 O$level build failed." }
    & $dexdump -f (Join-Path $output 'classes.dex') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "M5 O$level DEX inspection failed." }
    & $apksigner verify --verbose (Join-Path $output 'hello-signed.apk') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "M5 O$level signature verification failed." }
    $rows += [pscustomobject]@{
        optimizationLevel = $level
        dexBytes = (Get-Item -LiteralPath (Join-Path $output 'classes.dex')).Length
        unsignedApkBytes = (Get-Item -LiteralPath (Join-Path $output 'hello-aligned.apk')).Length
        signedApkBytes = (Get-Item -LiteralPath (Join-Path $output 'hello-signed.apk')).Length
    }
}
$evidence = Join-Path $workspace 'testdata/generated/m5'
$rows | Export-Csv -NoTypeInformation -LiteralPath (Join-Path $evidence 'size-results.csv')
$rows | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $evidence 'size-results.json')
if ($rows[1].dexBytes -ge $rows[0].dexBytes) { throw 'M5 requires O1 classes.dex to be strictly smaller than O0.' }
Write-Output "M5 size gate PASS: $($rows[0].dexBytes) -> $($rows[1].dexBytes) bytes"
