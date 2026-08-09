[CmdletBinding()]
param(
    [string[]]$TestPath = @("test"),
    [switch]$NoPub
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$toolLock = Enter-AmeRepositoryToolLock

Push-Location $repositoryRoot
try {
    $candidateTestFiles = @(
        foreach ($path in $TestPath) {
            if (Test-Path -LiteralPath $path -PathType Container) {
                Get-ChildItem -LiteralPath $path -Recurse -Filter "*_test.dart" -File |
                    Select-Object -ExpandProperty FullName
            } elseif (Test-Path -LiteralPath $path -PathType Leaf) {
                (Get-Item -LiteralPath $path).FullName
            } else {
                throw "Flutter test path does not exist: $path"
            }
        }
    )
    $testFiles = @($candidateTestFiles | Sort-Object -Unique)

    if ($testFiles.Count -eq 0) {
        throw "No Flutter test files were found"
    }

    $isFirstTest = $true
    foreach ($testFile in $testFiles) {
        $resolvedTestFile = [System.IO.Path]::GetFullPath($testFile)
        $repositoryPrefix = "$repositoryRoot$([System.IO.Path]::DirectorySeparatorChar)"
        if (-not $resolvedTestFile.StartsWith(
            $repositoryPrefix,
            [System.StringComparison]::OrdinalIgnoreCase
        )) {
            throw "Flutter test files must remain inside the repository: $testFile"
        }
        $relativePath = $resolvedTestFile.Substring($repositoryPrefix.Length)
        Write-Host "Flutter test: $relativePath"
        $arguments = @(
            "test",
            "--concurrency=1",
            "--reporter=expanded"
        )
        if ($NoPub -or -not $isFirstTest) {
            $arguments += "--no-pub"
        }
        $arguments += $resolvedTestFile
        Invoke-AmeChecked $toolchain.Flutter $arguments
        $isFirstTest = $false
    }
} finally {
    Pop-Location
    Exit-AmeRepositoryToolLock $toolLock
}
