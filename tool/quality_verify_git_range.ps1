[CmdletBinding()]
param(
    [string]$BaseRevision,
    [string]$HeadRevision = "HEAD"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

function Assert-AmeGitRevision {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Revision,
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    if ($Revision.StartsWith("-") -or $Revision -notmatch "^[0-9A-Za-z][0-9A-Za-z._/~^-]*$") {
        throw "$Name is not a supported Git revision: $Revision"
    }
}

function Test-AmeGitCommit {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Git,
        [Parameter(Mandatory = $true)]
        [string]$Revision
    )

    & $Git "rev-parse" "--verify" "--quiet" "$Revision^{commit}" 2>$null | Out-Null
    return $LASTEXITCODE -eq 0
}

$toolchain = Get-AmeToolchain
Assert-AmeGitRevision -Revision $HeadRevision -Name "Head revision"
if (-not (Test-AmeGitCommit -Git $toolchain.Git -Revision $HeadRevision)) {
    throw "Head revision does not resolve to a commit: $HeadRevision"
}

$isMissingBase = [string]::IsNullOrWhiteSpace($BaseRevision) -or
    $BaseRevision -match "^0{40}$"
if ($isMissingBase) {
    $parentRevision = "$HeadRevision^"
    if (Test-AmeGitCommit -Git $toolchain.Git -Revision $parentRevision) {
        Invoke-AmeChecked $toolchain.Git @(
            "diff",
            "--check",
            $parentRevision,
            $HeadRevision,
            "--"
        )
    } else {
        Invoke-AmeChecked $toolchain.Git @(
            "diff-tree",
            "--check",
            "--root",
            "-r",
            $HeadRevision,
            "--"
        )
    }
    return
}

Assert-AmeGitRevision -Revision $BaseRevision -Name "Base revision"
if (-not (Test-AmeGitCommit -Git $toolchain.Git -Revision $BaseRevision)) {
    throw "Base revision does not resolve to a commit: $BaseRevision"
}

Invoke-AmeChecked $toolchain.Git @(
    "diff",
    "--check",
    "$BaseRevision...$HeadRevision",
    "--"
)
