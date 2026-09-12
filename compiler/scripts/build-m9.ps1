param([string]$SdkRoot = $env:ANDROID_SDK_ROOT, [switch]$Offline)
$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot 'build-m8.ps1') -SdkRoot $SdkRoot -Offline:$Offline
if ($LASTEXITCODE -ne 0) { throw 'M8 prerequisite build failed.' }
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
& cargo test --manifest-path (Join-Path $root 'compiler/Cargo.toml') --workspace --locked
if ($LASTEXITCODE -ne 0) { throw 'M9 workspace tests failed.' }
& cargo clippy --manifest-path (Join-Path $root 'compiler/Cargo.toml') --workspace --all-targets --locked -- -D warnings
if ($LASTEXITCODE -ne 0) { throw 'M9 clippy failed.' }
