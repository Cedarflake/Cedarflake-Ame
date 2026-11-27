[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Tag,
    [Parameter(Mandatory)][string]$CandidateRun,
    [Parameter(Mandatory)][string]$CandidateArtifact,
    [string]$Repository = 'Cedarflake/Cedarflake-Ame'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release_store_artifacts.ps1')
if ($Tag -cnotmatch '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$') { throw 'Invalid Store release tag' }
$candidate = Receive-AmeStoreArtifact $Repository $CandidateRun $CandidateArtifact Candidate $Tag 'build/store-candidate'
$manifest = Get-Content -LiteralPath (Join-Path $candidate 'manifest.json') -Raw -Encoding utf8 | ConvertFrom-Json
if ($manifest.sourceCommit -cnotmatch '^[0-9a-f]{40}$') { throw 'Candidate source identity is invalid' }
Assert-AmeStoreReleaseTag $Repository ([pscustomobject]@{ releaseTag = $Tag; sourceCommit = $manifest.sourceCommit })
& git merge-base --is-ancestor $manifest.sourceCommit HEAD
if ($LASTEXITCODE -ne 0) { throw 'Store candidate is not an ancestor of the trusted tooling' }
$metadata = Join-Path $PWD 'build/store-source'
[void][IO.Directory]::CreateDirectory($metadata)
foreach ($name in @('pubspec.yaml', 'rust/Cargo.toml')) {
    $content = & git show "$($manifest.sourceCommit):$name"
    if ($LASTEXITCODE -ne 0) { throw 'Tagged application version metadata is unavailable' }
    [IO.File]::WriteAllText((Join-Path $metadata ([IO.Path]::GetFileName($name))),
        ($content -join "`n"), [Text.UTF8Encoding]::new($false))
}
& (Join-Path $PSScriptRoot 'release_package_msix_windows.ps1') -Tag $Tag -CandidateRoot $candidate `
    -SourceCommit $manifest.sourceCommit -PubspecPath (Join-Path $metadata 'pubspec.yaml') `
    -CargoManifestPath (Join-Path $metadata 'Cargo.toml') -OutputDirectory (Join-Path $PWD 'build/store-package')
