$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolLock = Enter-AmeRepositoryToolLock
Push-Location $repositoryRoot

try {
    & flutter_rust_bridge_codegen generate
    if ($LASTEXITCODE -ne 0) {
        throw "flutter_rust_bridge_codegen failed with exit code $LASTEXITCODE"
    }

    $generatedRoot = Join-Path $repositoryRoot "lib\src\rust"
    $freezedFiles = Get-ChildItem -LiteralPath $generatedRoot -Recurse -File -Filter "*.freezed.dart"
    foreach ($freezedFile in $freezedFiles) {
        $content = [System.IO.File]::ReadAllText($freezedFile.FullName, [System.Text.Encoding]::UTF8)
        $content = [System.Text.RegularExpressions.Regex]::Replace(
            $content,
            "[ \t]+(?=\r?$)",
            "",
            [System.Text.RegularExpressions.RegexOptions]::Multiline
        )
        [System.IO.File]::WriteAllText(
            $freezedFile.FullName,
            $content,
            [System.Text.UTF8Encoding]::new($false)
        )
    }

    & cargo fmt --manifest-path rust\Cargo.toml
    if ($LASTEXITCODE -ne 0) {
        throw "cargo fmt failed with exit code $LASTEXITCODE"
    }

    & cargo build --manifest-path rust\Cargo.toml --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo release build failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
    Exit-AmeRepositoryToolLock $toolLock
}
