param(
    [ValidateSet('ollama','llamacpp')][string]$Provider = 'llamacpp',
    [string]$LlamaRoot = 'F:\AI',
    [string]$Model,
    [switch]$AdbReverse
)
$ErrorActionPreference = 'Stop'
if ($Provider -eq 'ollama') {
    if (-not (Get-Process ollama -ErrorAction SilentlyContinue)) { Start-Process -FilePath 'ollama' -ArgumentList @('serve') -WindowStyle Hidden }
    if ($AdbReverse) { & adb reverse tcp:11434 tcp:11434 }
    Write-Output 'Ollama URL: http://127.0.0.1:11434; model: llama3.2:latest'
    exit
}
$server = Join-Path $LlamaRoot 'llama-server.exe'
if (-not $Model) { $Model = Get-ChildItem -LiteralPath $LlamaRoot -Recurse -Filter '*.gguf' | Where-Object Name -eq 'Qwen3-8B-Q4_K_M.gguf' | Select-Object -First 1 -ExpandProperty FullName }
if (-not (Test-Path -LiteralPath $server)) { throw "Missing llama-server: $server" }
if (-not $Model -or -not (Test-Path -LiteralPath $Model)) { throw 'Specify an installed GGUF model with -Model.' }
if (-not (Get-Process llama-server -ErrorAction SilentlyContinue)) {
    Start-Process -FilePath $server -ArgumentList @('-m',$Model,'--alias','qwen3-8b','--host','127.0.0.1','--port','8080','--jinja') -WindowStyle Hidden
}
if ($AdbReverse) { & adb reverse tcp:8080 tcp:8080 }
Write-Output 'llama.cpp URL: http://127.0.0.1:8080; model: qwen3-8b'
