param([string]$SdkRoot = $env:ANDROID_SDK_ROOT, [switch]$Offline)
$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot 'build-m7.ps1') -SdkRoot $SdkRoot -Offline:$Offline
if ($LASTEXITCODE -ne 0) { throw 'M8 host build failed.' }
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
& python (Join-Path $PSScriptRoot 'verify-m8.py') --host-apk (Join-Path $root 'host/app/build/outputs/apk/debug/app-debug.apk')
if ($LASTEXITCODE -ne 0) { throw 'M8 independent verification failed.' }
