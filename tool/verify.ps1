$ErrorActionPreference = "Stop"

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Command,
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command failed with exit code $LASTEXITCODE"
    }
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repositoryRoot

try {
    Invoke-Checked "cargo" @("fmt", "--manifest-path", "rust\Cargo.toml", "--", "--check")
    Invoke-Checked "cargo" @(
        "clippy",
        "--manifest-path",
        "rust\Cargo.toml",
        "--all-targets",
        "--",
        "-D",
        "warnings"
    )
    Invoke-Checked "cargo" @("test", "--manifest-path", "rust\Cargo.toml", "--all-targets")
    Invoke-Checked "dart" @(
        "format",
        "--output=none",
        "--set-exit-if-changed",
        "lib",
        "test",
        "integration_test"
    )
    Invoke-Checked "flutter" @("analyze")
    Invoke-Checked "flutter" @("test")

    $rustHashLine = Select-String -LiteralPath "rust\src\frb_generated.rs" -Pattern (
        "FLUTTER_RUST_BRIDGE_CODEGEN_CONTENT_HASH"
    )
    $dartHashLine = Select-String -LiteralPath "lib\src\rust\frb_generated.dart" -Pattern (
        "rustContentHash =>"
    )
    $rustHash = [regex]::Match($rustHashLine.Line, "=\s*(-?\d+)").Groups[1].Value
    $dartHash = [regex]::Match($dartHashLine.Line, "=>\s*(-?\d+)").Groups[1].Value
    if (-not $rustHash -or $rustHash -ne $dartHash) {
        throw "Generated Rust and Dart bridge hashes do not match"
    }

    Invoke-Checked "git" @("diff", "--check")
} finally {
    Pop-Location
}
