$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot

Push-Location $workspace
try {
    cargo fmt --all --check
    if ($LASTEXITCODE -ne 0) { throw "cargo fmt exited with code $LASTEXITCODE." }
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) { throw "cargo test exited with code $LASTEXITCODE." }
    cargo clippy --workspace --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "cargo clippy exited with code $LASTEXITCODE." }
    cargo run -p aic-cli -- emit-minimal
    if ($LASTEXITCODE -ne 0) { throw "cargo run exited with code $LASTEXITCODE." }

    $fixture = Join-Path $workspace 'testdata/generated/minimal.dex'
    $firstHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $fixture).Hash
    cargo run -p aic-cli -- emit-minimal
    if ($LASTEXITCODE -ne 0) { throw "cargo run exited with code $LASTEXITCODE." }
    $secondHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $fixture).Hash
    if ($firstHash -ne $secondHash) {
        throw 'Repeated generation was not byte-for-byte reproducible.'
    }

    $dexdump = Get-Command dexdump -ErrorAction SilentlyContinue
    if ($null -ne $dexdump) {
        & $dexdump.Source -c $fixture
        if ($LASTEXITCODE -ne 0) { throw "dexdump checksum verification exited with code $LASTEXITCODE." }
        & $dexdump.Source $fixture
        if ($LASTEXITCODE -ne 0) { throw "dexdump structural verification exited with code $LASTEXITCODE." }
    } else {
        Write-Warning 'dexdump is unavailable; built-in independent structural verification ran under cargo test.'
    }
} finally {
    Pop-Location
}
