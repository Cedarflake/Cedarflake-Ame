function Assert-AmeApplicationBundleSource {
    param(
        [Parameter(Mandatory = $true)]
        [string]$BundlePath,
        [AllowEmptyString()]
        [string]$ExpectedPublisher = ""
    )

    $resolved = [System.IO.Path]::GetFullPath($BundlePath)
    if (-not (Test-Path -LiteralPath $resolved -PathType Container)) {
        throw "The Ame application bundle was not found: $resolved"
    }
    $rootItem = Get-Item -LiteralPath $resolved -Force
    if ($rootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) {
        throw "The Ame application bundle root must not be a reparse point"
    }
    $entries = @(Get-AmeBrokerNoFollowTreeEntries -RootPath $resolved)
    if ($entries.Count -eq 0 -or $entries.Count -gt 4096) {
        throw "The Ame application bundle entry count is empty or unbounded"
    }
    $totalBytes = [uint64]0
    foreach ($entry in $entries) {
        $item = $entry.Item
        if (-not $entry.IsDirectory) {
            if ([uint64]$item.Length -gt ([uint64](2GB) - $totalBytes)) {
                throw "The Ame application bundle exceeds the bounded installer payload"
            }
            $totalBytes += [uint64]$item.Length
            if ($totalBytes -gt [uint64](2GB)) {
                throw "The Ame application bundle exceeds the bounded installer payload"
            }
        }
    }
    $clientBinary = Join-Path $resolved $script:AmeApplicationBinaryName
    Assert-AmeBrokerPayload `
        -Path $clientBinary `
        -ExpectedPublisher $ExpectedPublisher | Out-Null
    return [pscustomobject]@{
        BundlePath = $resolved
        ClientBinaryPath = $clientBinary
        EntryCount = $entries.Count
        TotalBytes = $totalBytes
    }
}

function New-AmeApplicationProtectedInstallTree {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $createdDirectories = @()
    try {
        $facts = Assert-AmeApplicationFixedInstallTree -InstallDirectory $InstallDirectory
        for (
            $index = $facts.FirstMissingIndex;
            $index -lt $facts.ExpectedPaths.Count;
            $index += 1
        ) {
            $directory = $facts.ExpectedPaths[$index]
            $sddl = if ($index -eq $facts.ExpectedPaths.Count - 1) {
                $script:AmeApplicationProtectedDirectorySddl
            } else {
                $script:AmeBrokerProtectedDirectorySddl
            }
            [Ame.JournalBrokerInstaller.NativeMethods]::CreateDirectoryWithSddl($directory, $sddl)
            $createdDirectories += $directory
            $facts = Assert-AmeApplicationFixedInstallTree -InstallDirectory $InstallDirectory
        }
        Set-AmeApplicationPathAcl -Path $InstallDirectory -Kind "Directory"
        Assert-AmeApplicationFixedInstallTree `
            -InstallDirectory $InstallDirectory `
            -RequireInstallDirectory | Out-Null
    } catch {
        $originalError = $_
        $rollbackFailures = @(Remove-AmeCreatedEmptyDirectories `
            -CreatedDirectories $createdDirectories)
        Throw-AmeBrokerTransactionFailure `
            -OriginalError $originalError `
            -RollbackFailures $rollbackFailures
    }
    return [pscustomobject]@{
        InstallDirectory = $InstallDirectory
        CreatedDirectories = $createdDirectories
    }
}

function Set-AmeApplicationTreeAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $entries = @(Get-AmeBrokerNoFollowTreeEntries -RootPath $InstallDirectory)
    if ($entries.Count -gt 4096) {
        throw "The installed Ame application tree exceeds its bounded entry count"
    }
    Set-AmeApplicationPathAcl -Path $InstallDirectory -Kind "Directory"
    foreach ($entry in $entries) {
        $kind = if ($entry.IsDirectory) { "Directory" } else { "File" }
        Set-AmeApplicationPathAcl -Path $entry.FullName -Kind $kind
    }
}

function Assert-AmeApplicationInstallation {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory,
        [AllowEmptyString()]
        [string]$ExpectedPublisher = ""
    )

    Assert-AmeApplicationFixedInstallTree `
        -InstallDirectory $InstallDirectory `
        -RequireInstallDirectory | Out-Null
    $entries = @(
        [pscustomobject]@{
            FullName = (Get-Item -LiteralPath $InstallDirectory -Force).FullName
            IsDirectory = $true
        }
        Get-AmeBrokerNoFollowTreeEntries -RootPath $InstallDirectory
    )
    if ($entries.Count -le 1 -or $entries.Count -gt 4097) {
        throw "The installed Ame application tree is empty or unbounded"
    }
    foreach ($entry in $entries) {
        $expected = ConvertTo-AmeBrokerComparablePath -Path $entry.FullName
        $final = ConvertTo-AmeBrokerComparablePath -Path (
            [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath($entry.FullName)
        )
        if (-not $expected.Equals($final, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "The installed Ame application entry resolves outside its fixed physical tree"
        }
        $kind = if ($entry.IsDirectory) { "Directory" } else { "File" }
        Assert-AmeApplicationExactAclFacts `
            -Facts (Get-AmeBrokerAclFacts -Path $entry.FullName) `
            -Kind $kind
    }
    $clientBinary = Join-Path $InstallDirectory $script:AmeApplicationBinaryName
    Assert-AmeBrokerPhysicalFile -ExpectedPath $clientBinary
    Assert-AmeBrokerPayload `
        -Path $clientBinary `
        -ExpectedPublisher $ExpectedPublisher | Out-Null
    return $clientBinary
}

function Copy-AmeApplicationBundleNoFollow {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourceBundlePath,
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $source = [IO.Path]::GetFullPath($SourceBundlePath)
    $destination = [IO.Path]::GetFullPath($InstallDirectory)
    $sourcePrefix = "$($source.TrimEnd([IO.Path]::DirectorySeparatorChar))$([IO.Path]::DirectorySeparatorChar)"
    $destinationPrefix = "$($destination.TrimEnd([IO.Path]::DirectorySeparatorChar))$([IO.Path]::DirectorySeparatorChar)"
    $entries = @(
        Get-AmeBrokerNoFollowTreeEntries -RootPath $source |
            Sort-Object { ([string]$_.FullName).Length }
    )
    foreach ($entry in $entries) {
        $live = Get-Item -LiteralPath $entry.FullName -Force
        if ($live.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "The Ame application source changed into a reparse point during copy"
        }
        $sourcePath = [IO.Path]::GetFullPath($live.FullName)
        if (-not $sourcePath.StartsWith($sourcePrefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw "The Ame application source escaped during copy"
        }
        $relative = $sourcePath.Substring($sourcePrefix.Length)
        $target = [IO.Path]::GetFullPath((Join-Path $destination $relative))
        if (-not $target.StartsWith(
            $destinationPrefix,
            [StringComparison]::OrdinalIgnoreCase
        )) {
            throw "The Ame application destination escaped during copy"
        }
        if ($live.PSIsContainer) {
            if (-not (Test-Path -LiteralPath $target -PathType Container)) {
                New-Item -ItemType Directory -Path $target | Out-Null
            }
        } else {
            Copy-Item -LiteralPath $sourcePath -Destination $target
        }
    }
}

function Install-AmeApplicationBundle {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourceBundlePath,
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory,
        [AllowEmptyString()]
        [string]$ExpectedPublisher = ""
    )

    $source = Assert-AmeApplicationBundleSource `
        -BundlePath $SourceBundlePath `
        -ExpectedPublisher $ExpectedPublisher
    if (Test-Path -LiteralPath $InstallDirectory) {
        throw "The fixed Ame application directory already exists"
    }
    $treeState = New-AmeApplicationProtectedInstallTree -InstallDirectory $InstallDirectory
    try {
        Copy-AmeApplicationBundleNoFollow `
            -SourceBundlePath $source.BundlePath `
            -InstallDirectory $InstallDirectory
        Set-AmeApplicationTreeAcl -InstallDirectory $InstallDirectory
        $clientBinary = Assert-AmeApplicationInstallation `
            -InstallDirectory $InstallDirectory `
            -ExpectedPublisher $ExpectedPublisher
    } catch {
        $originalError = $_
        $actions = @(
            New-AmeBrokerRollbackAction `
                -Name "remove owned Application bundle" `
                -Action {
                    if (Test-Path -LiteralPath $InstallDirectory -PathType Container) {
                        Assert-AmeApplicationFixedInstallTree `
                            -InstallDirectory $InstallDirectory `
                            -RequireInstallDirectory | Out-Null
                        Remove-AmeBrokerOwnedTreeNoFollow `
                            -Path $InstallDirectory `
                            -ExpectedParent (Get-AmeProductParentDirectory)
                    }
                }
        )
        $rollbackFailures = @(Invoke-AmeBrokerRollbackActions -Actions $actions)
        $parentDirectories = @($treeState.CreatedDirectories | Where-Object {
            -not $_.Equals($InstallDirectory, [StringComparison]::OrdinalIgnoreCase)
        })
        $rollbackFailures += @(Remove-AmeCreatedEmptyDirectories `
            -CreatedDirectories $parentDirectories)
        Throw-AmeBrokerTransactionFailure `
            -OriginalError $originalError `
            -RollbackFailures $rollbackFailures
    }
    return [pscustomobject]@{
        ClientBinaryPath = $clientBinary
        CreatedDirectories = $treeState.CreatedDirectories
    }
}
