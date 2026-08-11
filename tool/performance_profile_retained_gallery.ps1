[CmdletBinding()]
param(
    [ValidateRange(20, 1000)]
    [int]$Iterations = 80,
    [string]$BaselineRevision = "6d3f0686a91b85402251fe07fcc1690f268effd5"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$buildRoot = Join-Path $repositoryRoot "build"
$profileRoot = Join-Path $buildRoot "performance"
$libraryPath = Join-Path (
    $buildRoot
) "windows\x64\runner\Profile\rust_lib_cedarflake_ame.dll"
$evidencePath = Join-Path $profileRoot "r2b-retained-gallery-profile-$Iterations.json"
$resolvedBuildRoot = [System.IO.Path]::GetFullPath($buildRoot)
$resolvedEvidencePath = [System.IO.Path]::GetFullPath($evidencePath)

if (-not $resolvedEvidencePath.StartsWith(
    "$resolvedBuildRoot$([System.IO.Path]::DirectorySeparatorChar)",
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Profile evidence must remain inside the repository build directory"
}

$frozenInteractionFiles = @(
    "lib/features/library/presentation/unified_library_screen.dart",
    "lib/features/library/presentation/widgets/library_gallery_wall.dart",
    "lib/features/library/presentation/widgets/library_time_navigation.dart",
    "lib/features/library/presentation/widgets/library_virtual_gallery_geometry.dart"
)
& git -C $repositoryRoot diff --quiet $BaselineRevision -- $frozenInteractionFiles
if ($LASTEXITCODE -eq 1) {
    throw "Frozen gallery interaction files differ from baseline $BaselineRevision"
}
if ($LASTEXITCODE -ne 0) {
    throw "Could not verify the frozen gallery interaction baseline"
}

$toolLock = Enter-AmeRepositoryToolLock
$previousLibraryPath = $env:CEDARFLAKE_AME_PROFILE_LIBRARY_PATH
$previousEvidencePath = $env:CEDARFLAKE_AME_PROFILE_EVIDENCE_PATH
$previousIterations = $env:CEDARFLAKE_AME_PROFILE_ITERATIONS
try {
    New-Item -ItemType Directory -Path $profileRoot -Force | Out-Null
    $env:CEDARFLAKE_AME_PROFILE_LIBRARY_PATH = $libraryPath
    $env:CEDARFLAKE_AME_PROFILE_EVIDENCE_PATH = $resolvedEvidencePath
    $env:CEDARFLAKE_AME_PROFILE_ITERATIONS = $Iterations.ToString(
        [System.Globalization.CultureInfo]::InvariantCulture
    )
    Push-Location $repositoryRoot
    try {
        Invoke-AmeChecked $toolchain.Flutter @(
            "drive",
            "--driver",
            "test_driver\integration_test.dart",
            "--target",
            "integration_test\profile_retained_gallery_test.dart",
            "-d",
            "windows",
            "--profile",
            "--no-pub",
            "--timeout",
            "300"
        )
    } finally {
        Pop-Location
    }
} finally {
    $env:CEDARFLAKE_AME_PROFILE_LIBRARY_PATH = $previousLibraryPath
    $env:CEDARFLAKE_AME_PROFILE_EVIDENCE_PATH = $previousEvidencePath
    $env:CEDARFLAKE_AME_PROFILE_ITERATIONS = $previousIterations
    Exit-AmeRepositoryToolLock $toolLock
}

if (-not (Test-Path -LiteralPath $resolvedEvidencePath -PathType Leaf)) {
    throw "Retained-gallery Profile evidence was not written"
}
Write-Output "Retained-gallery Profile evidence: $resolvedEvidencePath"
