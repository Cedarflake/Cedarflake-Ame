$ErrorActionPreference = 'Stop'

function Read-AmeReleaseApi {
    param([string]$Endpoint)
    $json = & gh api $Endpoint
    if ($LASTEXITCODE -ne 0) { throw "Release provenance lookup failed: $Endpoint" }
    return (($json -join "`n") | ConvertFrom-Json)
}

function Read-AmeReleaseApiPages {
    param([string]$Endpoint, [string]$Property)
    for ($page = 1; $page -le 10; $page++) {
        $response = Read-AmeReleaseApi "${Endpoint}?per_page=100&page=$page"
        $items = @($response.$Property)
        $items
        if ($items.Count -lt 100) { return }
    }
    throw 'Release provenance exceeded the bounded API page limit'
}

function Get-AmeReleaseToolChanges {
    param([string]$PreviousCommit, [string]$CurrentCommit)
    & git merge-base --is-ancestor $PreviousCommit $CurrentCommit
    if ($LASTEXITCODE -ne 0) { throw 'Artifact producer is not an ancestor of the current trusted workflow' }
    $paths = @(& git -c core.quotepath=false diff --name-only --no-renames $PreviousCommit $CurrentCommit --)
    if ($LASTEXITCODE -ne 0) { throw 'Could not compare release tooling revisions' }
    foreach ($path in $paths) {
        if ($path -cmatch '^\.github/workflows/[^/]+\.yml$') {
            $before = (& git show "${PreviousCommit}:$path" 2>$null) -join "`n"
            $beforeExists = $LASTEXITCODE -eq 0
            $after = (& git show "${CurrentCommit}:$path" 2>$null) -join "`n"
            $afterExists = $LASTEXITCODE -eq 0
            $retention = '(?m)^(\s*retention-days:)\s*[0-9]+\s*$'
            if ($beforeExists -and $afterExists -and
                [regex]::Replace($before, $retention, '$1 retention') -ceq
                [regex]::Replace($after, $retention, '$1 retention')) { continue }
        }
        $path
    }
}

function Read-AmePackageReceiptArtifact {
    param([string]$Repository, [object]$Artifact, [string]$Directory)

    if ($Artifact.size_in_bytes -gt 65536) { throw 'Package receipt artifact is too large' }
    [void][IO.Directory]::CreateDirectory($Directory)
    $archivePath = Join-Path $Directory "receipt-$($Artifact.id).zip"
    Invoke-WebRequest -Uri "https://api.github.com/repos/$Repository/actions/artifacts/$($Artifact.id)/zip" `
        -Headers @{ Authorization = "Bearer $env:GH_TOKEN"; Accept = 'application/vnd.github+json' } `
        -OutFile $archivePath -TimeoutSec 60
    if ('sha256:' + (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant() -cne $Artifact.digest) {
        throw 'Package receipt transport digest changed'
    }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($archivePath)
    try {
        if ($archive.Entries.Count -ne 1 -or $archive.Entries[0].FullName -cne 'release-package-receipt.json' -or
            $archive.Entries[0].Length -gt 65536) { throw 'Package receipt archive has unexpected entries' }
        $reader = [IO.StreamReader]::new($archive.Entries[0].Open(), [Text.UTF8Encoding]::new($false, $true))
        try { return ($reader.ReadToEnd() | ConvertFrom-Json) } finally { $reader.Dispose() }
    } finally { $archive.Dispose() }
}

function Expand-AmeReleaseReuseArtifact {
    param([string]$Repository, [object]$Selection, [string]$Directory)

    $destination = [IO.Path]::GetFullPath($Directory)
    if (Test-Path -LiteralPath $destination) { throw 'Reuse download requires a fresh destination' }
    $archivePath = "$destination.zip"
    Invoke-WebRequest -Uri "https://api.github.com/repos/$Repository/actions/artifacts/$($Selection.artifactId)/zip" `
        -Headers @{ Authorization = "Bearer $env:GH_TOKEN"; Accept = 'application/vnd.github+json' } `
        -OutFile $archivePath -TimeoutSec 300
    if ('sha256:' + (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant() -cne $Selection.digest) {
        throw 'Reused artifact transport digest changed'
    }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($archivePath)
    try {
        if ($archive.Entries.Count -eq 0 -or $archive.Entries.Count -gt 4098) { throw 'Reused artifact has an invalid entry count' }
        $paths = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
        $total = 0L
        foreach ($entry in $archive.Entries) {
            $path = $entry.FullName
            if ([string]::IsNullOrWhiteSpace($path) -or $path.Contains('\') -or $path.Contains(':') -or
                $path.StartsWith('/') -or $path.Split('/') -contains '..' -or $path.Split('/') -contains '.' -or
                -not $paths.Add($path)) { throw 'Reused artifact contains an unsafe or duplicate path' }
            $total += $entry.Length
            if ($total -gt 3GB) { throw 'Reused artifact expands beyond the release bound' }
        }
    } finally { $archive.Dispose() }
    [IO.Compression.ZipFile]::ExtractToDirectory($archivePath, $destination)
}

function Find-AmeReleaseReuse {
    param([string]$Repository, [string]$Tag, [string]$SourceCommit, [string]$WorkflowCommit,
        [string]$SigningMode, [string]$ExpectedPublisher, [string]$PreviousRun, [string]$Directory)

    if ($PreviousRun -ceq 'none') { return $null }
    $endpoint = "repos/$Repository/actions"
    if ($PreviousRun -ceq 'auto') {
        $runs = @( (Read-AmeReleaseApi "$endpoint/workflows/release_candidate_windows.yml/runs?event=workflow_dispatch&branch=main&per_page=30").workflow_runs )
    } elseif ($PreviousRun -cmatch '^[1-9][0-9]*$') {
        $runs = @(Read-AmeReleaseApi "$endpoint/runs/$PreviousRun")
    } else { throw 'Reuse source must be auto, none or one workflow run ID' }

    $candidate = $null
    foreach ($run in $runs) {
        if (-not (Test-AmeReusableReleaseRun -Run $run -Repository $Repository)) { continue }
        $changes = @(Get-AmeReleaseToolChanges -PreviousCommit $run.head_sha -CurrentCommit $WorkflowCommit)
        $stage = Get-AmeReleaseReuseStage -ChangedPaths $changes
        if ($stage -ceq 'fresh') { continue }
        $artifacts = @(Read-AmeReleaseApiPages "$endpoint/runs/$($run.id)/artifacts" 'artifacts')
        $jobsByAttempt = @{}
        $tagPattern = [regex]::Escape($Tag)
        foreach ($artifact in $artifacts) {
            if ($artifact.expired) { continue }
            $kind = $null
            if ($stage -ceq 'packages' -and $artifact.name -cmatch "^Cedarflake-Ame-$tagPattern-windows-$SourceCommit-$($run.id)-([1-9][0-9]*)$") {
                $kind = 'packages'
            } elseif ($null -eq $candidate -and $artifact.name -cmatch "^Cedarflake-Ame-$tagPattern-$SigningMode-$($run.id)-([1-9][0-9]*)$") {
                $kind = 'candidate'
            }
            if ($null -eq $kind) { continue }
            $attempt = [int]$Matches[1]
            Assert-AmeReleaseArtifact -Artifact $artifact -Run $run
            if (-not $jobsByAttempt.ContainsKey($attempt)) {
                $jobsByAttempt[$attempt] = @(Read-AmeReleaseApiPages "$endpoint/runs/$($run.id)/attempts/$attempt/jobs" 'jobs')
            }
            $passed = @($jobsByAttempt[$attempt] | Where-Object conclusion -CEQ 'success' | ForEach-Object name)
            if ($kind -ceq 'packages') {
                if ($passed -cnotcontains 'Windows Packages / Windows ZIP and MSI Packages') { continue }
                $receipts = @($artifacts | Where-Object { $_.name -ceq "$($artifact.name)-receipt" -and -not $_.expired })
                if ($receipts.Count -eq 0) { continue }
                if ($receipts.Count -ne 1) { throw 'Package receipt identity is ambiguous' }
                Assert-AmeReleaseArtifact -Artifact $receipts[0] -Run $run
                $receipt = Read-AmePackageReceiptArtifact -Repository $Repository -Artifact $receipts[0] -Directory $Directory
                if ($receipt.signingMode -cne $SigningMode -or $receipt.expectedPublisher -cne $ExpectedPublisher) { continue }
                Assert-AmeReleasePackageReceipt -Receipt $receipt -Run $run -Tag $Tag -SourceCommit $SourceCommit `
                    -SigningMode $SigningMode -ExpectedPublisher $ExpectedPublisher -ArtifactName $artifact.name -Attempt $attempt
                return [pscustomobject]@{ mode = $kind; runId = $run.id; artifactId = $artifact.id; artifactName = $artifact.name; digest = $artifact.digest; receipt = $receipt }
            }
            $verifier = if ($SigningMode -ceq 'unsigned') { 'Verify Unsigned Candidate' } else { 'Signed Verification' }
            if ($passed -ccontains 'Windows Candidate Gate / Windows Gate' -and
                $passed -ccontains "Windows Candidate Gate / Release / $verifier") {
                $candidate = [pscustomobject]@{ mode = $kind; runId = $run.id; artifactId = $artifact.id; artifactName = $artifact.name; digest = $artifact.digest; receipt = $null }
            }
        }
    }
    if ($null -eq $candidate -and $PreviousRun -cne 'auto') {
        throw 'Requested run has no compatible, unexpired artifact with successful prerequisite jobs'
    }
    return $candidate
}
