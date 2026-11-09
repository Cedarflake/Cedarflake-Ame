$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_ci_policy.ps1")

foreach ($paths in @(
    @("lib/features/library/presentation/widgets/example.dart"),
    @("lib/app/presentation/example.dart", "test/example_test.dart", "docs/product/gallery.md")
)) {
    $policy = Get-AmeCiPolicy -ChangedPaths $paths
    if ($policy.Mode -ne "quick" -or $policy.Extended) {
        throw "Presentation changes must use quick checks"
    }
}
if ((Get-AmeCiPolicy -ChangedPaths @('README.md', 'docs/product/gallery.md')).Mode -ne 'documentation') {
    throw "Documentation-only edits must not start product builds or tests"
}
foreach ($paths in @(
    @('tool/release_reuse_policy.ps1', 'docs/releases/v1.0.0.md'),
    @('.github/workflows/release_publish_windows.yml'),
    @('tool/release_package_installer_windows.ps1', '.github/workflows/quality_ci.yml')
)) {
    if ((Get-AmeCiPolicy -ChangedPaths $paths).Mode -cne 'release-tooling') {
        throw 'Release-tooling-only changes must use script verification without application builds'
    }
}
if ((Get-AmeCiPolicy -ChangedPaths @('tool/release_publish.ps1', 'rust/src/main.rs')).Mode -cne 'daily') {
    throw 'Release tooling cannot hide changed application source'
}
foreach ($path in @(
    "rust/src/adapters/local_files.rs", "rust/src/domain/mod.rs",
    "lib/features/library/application/query.dart", "lib/app/bootstrap/library.dart",
    "lib/features/library/adapters/rust/query.dart", "lib/src/rust/api/query.dart",
    "windows/runner/main.cpp", "pubspec.lock", "tool/quality_ci_policy.ps1",
    ".github/workflows/quality_ci.yml", "unknown/new_owner.txt"
)) {
    $policy = Get-AmeCiPolicy -ChangedPaths @("test/example_test.dart", $path)
    if ($policy.Mode -ne "daily" -or $policy.Extended) {
        throw "Critical and unknown changes must retain full checks: $path"
    }
}
if ((Get-AmeCiPolicy -ChangedPaths @()).Mode -ne "daily") {
    throw "An unavailable change range cannot select quick checks"
}
foreach ($request in @("full", "extended")) {
    $policy = Get-AmeCiPolicy -ChangedPaths @("README.md") -Requested $request
    if ($policy.Mode -ne "daily" -or $policy.Extended -ne ($request -eq "extended")) {
        throw "An explicit verification request must retain its workload"
    }
}
Write-Output "CI selection preserves critical, unknown and explicitly requested workloads"
