. (Join-Path $PSScriptRoot "release_journal_broker_acl.ps1")
. (Join-Path $PSScriptRoot "release_journal_broker_identity.ps1")
. (Join-Path $PSScriptRoot "release_journal_broker_uninstall_policy.ps1")
. (Join-Path $PSScriptRoot "release_journal_broker_privileges.ps1")

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$script:AmeBrokerServiceName = "CedarflakeAmeJournalBroker"
$script:AmeBrokerDisplayName = "Cedarflake Ame Journal Broker"
$script:AmeBrokerBinaryName = "cedarflake_ame_journal_broker.exe"
$script:AmeBrokerRelativeInstallDirectory = "Cedarflake Ame\Journal Broker"
$script:AmeApplicationBinaryName = "cedarflake_ame.exe"
$script:AmeApplicationRelativeInstallDirectory = "Cedarflake Ame\Application"
$script:AmeBrokerServiceDacl = "O:BAG:BAD:(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCRPLO;;;IU)"
$script:AmeBrokerProgramFilesX64KnownFolderId = "6D809377-6AF0-444b-8957-A3773F02200E"
$script:AmeBrokerAdministratorsSid = "S-1-5-32-544"
$script:AmeBrokerSystemSid = "S-1-5-18"
$script:AmeBrokerTrustedInstallerSid = "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464"
$script:AmeBrokerServiceSid = "S-1-5-80-2098581772-366539847-2170527201-1913264099-3006267874"
$script:AmeBrokerUsersSid = "S-1-5-32-545"
$script:AmeBrokerProtectedDirectorySddl = "O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1200A9;;;BU)"
$script:AmeApplicationProtectedDirectorySddl = "O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1200A9;;;BU)"
$script:AmeBrokerProtocolVersion = 5
$script:AmeBrokerMaximumFrameBytes = 1048576
$script:AmeBrokerTransactionSchema = 3
$script:AmeBrokerTransactionMarkerName = ".cedarflake-ame-journal-broker-transaction-v2.json"
$script:AmeBrokerTransactionMaximumBytes = 65536

. (Join-Path $PSScriptRoot "release_journal_broker_native.ps1")
. (Join-Path $PSScriptRoot "release_journal_broker_transaction_state.ps1")
. (Join-Path $PSScriptRoot "release_journal_broker_transaction_moves.ps1")
. (Join-Path $PSScriptRoot "release_journal_broker_uninstall.ps1")
. (Join-Path $PSScriptRoot "release_journal_broker_recovery.ps1")
. (Join-Path $PSScriptRoot "release_journal_broker_service.ps1")
. (Join-Path $PSScriptRoot "release_journal_broker_application.ps1")

function Get-AmeBrokerProgramFilesX64 {
    if (-not [Environment]::Is64BitProcess) {
        throw "The journal broker installer must run in a native Windows x64 process"
    }
    $path = [Ame.JournalBrokerInstaller.NativeMethods]::GetProgramFilesX64(
        $script:AmeBrokerProgramFilesX64KnownFolderId
    )
    if (-not [System.IO.Path]::IsPathRooted($path)) {
        throw "Program Files x64 known folder is not an absolute path"
    }
    return [System.IO.Path]::GetFullPath($path)
}

function Get-AmeBrokerInstallDirectory {
    $programFilesPath = Get-AmeBrokerProgramFilesX64
    return [System.IO.Path]::GetFullPath(
        (Join-Path $programFilesPath $script:AmeBrokerRelativeInstallDirectory)
    )
}

function Get-AmeApplicationInstallDirectory {
    $programFilesPath = Get-AmeBrokerProgramFilesX64
    return [System.IO.Path]::GetFullPath(
        (Join-Path $programFilesPath $script:AmeApplicationRelativeInstallDirectory)
    )
}

function Get-AmeApplicationInstalledBinaryPath {
    return Join-Path (Get-AmeApplicationInstallDirectory) $script:AmeApplicationBinaryName
}

function Get-AmeProductParentDirectory {
    return [IO.Path]::GetDirectoryName((Get-AmeBrokerInstallDirectory))
}

function Assert-AmeBrokerNoFollowEntryFacts {
    param(
        [Parameter(Mandatory = $true)]
        [object[]]$Entries,
        [Parameter(Mandatory = $true)]
        [string]$RootPath
    )

    $root = ConvertTo-AmeBrokerComparablePath -Path $RootPath
    $prefix = "$($root.TrimEnd([IO.Path]::DirectorySeparatorChar))$([IO.Path]::DirectorySeparatorChar)"
    foreach ($entry in $Entries) {
        $path = ConvertTo-AmeBrokerComparablePath -Path ([string]$entry.FullName)
        if (-not $path.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw "A no-follow tree entry escaped its admitted root"
        }
        if ([bool]$entry.IsReparsePoint) {
            throw "A no-follow tree operation refuses descendant reparse points"
        }
    }
}

function Get-AmeBrokerNoFollowTreeEntries {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RootPath,
        [int]$MaximumEntries = 4096
    )

    $root = [IO.Path]::GetFullPath($RootPath)
    $rootItem = Get-Item -LiteralPath $root -Force
    if (-not $rootItem.PSIsContainer -or
        ($rootItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "A no-follow tree root must be a physical directory"
    }
    $pending = [Collections.Generic.Stack[string]]::new()
    $pending.Push($root)
    $entries = [Collections.Generic.List[object]]::new()
    while ($pending.Count -gt 0) {
        $directory = $pending.Pop()
        foreach ($entry in @(Get-ChildItem -LiteralPath $directory -Force)) {
            if ($entries.Count -ge $MaximumEntries) {
                throw "A no-follow tree operation exceeded its bounded entry count"
            }
            $fact = [pscustomobject]@{
                FullName = $entry.FullName
                IsDirectory = [bool]$entry.PSIsContainer
                IsReparsePoint = [bool](
                    $entry.Attributes -band [IO.FileAttributes]::ReparsePoint
                )
                Item = $entry
            }
            Assert-AmeBrokerNoFollowEntryFacts -Entries @($fact) -RootPath $root
            $entries.Add($fact)
            if ($entry.PSIsContainer) {
                $pending.Push($entry.FullName)
            }
        }
    }
    return @($entries)
}

function Remove-AmeBrokerOwnedTreeNoFollow {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedParent
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        return
    }
    $root = [IO.Path]::GetFullPath($Path)
    $parent = [IO.Path]::GetDirectoryName($root)
    if (-not $parent.Equals(
        [IO.Path]::GetFullPath($ExpectedParent),
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "A transaction-owned tree is outside its fixed parent"
    }
    $entries = @(Get-AmeBrokerNoFollowTreeEntries -RootPath $root)
    foreach ($entry in @($entries | Sort-Object { ([string]$_.FullName).Length } -Descending)) {
        if ([bool]$entry.IsDirectory) {
            Remove-Item -LiteralPath $entry.FullName
        } else {
            Remove-Item -LiteralPath $entry.FullName -Force
        }
    }
    Remove-Item -LiteralPath $root
}

function Assert-AmeBrokerPlatformFacts {
    param(
        [Parameter(Mandatory = $true)]
        [bool]$IsWindowsPlatform,
        [Parameter(Mandatory = $true)]
        [bool]$Is64BitOperatingSystem,
        [Parameter(Mandatory = $true)]
        [bool]$Is64BitProcess,
        [Parameter(Mandatory = $true)]
        [int]$BuildNumber,
        [Parameter(Mandatory = $true)]
        [int]$ProductType
    )

    if (-not $IsWindowsPlatform -or -not $Is64BitOperatingSystem -or -not $Is64BitProcess) {
        throw "The journal broker supports only Windows 11 x64"
    }
    if ($BuildNumber -lt 22000 -or $ProductType -ne 1) {
        throw "The journal broker supports only Windows 11 client x64"
    }
}

function Assert-AmeBrokerPlatform {
    $isWindowsPlatform = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
        [System.Runtime.InteropServices.OSPlatform]::Windows
    )
    $operatingSystem = Get-CimInstance -ClassName Win32_OperatingSystem
    Assert-AmeBrokerPlatformFacts `
        -IsWindowsPlatform $isWindowsPlatform `
        -Is64BitOperatingSystem ([Environment]::Is64BitOperatingSystem) `
        -Is64BitProcess ([Environment]::Is64BitProcess) `
        -BuildNumber ([int]$operatingSystem.BuildNumber) `
        -ProductType ([int]$operatingSystem.ProductType)
}

function Assert-AmeBrokerAdministrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw "Journal broker lifecycle operations require an elevated administrator token"
    }
}

function Assert-AmeBrokerInstallDirectoryFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CandidatePath,
        [Parameter(Mandatory = $true)]
        [string]$TrustedProgramFilesPath
    )

    $expected = [System.IO.Path]::GetFullPath(
        (Join-Path $TrustedProgramFilesPath $script:AmeBrokerRelativeInstallDirectory)
    )
    $candidate = [System.IO.Path]::GetFullPath($CandidatePath)
    if (-not $candidate.Equals($expected, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "The broker install directory must be the fixed Ame-owned Program Files location"
    }
}

function Assert-AmeApplicationInstallDirectoryFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CandidatePath,
        [Parameter(Mandatory = $true)]
        [string]$TrustedProgramFilesPath
    )

    $expected = [System.IO.Path]::GetFullPath(
        (Join-Path $TrustedProgramFilesPath $script:AmeApplicationRelativeInstallDirectory)
    )
    $candidate = [System.IO.Path]::GetFullPath($CandidatePath)
    if (-not $candidate.Equals($expected, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "The Ame application directory must be the fixed Program Files location"
    }
}

function ConvertTo-AmeBrokerComparablePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $normalized = $Path
    if ($normalized.StartsWith("\\?\UNC\", [System.StringComparison]::OrdinalIgnoreCase)) {
        $normalized = "\\" + $normalized.Substring(8)
    } elseif ($normalized.StartsWith("\\?\", [System.StringComparison]::OrdinalIgnoreCase)) {
        $normalized = $normalized.Substring(4)
    }
    $fullPath = [System.IO.Path]::GetFullPath($normalized)
    $root = [System.IO.Path]::GetPathRoot($fullPath)
    if ($fullPath.Length -gt $root.Length) {
        return $fullPath.TrimEnd([System.IO.Path]::DirectorySeparatorChar)
    }
    return $root
}

function Get-AmeBrokerDirectoryIdentityFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPath,
        [Parameter(Mandatory = $true)]
        [bool]$IsVolumeRoot
    )

    $item = Get-Item -LiteralPath $ExpectedPath -Force
    $finalPath = [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath(
        $item.FullName
    )
    return [pscustomobject]@{
        ExpectedPath = ConvertTo-AmeBrokerComparablePath -Path $item.FullName
        FinalPath = ConvertTo-AmeBrokerComparablePath -Path $finalPath
        IsDirectory = [bool]$item.PSIsContainer
        IsReparsePoint = [bool](
            $item.Attributes -band [System.IO.FileAttributes]::ReparsePoint
        )
        IsVolumeRoot = $IsVolumeRoot
        Acl = Get-AmeBrokerAclFacts -Path $item.FullName
    }
}

function Get-AmeBrokerInstallTreeFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $trustedProgramFilesPath = Get-AmeBrokerProgramFilesX64
    Assert-AmeBrokerInstallDirectoryFacts `
        -CandidatePath $InstallDirectory `
        -TrustedProgramFilesPath $trustedProgramFilesPath
    $candidate = ConvertTo-AmeBrokerComparablePath -Path $InstallDirectory
    $root = [System.IO.Path]::GetPathRoot($candidate)
    if ([string]::IsNullOrWhiteSpace($root) -or $root.StartsWith("\\")) {
        throw "The broker install directory must be on a local Windows volume"
    }
    $expectedPaths = @($root)
    $currentPath = $root
    $relativePath = $candidate.Substring($root.Length)
    foreach ($component in $relativePath.Split(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.StringSplitOptions]::RemoveEmptyEntries
    )) {
        $currentPath = Join-Path $currentPath $component
        $expectedPaths += $currentPath
    }

    $entries = @()
    $firstMissingIndex = $expectedPaths.Count
    for ($index = 0; $index -lt $expectedPaths.Count; $index += 1) {
        try {
            $entry = Get-AmeBrokerDirectoryIdentityFacts `
                -ExpectedPath $expectedPaths[$index] `
                -IsVolumeRoot ($index -eq 0)
        } catch [System.Management.Automation.ItemNotFoundException] {
            $firstMissingIndex = $index
            break
        }
        $entries += $entry
    }
    return [pscustomobject]@{
        TrustedProgramFilesPath = $trustedProgramFilesPath
        InstallDirectory = $candidate
        ExpectedPaths = $expectedPaths
        ExistingEntries = $entries
        FirstMissingIndex = $firstMissingIndex
    }
}

function Get-AmeApplicationTreeFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $trustedProgramFilesPath = Get-AmeBrokerProgramFilesX64
    Assert-AmeApplicationInstallDirectoryFacts `
        -CandidatePath $InstallDirectory `
        -TrustedProgramFilesPath $trustedProgramFilesPath
    $candidate = ConvertTo-AmeBrokerComparablePath -Path $InstallDirectory
    $root = [System.IO.Path]::GetPathRoot($candidate)
    if ([string]::IsNullOrWhiteSpace($root) -or $root.StartsWith("\")) {
        throw "The Ame application directory must be on a local Windows volume"
    }
    $expectedPaths = @($root)
    $currentPath = $root
    $relativePath = $candidate.Substring($root.Length)
    foreach ($component in $relativePath.Split(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.StringSplitOptions]::RemoveEmptyEntries
    )) {
        $currentPath = Join-Path $currentPath $component
        $expectedPaths += $currentPath
    }

    $entries = @()
    $firstMissingIndex = $expectedPaths.Count
    for ($index = 0; $index -lt $expectedPaths.Count; $index += 1) {
        try {
            $entry = Get-AmeBrokerDirectoryIdentityFacts `
                -ExpectedPath $expectedPaths[$index] `
                -IsVolumeRoot ($index -eq 0)
        } catch [System.Management.Automation.ItemNotFoundException] {
            $firstMissingIndex = $index
            break
        }
        $entries += $entry
    }
    return [pscustomobject]@{
        TrustedProgramFilesPath = $trustedProgramFilesPath
        InstallDirectory = $candidate
        ExpectedPaths = $expectedPaths
        ExistingEntries = $entries
        FirstMissingIndex = $firstMissingIndex
    }
}

function Assert-AmeBrokerInstallTreeFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Facts,
        [switch]$RequireInstallDirectory
    )

    $trustedOwnerSids = @(
        $script:AmeBrokerSystemSid,
        $script:AmeBrokerAdministratorsSid,
        $script:AmeBrokerTrustedInstallerSid
    )
    $replaceMask = [uint64]0x000D0000
    $directoryWriteMask = [uint64]0x500D0156
    for ($index = 0; $index -lt $Facts.ExistingEntries.Count; $index += 1) {
        $entry = $Facts.ExistingEntries[$index]
        $expectedPath = ConvertTo-AmeBrokerComparablePath -Path $entry.ExpectedPath
        $finalPath = ConvertTo-AmeBrokerComparablePath -Path $entry.FinalPath
        if (-not $expectedPath.Equals(
            $finalPath,
            [System.StringComparison]::OrdinalIgnoreCase
        )) {
            throw "The broker install ancestor resolves outside its fixed physical path"
        }
        if (-not $entry.IsDirectory -or $entry.IsReparsePoint) {
            throw "The broker install ancestor must be a non-reparse directory"
        }
        if ($trustedOwnerSids -notcontains [string]$entry.Acl.OwnerSid) {
            throw "The broker install ancestor has an untrusted owner"
        }

        $dangerousMask = if ($entry.IsVolumeRoot) {
            $replaceMask -bor [uint64]0x40
        } else {
            $directoryWriteMask
        }
        foreach ($rule in $entry.Acl.Rules) {
            $isInheritOnly = ($rule.PropagationFlags -band 2) -ne 0
            if (
                $rule.AccessControlType -cne "Allow" -or
                $isInheritOnly -or
                $trustedOwnerSids -contains [string]$rule.Sid
            ) {
                continue
            }
            if (([uint64]$rule.Rights -band $dangerousMask) -ne 0) {
                throw "The broker install ancestor grants an untrusted write or replacement right"
            }
        }
    }
    if (
        $RequireInstallDirectory -and
        $Facts.FirstMissingIndex -lt $Facts.ExpectedPaths.Count
    ) {
        throw "The fixed broker install directory is missing"
    }
}

function Assert-AmeBrokerFixedInstallTree {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory,
        [switch]$RequireInstallDirectory
    )

    $facts = Get-AmeBrokerInstallTreeFacts -InstallDirectory $InstallDirectory
    Assert-AmeBrokerInstallTreeFacts `
        -Facts $facts `
        -RequireInstallDirectory:$RequireInstallDirectory
    return $facts
}

function Assert-AmeApplicationFixedInstallTree {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory,
        [switch]$RequireInstallDirectory
    )

    $facts = Get-AmeApplicationTreeFacts -InstallDirectory $InstallDirectory
    Assert-AmeBrokerInstallTreeFacts `
        -Facts $facts `
        -RequireInstallDirectory:$RequireInstallDirectory
    return $facts
}

function New-AmeBrokerProtectedInstallTree {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $createdDirectories = @()
    try {
        $facts = Assert-AmeBrokerFixedInstallTree -InstallDirectory $InstallDirectory
        for (
            $index = $facts.FirstMissingIndex;
            $index -lt $facts.ExpectedPaths.Count;
            $index += 1
        ) {
            $directory = $facts.ExpectedPaths[$index]
            [Ame.JournalBrokerInstaller.NativeMethods]::CreateDirectoryWithSddl(
                $directory,
                $script:AmeBrokerProtectedDirectorySddl
            )
            $createdDirectories += $directory
            Assert-AmeBrokerExactAclFacts `
                -Facts (Get-AmeBrokerAclFacts -Path $directory) `
                -Kind "Directory" `
                -IncludeServiceSid $false
            $facts = Assert-AmeBrokerFixedInstallTree -InstallDirectory $InstallDirectory
        }
        Assert-AmeBrokerFixedInstallTree `
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

function Remove-AmeCreatedEmptyDirectories {
    param(
        [string[]]$CreatedDirectories = @()
    )

    $actions = @()
    foreach ($directory in $CreatedDirectories) {
        $capturedDirectory = $directory
        $actions += New-AmeBrokerRollbackAction `
            -Name "remove owned directory $capturedDirectory" `
            -Action ({
                if (
                    (Test-Path -LiteralPath $capturedDirectory -PathType Container) -and
                    @(Get-ChildItem -LiteralPath $capturedDirectory -Force).Count -eq 0
                ) {
                    Remove-Item -LiteralPath $capturedDirectory
                }
            }.GetNewClosure())
    }
    return Invoke-AmeBrokerRollbackActions -Actions $actions
}

function Get-AmeBrokerConfiguredIdentityManifest {
    $serviceKeyPath = "HKLM:\SYSTEM\CurrentControlSet\Services\$script:AmeBrokerServiceName"
    $manifest = [string](Get-ItemProperty -LiteralPath $serviceKeyPath).Description
    Assert-AmeBrokerIdentityManifestFacts -Manifest $manifest
    return $manifest
}

function Assert-AmeBrokerReleaseBinaryName {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    if ([System.IO.Path]::GetFileName($Path) -cne $script:AmeBrokerBinaryName) {
        throw "The broker binary must retain its release filename"
    }
}

function Assert-AmeBrokerPhysicalFile {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPath
    )

    $item = Get-Item -LiteralPath $ExpectedPath -Force
    if ($item.PSIsContainer) {
        throw "The broker binary path must be a file"
    }
    if ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) {
        throw "The broker binary must not be a reparse point"
    }
    $expected = ConvertTo-AmeBrokerComparablePath -Path $item.FullName
    $final = ConvertTo-AmeBrokerComparablePath -Path (
        [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath($item.FullName)
    )
    if (-not $expected.Equals($final, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "The broker binary resolves outside its fixed physical path"
    }
}

function Assert-AmeBrokerInstallation {
    param(
        [Parameter(Mandatory = $true)]
        [string]$BinaryPath,
        [string]$ClientBinaryPath = "",
        [AllowEmptyString()]
        [string]$ExpectedPublisher = ""
    )

    $installDirectory = [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($BinaryPath))
    if ($ClientBinaryPath.Length -gt 0) {
        $applicationDirectory = [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($ClientBinaryPath))
        $verifiedClientBinary = Assert-AmeApplicationInstallation `
            -InstallDirectory $applicationDirectory `
            -ExpectedPublisher $ExpectedPublisher
        if (-not $verifiedClientBinary.Equals(
            ([IO.Path]::GetFullPath($ClientBinaryPath)), [StringComparison]::OrdinalIgnoreCase
        )) {
            throw "The optional Ame application path does not match its installation"
        }
    }
    Assert-AmeBrokerFixedInstallTree `
        -InstallDirectory $installDirectory `
        -RequireInstallDirectory | Out-Null
    Assert-AmeBrokerPhysicalFile -ExpectedPath $BinaryPath
    $identityManifest = New-AmeBrokerIdentityManifest `
        -BinaryPath $BinaryPath `
        -ExpectedPublisher $ExpectedPublisher
    Assert-AmeBrokerServiceConfiguration `
        -BinaryPath $BinaryPath `
        -IdentityManifest $identityManifest
    Assert-AmeBrokerBinaryAcl `
        -InstallDirectory $installDirectory `
        -BinaryPath $BinaryPath
}
