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

    $relativePaths = [System.Collections.Generic.List[string]]::new()
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
        $relativePaths.Add($relativePath)
    }

    # Reuse Flutter's compiler between files; its runner still isolates each suite.
    # Bounded batches stay below cmd.exe's command-line limit on Windows.
    $isFirstBatch = $true
    $offset = 0
    while ($offset -lt $relativePaths.Count) {
        $batch = [System.Collections.Generic.List[string]]::new()
        $characters = 0
        while ($offset -lt $relativePaths.Count) {
            $path = $relativePaths[$offset]
            if ($path.Length + 3 -gt 6000) { throw "Flutter test path is too long: $path" }
            if ($characters + $path.Length + 3 -gt 6000) { break }
            $batch.Add($path)
            $characters += $path.Length + 3
            $offset++
        }
        Write-Host "Flutter tests: $($batch.Count) files"
        $arguments = @(
            "test",
            "--concurrency=1",
            "--reporter=expanded"
        )
        if ($NoPub -or -not $isFirstBatch) {
            $arguments += "--no-pub"
        }
        $arguments += $batch.ToArray()
        Invoke-AmeChecked $toolchain.Flutter $arguments
        $isFirstBatch = $false
    }
} finally {
    Pop-Location
    Exit-AmeRepositoryToolLock $toolLock
}
