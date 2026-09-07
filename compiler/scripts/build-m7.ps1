param([string]$SdkRoot = $env:ANDROID_SDK_ROOT, [switch]$PrepareBootstrap, [switch]$Offline)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
& (Join-Path $PSScriptRoot 'build-m6.ps1') -SdkRoot $SdkRoot -PrepareBootstrap:$PrepareBootstrap -Offline:$Offline
if ($LASTEXITCODE -ne 0) { throw "M6 host build failed with $LASTEXITCODE" }
foreach ($required in @('compiler/schema/aic-ir-0.1.ebnf','compiler/schema/model-proposal-1.json','host/app/src/main/assets/ai/proposal-schema.json')) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $required))) { throw "Missing M7 contract: $required" }
}
Write-Output 'M7 host and schema checks passed. Live-provider/device acceptance requires an API key configured in the host.'
