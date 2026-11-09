[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$PlanPath,
    [Parameter(Mandatory = $true)][string]$ArtifactDirectory,
    [string]$GitHubOutput
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release_reuse_policy.ps1')
. (Join-Path $PSScriptRoot 'release_reuse_github.ps1')
$plan = Get-Content -LiteralPath $PlanPath -Raw -Encoding utf8 | ConvertFrom-Json
$buildPrefix = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../build')) + [IO.Path]::DirectorySeparatorChar
$artifactPath = [IO.Path]::GetFullPath($ArtifactDirectory)
if (-not $artifactPath.StartsWith($buildPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Reused artifacts must stay inside repository build storage'
}
Expand-AmeReleaseReuseArtifact -Repository $plan.repository -Selection $plan.selection -Directory $artifactPath
switch -CaseSensitive ($plan.mode) {
    'candidate' {
        & (Join-Path $PSScriptRoot 'release_verify_windows_candidate.ps1') `
            -CandidateRoot $ArtifactDirectory -Tag $plan.tag -SourceCommit $plan.sourceCommit `
            -SigningMode $plan.signingMode -ExpectedBrokerPublisher $plan.expectedPublisher
        $manifestHash = (Get-FileHash -LiteralPath (Join-Path $artifactPath 'manifest.json') -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($GitHubOutput) { "manifest_sha256=$manifestHash" | Out-File -LiteralPath $GitHubOutput -Append -Encoding utf8 }
    }
    'packages' { Assert-AmeReusedPackageFiles -Directory $ArtifactDirectory -Receipt $plan.selection.receipt }
    default { throw 'Artifact validation requires a selected candidate or package pair' }
}
Write-Host "Verified reused $($plan.mode) without building or publishing"
