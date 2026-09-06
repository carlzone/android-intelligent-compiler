param(
    [string]$SdkRoot = $env:ANDROID_SDK_ROOT,
    [string]$Keystore,
    [string]$OutputDir,
    [string]$InputFile = 'testdata/hello.aic',
    [ValidateSet(0, 1)][int]$OptLevel = 1
)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($SdkRoot)) { throw 'Set ANDROID_SDK_ROOT or pass -SdkRoot.' }
if ([string]::IsNullOrWhiteSpace($OutputDir)) { $OutputDir = Join-Path $workspace 'testdata/generated/m1' }
if ([string]::IsNullOrWhiteSpace($Keystore)) { $Keystore = Join-Path $workspace '.aic/debug.keystore' }
$buildTools = Join-Path $SdkRoot 'build-tools/35.0.0'
$aapt2 = Join-Path $buildTools 'aapt2.exe'
$zipalign = Join-Path $buildTools 'zipalign.exe'
$apksigner = Join-Path $buildTools 'apksigner.bat'
$androidJar = Join-Path $SdkRoot 'platforms/android-35/android.jar'
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if ($null -eq $cargo) { $cargoPath = Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe' } else { $cargoPath = $cargo.Source }
foreach ($tool in @($aapt2, $zipalign, $apksigner, $androidJar, $cargoPath)) { if (-not (Test-Path -LiteralPath $tool)) { throw "Required tool not found: $tool" } }
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$log = Join-Path $OutputDir 'tool-invocations.log'
Set-Content -LiteralPath $log -Value "AIC M1 generated-app tool audit`n"
function Invoke-AicTool([string]$Display, [string]$File, [string[]]$Arguments) {
    Add-Content -LiteralPath $log -Value $Display
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$File exited with code $LASTEXITCODE." }
}
Push-Location $workspace
try {
    Invoke-AicTool "cargo run -p aic-cli -- compile --input $InputFile --output-dir [output] --profile android-35 --opt-level $OptLevel" $cargoPath @('run','-p','aic-cli','--','compile','--input',$InputFile,'--output-dir',$OutputDir,'--profile','android-35','--opt-level',"$OptLevel")
    $manifest = Join-Path $OutputDir 'AndroidManifest.xml'; $dex = Join-Path $OutputDir 'classes.dex'; $base = Join-Path $OutputDir 'base.apk'; $unaligned = Join-Path $OutputDir 'hello-unaligned.apk'; $aligned = Join-Path $OutputDir 'hello-aligned.apk'; $signed = Join-Path $OutputDir 'hello-signed.apk'
    Invoke-AicTool 'aapt2 link --manifest AndroidManifest.xml -I android-35/android.jar -o base.apk' $aapt2 @('link','--manifest',$manifest,'-I',$androidJar,'-o',$base)
    Invoke-AicTool 'cargo run -p aic-cli -- assemble-apk --base base.apk --dex classes.dex --output hello-unaligned.apk' $cargoPath @('run','-p','aic-cli','--','assemble-apk','--base',$base,'--dex',$dex,'--output',$unaligned)
    Invoke-AicTool 'zipalign -f 4 hello-unaligned.apk hello-aligned.apk' $zipalign @('-f','4',$unaligned,$aligned)
    if (-not (Test-Path -LiteralPath $Keystore)) {
        $keytool = (Get-Command keytool -ErrorAction Stop).Source
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Keystore) | Out-Null
        Invoke-AicTool 'keytool -genkeypair [debug keystore; credentials redacted]' $keytool @('-genkeypair','-keystore',$Keystore,'-storepass','android','-alias','androiddebugkey','-keypass','android','-dname','CN=Android Debug,O=Android,C=US','-keyalg','RSA','-keysize','2048','-validity','10000','-noprompt')
    }
    Invoke-AicTool 'apksigner sign --ks [local debug keystore] --out hello-signed.apk hello-aligned.apk' $apksigner @('sign','--ks',$Keystore,'--ks-key-alias','androiddebugkey','--ks-pass','pass:android','--key-pass','pass:android','--out',$signed,$aligned)
    if (Select-String -LiteralPath $log -Pattern '(^|[\\/ ])(javac|kotlinc|gradle|gradlew)(\.exe|\.bat|\.cmd)?([ .\\/]|$)' -CaseSensitive:$false) { throw 'Forbidden generated-app compiler appeared in the tool audit.' }
    Write-Output $signed
} finally { Pop-Location }
