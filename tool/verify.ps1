$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
Push-Location $repositoryRoot

try {
    & (Join-Path $PSScriptRoot "lint.ps1")
    Invoke-AmeChecked $toolchain.Cargo @(
        "test",
        "--locked",
        "--manifest-path",
        "rust\Cargo.toml",
        "--all-targets",
        "--all-features"
    )
    Invoke-AmeChecked $toolchain.Flutter @("test")
    & (Join-Path $PSScriptRoot "test_windows_integration.ps1")

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

    Invoke-AmeChecked $toolchain.Git @("diff", "HEAD", "--check", "--")
} finally {
    Pop-Location
}
