[CmdletBinding()]
param(
    [string]$BaseRevision,
    [ValidateSet("auto", "full", "extended")][string]$Requested = "auto"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_ci_policy.ps1")
. (Join-Path $PSScriptRoot 'release_reuse_github.ps1')

$paths = @()
if ($Requested -eq "auto") {
    if ($BaseRevision -and $BaseRevision -notmatch '^0{40}$') {
        if ($BaseRevision -cnotmatch '^[0-9a-f]{40}$') {
            throw "The CI comparison requires an immutable base commit"
        }
        $paths = @(Get-AmeReleaseToolChanges -PreviousCommit $BaseRevision -CurrentCommit HEAD)
    }
}
$policy = Get-AmeCiPolicy -ChangedPaths $paths -Requested $Requested
Write-Output "Selected $($policy.Mode) checks; extended workloads: $($policy.Extended)"
if ($env:GITHUB_OUTPUT) {
    "mode=$($policy.Mode)" | Out-File -LiteralPath $env:GITHUB_OUTPUT -Encoding utf8 -Append
    "extended=$($policy.Extended.ToString().ToLowerInvariant())" |
        Out-File -LiteralPath $env:GITHUB_OUTPUT -Encoding utf8 -Append
}
