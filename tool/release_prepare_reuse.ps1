[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Repository,
    [Parameter(Mandatory = $true)][string]$Tag,
    [Parameter(Mandatory = $true)][string]$WorkflowCommit,
    [ValidateSet('unsigned', 'signed')][string]$SigningMode = 'unsigned',
    [string]$ExpectedPublisher = '',
    [string]$PreviousRun = 'auto',
    [string]$Directory = 'build/release-reuse',
    [string]$GitHubOutput
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release_reuse_policy.ps1')
. (Join-Path $PSScriptRoot 'release_reuse_github.ps1')

if ($Repository -cnotmatch '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$' -or
    $WorkflowCommit -cnotmatch '^[0-9a-f]{40}$' -or
    $Tag -cnotmatch '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$') {
    throw 'Release reuse requires one repository, version tag and immutable workflow commit'
}
if (($SigningMode -ceq 'signed' -and [string]::IsNullOrWhiteSpace($ExpectedPublisher)) -or
    ($SigningMode -ceq 'unsigned' -and -not [string]::IsNullOrWhiteSpace($ExpectedPublisher))) {
    throw 'Release signing mode and publisher must agree'
}
$headCommit = (& git rev-parse --verify HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $headCommit -cne $WorkflowCommit) { throw 'Reuse planning checkout does not match the trusted workflow' }
$sourceCommit = (& git rev-parse --verify "refs/tags/$Tag^{commit}").Trim()
if ($LASTEXITCODE -ne 0 -or $sourceCommit -cnotmatch '^[0-9a-f]{40}$') { throw 'Release tag is missing or invalid' }
& git merge-base --is-ancestor $sourceCommit $WorkflowCommit
if ($LASTEXITCODE -ne 0) { throw 'Release source is not an ancestor of the trusted workflow' }
$directoryPath = [IO.Path]::GetFullPath($Directory)
$buildPrefix = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../build')) + [IO.Path]::DirectorySeparatorChar
if (-not $directoryPath.StartsWith($buildPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Release reuse records must stay inside repository build storage'
}
[void][IO.Directory]::CreateDirectory($directoryPath)
$selection = Find-AmeReleaseReuse -Repository $Repository -Tag $Tag -SourceCommit $sourceCommit `
    -WorkflowCommit $WorkflowCommit -SigningMode $SigningMode -ExpectedPublisher $ExpectedPublisher `
    -PreviousRun $PreviousRun -Directory $directoryPath
$mode = if ($null -eq $selection) { 'fresh' } else { $selection.mode }
$plan = [ordered]@{
    repository = $Repository; tag = $Tag; sourceCommit = $sourceCommit; workflowCommit = $WorkflowCommit
    signingMode = $SigningMode; expectedPublisher = $ExpectedPublisher; mode = $mode; selection = $selection
}
$plan | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $directoryPath 'plan.json') -Encoding utf8
$outputs = [ordered]@{ mode = $mode; source_commit = $sourceCommit }
if ($null -ne $selection) {
    $outputs.reuse_run = [string]$selection.runId
    $outputs.reuse_artifact_id = [string]$selection.artifactId
    $outputs.reuse_artifact = $selection.artifactName
    if ($mode -ceq 'packages') {
        $outputs.archive_name = $selection.receipt.archiveName
        $outputs.archive_sha256 = $selection.receipt.archiveSha256
        $outputs.installer_name = $selection.receipt.installerName
        $outputs.installer_sha256 = $selection.receipt.installerSha256
    }
}
foreach ($entry in $outputs.GetEnumerator()) {
    if ([string]$entry.Value -match '[\r\n]') { throw 'Release output must occupy one line' }
    if ($GitHubOutput) { "$($entry.Key)=$($entry.Value)" | Out-File -LiteralPath $GitHubOutput -Append -Encoding utf8 }
}
Write-Host "Release execution: $mode; application source: $sourceCommit"
if ($null -ne $selection) { Write-Host "Reuse run $($selection.runId), artifact $($selection.artifactId) ($($selection.artifactName))" }
