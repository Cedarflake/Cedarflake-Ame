$ErrorActionPreference = "Stop"

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

    # Only known presentation/documentation edits take the fast route. New or
    # unknown owners retain full regression, including deleted and renamed files.
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
