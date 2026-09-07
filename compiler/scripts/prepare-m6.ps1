param([string]$SdkRoot = $env:ANDROID_SDK_ROOT)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$toolsRoot = Join-Path $root '.aic-tools'
New-Item -ItemType Directory -Force $toolsRoot | Out-Null
function Run([string]$File, [string[]]$Arguments) {
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$File exited with $LASTEXITCODE" }
}
function Fetch([string]$Url, [string]$Path, [string]$Hash) {
    if (-not (Test-Path -LiteralPath $Path)) { Invoke-WebRequest $Url -OutFile $Path }
    if ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -ne $Hash) { throw "Checksum mismatch: $Path" }
}
$source = Join-Path $toolsRoot 'aapt2-src'
if (-not (Test-Path -LiteralPath $source)) {
    Run 'git' @('clone','--depth','1','--branch','v1.1.0','https://github.com/ReVanced/aapt2.git',$source)
}
if ((& git -C $source rev-parse HEAD).Trim() -ne 'adb1d7acef11a67849b8a6204a824e271c4c1952') { throw 'Unexpected AAPT2 revision' }
Run 'git' @('-C',$source,'submodule','update','--init','--depth','1','--jobs','6')
$protocZip = Join-Path $toolsRoot 'protoc.zip'
Fetch 'https://github.com/protocolbuffers/protobuf/releases/download/v21.12/protoc-21.12-win64.zip' $protocZip '71852A30CF62975358EDFCBBFF93086E8857A079C8E4D6904881AA968D65C7F9'
Expand-Archive -LiteralPath $protocZip -DestinationPath (Join-Path $toolsRoot 'protoc') -Force
Run 'python' @((Join-Path $root 'host/scripts/prepare-aapt2-source.py'),$source)
$ndk = Join-Path $SdkRoot 'ndk/27.1.12297006'
$cmake = Join-Path $SdkRoot 'cmake/3.22.1/bin/cmake.exe'
$build = Join-Path $toolsRoot 'aapt2-build'
Run $cmake @('-S',$source,'-B',$build,'-G','Ninja',"-DCMAKE_MAKE_PROGRAM=$SdkRoot/cmake/3.22.1/bin/ninja.exe", "-DCMAKE_TOOLCHAIN_FILE=$ndk/build/cmake/android.toolchain.cmake",'-DANDROID_ABI=arm64-v8a','-DANDROID_PLATFORM=android-30','-DCMAKE_BUILD_TYPE=Release','-DPNG_SHARED=OFF',"-DZLIB_LIBRARY_RELEASE=$ndk/toolchains/llvm/prebuilt/windows-x86_64/sysroot/usr/lib/aarch64-linux-android/libz.a","-DProtobuf_PROTOC_EXECUTABLE=$toolsRoot/protoc/bin/protoc.exe")
Run $cmake @('--build',$build,'--target','aapt2','-j','6')
$binary = Join-Path $build 'bin/aapt2-arm64-v8a'
Run (Join-Path $ndk 'toolchains/llvm/prebuilt/windows-x86_64/bin/llvm-strip.exe') @('--strip-unneeded',$binary)
$native = Join-Path $root 'host/app/src/main/jniLibs/arm64-v8a'
$assets = Join-Path $root 'host/app/src/main/assets/bootstrap'
New-Item -ItemType Directory -Force $native,$assets | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $native 'libaapt2.so')
$platform = Join-Path $SdkRoot 'platforms/android-35/android.jar'
if ((Get-FileHash $platform).Hash -ne '4566663C3876E022B4FA4CED8C8697C4AB1688267F090114FD92D027B32E619B') { throw 'Android platform 35 revision differs from the M6 pinned oracle; review before updating the hash.' }
Copy-Item -LiteralPath $platform -Destination (Join-Path $assets 'android.jar')
$metadata = @('AAPT2 source: https://github.com/ReVanced/aapt2','revision=adb1d7acef11a67849b8a6204a824e271c4c1952','port=v1.1.0','NDK=27.1.12297006','protoc=21.12',"aapt2.sha256=$((Get-FileHash $binary).Hash)","platform.sha256=$((Get-FileHash $platform).Hash)")
Set-Content -LiteralPath (Join-Path $assets 'versions.txt') -Value $metadata -Encoding UTF8
Run 'python' @((Join-Path $root 'host/scripts/bootstrap-notices.py'),$source,$assets)
Write-Output 'M6 bootstrap assets prepared from pinned source.'
