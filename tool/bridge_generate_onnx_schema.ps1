[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$PythonPath)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
$lock = Enter-AmeRepositoryToolLock
Push-Location (Get-AmeRepositoryRoot)
try {
    & $PythonPath tool/bridge_onnx_schema/export_descriptor.py
    if ($LASTEXITCODE -ne 0) { throw "ONNX descriptor export failed" }
    & cargo run --locked --manifest-path tool/bridge_onnx_schema/Cargo.toml --target-dir build/onnx-schema/target
    if ($LASTEXITCODE -ne 0) { throw "ONNX Rust schema generation failed" }
    Copy-Item -LiteralPath build/onnx-schema/generated/onnx.rs -Destination rust/src/adapters/classification_runtime/precision/onnx.rs
    & cargo fmt --manifest-path rust/Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw "ONNX schema formatting failed" }
} finally {
    Pop-Location
    Exit-AmeRepositoryToolLock $lock
}
