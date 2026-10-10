[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$PreSignedBundleV1Path,
    [Parameter(Mandatory = $true)]
    [string]$PreSignedBundleV2Path,
    [Parameter(Mandatory = $true)]
    [string]$ExpectedPublisher,
    [Parameter(Mandatory = $true)]
    [string]$AcceptanceWorkspace,
    [Parameter(Mandatory = $true)]
    [string]$DisposableRoot,
    [Parameter(Mandatory = $true)]
    [string]$ExternalSiblingRoot,
    [Parameter(Mandatory = $true)]
    [string]$AuthorizationToken,
    [switch]$AcknowledgeDisposableSystemChanges,
    [switch]$ValidationOnly
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")
. (Join-Path $PSScriptRoot "acceptance_windows_journal_broker_common.ps1")

function Invoke-AmeBrokerLimitedAcceptanceClient {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ClientBinaryPath,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher,
        [Parameter(Mandatory = $true)]
        [string]$DisposableRoot,
        [Parameter(Mandatory = $true)]
        [string]$InternalMarker,
        [Parameter(Mandatory = $true)]
        [string]$ExternalMarker,
        [Parameter(Mandatory = $true)]
        [string]$AuthorizationToken,
        [Parameter(Mandatory = $true)]
        [ValidatePattern('\A(first|second)\z')]
        [string]$Phase
    )

    $nonce = New-AmeBrokerAcceptanceRandomHex -ByteCount 32
    $instance = New-AmeBrokerAcceptanceRandomHex -ByteCount 16
    $pipeName = "CedarflakeAme.Acceptance.$instance"
    $taskName = "CedarflakeAmeBrokerAcceptance-$PID-$Phase-$($instance.Substring(0, 8))"
    $userSid = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $pipe = New-AmeBrokerLimitedResultServer `
        -PipeName $pipeName `
        -UserSid $userSid
    $expectedClientPath = [IO.Path]::GetFullPath($ClientBinaryPath)
    $expectedClientHash = (Get-FileHash `
        -LiteralPath $expectedClientPath `
        -Algorithm SHA256).Hash.ToUpperInvariant()
    $registered = $false
    $primaryError = $null
    $result = $null
    $startedAfterUtc = [DateTime]::UtcNow.AddSeconds(-1)
    if (Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue) {
        throw "The scoped limited-client scheduled task already exists"
    }
    try {
        foreach ($value in @(
            $pipeName,
            $nonce,
            $Phase,
            $instance,
            $DisposableRoot,
            $InternalMarker,
            $ExternalMarker,
            $AuthorizationToken
        )) {
            if ([string]$value -match '["\r\n]') {
                throw "The limited-client task argument contains an unsafe character"
            }
        }
        $arguments = @(
            "--pipe", $pipeName,
            "--nonce", $nonce,
            "--phase", $Phase,
            "--instance", $instance,
            "--root", $DisposableRoot,
            "--internal-marker", $InternalMarker,
            "--external-marker", $ExternalMarker,
            "--authorization-token", $AuthorizationToken
        ).ForEach({ '"{0}"' -f $_ }) -join " "
        $action = New-ScheduledTaskAction `
            -Execute $expectedClientPath `
            -Argument $arguments `
            -WorkingDirectory ([IO.Path]::GetDirectoryName($expectedClientPath))
        $principal = New-ScheduledTaskPrincipal `
            -UserId $userSid.Value `
            -LogonType Interactive `
            -RunLevel Limited
        $settings = New-ScheduledTaskSettingsSet `
            -ExecutionTimeLimit ([TimeSpan]::FromMinutes(5)) `
            -AllowStartIfOnBatteries `
            -DontStopIfGoingOnBatteries
        Register-ScheduledTask `
            -TaskName $taskName `
            -Action $action `
            -Principal $principal `
            -Settings $settings | Out-Null
        $registered = $true
        $connection = $pipe.BeginWaitForConnection($null, $null)
        Start-ScheduledTask -TaskName $taskName
        try {
            if (-not $connection.AsyncWaitHandle.WaitOne([TimeSpan]::FromMinutes(3))) {
                throw "Limited installed-broker result-pipe connection timed out"
            }
            $pipe.EndWaitForConnection($connection)
        } finally {
            $connection.AsyncWaitHandle.Dispose()
        }
        $clientFacts = [Ame.JournalBrokerAcceptance.NativeMethods]::GetClientProcessFacts(
            $pipe.SafePipeHandle.DangerousGetHandle()
        )
        $actualClientHash = (Get-FileHash `
            -LiteralPath $clientFacts.ImagePath `
            -Algorithm SHA256).Hash.ToUpperInvariant()
        Assert-AmeBrokerLimitedClientFacts `
            -ActualPath $clientFacts.ImagePath `
            -ExpectedPath $expectedClientPath `
            -IsElevated $clientFacts.IsElevated `
            -ActualSha256 $actualClientHash `
            -ExpectedSha256 $expectedClientHash `
            -CreationTimeUtcFileTime $clientFacts.CreationTimeUtcFileTime `
            -StartedAfterUtcFileTime $startedAfterUtc.ToFileTimeUtc()
        Assert-AmeBrokerAcceptanceClientBinary `
            -Path $clientFacts.ImagePath `
            -ExpectedPublisher $ExpectedPublisher | Out-Null
        $record = Read-AmeBrokerLimitedResultRecord -Pipe $pipe
        $result = Assert-AmeBrokerLimitedResultFacts `
            -Record $record `
            -ExpectedNonce $nonce `
            -ExpectedPhase $Phase `
            -ExpectedInstance $instance `
            -ExpectedProcessId $clientFacts.ProcessId
        $pipe.Write([byte[]](0xA5), 0, 1)
        $pipe.Flush()
        $deadline = [DateTime]::UtcNow.AddSeconds(30)
        while ((Get-ScheduledTask -TaskName $taskName).State -eq "Running") {
            if ([DateTime]::UtcNow -ge $deadline) {
                throw "Limited installed-broker task did not exit after result acknowledgement"
            }
            Start-Sleep -Milliseconds 50
        }
        $taskInfo = Get-ScheduledTaskInfo -TaskName $taskName
        Assert-AmeBrokerTaskCompletionFacts `
            -LastTaskResult ([uint32]$taskInfo.LastTaskResult) `
            -LastRunTimeUtc $taskInfo.LastRunTime.ToUniversalTime() `
            -StartedAfterUtc $startedAfterUtc
        if ($result.Status -cne "passed") {
            throw "Limited installed-broker acceptance client failed: $($result.Payload)"
        }
    } catch {
        $primaryError = $_
    } finally {
        $cleanupFailures = [System.Collections.Generic.List[string]]::new()
        try {
            $pipe.Dispose()
        } catch {
            $cleanupFailures.Add("dispose protected result pipe: $($_.Exception.Message)")
        }
        if ($registered) {
            try {
                $task = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
                if ($null -ne $task -and $task.State -eq "Running") {
                    Stop-ScheduledTask -TaskName $taskName
                }
            } catch {
                $cleanupFailures.Add("stop limited task: $($_.Exception.Message)")
            }
            try {
                Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
            } catch {
                $cleanupFailures.Add("unregister limited task: $($_.Exception.Message)")
            }
            try {
                if (Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue) {
                    throw "the task still exists"
                }
            } catch {
                $cleanupFailures.Add("confirm limited task removal: $($_.Exception.Message)")
            }
        }
        if ($cleanupFailures.Count -gt 0) {
            if ($null -eq $primaryError) {
                throw ($cleanupFailures -join "; ")
            }
            throw "$($primaryError.Exception.Message); cleanup failures: " +
                ($cleanupFailures -join "; ")
        }
    }
    if ($null -ne $primaryError) {
        throw $primaryError
    }
    return $result.Payload
}

function Start-AmeBrokerServiceBounded {
    Invoke-AmeBrokerSc `
        -Arguments @("start", (Get-AmeBrokerServicePlan).ServiceName) | Out-Null
    $service = Get-Service -Name (Get-AmeBrokerServicePlan).ServiceName
    try {
        $service.WaitForStatus(
            [System.ServiceProcess.ServiceControllerStatus]::Running,
            [TimeSpan]::FromSeconds(30)
        )
    } finally {
        $service.Dispose()
    }
}

function Remove-AmeBrokerCreatedDirectory {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedParent,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedParentFinal
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        return
    }
    $resolved = (Get-Item -LiteralPath $Path -Force).FullName
    if (-not (Split-Path -Parent $resolved).Equals(
        $ExpectedParent,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Refusing to remove a disposable directory outside the acceptance workspace"
    }
    $item = Get-Item -LiteralPath $resolved -Force
    if ($item.Attributes.HasFlag([System.IO.FileAttributes]::ReparsePoint)) {
        throw "Refusing to recursively remove a disposable directory that became a reparse point"
    }
    $expectedRootFinal = Join-Path $ExpectedParentFinal (Split-Path -Leaf $resolved)
    $rootFinal = [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath($resolved)
    if (-not $rootFinal.Equals(
        $expectedRootFinal,
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Refusing to remove a disposable directory whose pinned workspace identity changed"
    }
    $rootFinalPrefix = "$($rootFinal.TrimEnd([IO.Path]::DirectorySeparatorChar))$([IO.Path]::DirectorySeparatorChar)"
    $entries = @(Get-AmeBrokerNoFollowEntries -RootPath $resolved)
    foreach ($entry in @($entries | Sort-Object { $_.FullName.Length } -Descending)) {
        $entryFinal = [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath($entry.FullName)
        if (-not $entryFinal.StartsWith(
            $rootFinalPrefix,
            [StringComparison]::OrdinalIgnoreCase
        )) {
            throw "Refusing to remove a disposable entry outside the pinned fixture root"
        }
        [Ame.JournalBrokerAcceptance.NativeMethods]::DeletePhysicalEntry(
            [string]$entry.FullName,
            $entryFinal,
            [bool]$entry.PSIsContainer
        )
    }
    [Ame.JournalBrokerAcceptance.NativeMethods]::DeletePhysicalEntry(
        $resolved,
        $rootFinal,
        $true
    )
}

Assert-AmeBrokerAcceptanceToken -AuthorizationToken $AuthorizationToken
$paths = Assert-AmeBrokerDisposablePathFacts `
    -AcceptanceWorkspace $AcceptanceWorkspace `
    -DisposableRoot $DisposableRoot `
    -ExternalSiblingRoot $ExternalSiblingRoot
$signedBundleV1 = Assert-AmeBrokerPretrustedAcceptanceBundle `
    -BundlePath $PreSignedBundleV1Path `
    -ExpectedPublisher $ExpectedPublisher
$signedBundleV2 = Assert-AmeBrokerPretrustedAcceptanceBundle `
    -BundlePath $PreSignedBundleV2Path `
    -ExpectedPublisher $ExpectedPublisher
Assert-AmeBrokerAcceptanceBundlePairFacts `
    -Version1 $signedBundleV1 `
    -Version2 $signedBundleV2
$installerGuardrailOutput = @(
    & "$PSScriptRoot\release_test_journal_broker_installer_guardrails.ps1"
)
if (@($installerGuardrailOutput | Where-Object {
    [string]$_ -ceq "journal_broker_installer_guardrails_passed"
}).Count -ne 1) {
    throw "The installer repair, upgrade, and rollback guardrail did not complete exactly once"
}
$integrationOutput = @(& "$PSScriptRoot\integration_test_windows_journal_broker.ps1")
if (@($integrationOutput | Where-Object {
    [string]$_ -ceq "windows_journal_broker_integration_passed"
}).Count -ne 1) {
    throw "The bounded broker integration matrix did not complete exactly once"
}

if ($ValidationOnly) {
    Write-Output "AME_BROKER_ACCEPTANCE_VALIDATION status=passed"
    return
}
Assert-AmeBrokerPlatform
if (-not $AcknowledgeDisposableSystemChanges) {
    throw "Disposable SCM installation and rollback acknowledgement is required"
}
Assert-AmeBrokerAdministrator
Get-AmeBrokerAcceptanceAncestorFacts -Path $paths.Workspace | Out-Null
if (Test-AmeBrokerServiceExists) {
    throw "The fixed journal broker service must be absent before disposable acceptance"
}
if (Test-Path -LiteralPath (Get-AmeApplicationInstallDirectory)) {
    throw "The fixed Ame Application directory must be absent before disposable acceptance"
}
$acceptanceParentPrestate = Get-AmeBrokerParentPrestate

$firstInternalMarker = "ame-broker-internal-first-$PID.jpg"
$firstExternalMarker = "ame-broker-external-first-$PID.jpg"
$secondInternalMarker = "ame-broker-internal-second-$PID.jpg"
$secondExternalMarker = "ame-broker-external-second-$PID.jpg"
$createdRoot = $false
$createdExternal = $false
$installAttempted = $false
$installedByHarness = $false
$installedApplicationByHarness = $false
$fixtureBefore = $null
$externalBefore = $null
$acceptanceError = $null
$cleanupFailures = [System.Collections.Generic.List[string]]::new()
$workspaceAclPrestate = Get-AmeBrokerAcceptanceWorkspaceAclPrestate `
    -Path $paths.Workspace `
    -UserSid ([Security.Principal.WindowsIdentity]::GetCurrent().User)

try {
    Set-AmeBrokerAcceptanceWorkspaceAcl -Prestate $workspaceAclPrestate
    New-Item -ItemType Directory -Path $paths.Root | Out-Null
    $createdRoot = $true
    New-Item -ItemType Directory -Path $paths.External | Out-Null
    $createdExternal = $true
    [System.IO.File]::WriteAllBytes(
        (Join-Path $paths.Root "source-sentinel.jpg"),
        [byte[]](0x41, 0x4d, 0x45, 0x2d, 0x53, 0x41, 0x46, 0x45)
    )
    $installAttempted = $true
    & "$PSScriptRoot\release_install_journal_broker.ps1" `
        -BrokerBinaryPath $signedBundleV1.BrokerBinaryPath `
        -ApplicationBundlePath $signedBundleV1.ApplicationBundlePath `
        -ExpectedPublisher $ExpectedPublisher `
        -Confirm:$false | Out-Null
    $installedByHarness = $true
    $installedApplicationByHarness = $true
    $installedClientBinary = Get-AmeApplicationInstalledBinaryPath
    $installedBrokerBinary = Join-Path `
        (Get-AmeBrokerInstallDirectory) `
        (Get-AmeBrokerServicePlan).BinaryName
    Start-AmeBrokerServiceBounded
    Assert-AmeBrokerInstallation `
        -BinaryPath $installedBrokerBinary `
        -ClientBinaryPath $installedClientBinary `
        -ExpectedPublisher $ExpectedPublisher

    [System.IO.File]::WriteAllBytes(
        (Join-Path $paths.Root $firstInternalMarker),
        [byte[]](0x49, 0x4e, 0x54, 0x45, 0x52, 0x4e, 0x41, 0x4c)
    )
    [System.IO.File]::WriteAllBytes(
        (Join-Path $paths.External $firstExternalMarker),
        [byte[]](0x45, 0x58, 0x54, 0x45, 0x52, 0x4e, 0x41, 0x4c)
    )
    $fixtureBefore = Get-AmeBrokerFixtureState -Path $paths.Root
    $externalBefore = Get-AmeBrokerFixtureState -Path $paths.External

    $firstClient = Invoke-AmeBrokerLimitedAcceptanceClient `
        -ClientBinaryPath $installedClientBinary `
        -ExpectedPublisher $ExpectedPublisher `
        -DisposableRoot $paths.Root `
        -InternalMarker $firstInternalMarker `
        -ExternalMarker $firstExternalMarker `
        -AuthorizationToken $AuthorizationToken `
        -Phase first
    if ((Get-AmeBrokerFixtureState -Path $paths.Root) -cne $fixtureBefore -or
        (Get-AmeBrokerFixtureState -Path $paths.External) -cne $externalBefore) {
        throw "Installed broker acceptance changed first-phase source metadata or content"
    }

    Stop-AmeBrokerServiceBounded
    Assert-AmeBrokerPhysicalFile -ExpectedPath $installedBrokerBinary
    Remove-AmeBrokerOwnedLeafNoFollow `
        -Path $installedBrokerBinary `
        -AllowedPaths @($installedBrokerBinary)
    if (Test-Path -LiteralPath $installedBrokerBinary) {
        throw "The disposable missing-binary repair fixture was not established"
    }
    & "$PSScriptRoot\release_repair_journal_broker.ps1" `
        -BrokerBinaryPath $signedBundleV1.BrokerBinaryPath `
        -ExpectedPublisher $ExpectedPublisher `
        -Confirm:$false | Out-Null
    Start-AmeBrokerServiceBounded
    Assert-AmeBrokerInstallation `
        -BinaryPath $installedBrokerBinary `
        -ClientBinaryPath $installedClientBinary `
        -ExpectedPublisher $ExpectedPublisher
    $version1InstalledHash = (
        Get-FileHash -LiteralPath $installedBrokerBinary -Algorithm SHA256
    ).Hash.ToUpperInvariant()
    $version1ClientHash = (
        Get-FileHash -LiteralPath $installedClientBinary -Algorithm SHA256
    ).Hash.ToUpperInvariant()
    $version1Manifest = Get-AmeBrokerConfiguredIdentityManifest
    $version1Protocol = Get-AmeBrokerProtocolFacts -Path $installedBrokerBinary
    if ($version1InstalledHash -cne [string]$signedBundleV1.BrokerSha256 -or
        $version1ClientHash -cne [string]$signedBundleV1.ClientSha256 -or
        [int]$version1Protocol.Protocol -ne [int]$signedBundleV1.Protocol -or
        [int]$version1Protocol.MaximumFrameBytes -ne
            [int]$signedBundleV1.MaximumFrameBytes) {
        throw "The repaired V1 installation does not match its signed bundle identity"
    }
    & "$PSScriptRoot\release_upgrade_journal_broker.ps1" `
        -BrokerBinaryPath $signedBundleV2.BrokerBinaryPath `
        -ExpectedPublisher $ExpectedPublisher `
        -Confirm:$false | Out-Null
    Assert-AmeBrokerInstallation `
        -BinaryPath $installedBrokerBinary `
        -ClientBinaryPath $installedClientBinary `
        -ExpectedPublisher $ExpectedPublisher
    $version2InstalledHash = (
        Get-FileHash -LiteralPath $installedBrokerBinary -Algorithm SHA256
    ).Hash.ToUpperInvariant()
    $version2ClientHash = (
        Get-FileHash -LiteralPath $installedClientBinary -Algorithm SHA256
    ).Hash.ToUpperInvariant()
    $version2Manifest = Get-AmeBrokerConfiguredIdentityManifest
    $version2Protocol = Get-AmeBrokerProtocolFacts -Path $installedBrokerBinary
    if ($version2InstalledHash -cne [string]$signedBundleV2.BrokerSha256 -or
        $version2InstalledHash -ceq $version1InstalledHash -or
        $version2ClientHash -cne [string]$signedBundleV2.ClientSha256 -or
        $version2Manifest -ceq $version1Manifest -or
        [int]$version2Protocol.Protocol -ne [int]$signedBundleV2.Protocol -or
        [int]$version2Protocol.MaximumFrameBytes -ne
            [int]$signedBundleV2.MaximumFrameBytes) {
        throw "The V2 upgrade did not publish its distinct signed broker and manifest identity"
    }

    [System.IO.File]::WriteAllBytes(
        (Join-Path $paths.Root $secondInternalMarker),
        [byte[]](0x53, 0x45, 0x43, 0x4f, 0x4e, 0x44, 0x2d, 0x49)
    )
    [System.IO.File]::WriteAllBytes(
        (Join-Path $paths.External $secondExternalMarker),
        [byte[]](0x53, 0x45, 0x43, 0x4f, 0x4e, 0x44, 0x2d, 0x45)
    )
    $fixtureBefore = Get-AmeBrokerFixtureState -Path $paths.Root
    $externalBefore = Get-AmeBrokerFixtureState -Path $paths.External
    Stop-AmeBrokerServiceBounded
    Start-AmeBrokerServiceBounded
    $secondClient = Invoke-AmeBrokerLimitedAcceptanceClient `
        -ClientBinaryPath $installedClientBinary `
        -ExpectedPublisher $ExpectedPublisher `
        -DisposableRoot $paths.Root `
        -InternalMarker $secondInternalMarker `
        -ExternalMarker $secondExternalMarker `
        -AuthorizationToken $AuthorizationToken `
        -Phase second
    $firstIdentity = [regex]::Match(
        $firstClient,
        "identity=([0-9a-f]{32}) generation=([0-9]+) nonce=([0-9]+)"
    )
    $secondIdentity = [regex]::Match(
        $secondClient,
        "identity=([0-9a-f]{32}) generation=([0-9]+) nonce=([0-9]+)"
    )
    if (-not $firstIdentity.Success -or -not $secondIdentity.Success -or
        $firstIdentity.Groups[1].Value -ceq $secondIdentity.Groups[1].Value -or
        $firstIdentity.Groups[3].Value -ceq $secondIdentity.Groups[3].Value) {
        throw "Broker restart did not produce fresh server connection identity and nonce"
    }
    if ((Get-AmeBrokerFixtureState -Path $paths.Root) -cne $fixtureBefore -or
        (Get-AmeBrokerFixtureState -Path $paths.External) -cne $externalBefore) {
        throw "Installed broker acceptance changed restart-phase source metadata or content"
    }
    if ($firstClient.IndexOf("cancel=", [System.StringComparison]::Ordinal) -lt 0 -or
        $secondClient.IndexOf("cancel=", [System.StringComparison]::Ordinal) -lt 0) {
        throw "Installed broker cancellation evidence is missing"
    }
    Write-Output "AME_BROKER_SCM_ACCEPTANCE status=passed"
} catch {
    $acceptanceError = $_
} finally {
    $servicePresent = $false
    try {
        $servicePresent = Test-AmeBrokerServiceExists
    } catch {
        $cleanupFailures.Add("query broker service for cleanup: $($_.Exception.Message)")
    }
    if ($installAttempted -and $servicePresent) {
        try {
            & "$PSScriptRoot\release_stop_journal_broker.ps1" -Confirm:$false | Out-Null
        } catch {
            $cleanupFailures.Add("stop installed broker: $($_.Exception.Message)")
        }
        try {
            & "$PSScriptRoot\release_uninstall_journal_broker.ps1" `
                -ExpectedPublisher $ExpectedPublisher `
                -Confirm:$false | Out-Null
        } catch {
            $cleanupFailures.Add("uninstall installed broker: $($_.Exception.Message)")
        }
    }
    try {
        $servicePresent = Test-AmeBrokerServiceExists
        if ($servicePresent) {
            throw "Disposable journal broker service cleanup was incomplete"
        }
        $installedByHarness = $false
    } catch {
        $cleanupFailures.Add("confirm broker service removal: $($_.Exception.Message)")
    }
    $installedApplicationDirectory = Get-AmeApplicationInstallDirectory
    if (
        $installAttempted -and
        -not $servicePresent -and
        (Test-Path -LiteralPath $installedApplicationDirectory -PathType Container)
    ) {
        try {
            Assert-AmeApplicationInstallation `
                -InstallDirectory $installedApplicationDirectory `
                -ExpectedPublisher $ExpectedPublisher | Out-Null
            Remove-AmeBrokerOwnedTreeNoFollow `
                -Path $installedApplicationDirectory `
                -ExpectedParent (Get-AmeProductParentDirectory)
        } catch {
            $cleanupFailures.Add("remove installed acceptance Application: $($_.Exception.Message)")
        }
    }
    try {
        if (
            $installedApplicationByHarness -and
            (Test-Path -LiteralPath $installedApplicationDirectory)
        ) {
            throw "The installed acceptance Application directory still exists"
        }
        $installedApplicationByHarness = $false
    } catch {
        $cleanupFailures.Add("confirm acceptance Application removal: $($_.Exception.Message)")
    }
    if (-not $servicePresent) {
        try {
            Restore-AmeBrokerParentPrestate -Prestate $acceptanceParentPrestate
            $productParent = Get-AmeProductParentDirectory
            if ([bool]$acceptanceParentPrestate.Existed) {
                if (-not (Test-Path -LiteralPath $productParent -PathType Container) -or
                    -not ([string](Get-Acl -LiteralPath $productParent).Sddl).Equals(
                        [string]$acceptanceParentPrestate.Sddl,
                        [StringComparison]::Ordinal
                    )) {
                    throw "The pre-existing Ame parent was not restored exactly"
                }
            } elseif (Test-Path -LiteralPath $productParent) {
                throw "The acceptance-created Ame parent still exists"
            }
        } catch {
            $cleanupFailures.Add("restore Ame parent prestate: $($_.Exception.Message)")
        }
    }
    if ($createdExternal) {
        try {
            Remove-AmeBrokerCreatedDirectory `
                -Path $paths.External `
                -ExpectedParent $paths.Workspace `
                -ExpectedParentFinal ([string]$workspaceAclPrestate.FinalPath)
        } catch {
            $cleanupFailures.Add("remove external fixture: $($_.Exception.Message)")
        }
    }
    if ($createdRoot) {
        try {
            Remove-AmeBrokerCreatedDirectory `
                -Path $paths.Root `
                -ExpectedParent $paths.Workspace `
                -ExpectedParentFinal ([string]$workspaceAclPrestate.FinalPath)
        } catch {
            $cleanupFailures.Add("remove disposable root fixture: $($_.Exception.Message)")
        }
    }
    try {
        if (@(Get-ChildItem -LiteralPath $paths.Workspace -Force).Count -ne 0) {
            throw "The disposable acceptance workspace is not empty after cleanup"
        }
    } catch {
        $cleanupFailures.Add("confirm empty acceptance workspace: $($_.Exception.Message)")
    }
    try {
        Restore-AmeBrokerAcceptanceWorkspaceAcl -Prestate $workspaceAclPrestate
    } catch {
        $cleanupFailures.Add("restore acceptance workspace ACL: $($_.Exception.Message)")
    }
    try {
        Close-AmeBrokerAcceptanceWorkspacePins -Prestate $workspaceAclPrestate
    } catch {
        $cleanupFailures.Add("close acceptance workspace pins: $($_.Exception.Message)")
    }
}

if ($cleanupFailures.Count -gt 0) {
    $cleanupMessage = $cleanupFailures -join "; "
    if ($null -ne $acceptanceError) {
        throw "$($acceptanceError.Exception.Message); cleanup failures: $cleanupMessage"
    }
    throw $cleanupMessage
}
if ($null -ne $acceptanceError) {
    throw $acceptanceError
}
