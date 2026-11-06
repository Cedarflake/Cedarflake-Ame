$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'quality_unsigned_windows_payload.ps1')

function Get-AmeInstallerSource {
    [CmdletBinding(DefaultParameterSetName = 'Quality')]
    param(
        [Parameter(Mandatory = $true, ParameterSetName = 'Quality')][string]$BundlePath,
        [Parameter(Mandatory = $true, ParameterSetName = 'Quality')][string]$UnsignedEvidencePath,
        [Parameter(Mandatory = $true, ParameterSetName = 'Candidate')][string]$CandidateRoot,
        [Parameter(Mandatory = $true, ParameterSetName = 'Candidate')][string]$Tag,
        [Parameter(Mandatory = $true, ParameterSetName = 'Candidate')][string]$SourceCommit
    )

    if ($PSCmdlet.ParameterSetName -ceq 'Candidate') {
        Assert-AmeUnsignedBuildPath $CandidateRoot
        & (Join-Path $PSScriptRoot 'release_verify_unsigned_candidate.ps1') `
            -CandidateRoot $CandidateRoot -Tag $Tag -SourceCommit $SourceCommit | Out-Host
        $manifest = Get-Content -LiteralPath (Join-Path $CandidateRoot 'manifest.json') `
            -Raw -Encoding utf8 | ConvertFrom-Json
        return [pscustomobject]@{
            Root = [IO.Path]::GetFullPath((Join-Path $CandidateRoot 'payload'))
            SourceCommit = $manifest.sourceCommit
            DirtyWorktree = $false
            Files = @($manifest.files)
            EvidenceFormat = $manifest.format
        }
    }

    Assert-AmeUnsignedBuildPath $BundlePath
    Assert-AmeUnsignedBuildPath $UnsignedEvidencePath
    $evidence = Get-Content -LiteralPath $UnsignedEvidencePath -Raw -Encoding utf8 | ConvertFrom-Json
    if ($evidence.format -cne 'cedarflake-ame-unsigned-quality-v1' -or $evidence.status -cne 'passed' -or
        $evidence.payload.machine -cne 'windows-x64' -or $evidence.payload.configuration -cne 'Release' -or
        $evidence.bridgeSmoke -cne 'passed_without_catalog_access' -or
        @($evidence.payload.files).Count -ne $evidence.payload.fileCount) {
        throw 'Installer requires complete unsigned Release payload verification'
    }
    return [pscustomobject]@{
        Root = [IO.Path]::GetFullPath($BundlePath)
        SourceCommit = $evidence.sourceCommit
        DirtyWorktree = $evidence.dirtyWorktree
        Files = @($evidence.payload.files)
        EvidenceFormat = $evidence.format
    }
}
