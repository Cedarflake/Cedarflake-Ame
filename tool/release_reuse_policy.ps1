$ErrorActionPreference = 'Stop'

function Get-AmeReleaseReuseStage {
    param([string[]]$ChangedPaths)

    $stage = 'packages'
    foreach ($path in $ChangedPaths) {
        $path = $path.Replace('\', '/')
        # Application identity is checked against the immutable release tag. These
        # main-branch files are not inputs to current packaging or publication.
        if ($path -cmatch '^(docs/|lib/|rust/|test/|integration_test/|assets/|[^/]+\.md$|pubspec\.(yaml|lock)$)' -or
            $path -cin @('.github/workflows/release_candidate_windows.yml',
                '.github/workflows/release_publish_windows.yml', '.github/workflows/release_verify_published.yml',
                '.github/workflows/release_tooling_checks.yml', '.github/workflows/quality_ci.yml',
                'tool/quality_ci_policy.ps1', 'tool/quality_select_ci.ps1', 'tool/quality_test_ci_policy.ps1') -or
            $path -cmatch '^tool/release_(reuse_[^/]+|prepare_reuse|validate_reuse|test_reuse|test_tooling|test_windows_publication[^/]*)\.ps1$') {
            continue
        }
        if ($path -ceq '.github/workflows/release_package_windows.yml' -or
            $path -cmatch '^windows/installer/' -or
            $path -cmatch '^tool/release_(package_[^/]+|installer_[^/]+|prepare_installer_[^/]+|prepare_classification_models|classification_models|test_classification_models|test_installer[^/]*)\.ps1$') {
            $stage = 'candidate'
            continue
        }
        return 'fresh'
    }
    return $stage
}

function Test-AmeReleaseToolingPath {
    param([string]$Path)

    return $Path -cmatch '^tool/release_(reuse_[^/]+|prepare_reuse|validate_reuse|test_reuse|test_tooling|package_[^/]+|installer_[^/]+|prepare_installer_[^/]+|test_windows_publication[^/]*)\.ps1$' -or
        $Path -cmatch '^\.github/workflows/release_(candidate_windows|package_windows|publish_windows|verify_published|tooling_checks)\.yml$' -or
        $Path -cin @('tool/release_classification_models.ps1', 'tool/release_prepare_classification_models.ps1',
            'tool/release_test_classification_models.ps1', '.github/workflows/quality_ci.yml', 'tool/quality_ci_policy.ps1',
            'tool/quality_select_ci.ps1', 'tool/quality_test_ci_policy.ps1')
}

function Test-AmeReusableReleaseRun {
    param([object]$Run, [string]$Repository)

    return $Run.event -ceq 'workflow_dispatch' -and $Run.head_branch -ceq 'main' -and
        $Run.path -ceq '.github/workflows/release_candidate_windows.yml' -and
        $Run.repository.full_name -ceq $Repository -and $Run.head_repository.full_name -ceq $Repository -and
        $Run.status -ceq 'completed' -and $Run.head_sha -cmatch '^[0-9a-f]{40}$' -and
        [string]$Run.id -cmatch '^[1-9][0-9]*$'
}

function Assert-AmeReleaseArtifact {
    param([object]$Artifact, [object]$Run)

    if ($Artifact.expired -or $Artifact.size_in_bytes -le 0 -or $Artifact.size_in_bytes -gt 3GB -or
        $Artifact.digest -cnotmatch '^sha256:[0-9a-f]{64}$' -or
        [string]$Artifact.id -cnotmatch '^[1-9][0-9]*$' -or
        $Artifact.workflow_run.id -ne $Run.id -or $Artifact.workflow_run.head_sha -cne $Run.head_sha) {
        throw 'Release artifact is expired or has an untrusted run identity or digest'
    }
}

function Assert-AmeReleasePackageReceipt {
    param([object]$Receipt, [object]$Run, [string]$Tag, [string]$SourceCommit,
        [string]$SigningMode, [string]$ExpectedPublisher, [string]$ArtifactName, [int]$Attempt)

    if ($Receipt.format -cne 'cedarflake-ame-release-packages-v1' -or
        $Receipt.tag -cne $Tag -or $Receipt.sourceCommit -cne $SourceCommit -or
        $Receipt.workflowCommit -cne $Run.head_sha -or $Receipt.runId -ne $Run.id -or
        $Receipt.runAttempt -ne $Attempt -or $Receipt.packageArtifact -cne $ArtifactName -or
        $Receipt.signingMode -cne $SigningMode -or $Receipt.expectedPublisher -cne $ExpectedPublisher -or
        $Receipt.archiveName -cne "Cedarflake-Ame-$Tag-windows-x64-portable.zip" -or
        $Receipt.installerName -cne "Cedarflake-Ame-$Tag-windows-x64.msi" -or
        $Receipt.archiveSha256 -cnotmatch '^[0-9a-f]{64}$' -or
        $Receipt.installerSha256 -cnotmatch '^[0-9a-f]{64}$') {
        throw 'Package receipt does not match the requested release, channel and producing run'
    }
}

function Assert-AmeReusedPackageFiles {
    param([string]$Directory, [object]$Receipt)

    $files = @(Get-ChildItem -LiteralPath $Directory -Force)
    if ($files.Count -ne 2) { throw 'Reused packages require exactly one ZIP and one MSI' }
    $hashes = @{}
    $hashes[$Receipt.archiveName] = $Receipt.archiveSha256
    $hashes[$Receipt.installerName] = $Receipt.installerSha256
    foreach ($file in $files) {
        if ($file.PSIsContainer -or ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or
            -not $hashes.ContainsKey($file.Name) -or
            (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $hashes[$file.Name]) {
            throw 'Downloaded package bytes differ from the verified package receipt'
        }
    }
}
