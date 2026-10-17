function Get-AmeBrokerTransactionMarkerPath {
    return Join-Path (Get-AmeBrokerProgramFilesX64) $script:AmeBrokerTransactionMarkerName
}

function Get-AmeBrokerParentPrestate {
    $parent = Get-AmeProductParentDirectory
    if (-not (Test-Path -LiteralPath $parent)) {
        return [pscustomobject]@{
            Path = $parent
            Existed = $false
            Sddl = $null
        }
    }
    $item = Get-Item -LiteralPath $parent -Force
    if (-not $item.PSIsContainer -or
        ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "The fixed Ame parent must be a physical directory"
    }
    return [pscustomobject]@{
        Path = $parent
        Existed = $true
        Sddl = [string](Get-Acl -LiteralPath $parent).Sddl
    }
}

function Assert-AmeBrokerParentPrestateRestoredFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Prestate,
        [Parameter(Mandatory = $true)]
        [bool]$CurrentExists,
        [bool]$CurrentIsReparsePoint = $false,
        [AllowNull()]
        [string]$CurrentSddl
    )

    if ([bool]$Prestate.Existed) {
        if (-not $CurrentExists -or $CurrentIsReparsePoint -or
            -not ([string]$CurrentSddl).Equals(
                [string]$Prestate.Sddl,
                [StringComparison]::Ordinal
            )) {
            throw "The pre-existing Ame parent was not restored exactly"
        }
    } elseif ($CurrentExists) {
        throw "The transaction-created Ame parent was not removed"
    }
}

function Restore-AmeBrokerParentPrestate {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Prestate
    )

    $expectedParent = Get-AmeProductParentDirectory
    if (-not ([string]$Prestate.Path).Equals(
        $expectedParent,
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "The transaction parent prestate is outside the fixed Ame tree"
    }
    if ([bool]$Prestate.Existed) {
        if (-not (Test-Path -LiteralPath $expectedParent -PathType Container)) {
            throw "The pre-existing Ame parent disappeared during rollback"
        }
        $item = Get-Item -LiteralPath $expectedParent -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "The pre-existing Ame parent became a reparse point"
        }
        $security = [Security.AccessControl.DirectorySecurity]::new()
        $security.SetSecurityDescriptorSddlForm([string]$Prestate.Sddl)
        Set-Acl -LiteralPath $expectedParent -AclObject $security
        Assert-AmeBrokerParentPrestateRestoredFacts `
            -Prestate $Prestate `
            -CurrentExists $true `
            -CurrentIsReparsePoint $false `
            -CurrentSddl ([string](Get-Acl -LiteralPath $expectedParent).Sddl)
        return
    }
    if (Test-Path -LiteralPath $expectedParent -PathType Container) {
        $item = Get-Item -LiteralPath $expectedParent -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "The transaction-owned Ame parent became a reparse point"
        }
        if (@(Get-ChildItem -LiteralPath $expectedParent -Force).Count -eq 0) {
            Remove-Item -LiteralPath $expectedParent
        }
    }
    Assert-AmeBrokerParentPrestateRestoredFacts `
        -Prestate $Prestate `
        -CurrentExists (Test-Path -LiteralPath $expectedParent)
}

function Set-AmeBrokerTransactionMarkerAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $security = [Security.AccessControl.FileSecurity]::new()
    $administrators = [Security.Principal.SecurityIdentifier]::new(
        $script:AmeBrokerAdministratorsSid
    )
    $system = [Security.Principal.SecurityIdentifier]::new($script:AmeBrokerSystemSid)
    $security.SetOwner($administrators)
    $security.SetGroup($administrators)
    $security.SetAccessRuleProtection($true, $false)
    foreach ($identity in @($system, $administrators)) {
        $security.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new(
            $identity,
            [Security.AccessControl.FileSystemRights]::FullControl,
            [Security.AccessControl.AccessControlType]::Allow
        )) | Out-Null
    }
    Set-Acl -LiteralPath $Path -AclObject $security
    Assert-AmeBrokerTransactionMarkerAclFacts -Facts (Get-AmeBrokerAclFacts -Path $Path)
}

function Assert-AmeBrokerTransactionMarkerAclFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Facts
    )

    if ([string]$Facts.OwnerSid -cne $script:AmeBrokerAdministratorsSid -or
        [string]$Facts.GroupSid -cne $script:AmeBrokerAdministratorsSid -or
        -not [bool]$Facts.AreAccessRulesProtected -or
        @($Facts.Rules).Count -ne 2) {
        throw "The broker transaction marker ACL is not protected exactly"
    }
    $expected = @($script:AmeBrokerSystemSid, $script:AmeBrokerAdministratorsSid)
    foreach ($rule in @($Facts.Rules)) {
        if ($expected -notcontains [string]$rule.Sid -or
            [uint64]$rule.Rights -ne [uint64]2032127 -or
            [string]$rule.AccessControlType -cne "Allow" -or
            [int]$rule.InheritanceFlags -ne 0 -or
            [int]$rule.PropagationFlags -ne 0 -or
            [bool]$rule.IsInherited) {
            throw "The broker transaction marker ACL grants unexpected authority"
        }
    }
}

function Assert-AmeBrokerAdoptableOrphanFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Facts
    )

    if (-not [bool]$Facts.DirectoryExists -or
        [bool]$Facts.ServiceExists -or
        [bool]$Facts.MarkerExists -or
        [bool]$Facts.BinaryExists -or
        [int]$Facts.EntryCount -ne 0) {
        throw "The pre-marker broker tree is not an empty unowned orphan"
    }
    Assert-AmeBrokerExactAclFacts `
        -Facts $Facts.Acl `
        -Kind "Directory" `
        -IncludeServiceSid $false
}

function Assert-AmeBrokerAdoptableOrphan {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $binary = Join-Path $InstallDirectory $script:AmeBrokerBinaryName
    $marker = Join-Path $InstallDirectory $script:AmeBrokerTransactionMarkerName
    $facts = [pscustomobject]@{
        DirectoryExists = Test-Path -LiteralPath $InstallDirectory -PathType Container
        ServiceExists = Test-AmeBrokerServiceExists
        MarkerExists = Test-Path -LiteralPath $marker
        BinaryExists = Test-Path -LiteralPath $binary
        EntryCount = if (Test-Path -LiteralPath $InstallDirectory -PathType Container) {
            @(Get-ChildItem -LiteralPath $InstallDirectory -Force).Count
        } else {
            -1
        }
        Acl = if (Test-Path -LiteralPath $InstallDirectory -PathType Container) {
            Get-AmeBrokerAclFacts -Path $InstallDirectory
        } else {
            $null
        }
    }
    Assert-AmeBrokerAdoptableOrphanFacts -Facts $facts
}

function New-AmeBrokerTransactionState {
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("install", "repair", "upgrade", "uninstall")]
        [string]$Operation,
        [Parameter(Mandatory = $true)]
        [psobject]$ParentPrestate,
        [AllowEmptyString()]
        [string]$ExpectedPublisher = "",
        [string[]]$OwnedDirectories = @()
    )

    Assert-AmeBrokerPublisherPolicy -ExpectedPublisher $ExpectedPublisher
    $process = [Diagnostics.Process]::GetCurrentProcess()
    $installedBinary = Join-Path `
        (Get-AmeBrokerInstallDirectory) `
        $script:AmeBrokerBinaryName
    $stagedBinary = switch ($Operation) {
        "install" { "$installedBinary.installing-$PID" }
        "repair" { "$installedBinary.upgrading-$PID" }
        "upgrade" { "$installedBinary.upgrading-$PID" }
        default { $null }
    }
    $backupBinary = if ($Operation -in @("repair", "upgrade")) {
        "$installedBinary.previous-$PID"
    } else {
        $null
    }
    return [pscustomobject]@{
        Schema = $script:AmeBrokerTransactionSchema
        TransactionId = [Guid]::NewGuid().ToString("N").ToUpperInvariant()
        Operation = $Operation
        OwnerProcessId = [int]$PID
        OwnerStartUtcTicks = [int64]$process.StartTime.ToUniversalTime().Ticks
        Phase = "prepared"
        NextOperationId = 1
        PendingIntent = $null
        AppliedOperations = @()
        InstalledBinary = $installedBinary
        StagedBinary = $stagedBinary
        BackupBinary = $backupBinary
        ApplicationDirectory = Get-AmeApplicationInstallDirectory
        OwnedDirectories = @($OwnedDirectories)
        ApplicationOwned = $false
        RemoveApplicationBundle = $false
        InstalledOwned = $false
        BackupCreated = $false
        ServiceCreated = $false
        ConfigurationChanged = $false
        PreviousIdentityManifest = $null
        NewIdentityManifest = $null
        ExpectedPreviousBinary = $null
        ExpectedNewBinary = $null
        ExpectedPublisher = $ExpectedPublisher
        WasRunning = $false
        ParentPath = [string]$ParentPrestate.Path
        ParentExisted = [bool]$ParentPrestate.Existed
        ParentSddl = $ParentPrestate.Sddl
        UninstallPendingOperation = $null
        UninstallProtectedTreeReady = $false
        UninstallServiceRemoved = $false
        UninstallBinaryRemoved = $false
        UninstallApplicationRemoved = $false
        UninstallInstallTreeRemoved = $false
        UninstallOwnedDirectoriesRemoved = $false
        UninstallParentRestored = $false
    }
}

function Invoke-AmeBrokerInitialTreeCreation {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [Parameter(Mandatory = $true)]
        [scriptblock]$CreateTreeAction,
        [scriptblock]$StateWriter = {
            param([psobject]$Value)
            Write-AmeBrokerTransactionState -State $Value
        }
    )

    if ([string]$State.Operation -cne "install" -or
        [string]$State.Phase -cne "prepared") {
        throw "Only a prepared install may create the initial protected tree"
    }
    $State.Phase = "tree_creation_pending"
    & $StateWriter $State
    & $CreateTreeAction
    $State.Phase = "protected_tree_created"
    & $StateWriter $State
}

function Assert-AmeBrokerTransactionStateFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    $required = @(
        "Schema", "TransactionId", "Operation", "OwnerProcessId", "OwnerStartUtcTicks", "Phase",
        "NextOperationId", "PendingIntent", "AppliedOperations",
        "InstalledBinary", "ApplicationDirectory", "OwnedDirectories", "ApplicationOwned",
        "InstalledOwned", "BackupCreated", "ServiceCreated", "ConfigurationChanged",
        "PreviousIdentityManifest", "NewIdentityManifest", "ExpectedPreviousBinary",
        "ExpectedNewBinary", "ExpectedPublisher", "WasRunning", "ParentPath", "ParentExisted",
        "UninstallPendingOperation",
        "UninstallProtectedTreeReady", "UninstallServiceRemoved", "UninstallBinaryRemoved",
        "UninstallApplicationRemoved", "UninstallInstallTreeRemoved",
        "UninstallOwnedDirectoriesRemoved", "UninstallParentRestored"
    )
    foreach ($name in $required) {
        if ($null -eq $State.PSObject.Properties[$name]) {
            throw "The broker transaction marker is missing $name"
        }
    }
    if ([int]$State.Schema -notin @(2, $script:AmeBrokerTransactionSchema) -or
        @("install", "repair", "upgrade", "uninstall") -notcontains [string]$State.Operation -or
        [string]$State.TransactionId -notmatch '^[0-9A-F]{32}$' -or
        [int]$State.OwnerProcessId -le 0 -or
        [int64]$State.OwnerStartUtcTicks -le 0 -or
        [int]$State.NextOperationId -le 0 -or
        ([int]$State.Schema -eq 2 -and
            [string]::IsNullOrWhiteSpace([string]$State.ExpectedPublisher)) -or
        ([string]$State.ExpectedPublisher).Length -gt 512) {
        throw "The broker transaction marker identity is invalid"
    }
    Assert-AmeBrokerPublisherPolicy -ExpectedPublisher ([string]$State.ExpectedPublisher)
    $removeApplicationBundle = Test-AmeBrokerUninstallApplicationRemoval -State $State
    $installDirectory = Get-AmeBrokerInstallDirectory
    $expectedInstalled = Join-Path $installDirectory $script:AmeBrokerBinaryName
    foreach ($fixedPathProperty in @("InstalledBinary", "ApplicationDirectory", "ParentPath")) {
        if ([string]::IsNullOrWhiteSpace([string]$State.$fixedPathProperty)) {
            throw "The broker transaction marker lacks fixed path $fixedPathProperty"
        }
    }
    try {
        $stateInstalledBinary = [IO.Path]::GetFullPath([string]$State.InstalledBinary)
        $stateApplicationDirectory = [IO.Path]::GetFullPath([string]$State.ApplicationDirectory)
        $stateParentPath = [IO.Path]::GetFullPath([string]$State.ParentPath)
    } catch [ArgumentException] {
        throw "The broker transaction marker contains an invalid fixed path"
    }
    if (-not $stateInstalledBinary.Equals(
        $expectedInstalled,
        [StringComparison]::OrdinalIgnoreCase
    ) -or -not $stateApplicationDirectory.Equals(
        (Get-AmeApplicationInstallDirectory),
        [StringComparison]::OrdinalIgnoreCase
    ) -or -not $stateParentPath.Equals(
        (Get-AmeProductParentDirectory),
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "The broker transaction marker escaped the fixed install tree"
    }
    $pidText = [string][int]$State.OwnerProcessId
    $expectedStaged = switch ([string]$State.Operation) {
        "install" { "$expectedInstalled.installing-$pidText" }
        "repair" { "$expectedInstalled.upgrading-$pidText" }
        "upgrade" { "$expectedInstalled.upgrading-$pidText" }
        default { "" }
    }
    $expectedBackup = if ([string]$State.Operation -in @("repair", "upgrade")) {
        "$expectedInstalled.previous-$pidText"
    } else {
        ""
    }
    if ([string]$State.StagedBinary -cne $expectedStaged -or
        [string]$State.BackupBinary -cne $expectedBackup) {
        throw "The broker transaction marker contains an unowned staging path"
    }
    if ($null -ne $State.ExpectedPreviousBinary) {
        Assert-AmeBrokerExpectedBinaryFacts -Facts $State.ExpectedPreviousBinary
    }
    if ($null -ne $State.ExpectedNewBinary) {
        Assert-AmeBrokerExpectedBinaryFacts -Facts $State.ExpectedNewBinary
        if (-not [bool]$State.ExpectedNewBinary.Exists) {
            throw "The transaction marker expected new broker identity is absent"
        }
    }
    if (($null -ne $State.PendingIntent -or @($State.AppliedOperations).Count -gt 0) -and
        $null -eq $State.ExpectedNewBinary) {
        throw "A transaction with file moves lacks the expected new broker identity"
    }
    Assert-AmeBrokerMoveJournalFacts -State $State
    $allowedOwnedDirectories = @(
        Get-AmeProductParentDirectory
        Get-AmeBrokerInstallDirectory
        if ([string]$State.Operation -cne "uninstall" -or $removeApplicationBundle) {
            Get-AmeApplicationInstallDirectory
        }
    )
    foreach ($directory in @($State.OwnedDirectories)) {
        if ([string]::IsNullOrWhiteSpace([string]$directory)) {
            throw "The broker transaction marker claims an empty owned directory"
        }
        try {
            $full = [IO.Path]::GetFullPath([string]$directory)
        } catch [ArgumentException] {
            throw "The broker transaction marker claims an invalid owned directory"
        }
        if (-not @($allowedOwnedDirectories | Where-Object {
            $full.Equals($_, [StringComparison]::OrdinalIgnoreCase)
        }).Count) {
            throw "The broker transaction marker claims an unowned directory"
        }
    }
    if ([bool]$State.ParentExisted) {
        if ([string]::IsNullOrWhiteSpace([string]$State.ParentSddl) -or
            ([string]$State.ParentSddl).Length -gt 8192) {
            throw "The broker transaction marker lacks the pre-existing parent ACL"
        }
        $probe = [Security.AccessControl.DirectorySecurity]::new()
        $probe.SetSecurityDescriptorSddlForm([string]$State.ParentSddl)
    } elseif (-not [string]::IsNullOrEmpty([string]$State.ParentSddl)) {
        throw "A transaction-created parent must not claim a prior ACL"
    }
}

function Write-AmeBrokerTransactionState {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    Assert-AmeBrokerTransactionStateFacts -State $State
    $marker = Get-AmeBrokerTransactionMarkerPath
    $directory = [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($marker))
    $trustedProgramFiles = Get-AmeBrokerProgramFilesX64
    if (-not $directory.Equals(
        $trustedProgramFiles,
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "The broker transaction marker escaped Program Files x64"
    }
    Assert-AmeBrokerFixedInstallTree `
        -InstallDirectory (Get-AmeBrokerInstallDirectory) | Out-Null
    $temporary = "$marker.writing-$PID"
    if (Test-Path -LiteralPath $temporary) {
        throw "The broker transaction marker staging path already exists"
    }
    $json = $State | ConvertTo-Json -Depth 6 -Compress
    $bytes = [Text.UTF8Encoding]::new($false).GetBytes($json)
    if ($bytes.Length -gt $script:AmeBrokerTransactionMaximumBytes) {
        throw "The broker transaction marker exceeds its bounded size"
    }
    $stream = [IO.FileStream]::new(
        $temporary,
        [IO.FileMode]::CreateNew,
        [IO.FileAccess]::Write,
        [IO.FileShare]::None,
        4096,
        [IO.FileOptions]::WriteThrough
    )
    try {
        $stream.Write($bytes, 0, $bytes.Length)
        $stream.Flush($true)
    } finally {
        $stream.Dispose()
    }
    try {
        Set-AmeBrokerTransactionMarkerAcl -Path $temporary
        [Ame.JournalBrokerInstaller.NativeMethods]::ReplaceFileWriteThrough($temporary, $marker)
        Assert-AmeBrokerTransactionMarkerAclFacts -Facts (Get-AmeBrokerAclFacts -Path $marker)
    } finally {
        if (Test-Path -LiteralPath $temporary -PathType Leaf) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
}

function Read-AmeBrokerTransactionState {
    $marker = Get-AmeBrokerTransactionMarkerPath
    if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) {
        return $null
    }
    $item = Get-Item -LiteralPath $marker -Force
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint -or
        [int64]$item.Length -gt $script:AmeBrokerTransactionMaximumBytes) {
        throw "The broker transaction marker is redirected or unbounded"
    }
    Assert-AmeBrokerTransactionMarkerAclFacts -Facts (Get-AmeBrokerAclFacts -Path $marker)
    $state = Get-Content -LiteralPath $marker -Raw -Encoding UTF8 | ConvertFrom-Json
    Assert-AmeBrokerTransactionStateFacts -State $state
    return $state
}

function Test-AmeBrokerTransactionOwnerActive {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [scriptblock]$ProcessProbe = {
            param([int]$ProcessId)
            try {
                $process = [Diagnostics.Process]::GetProcessById($ProcessId)
                try {
                    return [pscustomobject]@{
                        Exists = $true
                        StartUtcTicks = [int64]$process.StartTime.ToUniversalTime().Ticks
                    }
                } finally {
                    $process.Dispose()
                }
            } catch [ArgumentException] {
                return [pscustomobject]@{ Exists = $false; StartUtcTicks = [int64]0 }
            }
        }
    )

    $fact = & $ProcessProbe ([int]$State.OwnerProcessId)
    return [bool]$fact.Exists -and
        [int64]$fact.StartUtcTicks -eq [int64]$State.OwnerStartUtcTicks
}

function Remove-AmeBrokerTransactionMarker {
    $marker = Get-AmeBrokerTransactionMarkerPath
    if (Test-Path -LiteralPath $marker -PathType Leaf) {
        $item = Get-Item -LiteralPath $marker -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "The broker transaction marker became a reparse point"
        }
        Assert-AmeBrokerTransactionMarkerAclFacts -Facts (Get-AmeBrokerAclFacts -Path $marker)
        Remove-Item -LiteralPath $marker -Force
    }
}

function New-AmeBrokerUpgradeOwnership {
    param(
        [Parameter(Mandatory = $true)]
        [bool]$InstalledBinaryExists
    )

    return [pscustomobject]@{
        HadPreviousBinary = $InstalledBinaryExists
        BackupCreated = $false
    }
}

function Remove-AmeBrokerTransactionBackup {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Ownership,
        [Parameter(Mandatory = $true)]
        [string]$BackupPath,
        [scriptblock]$PathExistsProbe = {
            param([string]$Path)
            Test-Path -LiteralPath $Path -PathType Leaf
        },
        [scriptblock]$RemoveAction = {
            param([string]$Path)
            Remove-Item -LiteralPath $Path -Force
        }
    )

    if (-not [bool]$Ownership.BackupCreated) {
        return
    }
    if (-not (& $PathExistsProbe $BackupPath)) {
        throw "The transaction-owned broker backup is unavailable"
    }
    & $RemoveAction $BackupPath
    $Ownership.BackupCreated = $false
}
