$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release_reuse_github.ps1')
. (Join-Path $PSScriptRoot 'release_reuse_policy.ps1')
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')

function Assert-AmeStoreArtifactProducer {
    param([object]$Run, [object]$Artifact, [string]$Repository,
        [ValidateSet('Candidate', 'Msix')][string]$Kind, [string]$Tag)
    $workflow = if ($Kind -ceq 'Candidate') { 'release_candidate_windows.yml' } else { 'release_store_windows.yml' }
    if ($Run.repository.full_name -cne $Repository -or $Run.head_repository.full_name -cne $Repository -or
        $Run.path -cne ".github/workflows/$workflow" -or $Run.event -cne 'workflow_dispatch' -or
        $Run.head_branch -cne 'main' -or $Run.status -cne 'completed' -or $Run.conclusion -cne 'success' -or
        $Run.head_sha -cnotmatch '^[0-9a-f]{40}$') { throw 'Store input must come from a successful main release workflow' }
    Assert-AmeReleaseArtifact $Artifact $Run
    $name = if ($Kind -ceq 'Candidate') {
        '^Cedarflake-Ame-' + [regex]::Escape($Tag) + '-unsigned-' + $Run.id + '-[1-9][0-9]*$'
    } else { '^store-msix-' + $Run.id + '-[1-9][0-9]*$' }
    if ($Artifact.name -cnotmatch $name) { throw 'Store artifact is not the requested candidate or package' }
}

function Receive-AmeStoreArtifact {
    param([string]$Repository, [string]$RunId, [string]$ArtifactId,
        [ValidateSet('Candidate', 'Msix')][string]$Kind, [string]$Tag, [string]$Directory)
    if ($Repository -cne 'Cedarflake/Cedarflake-Ame' -or $RunId -cnotmatch '^[1-9][0-9]*$' -or
        $ArtifactId -cnotmatch '^[1-9][0-9]*$') { throw 'Store artifact request identity is invalid' }
    $run = Read-AmeReleaseApi "repos/$Repository/actions/runs/$RunId"
    $artifact = Read-AmeReleaseApi "repos/$Repository/actions/artifacts/$ArtifactId"
    Assert-AmeStoreArtifactProducer $run $artifact $Repository $Kind $Tag
    $destination = Assert-AmeAcceptanceScratchPath $Directory
    $build = Join-Path (Split-Path $PSScriptRoot) 'build'
    if (-not $destination.StartsWith($build + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Store artifact download must remain beneath build'
    }
    [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($destination))
    Expand-AmeReleaseReuseArtifact $Repository ([pscustomobject]@{ artifactId = $ArtifactId; digest = $artifact.digest }) $destination
    return $destination
}

function Assert-AmeStoreReleaseTag {
    param([string]$Repository, [object]$Receipt)
    if ($Repository -cne 'Cedarflake/Cedarflake-Ame') { throw 'Store packages belong to the Ame repository' }
    $tag = [uri]::EscapeDataString($Receipt.releaseTag)
    $reference = Read-AmeReleaseApi "repos/$Repository/git/ref/tags/$tag"
    $target = $reference.object
    for ($depth = 0; $target.type -ceq 'tag' -and $depth -lt 8; $depth++) {
        if ($target.sha -cnotmatch '^[0-9a-f]{40}$') { throw 'Release tag object is invalid' }
        $target = (Read-AmeReleaseApi "repos/$Repository/git/tags/$($target.sha)").object
    }
    if ($target.type -cne 'commit' -or $target.sha -cne $Receipt.sourceCommit) {
        throw 'Store package source does not match the current Release tag'
    }
}

function Get-AmeStoreAssetDisposition {
    param([object[]]$Assets, [string]$Name, [string]$Sha256)
    $existing = @($Assets | Where-Object name -CEQ $Name)
    if ($existing.Count -eq 0) { return 'upload' }
    if ($existing.Count -ne 1 -or $existing[0].state -cne 'uploaded' -or
        $existing[0].digest -cne "sha256:$Sha256") { throw 'Existing Store Release asset has a different or unverified digest' }
    return 'retained'
}

function Publish-AmeStoreAsset {
    param([string]$Repository, [object]$Receipt, [string]$Path)
    $lease = [IO.FileStream]::new($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    try {
        $hash = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
        $name = [IO.Path]::GetFileName($Path)
        Assert-AmeStoreReleaseTag $Repository $Receipt
        $release = Read-AmeReleaseApi "repos/$Repository/releases/tags/$([uri]::EscapeDataString($Receipt.releaseTag))"
        if ($release.draft -ne $false -or $release.tag_name -cne $Receipt.releaseTag -or
            [string]$release.id -cnotmatch '^[1-9][0-9]*$') { throw 'Store attachment requires the existing published Release' }
        $state = Get-AmeStoreAssetDisposition @($release.assets) $name $hash
        if ($state -ceq 'upload') {
            & gh release upload $Receipt.releaseTag $Path --repo $Repository
            if ($LASTEXITCODE -ne 0) { throw 'Store asset upload failed; retries never overwrite an existing attachment' }
        }
        Assert-AmeStoreReleaseTag $Repository $Receipt
        $after = Read-AmeReleaseApi "repos/$Repository/releases/$($release.id)"
        if ((Get-AmeStoreAssetDisposition @($after.assets) $name $hash) -cne 'retained') {
            throw 'Uploaded Store asset is not present with its verified digest'
        }
        Write-Host "Store Release asset verified: $name ($hash)"
    } finally { $lease.Dispose() }
}
