param([string]$SdkRoot = $env:ANDROID_SDK_ROOT, [switch]$PrepareBootstrap, [switch]$Offline)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
function Run([string]$File, [string[]]$Arguments) { & $File @Arguments; if ($LASTEXITCODE -ne 0) { throw "$File exited with $LASTEXITCODE" } }
if ($PrepareBootstrap) { Write-Warning 'Bootstrap preparation is retired in M8; the switch is retained for script compatibility.' }
$native = Join-Path $root 'host/app/src/main/jniLibs/arm64-v8a'
New-Item -ItemType Directory -Force $native | Out-Null
$env:ANDROID_HOME = $SdkRoot
$env:ANDROID_SDK_ROOT = $SdkRoot
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = Join-Path $SdkRoot 'ndk/27.1.12297006/toolchains/llvm/prebuilt/windows-x86_64/bin/aarch64-linux-android30-clang.cmd'
$cargoArguments = @('build','--manifest-path',(Join-Path $root 'compiler/Cargo.toml'),'-p','aic-jni','--target','aarch64-linux-android','--release','--locked')
if ($Offline) { $cargoArguments += '--offline' }
Run (Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe') $cargoArguments
Copy-Item -LiteralPath (Join-Path $root 'compiler/target/aarch64-linux-android/release/libaic_jni.so') -Destination (Join-Path $root 'host/app/src/main/jniLibs/arm64-v8a/libaic_jni.so')
$key = Join-Path $root 'compiler/.aic/host-debug.keystore'
if (-not (Test-Path -LiteralPath $key)) {
    New-Item -ItemType Directory -Force (Split-Path -Parent $key) | Out-Null
    Run (Join-Path $env:JAVA_HOME 'bin/keytool.exe') @('-genkeypair','-keystore',$key,'-storepass','android','-alias','androiddebugkey','-keypass','android','-dname','CN=AIC Host Debug','-keyalg','RSA','-keysize','2048','-validity','10000','-noprompt')
}
$gradleArguments = @('-p',(Join-Path $root 'host'),'assembleDebug','assembleDebugAndroidTest','testDebugUnitTest','lintDebug','--console=plain')
if ($Offline) { $gradleArguments += '--offline' }
Run (Join-Path $root 'host/gradlew.bat') $gradleArguments
Write-Output (Join-Path $root 'host/app/build/outputs/apk/debug/app-debug.apk')
