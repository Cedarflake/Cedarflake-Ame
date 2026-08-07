$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repositoryRoot

try {
    & flutter_rust_bridge_codegen generate
    if ($LASTEXITCODE -ne 0) {
        throw "flutter_rust_bridge_codegen failed with exit code $LASTEXITCODE"
    }

    $freezedPath = Join-Path $repositoryRoot "lib\src\rust\domain.freezed.dart"
    if (Test-Path -LiteralPath $freezedPath) {
        $content = [System.IO.File]::ReadAllText($freezedPath)
        $content = [System.Text.RegularExpressions.Regex]::Replace(
            $content,
            "[ \t]+(?=\r?$)",
            "",
            [System.Text.RegularExpressions.RegexOptions]::Multiline
        )
        [System.IO.File]::WriteAllText(
            $freezedPath,
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
}
