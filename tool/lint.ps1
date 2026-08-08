$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain

Push-Location $repositoryRoot
try {
    $powerShellPaths = Get-ChildItem -LiteralPath "tool" -Filter "*.ps1" -File |
        Select-Object -ExpandProperty FullName
    Invoke-AmePowerShellSyntaxCheck $powerShellPaths
    Invoke-AmeJsonSyntaxCheck @(
        (Join-Path $repositoryRoot ".vscode\extensions.json"),
        (Join-Path $repositoryRoot ".vscode\settings.json")
    )
    & (Join-Path $PSScriptRoot "format.ps1") -Check
    Invoke-AmeChecked $toolchain.Cargo @(
        "clippy",
        "--locked",
        "--manifest-path",
        "rust\Cargo.toml",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings"
    )
    Invoke-AmeChecked $toolchain.Dart @(
        "analyze",
        "--fatal-infos",
        "--fatal-warnings"
    )
} finally {
    Pop-Location
}
