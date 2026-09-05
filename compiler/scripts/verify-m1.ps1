param([string]$SdkRoot = $env:ANDROID_SDK_ROOT, [string]$OutputDir)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($OutputDir)) { $OutputDir = Join-Path $workspace 'testdata/generated/m1' }
$tools = Join-Path $SdkRoot 'build-tools/35.0.0'
$dexdump = Join-Path $tools 'dexdump.exe'; $aapt2 = Join-Path $tools 'aapt2.exe'; $zipalign = Join-Path $tools 'zipalign.exe'; $apksigner = Join-Path $tools 'apksigner.bat'
$dex = Join-Path $OutputDir 'classes.dex'; $apk = Join-Path $OutputDir 'hello-signed.apk'; $report = Join-Path $OutputDir 'verification.txt'
if (-not (Test-Path -LiteralPath $apk)) { throw 'Run scripts/build-m1.ps1 first.' }
$lines = [System.Collections.Generic.List[string]]::new()
function Check([string]$Name, [scriptblock]$Action) { $text = & $Action 2>&1 | Out-String; if ($LASTEXITCODE -ne 0) { throw "$Name failed: $text" }; $lines.Add("=== $Name ===`n$text") }
Check 'dexdump' { & $dexdump -c $dex }
Check 'dexdump structure' { & $dexdump -d $dex }
Check 'manifest' { & $aapt2 dump xmltree $apk --file AndroidManifest.xml }
Check 'badging' { & $aapt2 dump badging $apk }
Check 'zipalign' { & $zipalign -c -v 4 $apk }
Check 'signature' { & $apksigner verify --verbose --print-certs $apk }
$joined = $lines -join "`n"
foreach ($required in @('dev.aic.generated.hello','MainActivity','android.intent.action.MAIN','android.intent.category.LAUNCHER','minSdkVersion','targetSdkVersion')) { if ($joined -notmatch [regex]::Escape($required)) { throw "Verification output is missing $required" } }
Set-Content -LiteralPath $report -Value $joined
Write-Output $report
