$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'quality_workflow_contract.ps1')
$repository = Get-AmeRepositoryRoot

function Read-Workflow([string]$Name) {
    [IO.File]::ReadAllText((Join-Path $repository ".github/workflows/$Name"), [Text.Encoding]::UTF8)
}
function Require-Text([string]$Source, [string[]]$Required, [string[]]$Forbidden) {
    foreach ($text in $Required) {
        if (-not $Source.Contains($text)) { throw "Windows publication contract is missing: $text" }
    }
    foreach ($text in $Forbidden) {
        if ($Source.IndexOf($text, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
            throw "Windows publication contract admits: $text"
        }
    }
}

$entry = Read-Workflow 'release_candidate_windows.yml'
$package = Read-Workflow 'release_package_windows.yml'
$publisher = Read-Workflow 'release_publish_windows.yml'
$published = Read-Workflow 'release_verify_published.yml'
$packageCall = Get-AmeWorkflowJobBlock $entry 'package_windows'
$publishCall = Get-AmeWorkflowJobBlock $entry 'publish_windows'
$verifyCall = Get-AmeWorkflowJobBlock $entry 'verify_published'
$packageJob = Get-AmeWorkflowJobBlock $package 'package_windows'
$publishJob = Get-AmeWorkflowJobBlock $publisher 'publish_windows'

Require-Text $packageCall @(
    '- release_request', '- release_windows', 'contents: read',
    'uses: ./.github/workflows/release_package_windows.yml',
    'source_commit: ${{ needs.release_windows.outputs.source_commit }}',
    'trusted_workflow_commit: ${{ needs.release_request.outputs.trusted_workflow_commit }}',
    'verified_bundle_artifact: ${{ needs.release_windows.outputs.verified_bundle_artifact }}'
) @('contents: write', 'secrets:', 'steps:')
Require-Text $packageJob @(
    'contents: read', 'ref: ${{ inputs.source_commit }}', 'persist-credentials: false',
    'git merge-base --is-ancestor $headCommit $env:AME_TRUSTED_WORKFLOW_COMMIT',
    'manifest.sourceCommit -cne $env:AME_SOURCE_COMMIT', 'cedarflake-ame-signed-v1',
    'release_verify_windows_candidate.ps1', 'release_package_portable_windows.ps1',
    'release_prepare_installer_toolchain.ps1', 'release_package_installer_windows.ps1',
    '-SigningMode $env:AME_RELEASE_SIGNING_MODE', '-ExpectedBrokerPublisher $env:AME_EXPECTED_PUBLISHER',
    'archive_sha256:', 'installer_sha256:', 'Upload immutable ZIP and MSI'
) @('contents: write', 'AME_SIGNING_PFX', 'Set-AuthenticodeSignature', 'gh release ')
Require-Text $publishCall @(
    '- release_windows', '- package_windows', 'contents: write',
    'uses: ./.github/workflows/release_publish_windows.yml',
    'source_commit: ${{ needs.release_windows.outputs.source_commit }}',
    'package_artifact: ${{ needs.package_windows.outputs.package_artifact }}',
    'archive_sha256: ${{ needs.package_windows.outputs.archive_sha256 }}',
    'installer_sha256: ${{ needs.package_windows.outputs.installer_sha256 }}'
) @('secrets:', 'steps:')
Require-Text $publishJob @(
    'contents: write', 'Download immutable ZIP and MSI',
    'Publication requires exactly one ZIP and one MSI',
    'repos/$env:GITHUB_REPOSITORY/git/ref/tags/$env:AME_RELEASE_TAG',
    'repos/$env:GITHUB_REPOSITORY/git/tags/$objectSha',
    'GitHub release tag moved after candidate verification',
    'Release package changed immediately before publication',
    "'--verify-tag', '--draft'", 'gh release upload',
    'Release requires both verified ZIP and MSI assets', '--draft=false'
) @('actions/checkout@', './tool/', '.\\tool\\', 'git ', 'flutter ', 'dart ', 'cargo ',
    'Set-AuthenticodeSignature', 'AME_SIGNING_PFX', '--clobber')
foreach ($workflow in @($entry, $publisher)) {
    if ([regex]::Matches($workflow, '(?m)^\s+contents: write\s*$').Count -ne 1) {
        throw 'Only the publication job and its caller may receive content-write permission'
    }
}
Require-Text $verifyCall @(
    '- publish_windows', '- package_windows', 'contents: read',
    'uses: ./.github/workflows/release_verify_published.yml',
    'release_tag: ${{ inputs.release_tag }}',
    'expected_broker_publisher: ${{ inputs.expected_broker_publisher }}',
    'archive_sha256: ${{ needs.package_windows.outputs.archive_sha256 }}',
    'installer_sha256: ${{ needs.package_windows.outputs.installer_sha256 }}'
) @('contents: write', 'runs-on:', 'steps:', 'secrets:', 'actions: write', 'GH_TOKEN')
Require-Text $published @(
    'release:', 'workflow_call:', 'workflow_dispatch:',
    'AME_RELEASE_TAG: ${{ inputs.release_tag || github.event.release.tag_name }}',
    'AME_EXPECTED_PUBLISHER: ${{ inputs.expected_broker_publisher || vars.AME_WINDOWS_EXPECTED_PUBLISHER }}',
    '--pattern $artifactName', '--pattern $installerName', 'Assert-AmePublishedPackages',
    '-ExpectedArchiveSha256 $env:AME_ARCHIVE_SHA256', '-ExpectedInstallerSha256 $env:AME_INSTALLER_SHA256',
    'release_verify_portable_signatures.ps1'
) @('contents: write')
Write-Host 'Windows package and publication workflow contracts passed'
