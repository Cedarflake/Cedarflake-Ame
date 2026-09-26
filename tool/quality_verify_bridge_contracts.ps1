[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_bridge_contracts.ps1")
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$utf8 = [System.Text.Encoding]::UTF8
$rust = [IO.File]::ReadAllText((Join-Path $repositoryRoot "rust/src/frb_generated.rs"), $utf8)
$dart = [IO.File]::ReadAllText((Join-Path $repositoryRoot "lib/src/rust/frb_generated.dart"), $utf8)
$rustCode = ConvertTo-AmeBridgeCode $rust Rust
$dartCode = ConvertTo-AmeBridgeCode $dart Dart
$rustHash = [regex]::Matches($rustCode, '\bFLUTTER_RUST_BRIDGE_CODEGEN_CONTENT_HASH\s*:\s*i32\s*=\s*(-?\d+)')
$dartHash = [regex]::Matches($dartCode, '\brustContentHash\s*=>\s*(-?\d+)')
if ($rustHash.Count -ne 1 -or $dartHash.Count -ne 1 -or
    $rustHash[0].Groups[1].Value -ne $dartHash[0].Groups[1].Value) {
    throw "Generated Rust and Dart bridge hashes are missing, duplicated, or inconsistent"
}
$rustApis = @{}
$dartApis = @{}
$contracts = @(Get-AmeAsyncBridgeContracts)
foreach ($contract in $contracts) {
    if (-not $rustApis.ContainsKey($contract.Module)) {
        $rustPath = Join-Path $repositoryRoot "rust/src/api/$($contract.Module).rs"
        $dartPath = Join-Path $repositoryRoot "lib/src/rust/api/$($contract.Module).dart"
        $rustApis[$contract.Module] = ConvertTo-AmeBridgeCode ([IO.File]::ReadAllText($rustPath, $utf8)) Rust
        $dartApis[$contract.Module] = ConvertTo-AmeBridgeCode ([IO.File]::ReadAllText($dartPath, $utf8)) Dart
    }
    Assert-AmeAsyncBridgeContract $contract $rustApis[$contract.Module] $dartApis[$contract.Module] $dartCode $rustCode
}
Write-Output "Verified $($contracts.Count) asynchronous bridge contracts and matching content hashes."
