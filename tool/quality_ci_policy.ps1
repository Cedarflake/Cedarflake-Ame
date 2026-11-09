$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot 'release_reuse_policy.ps1')

function Get-AmeCiPolicy {
    param(
        [string[]]$ChangedPaths,
        [ValidateSet("auto", "full", "extended")][string]$Requested = "auto"
    )

    if ($Requested -ne "auto" -or $ChangedPaths.Count -eq 0) {
        return [pscustomobject]@{
            Mode = "daily"
            Extended = $Requested -eq "extended"
        }
    }

    # Known release tooling and presentation/documentation edits have scoped
    # checks. Unknown owners retain full regression, including deletions/renames.
    $documentationOnly = $true
    $releaseOnly = $true
    foreach ($path in $ChangedPaths) {
        $normalized = $path.Replace('\', '/')
        if ($normalized -cmatch '^(docs/[^\r\n]+\.md|[^/]+\.md)$') { continue }
        if (-not (Test-AmeReleaseToolingPath $normalized)) { $releaseOnly = $false; break }
        $documentationOnly = $false
    }
    if ($releaseOnly -and -not $documentationOnly) {
        return [pscustomobject]@{ Mode = 'release-tooling'; Extended = $false }
    }
    $documentationOnly = $true
    foreach ($path in $ChangedPaths) {
        $normalized = $path.Replace('\', '/')
        $presentation = $normalized -cmatch '^lib/(app/presentation|features/[^/]+/presentation)/[^\r\n]+\.dart$'
        $flutterTest = $normalized -cmatch '^test/[^\r\n]+\.dart$'
        $documentation = $normalized -cmatch '^(docs/[^\r\n]+\.md|[^/]+\.md)$'
        if (-not ($presentation -or $flutterTest -or $documentation)) {
            return [pscustomobject]@{ Mode = "daily"; Extended = $false }
        }
        if (-not $documentation) { $documentationOnly = $false }
    }
    if ($documentationOnly) {
        return [pscustomobject]@{ Mode = "documentation"; Extended = $false }
    }
    return [pscustomobject]@{ Mode = "quick"; Extended = $false }
}
